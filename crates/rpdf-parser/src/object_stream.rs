//! PDF 1.5+ 객체 스트림(`/Type /ObjStm`) 파싱.
//!
//! ObjStm은 여러 간접 객체를 하나의 압축 스트림 안에 묶어 저장한다(ISO 32000 §7.5.7).
//! `parse_object_stream`은 스트림을 디코딩해 `ParsedObjectStream`으로 반환하고,
//! `ParsedObjectStream::get`은 객체 번호로 개별 객체를 조회한다.

use rpdf_core::types::PdfObject;

use crate::ParseError;
use crate::objects::{parse_indirect_object, parse_u64_val, skip_whitespace_and_comments};
use crate::xref_stream::decompress_flate;

/// ObjStm 파싱 결과. 객체 번호 → `PdfObject` 매핑.
///
/// `objects`는 스트림 헤더에 선언된 순서대로 `(obj_num, PdfObject)` 쌍을 보관한다.
#[derive(Debug, Clone)]
pub struct ParsedObjectStream {
    /// ObjStm이 포함하는 객체 목록 `(obj_num, object)`.
    pub objects: Vec<(u32, PdfObject)>,
}

impl ParsedObjectStream {
    /// `obj_num`에 해당하는 `PdfObject`를 반환한다.
    ///
    /// 존재하지 않으면 `None`. `XrefTable::get()`과 일관된 시그니처.
    ///
    /// **ObjStmObjNumMismatch 정책**: xref 번호와 헤더 번호가 다를 때 xref 우선 +
    /// `tracing::warn` 경고. `ObjStmObjNumMismatch` 에러 변형은 미발생이며
    /// 향후 strict 모드 옵션 도입 시 활용 예약.
    pub fn get(&self, obj_num: u32) -> Option<&PdfObject> {
        self.objects
            .iter()
            .find(|(num, _)| *num == obj_num)
            .map(|(_, obj)| obj)
    }
}

/// ObjStm 간접 객체를 파싱해 객체 목록을 반환한다.
///
/// `offset`은 xref table에서 읽은 ObjStm 객체의 파일 오프셋.
/// 반환된 `ParsedObjectStream.objects`는 `(obj_num, PdfObject)` 쌍 벡터.
///
/// ISO 32000 §7.5.7
#[allow(dead_code)] // Checkpoint C에서 호출됨
pub(crate) fn parse_object_stream(
    data: &[u8],
    offset: u64,
) -> Result<ParsedObjectStream, ParseError> {
    // a) ObjStm 간접 객체 파싱
    let (indirect, _) =
        parse_indirect_object(data, offset as usize).map_err(|e| ParseError::MalformedObjStm {
            offset,
            reason: e.to_string(),
        })?;

    // b) Stream인지 확인
    let stream = match indirect.object {
        PdfObject::Stream(s) => s,
        _ => {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: "not a stream".to_string(),
            });
        }
    };

    // b) /Type /ObjStm 확인
    match stream.dict.get(b"Type") {
        Some(PdfObject::Name(n)) if n.as_slice() == b"ObjStm" => {}
        _ => {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: "not /Type /ObjStm".to_string(),
            });
        }
    }

    // c) /Extends — v0.1 범위 외, 명시적 거부
    if stream.dict.get(b"Extends").is_some() {
        return Err(ParseError::ObjStmExtendsUnsupported { offset });
    }

    // c) /N — 필수 비음수 정수
    let n = match stream.dict.get(b"N") {
        Some(PdfObject::Integer(n)) if *n >= 0 => *n as u32,
        _ => {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: "/N missing or not a non-negative integer".to_string(),
            });
        }
    };

    // c) /First — 필수 비음수 정수
    let first = match stream.dict.get(b"First") {
        Some(PdfObject::Integer(f)) if *f >= 0 => *f as usize,
        _ => {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: "/First missing or not a non-negative integer".to_string(),
            });
        }
    };

    // c) /Filter — FlateDecode 또는 없음만 허용 (ISO 32000 §7.5.7)
    let use_flate = match stream.dict.get(b"Filter") {
        Some(PdfObject::Name(name)) if name.as_slice() == b"FlateDecode" => true,
        Some(PdfObject::Name(name)) => {
            return Err(ParseError::InvalidObjStmFilter {
                offset,
                filter: String::from_utf8_lossy(name).into_owned(),
            });
        }
        None => false,
        _ => {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: "/Filter value is not a Name".to_string(),
            });
        }
    };

    // d) 압축 해제
    let decompressed = if use_flate {
        decompress_flate(&stream.data, offset).map_err(|e| ParseError::MalformedObjStm {
            offset,
            reason: format!("FlateDecode 압축 해제 실패: {e}"),
        })?
    } else {
        stream.data
    };

    // e) 헤더 파싱: data[0..first]에서 N개 (obj_num, rel_offset) 쌍 추출
    let _header_pairs = parse_objstm_header(&decompressed, n, first, offset)?;

    // f) Placeholder — Checkpoint C에서 header_pairs로 본문 객체 추출
    Ok(ParsedObjectStream { objects: vec![] })
}

/// ObjStm 헤더(`data[0..first]` 영역)에서 `n`개의 `(obj_num, rel_offset)` 쌍을 추출한다.
///
/// 정수 구분자는 화이트스페이스(ISO 32000 §7.2.3). `%` 주석도 건너뛴다.
/// `first > data.len()`이면 `MalformedObjStm`을 반환한다.
fn parse_objstm_header(
    data: &[u8],
    n: u32,
    first: usize,
    offset: u64,
) -> Result<Vec<(u32, u64)>, ParseError> {
    if first > data.len() {
        return Err(ParseError::MalformedObjStm {
            offset,
            reason: format!(
                "/First ({first}) exceeds decompressed data length ({})",
                data.len()
            ),
        });
    }

    let header_data = &data[..first];
    let mut pos = skip_whitespace_and_comments(header_data, 0);
    let mut pairs = Vec::with_capacity(n as usize);

    for i in 0..(n as usize) {
        // obj_num 파싱
        let Some((obj_num_u64, consumed)) = parse_u64_val(&header_data[pos..]) else {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: format!("헤더 쌍 {i}: obj_num 파싱 실패 (위치 {pos})"),
            });
        };
        pos += consumed;
        pos = skip_whitespace_and_comments(header_data, pos);

        // rel_offset 파싱
        let Some((rel_offset, consumed)) = parse_u64_val(&header_data[pos..]) else {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: format!("헤더 쌍 {i}: rel_offset 파싱 실패 (위치 {pos})"),
            });
        };
        pos += consumed;
        pos = skip_whitespace_and_comments(header_data, pos);

        pairs.push((obj_num_u64 as u32, rel_offset));
    }

    Ok(pairs)
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    // ── 테스트 헬퍼 ─────────────────────────────────────────────────────────────

    /// ObjStm 형식의 간접 객체 바이트를 만든다 (B/C/D 재사용).
    ///
    /// `body`는 스트림 본문 (압축/비압축). /Length는 body.len()으로 자동 계산.
    fn make_objstm_indirect_object(
        obj_num: u32,
        n: u32,
        first: usize,
        filter: Option<&str>,
        extends: Option<&str>,
        body: &[u8],
    ) -> Vec<u8> {
        let mut dict_str = format!("/Type /ObjStm /N {n} /First {first} /Length {}", body.len());
        if let Some(f) = filter {
            dict_str.push_str(&format!(" /Filter /{f}"));
        }
        if let Some(ext) = extends {
            dict_str.push_str(&format!(" /Extends {ext}"));
        }
        let mut out = format!("{obj_num} 0 obj\n<< {dict_str} >>\nstream\n").into_bytes();
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendstream\nendobj");
        out
    }

    /// `plain` 데이터를 zlib (FlateDecode) 형식으로 압축한다 (B/C/D 재사용).
    #[allow(dead_code)] // Checkpoint C/D에서 사용됨
    fn make_zlib_compressed(plain: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(plain).unwrap();
        enc.finish().unwrap()
    }

    // ── B 단위 테스트 ────────────────────────────────────────────────────────────

    // 1. parse_objstm_header 직접 테스트: N=3 쌍 정확히 추출
    #[test]
    fn parse_objstm_header_extracts_n3_pairs() {
        // "3 0 17 9 25 18\n" = 15 bytes → First=15
        let data = b"3 0 17 9 25 18\n<< /Type /Catalog >>";
        let pairs = parse_objstm_header(data, 3, 15, 0).unwrap();
        assert_eq!(pairs, vec![(3, 0), (17, 9), (25, 18)]);
    }

    // 2. 스트림 아닌 간접 객체 → MalformedObjStm
    #[test]
    fn rejects_not_a_stream() {
        // Dictionary 객체 (stream 키워드 없음)
        let data = b"12 0 obj\n<< /Type /ObjStm >>\nendobj";
        let err = parse_object_stream(data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // 3. /Type /ObjStm 아님 → MalformedObjStm
    #[test]
    fn rejects_wrong_type() {
        // /Type /Catalog 스트림
        let data = b"12 0 obj\n<< /Type /Catalog /Length 0 >>\nstream\n\nendstream\nendobj";
        let err = parse_object_stream(data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // 4. /N 없음 → MalformedObjStm
    #[test]
    fn rejects_missing_n() {
        let data = b"12 0 obj\n<< /Type /ObjStm /First 0 /Length 0 >>\nstream\n\nendstream\nendobj";
        let err = parse_object_stream(data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // 5. /First 없음 → MalformedObjStm
    #[test]
    fn rejects_missing_first() {
        let data = b"12 0 obj\n<< /Type /ObjStm /N 0 /Length 0 >>\nstream\n\nendstream\nendobj";
        let err = parse_object_stream(data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // 6. /Extends 존재 → ObjStmExtendsUnsupported
    #[test]
    fn rejects_extends() {
        let data = make_objstm_indirect_object(12, 0, 0, None, Some("5 0 R"), b"");
        let err = parse_object_stream(&data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::ObjStmExtendsUnsupported { .. }),
            "expected ObjStmExtendsUnsupported, got {err:?}"
        );
    }

    // 7. /Filter /LZWDecode → InvalidObjStmFilter
    #[test]
    fn rejects_unsupported_filter() {
        let data = make_objstm_indirect_object(12, 0, 0, Some("LZWDecode"), None, b"");
        let err = parse_object_stream(&data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::InvalidObjStmFilter { .. }),
            "expected InvalidObjStmFilter, got {err:?}"
        );
    }

    // 8. /Filter 없음, 비압축 ObjStm → 헤더 파싱 성공
    #[test]
    fn accepts_uncompressed_objstm() {
        // N=1, First=4: header="5 0\n", body_section="true"
        let body = b"5 0\ntrue";
        let data = make_objstm_indirect_object(12, 1, 4, None, None, body);
        let result = parse_object_stream(&data, 0);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap().objects.len(), 0); // C에서 객체 추출 전
    }

    // 9. FlateDecode 압축 ObjStm → 헤더 파싱 성공
    #[test]
    fn accepts_flatedecode_objstm() {
        // N=1, First=4: header="5 0\n", body_section="true"
        let plain = b"5 0\ntrue";
        let compressed = make_zlib_compressed(plain);
        let data = make_objstm_indirect_object(12, 1, 4, Some("FlateDecode"), None, &compressed);
        let result = parse_object_stream(&data, 0);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    // 10. /First가 압축 해제 데이터 길이 초과 → MalformedObjStm
    #[test]
    fn rejects_first_exceeds_data_length() {
        // body 3바이트인데 First=100으로 설정
        let body = b"abc";
        let data = make_objstm_indirect_object(12, 1, 100, None, None, body);
        let err = parse_object_stream(&data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // 11. N=0 빈 ObjStm → 빈 ParsedObjectStream 반환
    #[test]
    fn accepts_empty_objstm_n0() {
        let data = make_objstm_indirect_object(12, 0, 0, None, None, b"");
        let result = parse_object_stream(&data, 0).unwrap();
        assert!(result.objects.is_empty());
    }
}
