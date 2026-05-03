//! PDF 기본 객체 파서 — trailer 딕셔너리 파싱에 필요한 최소 구현.
//!
//! 현재 지원 타입: 정수, 간접 참조, 이름, 중첩 딕셔너리, 배열, 리터럴 문자열, hex 문자열.
//! Task #4에서 나머지 타입(실수, 스트림, boolean, null)과 전체 객체 트리 파싱으로 확장될 예정.
use rpdf_core::types::ObjectId;

/// `data`가 `<<` 직후부터 시작할 때, 매칭되는 `>>` 의 첫 `>` 위치를 반환한다.
/// 중첩 딕셔너리(`<< >>`)와 괄호 문자열(`(...)`)을 올바르게 처리한다.
pub(crate) fn find_dict_close(data: &[u8]) -> Option<usize> {
    let mut depth: usize = 1;
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == b'<' && data[i + 1] == b'<' {
            depth += 1;
            i += 2;
        } else if data[i] == b'>' && data[i + 1] == b'>' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += 2;
        } else if data[i] == b'(' {
            i += skip_literal_string(&data[i..]);
        } else if data[i] == b'<' {
            // 단일 < — hex string: `>` 까지 스킵
            i += 1;
            while i < data.len() && data[i] != b'>' {
                i += 1;
            }
            if i < data.len() {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    None
}

/// `(` 로 시작하는 PDF 리터럴 문자열 전체 길이를 반환한다 (닫는 `)` 포함).
pub(crate) fn skip_literal_string(data: &[u8]) -> usize {
    // data[0] == b'('
    let mut depth: usize = 1;
    let mut i = 1;
    while i < data.len() && depth > 0 {
        match data[i] {
            b'\\' => i += 2,
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    i
}

/// 현재 위치의 값 하나를 건너뛰고 소비된 바이트 수를 반환한다.
/// 딕셔너리, 배열, 문자열, hex string, 이름, 숫자/간접 참조를 처리한다.
pub(crate) fn skip_value(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    if data.starts_with(b"<<") {
        let inner = &data[2..];
        let close = find_dict_close(inner).unwrap_or(inner.len());
        close + 4 // << ... >>
    } else if data[0] == b'(' {
        skip_literal_string(data)
    } else if data[0] == b'[' {
        let mut depth = 1usize;
        let mut i = 1;
        while i < data.len() && depth > 0 {
            match data[i] {
                b'[' => {
                    depth += 1;
                    i += 1;
                }
                b']' => {
                    depth -= 1;
                    i += 1;
                }
                b'(' => {
                    i += skip_literal_string(&data[i..]);
                }
                _ => {
                    i += 1;
                }
            }
        }
        i
    } else if data[0] == b'<' {
        // Hex string
        data.iter()
            .position(|&b| b == b'>')
            .map(|n| n + 1)
            .unwrap_or(data.len())
    } else if data[0] == b'/' {
        // Name
        1 + data[1..]
            .iter()
            .position(|&b| !is_name_char(b))
            .unwrap_or(data.len() - 1)
    } else if data.starts_with(b"true") {
        4
    } else if data.starts_with(b"false") {
        5
    } else if data.starts_with(b"null") {
        4
    } else {
        // 숫자 또는 간접 참조 (N G R)
        let end1 = data
            .iter()
            .position(|&b| is_whitespace(b) || b == b'/' || b == b'>' || b == b']')
            .unwrap_or(data.len());
        // 모두 숫자면 N G R 패턴인지 확인
        if data[..end1].iter().all(|b| b.is_ascii_digit()) && end1 > 0 {
            let i = end1 + skip_whitespace(&data[end1..]);
            if i < data.len() && data[i].is_ascii_digit() {
                let end2 = i + data[i..]
                    .iter()
                    .position(|&b| is_whitespace(b) || b == b'/' || b == b'>' || b == b']')
                    .unwrap_or(data.len() - i);
                let j = end2 + skip_whitespace(&data[end2..]);
                if j < data.len() && data[j] == b'R' {
                    return j + 1;
                }
            }
        }
        end1
    }
}

pub(crate) fn parse_u64_val(data: &[u8]) -> Option<(u64, usize)> {
    if data.is_empty() || !data[0].is_ascii_digit() {
        return None;
    }
    let end = data
        .iter()
        .position(|&b| !b.is_ascii_digit())
        .unwrap_or(data.len());
    let n: u64 = std::str::from_utf8(&data[..end]).ok()?.parse().ok()?;
    Some((n, end))
}

pub(crate) fn parse_indirect_ref(data: &[u8]) -> Option<(ObjectId, usize)> {
    if data.is_empty() || !data[0].is_ascii_digit() {
        return None;
    }
    let end1 = data
        .iter()
        .position(|&b| !b.is_ascii_digit())
        .unwrap_or(data.len());
    let number: u32 = std::str::from_utf8(&data[..end1]).ok()?.parse().ok()?;

    let mut i = end1 + skip_whitespace(&data[end1..]);

    if i >= data.len() || !data[i].is_ascii_digit() {
        return None;
    }
    let gen_start = i;
    while i < data.len() && data[i].is_ascii_digit() {
        i += 1;
    }
    let generation: u16 = std::str::from_utf8(&data[gen_start..i])
        .ok()?
        .parse()
        .ok()?;

    i += skip_whitespace(&data[i..]);

    if i >= data.len() || data[i] != b'R' {
        return None;
    }
    i += 1;

    // 'R' 다음이 이름 문자이면 더 긴 토큰의 일부 (예: "Rect")
    if i < data.len() && is_name_char(data[i]) {
        return None;
    }

    Some((ObjectId { number, generation }, i))
}

pub(crate) fn skip_whitespace(data: &[u8]) -> usize {
    data.iter()
        .position(|&b| !is_whitespace(b))
        .unwrap_or(data.len())
}

pub(crate) fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' | b'\x00')
}

pub(crate) fn is_name_char(b: u8) -> bool {
    b.is_ascii_graphic()
        && !matches!(
            b,
            b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'{' | b'}' | b'%'
        )
}

pub(crate) fn peek_str(data: &[u8], max: usize) -> String {
    String::from_utf8_lossy(&data[..data.len().min(max)]).to_string()
}
