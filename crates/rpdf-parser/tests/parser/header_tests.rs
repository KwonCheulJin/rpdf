use rpdf_core::types::PdfVersion;
use rpdf_parser::{ParseError, parse_header};

// ─── 정상 케이스 ───────────────────────────────────────────────────────────────

#[test]
fn parse_pdf_1_7_lf() {
    let data = b"%PDF-1.7\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::V1_7);
    assert_eq!(h.byte_offset, 0);
    assert!(!h.has_binary_marker);
}

#[test]
fn parse_pdf_1_4_lf() {
    let data = b"%PDF-1.4\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::V1_4);
}

#[test]
fn parse_pdf_2_0_lf() {
    let data = b"%PDF-2.0\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::V2_0);
}

#[test]
fn parse_pdf_crlf_line_ending() {
    // \r\n 줄바꿈 — \r만 확인하면 통과
    let data = b"%PDF-1.7\r\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::V1_7);
}

#[test]
fn parse_unknown_version_becomes_other() {
    // 단일 digit이지만 스펙에 없는 버전 → Other
    let data = b"%PDF-3.0\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::Other { major: 3, minor: 0 });
}

#[test]
fn header_not_at_file_start_bom() {
    // UTF-8 BOM(3바이트) 앞에 붙은 경우
    let data = b"\xEF\xBB\xBF%PDF-1.7\n".to_vec();
    let h = parse_header(&data).unwrap();
    assert_eq!(h.version, PdfVersion::V1_7);
    assert_eq!(h.byte_offset, 3);
}

#[test]
fn header_after_garbage_prefix() {
    // 앞에 "garbage\n" 8바이트가 있는 경우
    let data = b"garbage\n%PDF-1.7\n";
    let h = parse_header(data).unwrap();
    assert_eq!(h.version, PdfVersion::V1_7);
    assert_eq!(h.byte_offset, 8);
}

#[test]
fn detect_binary_marker_present() {
    // 헤더 다음 줄에 0x80 이상 바이트 4개
    let data = b"%PDF-1.7\n%\x80\x81\x82\x83\n";
    let h = parse_header(data).unwrap();
    assert!(h.has_binary_marker);
}

#[test]
fn detect_binary_marker_absent() {
    // 헤더 다음 줄에 고바이트가 3개만 있음 — 마커로 인정 안 됨
    let data = b"%PDF-1.7\n%\x80\x81\x82\n";
    let h = parse_header(data).unwrap();
    assert!(!h.has_binary_marker);
}

// ─── 에러 케이스 ───────────────────────────────────────────────────────────────

#[test]
fn reject_non_pdf_bytes() {
    let data = b"not a pdf at all";
    let err = parse_header(data).unwrap_err();
    assert!(matches!(err, ParseError::HeaderNotFound { .. }));
}

#[test]
fn reject_empty_input() {
    let err = parse_header(b"").unwrap_err();
    assert!(matches!(err, ParseError::HeaderNotFound { .. }));
}

#[test]
fn reject_pdf_marker_only_no_version() {
    // %PDF- 뒤에 아무것도 없음
    let err = parse_header(b"%PDF-").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_version_no_line_ending() {
    // 줄바꿈 없이 EOF
    let err = parse_header(b"%PDF-1.7").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_version_followed_by_space() {
    // 공백은 줄바꿈이 아님
    let err = parse_header(b"%PDF-1.7 \n").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_two_digit_minor_version() {
    // 두 자리 minor — 스펙 위반
    let err = parse_header(b"%PDF-1.77\n").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_non_digit_version() {
    let err = parse_header(b"%PDF-A.B\n").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_version_missing_dot() {
    let err = parse_header(b"%PDF-17\n").unwrap_err();
    assert!(matches!(err, ParseError::InvalidVersion { .. }));
}

#[test]
fn reject_header_beyond_1kb() {
    // 1025바이트 뒤에 있는 %PDF- 는 탐색 범위 밖
    let mut data = vec![b'x'; 1025];
    data.extend_from_slice(b"%PDF-1.7\n");
    let err = parse_header(&data).unwrap_err();
    assert!(matches!(
        err,
        ParseError::HeaderNotFound {
            searched_bytes: 1024
        }
    ));
}
