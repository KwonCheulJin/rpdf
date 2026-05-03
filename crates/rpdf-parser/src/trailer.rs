use crate::error::ParseError;
use crate::object_parser::{
    find_dict_close, is_name_char, parse_indirect_ref, parse_u64_val, peek_str, skip_value,
    skip_whitespace,
};
use crate::startxref::parse_startxref;
use rpdf_core::types::ObjectId;

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
                ParseError::XrefStreamUnsupported
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

    let dict_inner_start = after_kw_start + open_rel + 2;

    let dict_inner_data = &data[dict_inner_start..];

    let close_pos =
        find_dict_close(dict_inner_data).ok_or_else(|| ParseError::MalformedTrailer {
            reason: "trailer 딕셔너리가 닫히지 않음 (>> 없음)".to_string(),
        })?;

    if close_pos > DICT_MAX_BYTES {
        return Err(ParseError::TrailerTooLarge {
            limit_kb: DICT_MAX_BYTES / 1024,
        });
    }

    let trailer = parse_dict_fields(&dict_inner_data[..close_pos])?;

    Ok(ParsedTrailer {
        trailer,
        xref_offset,
    })
}

/// `xref_offset`이 가리키는 바이트가 `<N> <G> obj` 패턴이면 xref 스트림으로 판단한다.
fn is_xref_stream(data: &[u8], xref_offset: u64) -> bool {
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

/// `<<` ~ `>>` 사이 내용(inner bytes)에서 trailer 필드를 추출한다.
fn parse_dict_fields(data: &[u8]) -> Result<PdfTrailer, ParseError> {
    let mut size: Option<u32> = None;
    let mut root: Option<ObjectId> = None;
    let mut info: Option<ObjectId> = None;
    let mut prev: Option<u64> = None;

    let mut i = 0;
    while i < data.len() {
        i += skip_whitespace(&data[i..]);
        if i >= data.len() {
            break;
        }

        if data[i] != b'/' {
            i += 1;
            continue;
        }

        // Name 파싱 (/ 이후 name char 연속)
        i += 1;
        let name_end = data[i..]
            .iter()
            .position(|&b| !is_name_char(b))
            .map(|n| i + n)
            .unwrap_or(data.len());
        let name = &data[i..name_end];
        i = name_end;

        i += skip_whitespace(&data[i..]);
        if i >= data.len() {
            break;
        }

        match name {
            b"Size" => {
                let (n, len) =
                    parse_u64_val(&data[i..]).ok_or_else(|| ParseError::MalformedTrailer {
                        reason: format!("/Size 값이 정수가 아님: {}", peek_str(&data[i..], 16)),
                    })?;
                size = Some(n as u32);
                i += len;
            }
            b"Root" => {
                let (obj, len) =
                    parse_indirect_ref(&data[i..]).ok_or_else(|| ParseError::InvalidObjectRef {
                        found: peek_str(&data[i..], 20),
                    })?;
                root = Some(obj);
                i += len;
            }
            b"Info" => {
                if let Some((obj, len)) = parse_indirect_ref(&data[i..]) {
                    info = Some(obj);
                    i += len;
                } else {
                    i += skip_value(&data[i..]);
                }
            }
            b"Prev" => {
                let (n, len) =
                    parse_u64_val(&data[i..]).ok_or_else(|| ParseError::MalformedTrailer {
                        reason: format!("/Prev 값이 정수가 아님: {}", peek_str(&data[i..], 16)),
                    })?;
                prev = Some(n);
                i += len;
            }
            _ => {
                i += skip_value(&data[i..]);
            }
        }
    }

    let size = size.ok_or(ParseError::MissingRequiredKey { key: "Size" })?;
    let root = root.ok_or(ParseError::MissingRequiredKey { key: "Root" })?;

    Ok(PdfTrailer {
        size,
        root,
        info,
        prev,
    })
}
