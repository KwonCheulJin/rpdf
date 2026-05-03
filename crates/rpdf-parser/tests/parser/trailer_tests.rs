use rpdf_core::types::ObjectId;
use rpdf_parser::{ParseError, parse_trailer};

/// trailer\n<<\nfields\n>>\nstartxref\n{xref}\n%%EOF\n 조합 데이터를 생성하고
/// %%EOF 시작 오프셋을 함께 반환한다.
fn make_trailer(fields: &str, xref: u64) -> (Vec<u8>, usize) {
    let dict_part = format!("trailer\n<<\n{}\n>>\n", fields);
    let sxref_part = format!("startxref\n{}\n", xref);
    let eof_offset = dict_part.len() + sxref_part.len();
    let mut data = dict_part.into_bytes();
    data.extend_from_slice(sxref_part.as_bytes());
    data.extend_from_slice(b"%%EOF\n");
    (data, eof_offset)
}

// ─── 정상 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn parse_minimal_trailer() {
    let (data, eof_offset) = make_trailer("/Size 42\n/Root 1 0 R", 9);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 42);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
    assert_eq!(result.trailer.info, None);
    assert_eq!(result.trailer.prev, None);
    assert_eq!(result.xref_offset, 9);
}

#[test]
fn parse_with_optional_info() {
    let (data, eof_offset) = make_trailer("/Size 10\n/Root 1 0 R\n/Info 2 0 R", 100);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 10);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
    assert_eq!(
        result.trailer.info,
        Some(ObjectId {
            number: 2,
            generation: 0
        })
    );
    assert_eq!(result.trailer.prev, None);
}

#[test]
fn parse_with_optional_prev() {
    let (data, eof_offset) = make_trailer("/Size 20\n/Root 3 0 R\n/Prev 567890", 12345);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 20);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 3,
            generation: 0
        }
    );
    assert_eq!(result.trailer.prev, Some(567890));
}

#[test]
fn parse_with_all_known_keys() {
    let (data, eof_offset) = make_trailer("/Size 100\n/Root 1 0 R\n/Info 2 0 R\n/Prev 999", 5000);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 100);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
    assert_eq!(
        result.trailer.info,
        Some(ObjectId {
            number: 2,
            generation: 0
        })
    );
    assert_eq!(result.trailer.prev, Some(999));
    assert_eq!(result.xref_offset, 5000);
}

#[test]
fn parse_crlf_line_endings() {
    // \r\n 줄바꿈 전체
    let data = b"trailer\r\n<<\r\n/Size 7\r\n/Root 1 0 R\r\n>>\r\nstartxref\r\n88\r\n%%EOF\r\n";
    // eof_offset: "trailer\r\n<<\r\n/Size 7\r\n/Root 1 0 R\r\n>>\r\nstartxref\r\n88\r\n" = ?
    // 9+4+9+14+4+11+4 = 55
    let eof_offset = data.iter().position(|&b| b == b'%').unwrap();
    let result = parse_trailer(data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 7);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
}

#[test]
fn parse_indented_dict() {
    // 키-값 들여쓰기 (탭 + 스페이스 혼합)
    let fields = "  /Size\t200\n\t/Root\t5 0 R";
    let (data, eof_offset) = make_trailer(fields, 42);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 200);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 5,
            generation: 0
        }
    );
}

#[test]
fn parse_with_unknown_keys_ignored() {
    // /ID, /Encrypt 등 미지원 키는 무시
    let fields = "/Size 50\n/Root 1 0 R\n/ID [<abc123> <def456>]\n/Encrypt 9 0 R";
    let (data, eof_offset) = make_trailer(fields, 300);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 50);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
    assert_eq!(result.trailer.info, None);
}

#[test]
fn parse_nested_dict_depth_counting() {
    // /SomeDict 안에 << >> — 깊이 카운팅이 올바르게 동작해야 함
    let fields = "/Size 30\n/Root 1 0 R\n/SomeDict << /Key (hello) >>";
    let (data, eof_offset) = make_trailer(fields, 0);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.trailer.size, 30);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
}

#[test]
fn parse_last_of_multiple_trailers() {
    // incremental update: trailer가 두 번 등장 — 마지막 것 사용
    let first = b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n100\n%%EOF\n";
    let second = b"trailer\n<< /Size 15 /Root 3 0 R /Prev 100 >>\nstartxref\n200\n%%EOF\n";
    let mut data = first.to_vec();
    data.extend_from_slice(second);
    let eof_offset = data.iter().rposition(|&b| b == b'%').unwrap();
    let result = parse_trailer(&data, eof_offset).unwrap();
    // 두 번째 trailer의 값이어야 한다
    assert_eq!(result.trailer.size, 15);
    assert_eq!(
        result.trailer.root,
        ObjectId {
            number: 3,
            generation: 0
        }
    );
    assert_eq!(result.trailer.prev, Some(100));
    assert_eq!(result.xref_offset, 200);
}

#[test]
fn parse_very_large_xref_offset() {
    // xref_offset이 큰 수 (실제 큰 파일에서 발생)
    let (data, eof_offset) = make_trailer("/Size 9999\n/Root 1 0 R", 1_234_567_890);
    let result = parse_trailer(&data, eof_offset).unwrap();
    assert_eq!(result.xref_offset, 1_234_567_890);
}

// ─── 에러 케이스 ──────────────────────────────────────────────────────────────

#[test]
fn reject_missing_trailer_keyword() {
    // trailer 없이 startxref + %%EOF만 있는 경우
    let data = b"startxref\n100\n%%EOF\n";
    let eof_offset = 14;
    assert!(matches!(
        parse_trailer(data, eof_offset).unwrap_err(),
        ParseError::MissingTrailer
    ));
}

#[test]
fn reject_xref_stream_pdf() {
    // xref 스트림 방식 (trailer 키워드 없음, xref_offset이 obj를 가리킴)
    let body = b"1 0 obj\n<< /Type /XRef /Size 5 >>\nstream\nendstream\nendobj\n";
    let sxref = b"startxref\n0\n";
    let eof_offset = body.len() + sxref.len();
    let mut data = body.to_vec();
    data.extend_from_slice(sxref);
    data.extend_from_slice(b"%%EOF\n");
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::XrefStreamUnsupported
    ));
}

#[test]
fn reject_truncated_dict_no_close() {
    // << 가 닫히지 않음 — >> 없음
    let data = b"trailer\n<< /Size 100 /Root 1 0 R\nstartxref\n100\n%%EOF\n";
    let eof_offset = data.iter().position(|&b| b == b'%').unwrap();
    assert!(matches!(
        parse_trailer(data, eof_offset).unwrap_err(),
        ParseError::MalformedTrailer { .. }
    ));
}

#[test]
fn reject_missing_size_key() {
    let (data, eof_offset) = make_trailer("/Root 1 0 R", 9);
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::MissingRequiredKey { key: "Size" }
    ));
}

#[test]
fn reject_missing_root_key() {
    let (data, eof_offset) = make_trailer("/Size 10", 9);
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::MissingRequiredKey { key: "Root" }
    ));
}

#[test]
fn reject_size_not_integer() {
    // /Size 값이 정수가 아닌 문자열
    let (data, eof_offset) = make_trailer("/Size (abc)\n/Root 1 0 R", 9);
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::MalformedTrailer { .. }
    ));
}

#[test]
fn reject_root_not_indirect_ref() {
    // /Root 값이 간접 참조가 아닌 정수
    let (data, eof_offset) = make_trailer("/Size 10\n/Root 5", 9);
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::InvalidObjectRef { .. }
    ));
}

#[test]
fn reject_trailer_too_large() {
    // 딕셔너리 내용이 4KB 초과 (4200바이트 hex string을 /Comment 값으로 삽입)
    // SEARCH_WINDOW(8192) > DICT_MAX_BYTES(4096) 이므로 "trailer"는 탐색 창 안에 있고
    // dict content(>4096)가 TrailerTooLarge를 유발한다.
    let dict_content = {
        let mut v = b"/Size 100\n/Root 1 0 R\n/Comment <".to_vec();
        v.extend(vec![b'a'; 4200]);
        v.push(b'>');
        v
    };
    let middle = {
        let mut m = b"trailer\n<<\n".to_vec();
        m.extend_from_slice(&dict_content);
        m.extend_from_slice(b"\n>>\nstartxref\n9\n");
        m
    };
    let eof_offset = middle.len();
    let mut data = middle;
    data.extend_from_slice(b"%%EOF\n");
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::TrailerTooLarge { .. }
    ));
}

#[test]
fn reject_trailer_beyond_search_window() {
    // trailer 키워드가 탐색 윈도(SEARCH_WINDOW = 8192바이트) 밖에 있음
    let trailer_part = b"trailer\n<< /Size 5 /Root 1 0 R >>\n";
    let padding = vec![b'x'; 8200]; // 8200바이트 패딩 (> 8192)
    let sxref = b"startxref\n0\n";
    let mut data = trailer_part.to_vec();
    data.extend_from_slice(&padding);
    let eof_offset = data.len() + sxref.len();
    data.extend_from_slice(sxref);
    data.extend_from_slice(b"%%EOF\n");
    assert!(matches!(
        parse_trailer(&data, eof_offset).unwrap_err(),
        ParseError::MissingTrailer | ParseError::XrefStreamUnsupported
    ));
}
