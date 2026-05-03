use std::collections::HashSet;

use rpdf_core::types::{XrefEntry, XrefTable};

use crate::error::ParseError;
use crate::objects::{parse_dictionary, parse_u64_val, peek_str, skip_whitespace};
use crate::trailer::{
    PdfTrailer, extract_trailer_fields, is_xref_stream as is_xref_stream_heuristic,
};
use crate::xref_stream::{is_xref_stream, parse_xref_stream};

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

/// `parse_xref_section` / `parse_xref_stream` 공통 반환 타입.
///
/// `parse_xref_chain`이 전통 xref 섹션과 xref 스트림을 동일하게 처리할 수 있도록
/// 두 경로의 반환값을 통일한다 (Hybrid Chain 지원).
pub(crate) struct XrefSectionResult {
    pub entries: Vec<(u32, XrefEntry)>,
    pub trailer: PdfTrailer,
    pub section_info: XrefSectionInfo,
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
///
/// Hybrid Chain: 각 섹션이 전통 xref 테이블이면 `parse_xref_section`,
/// xref 스트림(PDF 1.5+)이면 `parse_xref_stream`으로 처리한다.
/// 두 형식은 하나의 chain에 혼재할 수 있다 (ISO 32000 §7.5.8).
/// visited / depth 검사는 스트림 / 전통 구분 없이 동일하게 적용된다.
fn parse_xref_chain(data: &[u8], start_offset: u64) -> Result<ParsedXref, ParseError> {
    let mut table = XrefTable::new();
    let mut sections: Vec<XrefSectionInfo> = Vec::new();
    let mut visited: HashSet<u64> = HashSet::new();
    let mut first_trailer: Option<PdfTrailer> = None;
    let mut depth: usize = 0;
    let mut current = start_offset;

    loop {
        // visited 검사가 depth 검사보다 반드시 먼저 수행된다.
        // 이 순서는 계획서의 보장 조건을 지킨다:
        //   "순환 chain은 항상 XrefChainCycle로 보고된다."
        // depth를 먼저 검사하면 정확히 100개 고유 오프셋의 순환에서
        // XrefChainCycle 대신 XrefChainTooDeep이 반환되는 명세 위반이 발생한다.
        // (참고: mydocs/troubleshootings/xref-chain-check-order.md)
        if visited.contains(&current) {
            return Err(ParseError::XrefChainCycle { offset: current });
        }
        // 비순환 비정상 chain(100개 초과 고유 오프셋)에서만 TooDeep이 발생한다.
        if depth >= MAX_XREF_CHAIN_DEPTH {
            return Err(ParseError::XrefChainTooDeep {
                max_depth: MAX_XREF_CHAIN_DEPTH,
            });
        }
        visited.insert(current);
        depth += 1;

        // 파싱 기반 is_xref_stream으로 전통/스트림 분기
        let result = if is_xref_stream(data, current) {
            parse_xref_stream(data, current)?
        } else {
            parse_xref_section(data, current)?
        };
        sections.push(result.section_info);

        for (obj_num, entry) in result.entries {
            table.insert_if_absent(obj_num, entry);
        }

        // 가장 최신 섹션(첫 순회)의 trailer를 보존한다.
        // 이전 섹션들의 trailer는 /Prev chain을 잇는 용도이며,
        // /Root, /Info 등 문서 수준 메타는 최신 trailer가 권위를 가진다.
        if first_trailer.is_none() {
            first_trailer = Some(result.trailer.clone());
        }

        match result.trailer.prev {
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
/// 반환: `XrefSectionResult` — `parse_xref_stream`과 동일 타입으로 Hybrid Chain 지원.
fn parse_xref_section(data: &[u8], offset: u64) -> Result<XrefSectionResult, ParseError> {
    let file_size = data.len() as u64;
    if offset >= file_size {
        return Err(ParseError::XrefOffsetOutOfBounds { offset, file_size });
    }

    let start = offset as usize;

    // xref 스트림 감지 (방어적 체크 — chain에서 먼저 분기하므로 실제 도달하지 않아야 함)
    if is_xref_stream_heuristic(data, offset) {
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

    let entry_count = entries.len();
    Ok(XrefSectionResult {
        entries,
        trailer,
        section_info: XrefSectionInfo {
            offset,
            entry_count,
        },
    })
}

/// `data[pos..]`가 `"trailer"` 키워드로 시작한다고 가정하고 딕셔너리를 파싱한다.
///
/// xref 섹션 직후 순방향(forward) 파싱. `parse_trailer` (역방향)와는 별개 함수이다.
fn parse_trailer_at(data: &[u8], pos: usize) -> Result<PdfTrailer, ParseError> {
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

    let dict_start = after_ws + open_rel;

    let (dict, _consumed) =
        parse_dictionary(data, dict_start, 0).map_err(|_| ParseError::MalformedTrailer {
            reason: "trailer 딕셔너리 파싱 실패".to_string(),
        })?;

    extract_trailer_fields(&dict)
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
/// `\r\n` → 2, `\n` → 1, 그 외 → None
///
/// `\r` 단독은 거부한다. 항목 EOL 정책과 일관성을 위해 엄격하게 처리한다.
/// (`\r\n` 또는 `\n`만 허용 — 계획서 EOL 정책과 동일)
/// 향후 관용 처리가 필요한 실 파일이 발견되면 별도 Issue로 등록 후 도입 검토한다.
fn consume_newline(data: &[u8], pos: usize) -> Option<usize> {
    if pos >= data.len() {
        return None;
    }
    match data[pos] {
        b'\r' if pos + 1 < data.len() && data[pos + 1] == b'\n' => Some(2),
        b'\n' => Some(1),
        _ => None,
    }
}

// parse_trailer_at 은 xref.rs 전용 private fn 이므로 재노출하지 않는다.
// 테스트는 tests/parser/xref_tests.rs 에 있다.
