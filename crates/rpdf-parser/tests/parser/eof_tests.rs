use rpdf_parser::{ParseError, find_eof};

fn make_pdf_with_eof_count(count: usize) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    for _ in 0..count {
        pdf.extend_from_slice(b"startxref\n9\n%%EOF\n");
    }
    pdf
}

// ─── 정상 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn eof_at_end_no_newline() {
    let data = b"some content\n%%EOF";
    let offset = find_eof(data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    assert_eq!(offset, 13);
}

#[test]
fn eof_followed_by_lf() {
    let data = b"some content\n%%EOF\n";
    let offset = find_eof(data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    assert_eq!(offset, 13);
}

#[test]
fn eof_followed_by_crlf() {
    let data = b"some content\n%%EOF\r\n";
    let offset = find_eof(data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    assert_eq!(offset, 13);
}

#[test]
fn eof_followed_by_trailing_spaces() {
    // %%EOF 뒤에 공백이 있어도 마커 위치는 동일
    let data = b"some content\n%%EOF   \n";
    let offset = find_eof(data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    assert_eq!(offset, 13);
}

#[test]
fn single_eof_marker() {
    let data = make_pdf_with_eof_count(1);
    assert!(find_eof(&data).is_ok());
}

#[test]
fn two_eof_markers_returns_last() {
    // incremental update 1회 → %%EOF 2개, 마지막 위치를 반환해야 함
    let data = make_pdf_with_eof_count(2);
    let offset = find_eof(&data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    // 반환된 위치 이후에 %%EOF가 없어야 함
    assert!(!data[offset + 5..].windows(5).any(|w| w == b"%%EOF"));
}

#[test]
fn three_eof_markers_returns_last() {
    // incremental update 2회 → %%EOF 3개, 여전히 마지막 위치 반환
    let data = make_pdf_with_eof_count(3);
    let offset = find_eof(&data).unwrap();
    assert_eq!(&data[offset..offset + 5], b"%%EOF");
    assert!(!data[offset + 5..].windows(5).any(|w| w == b"%%EOF"));
}

#[test]
fn eof_exactly_at_search_window_start() {
    // 1029바이트 파일에서 %%EOF가 탐색 윈도 첫 번째 바이트에 위치
    // search_start = 1029 - 1024 = 5, window = data[5..], %%EOF는 window[0]
    let mut data = vec![b'x'; 1029];
    data[5..10].copy_from_slice(b"%%EOF");
    let offset = find_eof(&data).unwrap();
    assert_eq!(offset, 5);
}

// ─── 에러 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn reject_no_eof_marker() {
    let data = b"some content without any eof marker";
    assert!(matches!(
        find_eof(data).unwrap_err(),
        ParseError::MissingEof
    ));
}

#[test]
fn reject_empty_input() {
    assert!(matches!(find_eof(b"").unwrap_err(), ParseError::MissingEof));
}

#[test]
fn reject_eof_beyond_search_window() {
    // %%EOF가 파일 맨 앞, 그 뒤에 1025바이트가 붙음
    // data.len() = 1031, search_start = 7 이므로 %%EOF(위치 0~4)는 윈도 밖
    let mut data = b"%%EOF\n".to_vec();
    data.extend(vec![b'x'; 1025]);
    assert!(matches!(
        find_eof(&data).unwrap_err(),
        ParseError::MissingEof
    ));
}

#[test]
fn reject_truncated_eof_marker() {
    let data = b"content\n%%EO";
    assert!(matches!(
        find_eof(data).unwrap_err(),
        ParseError::MissingEof
    ));
}

#[test]
fn reject_single_percent_eof() {
    // %%가 아닌 %EOF — false positive 거부
    let data = b"content\n%EOF\n";
    assert!(matches!(
        find_eof(data).unwrap_err(),
        ParseError::MissingEof
    ));
}

#[test]
fn reject_double_percent_without_eof_suffix() {
    // %%는 있지만 EOF가 없음
    let data = b"content\n%%\n";
    assert!(matches!(
        find_eof(data).unwrap_err(),
        ParseError::MissingEof
    ));
}
