use crate::error::ParseError;
use crate::objects::parse_dictionary;
use crate::startxref::parse_startxref;
use rpdf_core::types::{ObjectId, PdfDict, PdfObject};

/// PDF trailer 딕셔너리 파싱 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfTrailer {
    /// `/Size` — xref 엔트리 총 개수 (필수).
    pub size: u32,
    /// `/Root` — 문서 카탈로그 딕셔너리의 간접 참조 (필수).
    pub root: ObjectId,
    /// `/Info` — 문서 정보 딕셔너리의 간접 참조 (선택).
    pub info: Option<ObjectId>,
    /// `/Prev` — 이전 xref 테이블의 바이트 오프셋 (점진적 업데이트 시 존재).
    pub prev: Option<u64>,
}

/// `parse_trailer()`의 반환 타입.
#[derive(Debug)]
pub struct ParsedTrailer {
    pub trailer: PdfTrailer,

    /// `startxref` 키워드 다음에 기재된 값.
    /// xref 테이블(또는 xref 스트림)이 파일 내에서 시작하는 **절대 바이트 오프셋**.
    pub xref_offset: u64,
}

/// `search_end`(보통 `%%EOF` 시작 오프셋)에서 최대 4KB 역방향으로 `trailer` 키워드를
/// 탐색하고 딕셔너리를 파싱한다. 내부에서 `parse_startxref`를 호출한다.
///
/// # Errors
///
/// - [`ParseError::MissingStartXref`] / [`ParseError::InvalidStartXref`] — startxref 파싱 실패
/// - [`ParseError::XrefStreamUnsupported`] — xref 스트림 방식(PDF 1.5+) 감지
/// - [`ParseError::MissingTrailer`] — `trailer` 키워드 없음
/// - [`ParseError::MalformedTrailer`] — `<<`가 닫히지 않는 등 구조적 오류
/// - [`ParseError::TrailerTooLarge`] — 딕셔너리 내용이 4KB 초과
/// - [`ParseError::MissingRequiredKey`] — `/Size` 또는 `/Root` 누락
/// - [`ParseError::InvalidObjectRef`] — `/Root` 등 간접 참조 형식 오류
pub fn parse_trailer(data: &[u8], search_end: usize) -> Result<ParsedTrailer, ParseError> {
    const TRAILER_KEYWORD: &[u8] = b"trailer";
    // "trailer" 키워드 탐색 범위. DICT_MAX_BYTES보다 2배 크게 잡아
    // TrailerTooLarge가 실제로 도달 가능하게 한다.
    const SEARCH_WINDOW: usize = 8192;
    const DICT_MAX_BYTES: usize = 4096;

    let xref_offset = parse_startxref(data, search_end)?;

    let search_start = search_end.saturating_sub(SEARCH_WINDOW);
    let search_data = &data[search_start..search_end];

    let rel_pos = search_data
        .windows(TRAILER_KEYWORD.len())
        .rposition(|w| w == TRAILER_KEYWORD)
        .ok_or_else(|| {
            if is_xref_stream(data, xref_offset) {
                ParseError::XrefStreamUnsupported { xref_offset }
            } else {
                ParseError::MissingTrailer
            }
        })?;

    let after_kw_start = search_start + rel_pos + TRAILER_KEYWORD.len();

    // "<<"를 찾는다.
    let open_rel = data[after_kw_start..]
        .windows(2)
        .position(|w| w == b"<<")
        .ok_or(ParseError::MissingTrailer)?;

    let dict_start = after_kw_start + open_rel;

    let (dict, consumed) =
        parse_dictionary(data, dict_start, 0).map_err(|_| ParseError::MalformedTrailer {
            reason: "trailer 딕셔너리 파싱 실패".to_string(),
        })?;

    // consumed = << (2) + inner + >> (2). inner = consumed - 4
    let inner_len = consumed.saturating_sub(4);
    if inner_len > DICT_MAX_BYTES {
        return Err(ParseError::TrailerTooLarge {
            limit_kb: DICT_MAX_BYTES / 1024,
        });
    }

    let trailer = extract_trailer_fields(&dict)?;

    Ok(ParsedTrailer {
        trailer,
        xref_offset,
    })
}

/// `xref_offset`이 가리키는 바이트가 `<N> <G> obj` 패턴이면 xref 스트림으로 판단한다.
pub(crate) fn is_xref_stream(data: &[u8], xref_offset: u64) -> bool {
    let offset = xref_offset as usize;
    if offset >= data.len() {
        return false;
    }
    let bytes = &data[offset..];
    let skip = bytes
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    if skip >= bytes.len() || !bytes[skip].is_ascii_digit() {
        return false;
    }
    let window_end = (skip + 32).min(bytes.len());
    bytes[skip..window_end].windows(3).any(|w| w == b"obj")
}

/// `PdfDict`에서 trailer 필드를 추출해 `PdfTrailer`를 구성한다.
///
/// `xref.rs`의 `parse_trailer_at`에서도 공유한다.
pub(crate) fn extract_trailer_fields(dict: &PdfDict) -> Result<PdfTrailer, ParseError> {
    // /Size — 필수, 양의 정수
    let size_obj = dict
        .get(b"Size")
        .ok_or(ParseError::MissingRequiredKey { key: "Size" })?;
    let size = size_obj
        .as_u64()
        .map(|n| n as u32)
        .ok_or_else(|| ParseError::MalformedTrailer {
            reason: format!("/Size 값이 정수가 아님: {:?}", size_obj),
        })?;

    // /Root — 필수, 간접 참조
    let root_obj = dict
        .get(b"Root")
        .ok_or(ParseError::MissingRequiredKey { key: "Root" })?;
    let root = match root_obj {
        PdfObject::Reference(id) => *id,
        _ => {
            return Err(ParseError::InvalidObjectRef {
                found: format!("{root_obj:?}"),
            });
        }
    };

    // /Info — 선택, 간접 참조
    let info = match dict.get(b"Info") {
        Some(PdfObject::Reference(id)) => Some(*id),
        _ => None,
    };

    // /Prev — 선택, 양의 정수
    let prev = match dict.get(b"Prev") {
        None => None,
        Some(obj) => Some(obj.as_u64().ok_or_else(|| ParseError::MalformedTrailer {
            reason: "/Prev 값이 정수가 아님".to_string(),
        })?),
    };

    Ok(PdfTrailer {
        size,
        root,
        info,
        prev,
    })
}
