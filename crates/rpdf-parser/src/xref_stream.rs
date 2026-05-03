// C·D 단계 구현 완료 전까지 todo!() 스텁과 미사용 필드 경고 억제.
#![allow(dead_code)]

//! PDF 1.5+ cross-reference stream 파싱 (ISO 32000 §7.5.8).
//!
//! xref 스트림은 전통 xref 테이블 대신 간접 객체 형식의 스트림으로 저장된다.
//! FlateDecode 압축과 PNG 예측 필터를 지원한다.
//!
//! 책임 범위:
//! - xref 스트림 딕셔너리 파싱 (/W, /Index, /Filter, /DecodeParms)
//! - FlateDecode 압축 해제
//! - PNG Predictor 1·10–15 언필터링
//! - xref 엔트리 타입 0·1·2 디코딩
//!
//! 범위 외:
//! - 객체 스트림(/Type /ObjStm) 파싱 — 후속 Task
//! - FlateDecode 외 필터 — `InvalidXrefStreamFilter` 에러
//! - TIFF Predictor(값 2) — `UnsupportedPredictor` 에러

use rpdf_core::types::{IndirectObject, ObjectId, PdfObject, PdfStream, XrefEntry};

use crate::error::ParseError;
use crate::objects::parse_indirect_object;
use crate::trailer::PdfTrailer;
use crate::xref::XrefSectionResult;

/// xref 스트림 간접 객체를 파싱해 엔트리 목록, PdfTrailer, 섹션 정보를 반환한다.
///
/// `xref_offset`: `startxref` 또는 `/Prev`가 가리키는 간접 객체(`N G obj`)의 파일 오프셋.
/// 반환 타입이 `parse_xref_section`과 동일해 `parse_xref_chain`에서 투명 교체 가능.
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — 간접 객체가 스트림이 아니거나 구조 손상
/// - [`ParseError::XrefStreamInvalidW`] — `/W` 배열 누락 또는 형식 오류
/// - [`ParseError::XrefStreamInvalidIndex`] — `/Index` 배열 형식 오류
/// - [`ParseError::InvalidXrefStreamFilter`] — FlateDecode 외 필터
/// - [`ParseError::UnsupportedPredictor`] — TIFF 또는 알 수 없는 Predictor
/// - [`ParseError::XrefStreamDecompressError`] — zlib 압축 해제 실패
/// - [`ParseError::XrefStreamWFieldMismatch`] — W 크기와 데이터 길이 불일치
/// - [`ParseError::XrefStreamEntryCountMismatch`] — 엔트리 수가 /Index 선언과 불일치
pub(crate) fn parse_xref_stream(
    data: &[u8],
    xref_offset: u64,
) -> Result<XrefSectionResult, ParseError> {
    let (_indirect, stream, _body_offset) = parse_xref_stream_dict(data, xref_offset)?;
    let metadata = extract_xref_stream_metadata(&stream.dict, xref_offset)?;
    let _trailer = build_trailer_from_xref_stream_metadata(&metadata)?;
    // C 단계: decompress_flate + unpredict_png
    // D 단계: decode_entries + 체인 통합
    todo!("C·D 단계에서 구현")
}

/// `xref_offset` 위치의 간접 객체가 `/Type /XRef` 스트림인지 검사한다.
///
/// `trailer.rs::is_xref_stream`(빠른 휴리스틱)과 달리 실제 파싱을 수행해
/// stream 여부와 `/Type /XRef`를 모두 확인한다. ISO 32000 §7.5.8.
pub(crate) fn is_xref_stream(data: &[u8], xref_offset: u64) -> bool {
    let offset = xref_offset as usize;
    if offset >= data.len() {
        return false;
    }
    let Ok((indirect, _)) = parse_indirect_object(data, offset) else {
        return false;
    };
    let PdfObject::Stream(stream) = &indirect.object else {
        return false;
    };
    matches!(
        stream.dict.get(b"Type"),
        Some(PdfObject::Name(n)) if n == b"XRef"
    )
}

/// xref 스트림 간접 객체를 파싱해 `(IndirectObject, PdfStream, body_offset)` 튜플로 반환한다.
///
/// `body_offset`은 스트림 데이터가 시작하는 파일 내 절대 오프셋이다(에러 보고용).
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — stream이 아니거나 `/Type /XRef` 불일치
///
/// ISO 32000 §7.5.8
pub(crate) fn parse_xref_stream_dict(
    data: &[u8],
    xref_offset: u64,
) -> Result<(IndirectObject, PdfStream, u64), ParseError> {
    let offset = xref_offset as usize;
    let (indirect, _consumed) =
        parse_indirect_object(data, offset).map_err(|e| ParseError::MalformedXrefStream {
            offset: xref_offset,
            reason: format!("간접 객체 파싱 실패: {e}"),
        })?;

    let stream = match &indirect.object {
        PdfObject::Stream(s) => s.clone(),
        _ => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: "xref 스트림 객체가 stream이 아님".to_string(),
            });
        }
    };

    // /Type == /XRef 확인 (ISO 32000 §7.5.8.2)
    match stream.dict.get(b"Type") {
        Some(PdfObject::Name(n)) if n == b"XRef" => {}
        Some(other) => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: format!("/Type가 /XRef가 아님: {other:?}"),
            });
        }
        None => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: "/Type 키 누락".to_string(),
            });
        }
    }

    Ok((indirect, stream, xref_offset))
}

/// xref 스트림 딕셔너리에서 메타데이터를 추출한다.
///
/// `/W`, `/Index`, `/Filter`, `/DecodeParms`, `/Root`, `/Prev` 등을 파싱한다.
/// ISO 32000 §7.5.8.2 Table 17
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — `/Size` 누락·비정수, `/DecodeParms` 형식 오류
/// - [`ParseError::XrefStreamInvalidW`] — `/W` 없거나 3개 정수 아님
/// - [`ParseError::XrefStreamInvalidIndex`] — `/Index` 홀수 원소 또는 비정수
/// - [`ParseError::InvalidXrefStreamFilter`] — FlateDecode 외 필터
pub(crate) fn extract_xref_stream_metadata(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<XrefStreamMetadata, ParseError> {
    // /Size — 필수
    let size = match dict.get(b"Size") {
        None => {
            return Err(ParseError::MalformedXrefStream {
                offset,
                reason: "/Size 키 누락".to_string(),
            });
        }
        Some(obj) => {
            obj.as_u64()
                .map(|n| n as u32)
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/Size 값이 비음수 정수가 아님: {obj:?}"),
                })?
        }
    };

    // /W — 필수, [W1 W2 W3] 3개 비음수 정수
    let w = extract_w(dict, offset)?;

    // /Index — 선택, 기본값 [(0, size)]
    let index = extract_index(dict, size, offset)?;

    // /Filter — 선택
    let filter = extract_filter(dict, offset)?;

    // /DecodeParms — 선택
    let (predictor, columns) = extract_decode_parms(dict, offset)?;

    // /Prev — 선택
    let prev = match dict.get(b"Prev") {
        None => None,
        Some(obj) => Some(
            obj.as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: "/Prev 값이 정수가 아님".to_string(),
                })?,
        ),
    };

    // /Root — 선택 (PdfTrailer 생성 시 필수 검사)
    let root = match dict.get(b"Root") {
        Some(PdfObject::Reference(id)) => Some(*id),
        _ => None,
    };

    // /Info — 선택
    let info = match dict.get(b"Info") {
        Some(PdfObject::Reference(id)) => Some(*id),
        _ => None,
    };

    Ok(XrefStreamMetadata {
        size,
        w,
        index,
        filter,
        predictor,
        columns,
        prev,
        root,
        info,
    })
}

/// xref 스트림 메타데이터에서 `PdfTrailer`를 구성한다.
///
/// `/Root`이 없으면 [`ParseError::MissingRequiredKey`]를 반환한다.
pub(crate) fn build_trailer_from_xref_stream_metadata(
    meta: &XrefStreamMetadata,
) -> Result<PdfTrailer, ParseError> {
    let root = meta
        .root
        .ok_or(ParseError::MissingRequiredKey { key: "Root" })?;

    Ok(PdfTrailer {
        size: meta.size,
        root,
        info: meta.info,
        prev: meta.prev,
    })
}

/// `/W [W1 W2 W3]` 배열을 추출한다.
fn extract_w(dict: &rpdf_core::types::PdfDict, offset: u64) -> Result<[usize; 3], ParseError> {
    let arr = match dict.get(b"W") {
        None => {
            return Err(ParseError::XrefStreamInvalidW {
                offset,
                reason: "/W 키 누락".to_string(),
            });
        }
        Some(PdfObject::Array(a)) => a,
        Some(other) => {
            return Err(ParseError::XrefStreamInvalidW {
                offset,
                reason: format!("/W가 배열이 아님: {other:?}"),
            });
        }
    };

    if arr.len() != 3 {
        return Err(ParseError::XrefStreamInvalidW {
            offset,
            reason: format!("/W 원소 수가 3이 아님: {}", arr.len()),
        });
    }

    let mut w = [0usize; 3];
    for (i, obj) in arr.iter().enumerate() {
        w[i] = obj.as_u64().ok_or_else(|| ParseError::XrefStreamInvalidW {
            offset,
            reason: format!("/W[{i}]가 비음수 정수가 아님: {obj:?}"),
        })? as usize;
    }
    Ok(w)
}

/// `/Index` 배열을 추출한다. 없으면 `[(0, size)]` 기본값을 반환한다.
fn extract_index(
    dict: &rpdf_core::types::PdfDict,
    size: u32,
    offset: u64,
) -> Result<Vec<(u32, u32)>, ParseError> {
    let arr = match dict.get(b"Index") {
        None => return Ok(vec![(0, size)]),
        Some(PdfObject::Array(a)) => a,
        Some(other) => {
            return Err(ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index가 배열이 아님: {other:?}"),
            });
        }
    };

    if arr.len() % 2 != 0 {
        return Err(ParseError::XrefStreamInvalidIndex {
            offset,
            reason: format!("/Index 원소 수가 홀수: {}", arr.len()),
        });
    }

    let mut pairs = Vec::with_capacity(arr.len() / 2);
    for chunk in arr.chunks(2) {
        let first = chunk[0]
            .as_u64()
            .ok_or_else(|| ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index first 값이 비음수 정수가 아님: {:?}", chunk[0]),
            })? as u32;
        let count = chunk[1]
            .as_u64()
            .ok_or_else(|| ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index count 값이 비음수 정수가 아님: {:?}", chunk[1]),
            })? as u32;
        pairs.push((first, count));
    }
    Ok(pairs)
}

/// `/Filter` 를 추출한다.
fn extract_filter(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<Option<XrefStreamFilter>, ParseError> {
    match dict.get(b"Filter") {
        None => Ok(None),
        Some(PdfObject::Name(n)) => {
            if n == b"FlateDecode" {
                Ok(Some(XrefStreamFilter::FlateDecode))
            } else {
                Err(ParseError::InvalidXrefStreamFilter {
                    offset,
                    filter: String::from_utf8_lossy(n).into_owned(),
                })
            }
        }
        Some(PdfObject::Array(arr)) => {
            // [/FlateDecode] 단일 원소 배열 허용 (ISO 32000 §7.3.8.2)
            if arr.len() == 1 {
                match &arr[0] {
                    PdfObject::Name(n) if n == b"FlateDecode" => {
                        Ok(Some(XrefStreamFilter::FlateDecode))
                    }
                    PdfObject::Name(n) => Err(ParseError::InvalidXrefStreamFilter {
                        offset,
                        filter: String::from_utf8_lossy(n).into_owned(),
                    }),
                    other => Err(ParseError::InvalidXrefStreamFilter {
                        offset,
                        filter: format!("{other:?}"),
                    }),
                }
            } else {
                Err(ParseError::InvalidXrefStreamFilter {
                    offset,
                    filter: "다중 필터 체인 미지원".to_string(),
                })
            }
        }
        Some(other) => Err(ParseError::InvalidXrefStreamFilter {
            offset,
            filter: format!("{other:?}"),
        }),
    }
}

/// `/DecodeParms`에서 `(predictor, columns)` 를 추출한다.
fn extract_decode_parms(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<(Option<u8>, Option<usize>), ParseError> {
    let parms_obj = match dict.get(b"DecodeParms") {
        None => return Ok((None, None)),
        Some(obj) => obj,
    };

    let parms_dict = match parms_obj {
        PdfObject::Dictionary(d) => d,
        other => {
            return Err(ParseError::MalformedXrefStream {
                offset,
                reason: format!("/DecodeParms가 딕셔너리가 아님: {other:?}"),
            });
        }
    };

    let predictor = match parms_dict.get(b"Predictor") {
        None => None,
        Some(obj) => {
            let n = obj
                .as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/DecodeParms /Predictor가 정수가 아님: {obj:?}"),
                })? as u8;
            Some(n)
        }
    };

    let columns = match parms_dict.get(b"Columns") {
        None => None,
        Some(obj) => {
            let n = obj
                .as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/DecodeParms /Columns가 정수가 아님: {obj:?}"),
                })? as usize;
            Some(n)
        }
    };

    Ok((predictor, columns))
}

/// FlateDecode 압축을 해제한다.
///
/// `/Filter /FlateDecode`(또는 배열 `[/FlateDecode]`)에 대응.
/// 다른 필터는 `InvalidXrefStreamFilter` 반환.
fn decompress_flate(_raw: &[u8], _offset: u64) -> Result<Vec<u8>, ParseError> {
    todo!("C 단계에서 구현")
}

/// PNG 예측 필터를 제거해 원본 xref 엔트리 데이터를 복원한다.
///
/// Predictor 1(없음), 10–15(PNG 필터군) 지원.
/// Predictor 2(TIFF) → `UnsupportedPredictor`.
fn unpredict_png(
    _data: &[u8],
    _predictor: u8,
    _columns: usize,
    _offset: u64,
) -> Result<Vec<u8>, ParseError> {
    todo!("C 단계에서 구현")
}

/// 디코딩된 바이트를 `/W`와 `/Index`에 따라 xref 엔트리로 변환한다.
///
/// 타입 0 → `XrefEntry::Free`, 타입 1 → `XrefEntry::InUse`, 타입 2 → `XrefEntry::Compressed`.
/// W1=0이면 default 타입=1.
fn decode_entries(
    _decoded: &[u8],
    _w: [usize; 3],
    _index_pairs: &[(u32, u32)],
    _offset: u64,
) -> Result<Vec<(u32, XrefEntry)>, ParseError> {
    todo!("D 단계에서 구현")
}

/// xref 스트림 메타데이터 — `extract_xref_stream_metadata` 반환값.
///
/// ISO 32000 §7.5.8.2 Table 17
#[derive(Debug)]
pub(crate) struct XrefStreamMetadata {
    /// `/Size` — xref 엔트리 총 개수.
    pub size: u32,
    /// `/W [W1 W2 W3]` — 엔트리 필드별 바이트 너비.
    pub w: [usize; 3],
    /// `/Index` 서브섹션 쌍. 없으면 `[(0, size)]` 기본값 적용.
    pub index: Vec<(u32, u32)>,
    /// `/Filter` — None이면 비압축.
    pub filter: Option<XrefStreamFilter>,
    /// `/DecodeParms /Predictor` — None이면 1(기본값).
    pub predictor: Option<u8>,
    /// `/DecodeParms /Columns` — None이면 C 단계에서 W1+W2+W3 사용.
    pub columns: Option<usize>,
    /// `/Prev` — 이전 xref 위치.
    pub prev: Option<u64>,
    /// `/Root` — 문서 카탈로그 간접 참조. PdfTrailer 생성 시 필수.
    pub root: Option<ObjectId>,
    /// `/Info` — 문서 정보 간접 참조.
    pub info: Option<ObjectId>,
}

/// xref 스트림 압축 필터 종류.
///
/// ISO 32000 §7.4 — FlateDecode만 지원. 다른 필터는 `InvalidXrefStreamFilter`.
#[derive(Debug)]
pub(crate) enum XrefStreamFilter {
    /// `/FlateDecode` — zlib/deflate 압축.
    FlateDecode,
}

// ── 단위 테스트 ─────────────────────────────────────────────────────────────────
// pub(crate) 함수 테스트는 CLAUDE.md 규칙에 따라 인라인 배치.
// E 단계에서 parse_xref_stream이 pub 노출되면 통합 테스트 파일 추가.

#[cfg(test)]
mod tests {
    use super::*;
    use rpdf_core::types::{ObjectId, PdfDict, PdfObject};

    // ── 합성 데이터 헬퍼 ──────────────────────────────────────────────────────

    /// `/Type /XRef` 스트림 간접 객체 바이트를 생성한다.
    /// `dict_extra`는 기본 키(/Type, /Size, /W, /Root, /Length) 이외의 추가 내용.
    fn make_xref_stream_object(dict_extra: &str, body: &[u8]) -> Vec<u8> {
        let length = body.len();
        let extra = if dict_extra.is_empty() {
            String::new()
        } else {
            format!(" {dict_extra}")
        };
        let dict =
            format!("<< /Type /XRef /Size 10 /W [1 3 1] /Root 1 0 R /Length {length}{extra} >>");
        let mut buf = Vec::new();
        buf.extend_from_slice(format!("5 0 obj\n{dict}\nstream\n").as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        buf
    }

    /// `PdfDict`를 직접 구성하는 헬퍼.
    fn make_dict(entries: Vec<(&'static [u8], PdfObject)>) -> PdfDict {
        PdfDict(entries.into_iter().map(|(k, v)| (k.to_vec(), v)).collect())
    }

    fn name(s: &str) -> PdfObject {
        PdfObject::Name(s.as_bytes().to_vec())
    }

    fn int(n: i64) -> PdfObject {
        PdfObject::Integer(n)
    }

    fn arr(items: Vec<PdfObject>) -> PdfObject {
        PdfObject::Array(items)
    }

    fn ref_obj(num: u32) -> PdfObject {
        PdfObject::Reference(ObjectId {
            number: num,
            generation: 0,
        })
    }

    // ── is_xref_stream ────────────────────────────────────────────────────────

    #[test]
    fn is_xref_stream_with_xref_type_returns_true() {
        let data = make_xref_stream_object("", b"dummy");
        assert!(is_xref_stream(&data, 0));
    }

    #[test]
    fn is_xref_stream_with_catalog_type_returns_false() {
        let body = b"dummy";
        let dict = format!(
            "<< /Type /Catalog /Size 10 /W [1 3 1] /Root 1 0 R /Length {} >>",
            body.len()
        );
        let mut data = Vec::new();
        data.extend_from_slice(format!("5 0 obj\n{dict}\nstream\n").as_bytes());
        data.extend_from_slice(body);
        data.extend_from_slice(b"\nendstream\nendobj\n");
        assert!(!is_xref_stream(&data, 0));
    }

    #[test]
    fn is_xref_stream_non_stream_object_returns_false() {
        assert!(!is_xref_stream(b"5 0 obj\n42\nendobj\n", 0));
    }

    // ── parse_xref_stream_dict ────────────────────────────────────────────────

    #[test]
    fn parse_xref_stream_dict_valid_returns_ok() {
        let data = make_xref_stream_object("", b"rawbytes");
        let (indirect, stream, _body_offset) = parse_xref_stream_dict(&data, 0).unwrap();
        assert_eq!(indirect.id.number, 5);
        assert_eq!(stream.data, b"rawbytes");
        assert_eq!(
            stream.dict.get(b"Type"),
            Some(&PdfObject::Name(b"XRef".to_vec()))
        );
    }

    #[test]
    fn parse_xref_stream_dict_non_stream_returns_malformed() {
        let data = b"5 0 obj\n42\nendobj\n";
        let err = parse_xref_stream_dict(data, 0).unwrap_err();
        assert!(matches!(err, ParseError::MalformedXrefStream { .. }));
    }

    // ── extract_xref_stream_metadata ─────────────────────────────────────────

    #[test]
    fn extract_metadata_minimal_size_and_w_only() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert_eq!(meta.size, 10);
        assert_eq!(meta.w, [1, 3, 1]);
        // /Index 없으면 기본값 [(0, size)]
        assert_eq!(meta.index, vec![(0, 10)]);
        assert!(meta.filter.is_none());
        assert!(meta.predictor.is_none());
        assert!(meta.columns.is_none());
    }

    #[test]
    fn extract_metadata_with_explicit_index() {
        let dict = make_dict(vec![
            (b"Size", int(20)),
            (b"W", arr(vec![int(1), int(4), int(2)])),
            (b"Index", arr(vec![int(5), int(3), int(15), int(5)])),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert_eq!(meta.index, vec![(5, 3), (15, 5)]);
    }

    #[test]
    fn extract_metadata_with_flate_and_decode_parms() {
        let decode_parms = PdfObject::Dictionary(make_dict(vec![
            (b"Predictor", int(12)),
            (b"Columns", int(5)),
        ]));
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Filter", name("FlateDecode")),
            (b"DecodeParms", decode_parms),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert!(matches!(meta.filter, Some(XrefStreamFilter::FlateDecode)));
        assert_eq!(meta.predictor, Some(12));
        assert_eq!(meta.columns, Some(5));
    }

    #[test]
    fn extract_metadata_invalid_filter_returns_error() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Filter", name("LZWDecode")),
            (b"Root", ref_obj(1)),
        ]);
        let err = extract_xref_stream_metadata(&dict, 0).unwrap_err();
        assert!(
            matches!(&err, ParseError::InvalidXrefStreamFilter { filter, .. } if filter == "LZWDecode")
        );
    }

    #[test]
    fn extract_metadata_invalid_w_too_few_elements_returns_error() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3)])), // 2개 원소 (3개 필요)
            (b"Root", ref_obj(1)),
        ]);
        let err = extract_xref_stream_metadata(&dict, 0).unwrap_err();
        assert!(matches!(err, ParseError::XrefStreamInvalidW { .. }));
    }
}
