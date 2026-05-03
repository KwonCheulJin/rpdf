use std::collections::HashSet;

use rpdf_core::types::{XrefEntry, XrefTable};

use crate::error::ParseError;
use crate::object_parser::{
    is_name_char, parse_indirect_ref, parse_u64_val, peek_str, skip_value, skip_whitespace,
};
use crate::trailer::{PdfTrailer, is_xref_stream};

/// `parse_xref` 반환값: 병합된 xref 테이블, 권위 있는 trailer, 섹션 메타데이터.
#[derive(Debug, Clone)]
pub struct ParsedXref {
    /// 모든 incremental update 섹션을 병합한 xref 테이블.
    /// 최신 섹션의 엔트리가 우선한다 (`insert_if_absent`).
    pub table: XrefTable,
    /// 가장 최신 섹션의 trailer (/Root, /Info 등의 권위 있는 소스).
    pub trailer: PdfTrailer,
    /// 순회한 각 섹션의 위치와 엔트리 수 (디버그용).
    pub sections: Vec<XrefSectionInfo>,
}

/// 단일 xref 섹션의 위치 정보 (디버그·진단용).
///
/// Task #3 시점에서는 offset과 entry_count만 추적한다.
/// Task #8 디버그 CLI에서 더 상세한 정보가 필요해지면
/// `section_size_bytes`, `object_id_range`, `subsection_count` 등을 추가 검토한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefSectionInfo {
    pub offset: u64,
    pub entry_count: usize,
}

/// xref chain의 최대 허용 깊이.
///
/// 일반 PDF는 1-3 단계, 형식 채우기 PDF도 10-50 단계 이내.
/// 100을 초과하는 chain은 비정상 또는 손상된 파일로 간주한다.
const MAX_XREF_CHAIN_DEPTH: usize = 100;

/// PDF 파일에서 xref 테이블과 trailer를 파싱한다.
///
/// `xref_offset`은 `parse_startxref`가 반환한 값을 그대로 전달한다.
/// `/Prev` 포인터를 따라 incremental update chain 전체를 순회하며
/// `XrefTable`에 병합한다.
///
/// # 에러
///
/// - `XrefOffsetOutOfBounds` — `xref_offset >= data.len()`
/// - `XrefStreamUnsupported` — xref 스트림 형식(PDF 1.5+) 감지
/// - `InvalidXrefAtOffset` — 지정 오프셋에 `xref` 키워드 없음
/// - `MalformedXref` — 항목 형식 오류
/// - `XrefChainCycle` — `/Prev` chain에 순환 참조
/// - `XrefChainTooDeep` — chain 깊이 초과
pub fn parse_xref(data: &[u8], xref_offset: u64) -> Result<ParsedXref, ParseError> {
    parse_xref_chain(data, xref_offset)
}

/// `/Prev` chain 전체를 순회하며 xref 테이블을 병합한다.
fn parse_xref_chain(data: &[u8], start_offset: u64) -> Result<ParsedXref, ParseError> {
    let mut table = XrefTable::new();
    let mut sections: Vec<XrefSectionInfo> = Vec::new();
    let mut visited: HashSet<u64> = HashSet::new();
    let mut first_trailer: Option<PdfTrailer> = None;
    let mut depth: usize = 0;
    let mut current = start_offset;

    loop {
        // 깊이 검사는 방문 집합 검사보다 먼저 수행한다:
        // 비순환 비정상 chain(모든 오프셋이 다르지만 100개 초과)에서 TooDeep을 발생시킨다.
        // 순환 chain은 visited 검사가 잡으므로 TooDeep에 도달하지 못한다.
        if depth >= MAX_XREF_CHAIN_DEPTH {
            return Err(ParseError::XrefChainTooDeep {
                max_depth: MAX_XREF_CHAIN_DEPTH,
            });
        }
        if visited.contains(&current) {
            return Err(ParseError::XrefChainCycle { offset: current });
        }
        visited.insert(current);
        depth += 1;

        let (entries, section_trailer) = parse_xref_section(data, current)?;
        let entry_count = entries.len();
        sections.push(XrefSectionInfo {
            offset: current,
            entry_count,
        });

        for (obj_num, entry) in entries {
            table.insert_if_absent(obj_num, entry);
        }

        // 가장 최신 섹션(첫 순회)의 trailer를 보존한다.
        // 이전 섹션들의 trailer는 /Prev chain을 잇는 용도이며,
        // /Root, /Info 등 문서 수준 메타는 최신 trailer가 권위를 가진다.
        if first_trailer.is_none() {
            first_trailer = Some(section_trailer.clone());
        }

        match section_trailer.prev {
            Some(prev_offset) => current = prev_offset,
            None => break,
        }
    }

    Ok(ParsedXref {
        table,
        trailer: first_trailer.expect("loop이 한 번도 실행되지 않음 — 논리 오류"),
        sections,
    })
}

/// 단일 xref 섹션(xref 키워드부터 trailer 딕셔너리까지)을 파싱한다.
///
/// 반환: (객체번호 → XrefEntry 맵, trailer)
fn parse_xref_section(
    data: &[u8],
    offset: u64,
) -> Result<(Vec<(u32, XrefEntry)>, PdfTrailer), ParseError> {
    let file_size = data.len() as u64;
    if offset >= file_size {
        return Err(ParseError::XrefOffsetOutOfBounds { offset, file_size });
    }

    let start = offset as usize;

    // xref 스트림 감지 (obj 패턴)
    if is_xref_stream(data, offset) {
        return Err(ParseError::XrefStreamUnsupported {
            xref_offset: offset,
        });
    }

    // "xref" 키워드 확인
    let bytes = &data[start..];
    if !bytes.starts_with(b"xref") {
        return Err(ParseError::InvalidXrefAtOffset {
            offset,
            found: peek_str(bytes, 16),
        });
    }

    let mut pos = start + 4; // "xref" 소비

    // "xref" 다음 줄바꿈 소비 (\n 또는 \r\n)
    pos += consume_newline(data, pos).ok_or_else(|| ParseError::MalformedXref {
        offset: pos as u64,
        reason: "xref 키워드 다음에 줄바꿈 없음".to_string(),
    })?;

    let mut entries: Vec<(u32, XrefEntry)> = Vec::new();

    // subsection 루프: "trailer" 키워드 또는 파일 끝까지
    loop {
        // 공백/줄바꿈 건너뜀
        let ws = skip_whitespace(&data[pos..]);
        pos += ws;

        if pos >= data.len() {
            break;
        }

        // "trailer" 키워드 감지
        if data[pos..].starts_with(b"trailer") {
            break;
        }

        // 다음 섹션 헤더 없으면 (숫자로 시작하지 않으면) 종료
        if !data[pos].is_ascii_digit() {
            break;
        }

        // 서브섹션 헤더: "<first_obj> <count>"
        let (first_obj, count, header_end) = parse_xref_subsection_header(data, pos)?;
        pos = header_end;

        // 항목 파싱
        for i in 0..count {
            if pos + 20 > data.len() {
                return Err(ParseError::MalformedXref {
                    offset: pos as u64,
                    reason: format!(
                        "항목 수 불일치: 헤더 선언 {count}개, {i}번째 항목에서 파일 끝"
                    ),
                });
            }

            let entry_bytes = &data[pos..pos + 20];
            let entry = parse_xref_entry(entry_bytes, pos as u64)?;
            let obj_num = first_obj + i as u32;
            entries.push((obj_num, entry));
            pos += 20;
        }
    }

    // "trailer" 키워드 다음 딕셔너리 파싱
    let trailer = parse_trailer_at(data, pos)?;

    Ok((entries, trailer))
}

/// `data[pos..]`가 `"trailer"` 키워드로 시작한다고 가정하고 딕셔너리를 파싱한다.
///
/// xref 섹션 직후 순방향(forward) 파싱. `parse_trailer` (역방향)와는 별개 함수이다.
fn parse_trailer_at(data: &[u8], pos: usize) -> Result<PdfTrailer, ParseError> {
    use crate::object_parser::find_dict_close;

    let bytes = &data[pos..];
    if !bytes.starts_with(b"trailer") {
        return Err(ParseError::MissingTrailer);
    }

    let after_kw = pos + 7; // "trailer" 소비
    let after_ws = after_kw + skip_whitespace(&data[after_kw..]);

    let open_rel = data[after_ws..]
        .windows(2)
        .position(|w| w == b"<<")
        .ok_or(ParseError::MissingTrailer)?;

    let dict_inner_start = after_ws + open_rel + 2;
    let dict_inner_data = &data[dict_inner_start..];

    let close_pos =
        find_dict_close(dict_inner_data).ok_or_else(|| ParseError::MalformedTrailer {
            reason: "trailer 딕셔너리가 닫히지 않음 (>> 없음)".to_string(),
        })?;

    parse_trailer_dict_fields(&dict_inner_data[..close_pos])
}

/// `<<` ~ `>>` 사이 내용(inner bytes)에서 trailer 필드를 추출한다.
///
/// `trailer.rs`의 `parse_dict_fields`와 동일 로직이지만, 모듈 간 결합을 피하기 위해
/// `xref.rs`에 비공개 함수로 둔다. Task #4에서 공통 추출 가치가 확인되면 이동 검토.
fn parse_trailer_dict_fields(data: &[u8]) -> Result<PdfTrailer, ParseError> {
    use rpdf_core::types::ObjectId;

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

/// 서브섹션 헤더 `<first_obj> <count>` 를 파싱한다.
///
/// 반환: `(first_obj, count, pos_after_header)`
fn parse_xref_subsection_header(
    data: &[u8],
    pos: usize,
) -> Result<(u32, usize, usize), ParseError> {
    let bytes = &data[pos..];

    let (first_obj, len1) = parse_u64_val(bytes).ok_or_else(|| ParseError::MalformedXref {
        offset: pos as u64,
        reason: "섹션 헤더: 시작 객체 번호 파싱 실패".to_string(),
    })?;

    let after_first = len1 + skip_whitespace(&bytes[len1..]);

    let (count, len2) =
        parse_u64_val(&bytes[after_first..]).ok_or_else(|| ParseError::MalformedXref {
            offset: (pos + after_first) as u64,
            reason: "섹션 헤더: 항목 수 파싱 실패".to_string(),
        })?;

    let after_count = after_first + len2;

    // 헤더 줄바꿈 소비 (\n 또는 \r\n 모두 허용)
    let nl = consume_newline(data, pos + after_count).ok_or_else(|| ParseError::MalformedXref {
        offset: (pos + after_count) as u64,
        reason: "섹션 헤더: 줄바꿈 없음".to_string(),
    })?;

    Ok((first_obj as u32, count as usize, pos + after_count + nl))
}

/// 20바이트 xref 항목을 파싱한다.
///
/// 포맷: `oooooooooo ggggg k EOL` (정확히 20바이트)
/// - `data[0..10]` — 10자리 오프셋 또는 다음 free 번호
/// - `data[10]` — 공백
/// - `data[11..16]` — 5자리 세대 번호
/// - `data[16]` — 공백
/// - `data[17]` — `n` (in-use) 또는 `f` (free)
/// - `data[18..20]` — EOL: `\r\n` 또는 ` \n`
fn parse_xref_entry(entry: &[u8], file_offset: u64) -> Result<XrefEntry, ParseError> {
    debug_assert_eq!(entry.len(), 20);

    // 오프셋/다음 free 번호 (10자리)
    let offset_str = std::str::from_utf8(&entry[0..10]).map_err(|_| ParseError::MalformedXref {
        offset: file_offset,
        reason: "항목 첫 10바이트가 UTF-8이 아님".to_string(),
    })?;
    let num_val: u64 = offset_str.trim_start_matches('0').parse().unwrap_or(0); // "0000000000" 이면 0 반환

    // 공백 검사
    if entry[10] != b' ' {
        return Err(ParseError::MalformedXref {
            offset: file_offset + 10,
            reason: format!("항목 10번 위치에 공백 기대, 발견: {:?}", entry[10] as char),
        });
    }

    // 세대 번호 (5자리)
    let gen_str = std::str::from_utf8(&entry[11..16]).map_err(|_| ParseError::MalformedXref {
        offset: file_offset + 11,
        reason: "항목 세대 번호가 UTF-8이 아님".to_string(),
    })?;
    let generation: u16 = gen_str.trim_start_matches('0').parse().unwrap_or(0);

    // 공백 검사
    if entry[16] != b' ' {
        return Err(ParseError::MalformedXref {
            offset: file_offset + 16,
            reason: format!("항목 16번 위치에 공백 기대, 발견: {:?}", entry[16] as char),
        });
    }

    // 항목 종류
    let kind = entry[17];

    // EOL 검사: \r\n 또는 ' '\n 만 허용 (20바이트 고정 유지)
    match &entry[18..20] {
        b"\r\n" | b" \n" => {}
        eol => {
            return Err(ParseError::MalformedXref {
                offset: file_offset + 18,
                reason: format!("비표준 항목 EOL: {:?} (\\r\\n 또는 ' '\\n 만 허용)", eol),
            });
        }
    }

    match kind {
        b'n' => Ok(XrefEntry::InUse {
            offset: num_val,
            generation,
        }),
        b'f' => Ok(XrefEntry::Free {
            next_free_obj_num: num_val as u32,
            generation,
        }),
        other => Err(ParseError::MalformedXref {
            offset: file_offset + 17,
            reason: format!("알 수 없는 항목 타입: {:?}", other as char),
        }),
    }
}

/// `data[pos]`부터 줄바꿈을 소비하고 소비한 바이트 수를 반환한다.
///
/// `\r\n` → 2, `\n` → 1, 줄바꿈 없음 → None
fn consume_newline(data: &[u8], pos: usize) -> Option<usize> {
    if pos >= data.len() {
        return None;
    }
    match data[pos] {
        b'\n' => Some(1),
        b'\r' => {
            if pos + 1 < data.len() && data[pos + 1] == b'\n' {
                Some(2)
            } else {
                Some(1)
            }
        }
        _ => None,
    }
}

// ─── 내부 모듈 재노출 ────────────────────────────────────────────────────────
// parse_trailer_at 은 xref.rs 전용 private fn 이므로 재노출하지 않는다.

// ─── 단위 테스트 (파일 내부) ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ParseError;

    // ── 합성 데이터 헬퍼 ────────────────────────────────────────────────────

    /// (offset_or_next, generation, kind) → 정확히 20바이트 xref 항목
    fn make_entry(offset_or_next: u64, generation: u16, kind: char) -> Vec<u8> {
        format!("{offset_or_next:010} {generation:05} {kind}\r\n").into_bytes()
    }

    /// 완전한 xref 섹션 바이트 생성 (단일 서브섹션)
    fn make_xref_section(
        start_obj: u32,
        entries: &[(u64, u16, char)],
        trailer_dict: &str,
    ) -> Vec<u8> {
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

    // ── 기본 파싱 테스트 ─────────────────────────────────────────────────────

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
        // "0 0" 서브섹션: 항목 없음, 에러가 아님
        let mut buf = Vec::new();
        buf.extend_from_slice(b"xref\n0 0\ntrailer\n<< /Size 0 /Root 1 0 R >>");
        let result = parse_xref(&buf, 0).unwrap();
        assert_eq!(result.table.len(), 0);
    }

    #[test]
    fn parse_crlf_subsection_header() {
        // 섹션 헤더에 \r\n 허용
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
        // 객체 번호가 0부터 시작하지 않는 서브섹션
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

    // ── 에러 케이스 테스트 ───────────────────────────────────────────────────

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
        // startxref = 0 → 오프셋 0에 %PDF- 헤더 → InvalidXrefAtOffset
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
        entry.push(b'\n'); // \n 단독 (19바이트 효과)
        entry.push(b'\n'); // 20바이트 맞추기
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
        // 항목 종류 'x' (n 또는 f 이외)
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
}
