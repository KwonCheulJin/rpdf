//! PDF 1.5+ 객체 스트림(`/Type /ObjStm`) 파싱.
//!
//! ObjStm은 여러 간접 객체를 하나의 압축 스트림 안에 묶어 저장한다(ISO 32000 §7.5.7).
//! `parse_object_stream`은 스트림을 디코딩해 `ParsedObjectStream`으로 반환하고,
//! `ParsedObjectStream::get`은 객체 번호로 개별 객체를 조회한다.

use rpdf_core::types::PdfObject;

use crate::ParseError;
use crate::objects::{
    parse_indirect_object, parse_object, parse_u64_val, skip_whitespace_and_comments,
};
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
    /// 동일 `obj_num`이 여러 번 등장하면 **첫 번째** 항목을 반환한다.
    /// (ISO 32000 §7.3.7 "first occurrence" 정책과 일관.)
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
pub fn parse_object_stream(data: &[u8], offset: u64) -> Result<ParsedObjectStream, ParseError> {
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
    let header_pairs = parse_objstm_header(&decompressed, n, first, offset)?;

    // f) 본문 객체 추출: 각 (obj_num, rel_offset)에서 PdfObject 파싱
    let mut objects = Vec::with_capacity(header_pairs.len());
    for (i, (obj_num, rel_offset)) in header_pairs.into_iter().enumerate() {
        let abs_offset = first + rel_offset as usize;
        if abs_offset > decompressed.len() {
            return Err(ParseError::MalformedObjStm {
                offset,
                reason: format!(
                    "객체 {i} (obj#{obj_num}): rel_offset {rel_offset}이 본문 범위 초과 \
                     (first={first}, data_len={})",
                    decompressed.len()
                ),
            });
        }
        let (object, _) =
            parse_object(&decompressed, abs_offset).map_err(|e| ParseError::MalformedObjStm {
                offset,
                reason: format!("객체 {i} (obj#{obj_num}) 파싱 실패: {e}"),
            })?;
        objects.push((obj_num, object));
    }

    Ok(ParsedObjectStream { objects })
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

    /// ObjStm 스트림 본문 바이트를 만든다 (B/C/D 재사용).
    ///
    /// `objects`: `(obj_num, raw_bytes)` 슬라이스. 객체는 바이트 연속 배치.
    /// 반환: `(payload_bytes, first)` — payload를 ObjStm body로 사용.
    fn make_objstm_payload(objects: &[(u32, &[u8])]) -> (Vec<u8>, usize) {
        // rel_offset 계산 (객체 연속 배치)
        let mut current_rel = 0usize;
        let mut rel_offsets = Vec::new();
        for (_, bytes) in objects.iter() {
            rel_offsets.push(current_rel);
            current_rel += bytes.len();
        }

        // 헤더: "obj1 off1 obj2 off2 ...\n"
        let mut hdr = String::new();
        for (i, (obj_num, _)) in objects.iter().enumerate() {
            if i > 0 {
                hdr.push(' ');
            }
            hdr.push_str(&format!("{obj_num} {}", rel_offsets[i]));
        }
        hdr.push('\n');
        let first = hdr.len();

        let mut payload = hdr.into_bytes();
        for (_, bytes) in objects.iter() {
            payload.extend_from_slice(bytes);
        }
        (payload, first)
    }

    /// `plain` 데이터를 zlib (FlateDecode) 형식으로 압축한다 (B/C/D 재사용).
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

    // 8. /Filter 없음, 비압축 ObjStm → 헤더 파싱 + 객체 추출 성공
    #[test]
    fn accepts_uncompressed_objstm() {
        // N=1, First=4: header="5 0\n", body_section="true"
        let body = b"5 0\ntrue";
        let data = make_objstm_indirect_object(12, 1, 4, None, None, body);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.get(5), Some(&PdfObject::Boolean(true)));
    }

    // 9. FlateDecode 압축 ObjStm → 헤더 파싱 + 객체 추출 성공
    #[test]
    fn accepts_flatedecode_objstm() {
        // N=1, First=4: header="5 0\n", body_section="true"
        let plain = b"5 0\ntrue";
        let compressed = make_zlib_compressed(plain);
        let data = make_objstm_indirect_object(12, 1, 4, Some("FlateDecode"), None, &compressed);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.get(5), Some(&PdfObject::Boolean(true)));
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

    // ── C 단위 테스트 ────────────────────────────────────────────────────────────

    // C-1. Dictionary 객체 추출
    #[test]
    fn extracts_dictionary_object() {
        let (payload, first) = make_objstm_payload(&[(5, b"<< /Type /Catalog >>")]);
        let data = make_objstm_indirect_object(12, 1, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert!(matches!(result.get(5), Some(PdfObject::Dictionary(_))));
    }

    // C-2. Integer 객체 추출
    #[test]
    fn extracts_integer_object() {
        let (payload, first) = make_objstm_payload(&[(7, b"42")]);
        let data = make_objstm_indirect_object(12, 1, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.get(7), Some(&PdfObject::Integer(42)));
    }

    // C-3. Boolean 객체 추출
    #[test]
    fn extracts_boolean_object() {
        let (payload, first) = make_objstm_payload(&[(8, b"true")]);
        let data = make_objstm_indirect_object(12, 1, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.get(8), Some(&PdfObject::Boolean(true)));
    }

    // C-4. Array 객체 추출
    #[test]
    fn extracts_array_object() {
        let (payload, first) = make_objstm_payload(&[(9, b"[1 2 3]")]);
        let data = make_objstm_indirect_object(12, 1, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert!(matches!(result.get(9), Some(PdfObject::Array(_))));
    }

    // C-5. 3개 객체 전부 get() 조회 가능, 순서 보존
    #[test]
    fn extracts_multiple_objects() {
        let (payload, first) =
            make_objstm_payload(&[(3, b"<< /Type /Catalog >>"), (17, b"42"), (25, b"true")]);
        let data = make_objstm_indirect_object(12, 3, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.objects.len(), 3);
        assert!(matches!(result.get(3), Some(PdfObject::Dictionary(_))));
        assert_eq!(result.get(17), Some(&PdfObject::Integer(42)));
        assert_eq!(result.get(25), Some(&PdfObject::Boolean(true)));
    }

    // C-6. FlateDecode 압축 ObjStm — 전체 파이프라인
    #[test]
    fn flatedecoded_objstm_full_pipeline() {
        let (plain, first) = make_objstm_payload(&[(3, b"99"), (7, b"false")]);
        let compressed = make_zlib_compressed(&plain);
        let data =
            make_objstm_indirect_object(12, 2, first, Some("FlateDecode"), None, &compressed);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.get(3), Some(&PdfObject::Integer(99)));
        assert_eq!(result.get(7), Some(&PdfObject::Boolean(false)));
    }

    // C-7. 비압축 ObjStm 전체 파이프라인 (헤더+본문 통합)
    #[test]
    fn uncompressed_objstm_full_pipeline() {
        let (payload, first) = make_objstm_payload(&[(10, b"[1 2 3]"), (20, b"null")]);
        let data = make_objstm_indirect_object(12, 2, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert!(matches!(result.get(10), Some(PdfObject::Array(_))));
        assert_eq!(result.get(20), Some(&PdfObject::Null));
    }

    // C-8. get()이 없는 obj_num에 None 반환
    #[test]
    fn get_returns_none_for_missing_obj_num() {
        let (payload, first) = make_objstm_payload(&[(5, b"42")]);
        let data = make_objstm_indirect_object(12, 1, first, None, None, &payload);
        let result = parse_object_stream(&data, 0).unwrap();
        assert_eq!(result.get(5), Some(&PdfObject::Integer(42)));
        assert_eq!(result.get(99), None);
    }

    // C-9. 본문에 잘못된 객체 → MalformedObjStm
    #[test]
    fn rejects_object_parse_failure() {
        // "@@@" — 유효하지 않은 PDF 토큰
        let body = b"5 0\n@@@";
        let data = make_objstm_indirect_object(12, 1, 4, None, None, body);
        let err = parse_object_stream(&data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // C-10. rel_offset이 본문 범위 초과 → MalformedObjStm
    #[test]
    fn rejects_rel_offset_out_of_bounds() {
        // 헤더: "5 999\n" (First=6), 본문: "42" (2바이트)
        // abs_offset = 6 + 999 = 1005 > 8(총 길이) → 범위 초과
        let body = b"5 999\n42";
        let data = make_objstm_indirect_object(12, 1, 6, None, None, body);
        let err = parse_object_stream(&data, 0).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedObjStm { .. }),
            "expected MalformedObjStm, got {err:?}"
        );
    }

    // ── D proptest ──────────────────────────────────────────────────────────────

    proptest::proptest! {
        #[test]
        fn arbitrary_input_never_panics_parse_object_stream(data: Vec<u8>) {
            let _ = parse_object_stream(&data, 0);
        }
    }
}
