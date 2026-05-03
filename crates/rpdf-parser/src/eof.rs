use crate::error::ParseError;

/// 파일 끝부터 역방향으로 `%%EOF` 마커를 탐색한다.
///
/// PDF 스펙 §7.5.5에 따르면 `%%EOF` 마커는 파일 끝에 위치한다.
/// Incremental update가 적용된 PDF는 `%%EOF`가 여러 번 나타날 수 있으므로
/// **가장 마지막 `%%EOF`** 의 위치를 반환한다.
///
/// # 반환값
///
/// `%%EOF`의 첫 `%` 바이트의 절대 바이트 오프셋 (파일 시작 기준).
/// [`parse_startxref`]가 이 값을 `startxref` 키워드 탐색 상한으로 사용한다.
///
/// # Errors
///
/// [`ParseError::MissingEof`] — 파일 끝 1024바이트 이내에 `%%EOF` 없음
pub fn find_eof(data: &[u8]) -> Result<usize, ParseError> {
    // 파일 끝에서 최대 1024바이트 안에서 탐색.
    // parse_header의 SEARCH_LIMIT(파일 앞쪽 1024)과 대칭이지만 의미가 다르다 (뒤쪽 1024).
    const EOF_SEARCH_WINDOW: usize = 1024;
    const EOF_MARKER: &[u8] = b"%%EOF";

    let search_start = data.len().saturating_sub(EOF_SEARCH_WINDOW);
    let window = &data[search_start..];

    // rposition: 끝에서부터 첫 매칭 → 마지막 %%EOF 위치.
    // incremental update PDF는 %%EOF가 여러 개 존재하므로 마지막 것을 사용해야 한다.
    window
        .windows(EOF_MARKER.len())
        .rposition(|w| w == EOF_MARKER)
        .map(|pos| search_start + pos)
        .ok_or(ParseError::MissingEof)
}
