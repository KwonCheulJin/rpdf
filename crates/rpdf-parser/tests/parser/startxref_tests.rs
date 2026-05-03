use rpdf_parser::{ParseError, parse_startxref};

/// 헬퍼: prefix + `startxref\n{xref_offset}\n` + `%%EOF\n` 조합 데이터를 생성하고
/// `%%EOF` 시작 오프셋을 함께 반환한다.
fn make_pdf_with_startxref(xref_offset: u64) -> (Vec<u8>, usize) {
    let prefix = b"%PDF-1.7\nsome body\n";
    let middle = format!("startxref\n{}\n", xref_offset);
    let eof_offset = prefix.len() + middle.len();
    let mut data = prefix.to_vec();
    data.extend_from_slice(middle.as_bytes());
    data.extend_from_slice(b"%%EOF\n");
    (data, eof_offset)
}

// ─── 정상 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn parse_standard_xref_offset() {
    let (data, eof_offset) = make_pdf_with_startxref(12345);
    assert_eq!(parse_startxref(&data, eof_offset).unwrap(), 12345);
}

#[test]
fn parse_xref_offset_zero() {
    // xref 테이블이 파일 맨 앞에 있는 경우
    let (data, eof_offset) = make_pdf_with_startxref(0);
    assert_eq!(parse_startxref(&data, eof_offset).unwrap(), 0);
}

#[test]
fn parse_large_xref_offset() {
    let (data, eof_offset) = make_pdf_with_startxref(1_234_567_890);
    assert_eq!(parse_startxref(&data, eof_offset).unwrap(), 1_234_567_890);
}

#[test]
fn parse_crlf_line_endings() {
    // startxref\r\n{number}\r\n 형식
    // %PDF-1.7\r\n(10) some body\r\n(11) startxref\r\n(11) 12345\r\n(7) = 39
    let data = b"%PDF-1.7\r\nsome body\r\nstartxref\r\n12345\r\n%%EOF\r\n";
    let eof_offset = 39;
    assert_eq!(parse_startxref(data, eof_offset).unwrap(), 12345);
}

#[test]
fn parse_last_of_multiple_startxref() {
    // incremental update: startxref가 두 번 등장 — 마지막 것을 사용
    // "startxref\n9\n"(12) + "startxref\n999\n"(14) = 26 → eof_offset=26
    let data = b"startxref\n9\nstartxref\n999\n%%EOF\n";
    let eof_offset = 26;
    assert_eq!(parse_startxref(data, eof_offset).unwrap(), 999);
}

#[test]
fn parse_startxref_close_to_eof() {
    // startxref 바로 다음에 1자리 오프셋 + %%EOF (최소 거리)
    let data = b"startxref\n9\n%%EOF\n";
    let eof_offset = 12; // len("startxref\n9\n") = 12
    assert_eq!(parse_startxref(data, eof_offset).unwrap(), 9);
}

// ─── 에러 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn reject_missing_startxref_keyword() {
    let data = b"some content without the keyword\n%%EOF\n";
    let eof_offset = 33; // len("some content without the keyword\n") = 33
    assert!(matches!(
        parse_startxref(data, eof_offset).unwrap_err(),
        ParseError::MissingStartXref
    ));
}

#[test]
fn reject_empty_after_startxref() {
    // startxref 바로 다음 줄에 %%EOF — 숫자가 없음
    // after_keyword = b"\n", 줄바꿈만 있고 숫자 없음
    let data = b"startxref\n%%EOF\n";
    let eof_offset = 10; // len("startxref\n") = 10
    assert!(matches!(
        parse_startxref(data, eof_offset).unwrap_err(),
        ParseError::InvalidStartXref { .. }
    ));
}

#[test]
fn reject_non_digit_after_startxref() {
    // 숫자 대신 문자열
    let data = b"startxref\nxyz\n%%EOF\n";
    let eof_offset = 14; // len("startxref\nxyz\n") = 14
    assert!(matches!(
        parse_startxref(data, eof_offset).unwrap_err(),
        ParseError::InvalidStartXref { .. }
    ));
}

#[test]
fn reject_startxref_beyond_search_window() {
    // startxref 뒤에 1025바이트 패딩 → 탐색 윈도(1024) 밖에 위치
    // data = "startxref\n12345\n"(16) + [x;1025] + "%%EOF\n"
    // eof_offset = 1041, search_start = 17 → startxref(위치 0)는 윈도 밖
    let mut data = b"startxref\n12345\n".to_vec();
    data.extend(vec![b'x'; 1025]);
    let eof_offset = data.len();
    data.extend_from_slice(b"%%EOF\n");
    assert!(matches!(
        parse_startxref(&data, eof_offset).unwrap_err(),
        ParseError::MissingStartXref
    ));
}

#[test]
fn reject_u64_overflow() {
    // u64::MAX + 1 = 18446744073709551616
    let num = "18446744073709551616";
    let middle = format!("startxref\n{}\n", num);
    let eof_offset = middle.len();
    let mut data = middle.into_bytes();
    data.extend_from_slice(b"%%EOF\n");
    assert!(matches!(
        parse_startxref(&data, eof_offset).unwrap_err(),
        ParseError::InvalidStartXref { .. }
    ));
}

#[test]
fn reject_space_before_number() {
    // 공백은 줄바꿈이 아니므로 허용하지 않음
    // after_keyword = b"\n 12345\n", '\n' 건너뜀 후 ' ' 발견 → 숫자 0개
    let data = b"startxref\n 12345\n%%EOF\n";
    let eof_offset = 17; // len("startxref\n 12345\n") = 17
    assert!(matches!(
        parse_startxref(data, eof_offset).unwrap_err(),
        ParseError::InvalidStartXref { .. }
    ));
}
