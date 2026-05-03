use crate::error::ParseError;

/// `data`에서 `startxref` 키워드를 탐색하고 뒤따르는 xref 오프셋을 반환한다.
///
/// `eof_offset`(즉, `%%EOF` 시작 위치)에서 최대 1024바이트 이전 범위를 역방향으로 탐색한다.
/// Incremental update가 적용된 PDF에 `startxref`가 여러 개 있으면 **마지막 것**을 사용한다.
///
/// # 반환값
///
/// `startxref` 키워드 다음 줄에 기재된 u64 값.
/// 이 값은 xref 테이블(또는 xref 스트림)이 파일 내에서 시작하는 절대 바이트 오프셋이다.
///
/// # Errors
///
/// - [`ParseError::MissingStartXref`] — 탐색 범위 내에 `startxref` 키워드 없음
/// - [`ParseError::InvalidStartXref`] — 키워드 다음에 유효한 u64 숫자 없음
pub fn parse_startxref(data: &[u8], eof_offset: usize) -> Result<u64, ParseError> {
    const STARTXREF_MARKER: &[u8] = b"startxref";
    // startxref는 %%EOF 바로 앞에 위치하므로 1024바이트 이내 역방향 탐색으로 충분하다.
    const SEARCH_WINDOW: usize = 1024;

    let search_start = eof_offset.saturating_sub(SEARCH_WINDOW);
    let search_data = &data[search_start..eof_offset];

    let rel_pos = search_data
        .windows(STARTXREF_MARKER.len())
        .rposition(|w| w == STARTXREF_MARKER)
        .ok_or(ParseError::MissingStartXref)?;

    let after_keyword = &data[search_start + rel_pos + STARTXREF_MARKER.len()..eof_offset];

    // startxref 다음의 줄바꿈(\r, \n만)을 건너뜀. 공백은 허용하지 않는다 (PDF 스펙 §7.5.5).
    let digits_start = after_keyword
        .iter()
        .position(|&b| b != b'\r' && b != b'\n')
        .ok_or(ParseError::InvalidStartXref {
            found: String::new(),
        })?;

    let digits_data = &after_keyword[digits_start..];

    let digits_end = digits_data
        .iter()
        .position(|&b| !b.is_ascii_digit())
        .unwrap_or(digits_data.len());

    if digits_end == 0 {
        return Err(ParseError::InvalidStartXref {
            found: truncate_for_error(digits_data, 16),
        });
    }

    let num_bytes = &digits_data[..digits_end];
    let num_str = std::str::from_utf8(num_bytes).map_err(|_| ParseError::InvalidStartXref {
        found: truncate_for_error(num_bytes, 16),
    })?;

    num_str
        .parse::<u64>()
        .map_err(|_| ParseError::InvalidStartXref {
            found: num_str[..num_str.len().min(16)].to_string(),
        })
}

fn truncate_for_error(bytes: &[u8], max: usize) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(max)]).to_string()
}
