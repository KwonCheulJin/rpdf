use rpdf_core::types::XrefEntry;
use rpdf_parser::{ParseError, parse_xref};

// ── 합성 데이터 헬퍼 ─────────────────────────────────────────────────────────

/// (offset_or_next, generation, kind) → 정확히 20바이트 xref 항목 (\r\n EOL)
fn make_entry(offset_or_next: u64, generation: u16, kind: char) -> Vec<u8> {
    format!("{offset_or_next:010} {generation:05} {kind}\r\n").into_bytes()
}

/// 완전한 xref 섹션 바이트 생성 (단일 서브섹션)
fn make_xref_section(start_obj: u32, entries: &[(u64, u16, char)], trailer_dict: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("{} {}\n", start_obj, entries.len()).as_bytes());
    for &(off, genr, k) in entries {
        buf.extend_from_slice(&make_entry(off, genr, k));
    }
    buf.extend_from_slice(b"trailer\n");
    buf.extend_from_slice(trailer_dict.as_bytes());
    buf
}

/// `count`개 xref 섹션을 chain으로 연결한 합성 데이터를 생성한다.
///
/// - `cyclic=true`: 마지막 섹션의 /Prev가 첫 번째 섹션 오프셋을 가리킴 (순환 chain)
/// - `cyclic=false`: 마지막 섹션에 /Prev 없음 (정상 chain)
///
/// 각 섹션의 크기는 /Prev 값에 따라 달라지므로 안정될 때까지 반복 계산한다.
/// 반환: (data 바이트, start_offset)
fn build_chain(count: usize, cyclic: bool) -> (Vec<u8>, u64) {
    let section_bytes = |prev: Option<u64>| -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"xref\n0 1\n");
        buf.extend_from_slice(b"0000000000 65535 f\r\n");
        buf.extend_from_slice(b"trailer\n");
        match prev {
            Some(p) => {
                buf.extend_from_slice(format!("<< /Size 1 /Root 1 0 R /Prev {p} >>").as_bytes())
            }
            None => buf.extend_from_slice(b"<< /Size 1 /Root 1 0 R >>"),
        }
        buf
    };

    // 오프셋 수렴 반복 (i+1번 섹션의 오프셋을 i번 섹션의 /Prev로 사용)
    let mut offsets = vec![0u64; count];
    for _ in 0..10 {
        let mut new_offsets = vec![0u64; count];
        let mut cur = 0u64;
        for i in 0..count {
            new_offsets[i] = cur;
            let prev = if i + 1 < count {
                Some(offsets[i + 1])
            } else if cyclic {
                Some(offsets[0])
            } else {
                None
            };
            cur += section_bytes(prev).len() as u64;
        }
        if new_offsets == offsets {
            break;
        }
        offsets = new_offsets;
    }

    // 실제 데이터 생성
    let mut data = Vec::new();
    for i in 0..count {
        let prev = if i + 1 < count {
            Some(offsets[i + 1])
        } else if cyclic {
            Some(offsets[0])
        } else {
            None
        };
        data.extend_from_slice(&section_bytes(prev));
    }

    (data, offsets[0])
}

// ── 기본 파싱 테스트 ─────────────────────────────────────────────────────────

#[test]
fn parse_single_inuse_entry() {
    let section = make_xref_section(
        0,
        &[(0, 65535, 'f'), (9, 0, 'n')],
        "<< /Size 2 /Root 1 0 R >>",
    );
    let result = parse_xref(&section, 0).unwrap();
    assert_eq!(result.table.len(), 2);
    assert!(matches!(
        result.table.get(1),
        Some(XrefEntry::InUse {
            offset: 9,
            generation: 0
        })
    ));
    assert!(matches!(
        result.table.get(0),
        Some(XrefEntry::Free {
            next_free_obj_num: 0,
            generation: 65535
        })
    ));
}

#[test]
fn parse_multiple_entries() {
    let entries = [
        (0u64, 65535u16, 'f'),
        (9, 0, 'n'),
        (58, 0, 'n'),
        (200, 0, 'n'),
    ];
    let section = make_xref_section(0, &entries, "<< /Size 4 /Root 1 0 R >>");
    let result = parse_xref(&section, 0).unwrap();
    assert_eq!(result.table.len(), 4);
    assert_eq!(result.sections.len(), 1);
    assert_eq!(result.sections[0].entry_count, 4);
}

#[test]
fn parse_free_and_inuse_mixed() {
    let section = make_xref_section(
        0,
        &[(5, 1, 'f'), (100, 0, 'n'), (0, 65535, 'f')],
        "<< /Size 3 /Root 2 0 R >>",
    );
    let result = parse_xref(&section, 0).unwrap();
    assert!(matches!(result.table.get(0), Some(XrefEntry::Free { .. })));
    assert!(matches!(result.table.get(1), Some(XrefEntry::InUse { .. })));
    assert!(matches!(result.table.get(2), Some(XrefEntry::Free { .. })));
}

#[test]
fn parse_trailer_fields_extracted() {
    let section = make_xref_section(
        0,
        &[(0, 65535, 'f'), (9, 0, 'n')],
        "<< /Size 2 /Root 1 0 R /Info 2 0 R >>",
    );
    let result = parse_xref(&section, 0).unwrap();
    assert_eq!(result.trailer.size, 2);
    assert_eq!(result.trailer.root.number, 1);
    assert_eq!(result.trailer.info.map(|o| o.number), Some(2));
    assert_eq!(result.trailer.prev, None);
}

#[test]
fn parse_empty_subsection_ok() {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n0 0\ntrailer\n<< /Size 0 /Root 1 0 R >>");
    let result = parse_xref(&buf, 0).unwrap();
    assert_eq!(result.table.len(), 0);
}

#[test]
fn parse_crlf_subsection_header() {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\r\n0 1\r\n");
    buf.extend_from_slice(&make_entry(9, 0, 'n'));
    buf.extend_from_slice(b"trailer\n<< /Size 1 /Root 1 0 R >>");
    let result = parse_xref(&buf, 0).unwrap();
    assert_eq!(result.table.len(), 1);
}

#[test]
fn parse_space_newline_eol_entry() {
    // 항목 EOL: ' '\n 허용
    let mut entry = format!("{:010} {:05} {}", 9u64, 0u16, 'n').into_bytes();
    entry.extend_from_slice(b" \n");
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n0 1\n");
    buf.extend_from_slice(&entry);
    buf.extend_from_slice(b"trailer\n<< /Size 1 /Root 1 0 R >>");
    let result = parse_xref(&buf, 0).unwrap();
    assert_eq!(result.table.len(), 1);
}

#[test]
fn parse_nonzero_start_obj_num() {
    let section = make_xref_section(
        5,
        &[(500, 0, 'n'), (600, 0, 'n')],
        "<< /Size 7 /Root 5 0 R >>",
    );
    let result = parse_xref(&section, 0).unwrap();
    assert!(result.table.get(5).is_some());
    assert!(result.table.get(6).is_some());
    assert!(result.table.get(0).is_none());
}

// ── 에러 케이스 테스트 ───────────────────────────────────────────────────────

#[test]
fn reject_offset_out_of_bounds() {
    let data = b"short";
    let err = parse_xref(data, 100).unwrap_err();
    assert!(
        matches!(
            err,
            ParseError::XrefOffsetOutOfBounds {
                offset: 100,
                file_size: 5
            }
        ),
        "unexpected: {err:?}"
    );
}

#[test]
fn reject_invalid_xref_at_offset_zero_pdf_header() {
    let data = b"%PDF-1.4\nsome content\nstartxref\n0\n%%EOF\n";
    let err = parse_xref(data, 0).unwrap_err();
    assert!(
        matches!(err, ParseError::InvalidXrefAtOffset { offset: 0, .. }),
        "unexpected: {err:?}"
    );
}

#[test]
fn reject_malformed_entry_non_standard_eol() {
    // \n 단독 EOL은 비표준
    let mut entry = format!("{:010} {:05} {}", 9u64, 0u16, 'n').into_bytes();
    entry.push(b'\n');
    entry.push(b'\n');
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n0 1\n");
    buf.extend_from_slice(&entry);
    buf.extend_from_slice(b"trailer\n<< /Size 1 /Root 1 0 R >>");
    let err = parse_xref(&buf, 0).unwrap_err();
    assert!(
        matches!(err, ParseError::MalformedXref { .. }),
        "unexpected: {err:?}"
    );
}

#[test]
fn reject_malformed_entry_unknown_type() {
    let entry = format!("{:010} {:05} x\r\n", 9u64, 0u16).into_bytes();
    assert_eq!(entry.len(), 20);
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n0 1\n");
    buf.extend_from_slice(&entry);
    buf.extend_from_slice(b"trailer\n<< /Size 1 /Root 1 0 R >>");
    let err = parse_xref(&buf, 0).unwrap_err();
    assert!(
        matches!(err, ParseError::MalformedXref { .. }),
        "unexpected: {err:?}"
    );
}

#[test]
fn reject_offset_equal_to_file_size() {
    let data = b"xref\n";
    let err = parse_xref(data, data.len() as u64).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefOffsetOutOfBounds { .. }),
        "unexpected: {err:?}"
    );
}

// ── xref chain 순회 테스트 (Checkpoint C) ───────────────────────────────────

#[test]
fn chain_no_prev_is_single_section() {
    let section = make_xref_section(
        0,
        &[(0, 65535, 'f'), (9, 0, 'n')],
        "<< /Size 2 /Root 1 0 R >>",
    );
    let result = parse_xref(&section, 0).unwrap();
    assert_eq!(result.sections.len(), 1);
    assert_eq!(result.sections[0].offset, 0);
    assert_eq!(result.sections[0].entry_count, 2);
}

#[test]
fn chain_two_sections_prev_traversal() {
    // [섹션0: obj1=InUse] → [섹션1: obj2=InUse]
    // 섹션0의 /Prev = 섹션1의 오프셋
    let mut section1 = make_xref_section(2, &[(200, 0, 'n')], "<< /Size 3 /Root 1 0 R >>");
    let s1_offset = section1.len() as u64; // 섹션1이 데이터 끝에 위치
    // 실제로는 섹션0을 먼저 append하고 섹션1을 그 뒤에 붙임
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 3 /Root 1 0 R /Prev {s1_offset} >>"),
    );
    let s0_size = section0.len();
    let s1_offset_actual = s0_size as u64;
    // 섹션1의 오프셋이 s0_size이므로, 섹션0의 /Prev 재생성
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 3 /Root 1 0 R /Prev {s1_offset_actual} >>"),
    );
    let mut data = section0;
    data.extend_from_slice(&section1);

    let result = parse_xref(&data, 0).unwrap();
    assert_eq!(result.sections.len(), 2);
    assert_eq!(result.table.len(), 2);
    // 두 섹션의 항목 모두 포함
    assert!(result.table.get(1).is_some()); // 섹션0의 obj1
    assert!(result.table.get(2).is_some()); // 섹션1의 obj2
}

#[test]
fn chain_latest_entry_wins_for_same_object() {
    // 섹션0(최신)과 섹션1(이전)이 동일 객체 번호(1)를 다른 오프셋으로 가짐
    // 섹션0의 값(100)이 섹션1의 값(999)보다 우선해야 함
    let section1 = make_xref_section(1, &[(999, 0, 'n')], "<< /Size 2 /Root 1 0 R >>");
    let s1_offset = section1.len() as u64;
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 2 /Root 1 0 R /Prev {s1_offset} >>"),
    );
    // s1_offset 재계산
    let s1_offset = section0.len() as u64;
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 2 /Root 1 0 R /Prev {s1_offset} >>"),
    );
    let mut data = section0;
    data.extend_from_slice(&section1);

    let result = parse_xref(&data, 0).unwrap();
    assert_eq!(result.table.len(), 1);
    // 섹션0(최신)의 오프셋 100이 채택됨
    assert!(matches!(
        result.table.get(1),
        Some(XrefEntry::InUse { offset: 100, .. })
    ));
}

#[test]
fn chain_first_trailer_is_authority() {
    // 가장 최신 섹션(섹션0)의 /Root가 반환된 trailer의 /Root여야 한다
    let section1 = make_xref_section(1, &[(999, 0, 'n')], "<< /Size 2 /Root 5 0 R >>");
    let s1_offset = section1.len() as u64;
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 2 /Root 1 0 R /Prev {s1_offset} >>"),
    );
    let s1_offset = section0.len() as u64;
    let section0 = make_xref_section(
        1,
        &[(100, 0, 'n')],
        &format!("<< /Size 2 /Root 1 0 R /Prev {s1_offset} >>"),
    );
    let mut data = section0;
    data.extend_from_slice(&section1);

    let result = parse_xref(&data, 0).unwrap();
    // 섹션0(최신)의 /Root = 1, 섹션1(이전)의 /Root = 5
    // 최신 trailer 우선 → root.number = 1
    assert_eq!(result.trailer.root.number, 1);
}

#[test]
fn chain_prev_beyond_file_returns_out_of_bounds() {
    // /Prev가 파일 크기를 넘는 경우
    let section = make_xref_section(
        0,
        &[(0, 65535, 'f')],
        "<< /Size 1 /Root 1 0 R /Prev 99999 >>",
    );
    let err = parse_xref(&section, 0).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefOffsetOutOfBounds { .. }),
        "unexpected: {err:?}"
    );
}

// ── 회귀 테스트: xref chain 검사 순서 (mydocs/troubleshootings/xref-chain-check-order.md) ─

/// 정확히 100개 고유 오프셋으로 이루어진 순환 chain은 XrefChainCycle을 반환해야 한다.
///
/// visited 검사를 depth 검사보다 먼저 수행하지 않으면, 깊이가 100에 도달하는 순간
/// XrefChainTooDeep이 잘못 반환된다 (계획서 명세 위반).
#[test]
fn cycle_with_exactly_100_unique_offsets_returns_cycle_not_too_deep() {
    let (data, start) = build_chain(100, true);
    let err = parse_xref(&data, start).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefChainCycle { .. }),
        "100개 고유 오프셋 순환 chain은 XrefChainCycle이어야 함, 실제: {err:?}"
    );
}

/// 101개의 서로 다른 오프셋으로 이루어진 비순환 chain은 XrefChainTooDeep을 반환해야 한다.
#[test]
fn non_cyclic_chain_of_101_returns_too_deep() {
    let (data, start) = build_chain(101, false);
    let err = parse_xref(&data, start).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefChainTooDeep { max_depth: 100 }),
        "101개 비순환 chain은 XrefChainTooDeep이어야 함, 실제: {err:?}"
    );
}
