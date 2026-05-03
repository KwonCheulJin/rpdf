//! PDF 객체 파서 — 모든 기본 객체 타입 파싱.
//!
//! **Checkpoint B**: Boolean, Null, Integer, Real.
//! **Checkpoint C**: LiteralString, HexString, Name, Reference, Array, Dictionary.
//! **Checkpoint D**: Stream, IndirectObject (todo).
//!
//! **범위 외**:
//! - 스트림 필터 디코딩 (Task #5)
//! - 문자열 인코딩 해석 (Task #7)
//! - 간접 참조 해결 (Task #7)
use crate::error::ParseError;
use crate::object_parser::{is_name_char, is_whitespace, peek_str};
use rpdf_core::types::{IndirectObject, ObjectId, PdfDict, PdfObject, PdfStream};

/// 배열·딕셔너리 재귀 허용 최대 깊이. (ISO 32000 §7.3.6, §7.3.7)
pub(crate) const MAX_OBJECT_DEPTH: usize = 50;

/// 화이트스페이스와 주석(`%` ~ 줄 끝)을 건너뛴다.
///
/// `start`부터 탐색하여 첫 번째 비-화이트스페이스·비-주석 바이트의 절대 위치를 반환한다.
///
/// - PDF 표준 화이트스페이스 (ISO 32000 §7.2.2): `\0 \t \n \x0C \r ' '`
/// - 주석 (ISO 32000 §7.2.3): `%`부터 줄바꿈(`\n` 또는 `\r`) 직전까지
pub(crate) fn skip_whitespace_and_comments(data: &[u8], start: usize) -> usize {
    let mut pos = start;
    loop {
        if pos >= data.len() {
            return pos;
        }
        match data[pos] {
            b'\0' | b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => pos += 1,
            b'%' => {
                pos += 1;
                while pos < data.len() && data[pos] != b'\n' && data[pos] != b'\r' {
                    pos += 1;
                }
            }
            _ => return pos,
        }
    }
}

/// `data[offset..]`에서 boolean 키워드 (`true` 또는 `false`)를 파싱한다.
///
/// 키워드 직후에 이름 문자(name char)가 오면 다른 토큰의 일부로 간주하여
/// `InvalidObject` 반환 (예: `truefoo`, `falseX`).
///
/// ISO 32000-1 §7.3.2
pub(crate) fn parse_boolean(data: &[u8], offset: usize) -> Result<(bool, usize), ParseError> {
    let slice = &data[offset..];

    if slice.starts_with(b"true") {
        if slice.get(4).is_some_and(|&b| is_name_char(b)) {
            return Err(ParseError::InvalidObject {
                offset,
                reason: format!(
                    "boolean `true` followed by name char: {:?}",
                    peek_str(slice, 8)
                ),
            });
        }
        return Ok((true, 4));
    }

    if slice.starts_with(b"false") {
        if slice.get(5).is_some_and(|&b| is_name_char(b)) {
            return Err(ParseError::InvalidObject {
                offset,
                reason: format!(
                    "boolean `false` followed by name char: {:?}",
                    peek_str(slice, 9)
                ),
            });
        }
        return Ok((false, 5));
    }

    Err(ParseError::InvalidObject {
        offset,
        reason: format!("expected boolean keyword, found {:?}", peek_str(slice, 8)),
    })
}

/// `data[offset..]`에서 `null` 키워드를 파싱한다.
///
/// 키워드 직후에 이름 문자가 오면 `InvalidObject` 반환.
/// 성공 시 소비된 바이트 수(항상 4)를 반환한다.
///
/// ISO 32000-1 §7.3.9
pub(crate) fn parse_null(data: &[u8], offset: usize) -> Result<usize, ParseError> {
    let slice = &data[offset..];

    if slice.starts_with(b"null") {
        if slice.get(4).is_some_and(|&b| is_name_char(b)) {
            return Err(ParseError::InvalidObject {
                offset,
                reason: format!("`null` followed by name char: {:?}", peek_str(slice, 8)),
            });
        }
        return Ok(4);
    }

    Err(ParseError::InvalidObject {
        offset,
        reason: format!("expected null keyword, found {:?}", peek_str(slice, 8)),
    })
}

/// `data[offset..]`에서 정수를 파싱한다.
///
/// 문법: `[+|-]?[0-9]+`. i64 범위를 초과하면 `InvalidObject` 반환.
/// 소수점(`.`)에서 멈추며 실수 판별은 하지 않는다 (실수는 `parse_number` 사용).
///
/// ISO 32000-1 §7.3.3
pub(crate) fn parse_integer(data: &[u8], offset: usize) -> Result<(i64, usize), ParseError> {
    let slice = &data[offset..];

    if slice.is_empty() {
        return Err(ParseError::InvalidObject {
            offset,
            reason: "empty input".to_string(),
        });
    }

    let mut pos = 0;

    if slice[0] == b'+' || slice[0] == b'-' {
        pos += 1;
    }

    if pos >= slice.len() || !slice[pos].is_ascii_digit() {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!("expected digit, found {:?}", peek_str(&slice[pos..], 4)),
        });
    }

    while pos < slice.len() && slice[pos].is_ascii_digit() {
        pos += 1;
    }

    let num_str = std::str::from_utf8(&slice[..pos]).map_err(|_| ParseError::InvalidObject {
        offset,
        reason: "non-utf8 in number".to_string(),
    })?;

    let n = num_str
        .parse::<i64>()
        .map_err(|e| ParseError::InvalidObject {
            offset,
            reason: format!("integer out of range: {e}"),
        })?;

    Ok((n, pos))
}

/// `data[offset..]`에서 숫자(정수 또는 실수)를 파싱한다.
///
/// 소수점(`.`)의 유무로 `PdfObject::Integer(i64)` 또는 `PdfObject::Real(f64)`를 자동 판별한다.
///
/// - 정수: `[+|-]?[0-9]+`
/// - 실수: `[+|-]?[0-9]*\.[0-9]*` (소수점 앞 또는 뒤 숫자 생략 가능: `.5`, `3.`)
///
/// PDF 스펙(ISO 32000 §7.3.3)은 지수 표기법(`1e5`, `1.5e2`)을 지원하지 않는다.
/// `e`/`E` 앞까지만 인식하고 나머지는 별도 토큰으로 남긴다.
pub(crate) fn parse_number(data: &[u8], offset: usize) -> Result<(PdfObject, usize), ParseError> {
    let slice = &data[offset..];

    if slice.is_empty() {
        return Err(ParseError::InvalidObject {
            offset,
            reason: "empty input".to_string(),
        });
    }

    let mut pos = 0;

    if pos < slice.len() && (slice[pos] == b'+' || slice[pos] == b'-') {
        pos += 1;
    }

    let int_start = pos;
    while pos < slice.len() && slice[pos].is_ascii_digit() {
        pos += 1;
    }
    let has_int_digits = pos > int_start;

    if pos < slice.len() && slice[pos] == b'.' {
        pos += 1;
        let frac_start = pos;
        while pos < slice.len() && slice[pos].is_ascii_digit() {
            pos += 1;
        }
        let has_frac_digits = pos > frac_start;

        if !has_int_digits && !has_frac_digits {
            return Err(ParseError::InvalidObject {
                offset,
                reason: "real number has no digits (just '.')".to_string(),
            });
        }

        let num_str =
            std::str::from_utf8(&slice[..pos]).map_err(|_| ParseError::InvalidObject {
                offset,
                reason: "non-utf8 in number".to_string(),
            })?;

        let f = num_str
            .parse::<f64>()
            .map_err(|e| ParseError::InvalidObject {
                offset,
                reason: format!("real number parse error: {e}"),
            })?;

        return Ok((PdfObject::Real(f), pos));
    }

    if !has_int_digits {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!("expected number, found {:?}", peek_str(slice, 8)),
        });
    }

    // 정수 분기: parse_integer에 위임 (부호 + 정수 자리수 재파싱)
    let (n, consumed) = parse_integer(data, offset)?;
    Ok((PdfObject::Integer(n), consumed))
}

/// `data[offset..]`에서 PDF 리터럴 문자열 `(...)` 을 파싱한다.
///
/// 이스케이프 시퀀스, 중첩 괄호, EOL 정규화를 처리한다.
/// 결과는 raw bytes (인코딩 해석 없음). ISO 32000-1 §7.3.4.2
pub(crate) fn parse_literal_string(
    data: &[u8],
    offset: usize,
) -> Result<(Vec<u8>, usize), ParseError> {
    let slice = &data[offset..];
    if slice.is_empty() || slice[0] != b'(' {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!(
                "expected '(' for literal string, found {:?}",
                peek_str(slice, 4)
            ),
        });
    }

    let mut out = Vec::new();
    let mut pos = 1; // '(' 다음부터
    let mut depth: usize = 1;

    while pos < slice.len() && depth > 0 {
        match slice[pos] {
            b'(' => {
                depth += 1;
                out.push(b'(');
                pos += 1;
            }
            b')' => {
                depth -= 1;
                if depth > 0 {
                    out.push(b')');
                }
                pos += 1;
            }
            b'\\' => {
                pos += 1;
                if pos >= slice.len() {
                    break;
                }
                match slice[pos] {
                    b'n' => {
                        out.push(b'\n');
                        pos += 1;
                    }
                    b'r' => {
                        out.push(b'\r');
                        pos += 1;
                    }
                    b't' => {
                        out.push(b'\t');
                        pos += 1;
                    }
                    b'b' => {
                        out.push(0x08);
                        pos += 1;
                    }
                    b'f' => {
                        out.push(0x0C);
                        pos += 1;
                    }
                    b'\\' => {
                        out.push(b'\\');
                        pos += 1;
                    }
                    b'(' => {
                        out.push(b'(');
                        pos += 1;
                    }
                    b')' => {
                        out.push(b')');
                        pos += 1;
                    }
                    b'\r' => {
                        // \<CR> 또는 \<CR><LF> — line continuation, 무시
                        pos += 1;
                        if pos < slice.len() && slice[pos] == b'\n' {
                            pos += 1;
                        }
                    }
                    b'\n' => {
                        // \<LF> — line continuation, 무시
                        pos += 1;
                    }
                    c if c.is_ascii_digit() => {
                        // 8진수 1~3자리
                        let mut val: u32 = (c - b'0') as u32;
                        pos += 1;
                        for _ in 0..2 {
                            if pos < slice.len() && slice[pos].is_ascii_digit() {
                                val = val * 8 + (slice[pos] - b'0') as u32;
                                pos += 1;
                            } else {
                                break;
                            }
                        }
                        out.push((val & 0xFF) as u8);
                    }
                    _ => {
                        // 그 외 \X — \ 무시, X 그대로
                        out.push(slice[pos]);
                        pos += 1;
                    }
                }
            }
            b'\r' => {
                // EOL 정규화: \r 또는 \r\n → \n (ISO 32000 §7.3.4.2)
                out.push(b'\n');
                pos += 1;
                if pos < slice.len() && slice[pos] == b'\n' {
                    pos += 1;
                }
            }
            b => {
                out.push(b);
                pos += 1;
            }
        }
    }

    if depth != 0 {
        return Err(ParseError::InvalidObject {
            offset,
            reason: "unterminated literal string (missing ')')".to_string(),
        });
    }

    Ok((out, pos))
}

/// `data[offset..]`에서 PDF hex 문자열 `<...>` 을 파싱한다.
///
/// 홀수 길이면 마지막 nibble에 0을 추가한다. 화이트스페이스는 무시한다.
/// ISO 32000-1 §7.3.4.3
pub(crate) fn parse_hex_string(data: &[u8], offset: usize) -> Result<(Vec<u8>, usize), ParseError> {
    let slice = &data[offset..];
    if slice.is_empty() || slice[0] != b'<' {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!(
                "expected '<' for hex string, found {:?}",
                peek_str(slice, 4)
            ),
        });
    }
    // << 는 dict이므로 거부
    if slice.get(1) == Some(&b'<') {
        return Err(ParseError::InvalidObject {
            offset,
            reason: "'<<' is a dictionary delimiter, not a hex string".to_string(),
        });
    }

    let mut out = Vec::new();
    let mut pos = 1; // '<' 다음
    let mut high: Option<u8> = None;

    while pos < slice.len() {
        let b = slice[pos];
        if b == b'>' {
            pos += 1;
            break;
        }
        if is_whitespace(b) {
            pos += 1;
            continue;
        }
        let nibble = hex_nibble(b).ok_or_else(|| ParseError::InvalidObject {
            offset: offset + pos,
            reason: format!("invalid hex character in hex string: {b:#04x}"),
        })?;
        match high {
            None => {
                high = Some(nibble);
            }
            Some(h) => {
                out.push((h << 4) | nibble);
                high = None;
            }
        }
        pos += 1;
    }

    // 홀수 길이: 마지막 nibble에 0 추가 (§7.3.4.3)
    if let Some(h) = high {
        out.push(h << 4);
    }

    Ok((out, pos))
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// `data[offset..]`에서 PDF 이름 객체 `/Foo` 를 파싱한다.
///
/// `/` 를 제외한 나머지를 반환한다. `#HH` 이스케이프를 디코딩한다.
/// 빈 이름 (`/` 뒤에 구분자) 허용. ISO 32000-1 §7.3.5
pub(crate) fn parse_name(data: &[u8], offset: usize) -> Result<(Vec<u8>, usize), ParseError> {
    let slice = &data[offset..];
    if slice.is_empty() || slice[0] != b'/' {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!("expected '/' for name, found {:?}", peek_str(slice, 4)),
        });
    }

    let mut out = Vec::new();
    let mut pos = 1; // '/' 다음

    while pos < slice.len() {
        let b = slice[pos];
        // 이름 종료: 화이트스페이스 또는 PDF 구분자
        if is_whitespace(b) || matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'/' | b'%') {
            break;
        }
        if b == b'#' {
            // #HH 이스케이프
            if pos + 2 >= slice.len() {
                return Err(ParseError::InvalidObject {
                    offset: offset + pos,
                    reason: "incomplete #HH escape in name".to_string(),
                });
            }
            let hi = hex_nibble(slice[pos + 1]).ok_or_else(|| ParseError::InvalidObject {
                offset: offset + pos + 1,
                reason: format!("invalid hex digit in name escape: {:#04x}", slice[pos + 1]),
            })?;
            let lo = hex_nibble(slice[pos + 2]).ok_or_else(|| ParseError::InvalidObject {
                offset: offset + pos + 2,
                reason: format!("invalid hex digit in name escape: {:#04x}", slice[pos + 2]),
            })?;
            out.push((hi << 4) | lo);
            pos += 3;
        } else {
            out.push(b);
            pos += 1;
        }
    }

    Ok((out, pos))
}

/// `data[offset..]`에서 간접 참조 `N G R` 을 파싱한다.
///
/// 성공하면 `(ObjectId, consumed)` 반환. `R` 이 없으면 `None` 반환 (숫자로 폴백).
/// ISO 32000-1 §7.3.10
pub(crate) fn try_parse_reference(data: &[u8], offset: usize) -> Option<(ObjectId, usize)> {
    let slice = &data[offset..];

    // 첫 번째 정수 (object number)
    let mut pos = 0;
    while pos < slice.len() && slice[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == 0 {
        return None;
    }
    let number: u32 = std::str::from_utf8(&slice[..pos]).ok()?.parse().ok()?;

    // 화이트스페이스 건너뜀
    let ws1 = count_whitespace(&slice[pos..]);
    if ws1 == 0 {
        return None;
    }
    pos += ws1;

    // 두 번째 정수 (generation number)
    let gen_start = pos;
    while pos < slice.len() && slice[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == gen_start {
        return None;
    }
    let generation: u16 = std::str::from_utf8(&slice[gen_start..pos])
        .ok()?
        .parse()
        .ok()?;

    // 화이트스페이스 건너뜀
    let ws2 = count_whitespace(&slice[pos..]);
    if ws2 == 0 {
        return None;
    }
    pos += ws2;

    // 'R' 키워드
    if pos >= slice.len() || slice[pos] != b'R' {
        return None;
    }
    pos += 1;

    // 'R' 다음이 이름 문자이면 다른 토큰 (예: "Rect")
    if pos < slice.len() && is_name_char(slice[pos]) {
        return None;
    }

    Some((ObjectId { number, generation }, pos))
}

fn count_whitespace(data: &[u8]) -> usize {
    data.iter()
        .position(|&b| !is_whitespace(b))
        .unwrap_or(data.len())
}

/// `data[offset..]`에서 PDF 배열 `[...]` 을 파싱한다.
///
/// 재귀 깊이 `depth`로 호출된다. Array 진입 시 depth + 1 전달.
/// ISO 32000-1 §7.3.6
pub(crate) fn parse_array(
    data: &[u8],
    offset: usize,
    depth: usize,
) -> Result<(Vec<PdfObject>, usize), ParseError> {
    let slice = &data[offset..];
    if slice.is_empty() || slice[0] != b'[' {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!("expected '[' for array, found {:?}", peek_str(slice, 4)),
        });
    }

    let mut items = Vec::new();
    let mut pos = 1; // '[' 다음

    loop {
        // 화이트스페이스·주석 건너뜀
        let ws = skip_whitespace_and_comments(data, offset + pos) - (offset + pos);
        pos += ws;

        if offset + pos >= data.len() {
            return Err(ParseError::InvalidObject {
                offset,
                reason: "unterminated array (missing ']')".to_string(),
            });
        }

        if data[offset + pos] == b']' {
            pos += 1;
            break;
        }

        let (obj, consumed) = parse_object_with_depth(data, offset + pos, depth)?;
        pos += consumed;
        items.push(obj);
    }

    Ok((items, pos))
}

/// `data[offset..]`에서 PDF 딕셔너리 `<<...>>` 를 파싱한다.
///
/// key는 반드시 Name. 재귀 깊이 `depth`로 호출된다. ISO 32000-1 §7.3.7
pub(crate) fn parse_dictionary(
    data: &[u8],
    offset: usize,
    depth: usize,
) -> Result<(PdfDict, usize), ParseError> {
    let slice = &data[offset..];
    if slice.len() < 2 || slice[0] != b'<' || slice[1] != b'<' {
        return Err(ParseError::InvalidObject {
            offset,
            reason: format!(
                "expected '<<' for dictionary, found {:?}",
                peek_str(slice, 4)
            ),
        });
    }

    let mut entries = Vec::new();
    let mut pos = 2; // '<<' 다음

    loop {
        // 화이트스페이스·주석 건너뜀
        let ws = skip_whitespace_and_comments(data, offset + pos) - (offset + pos);
        pos += ws;

        if offset + pos + 1 >= data.len() {
            return Err(ParseError::InvalidObject {
                offset,
                reason: "unterminated dictionary (missing '>>')".to_string(),
            });
        }

        // '>>' 종료 확인
        if data[offset + pos] == b'>' && data[offset + pos + 1] == b'>' {
            pos += 2;
            break;
        }

        // key: 반드시 Name
        if data[offset + pos] != b'/' {
            return Err(ParseError::InvalidObject {
                offset: offset + pos,
                reason: format!(
                    "dictionary key must be a Name, found {:?}",
                    peek_str(&data[offset + pos..], 8)
                ),
            });
        }
        let (key_bytes, key_consumed) = parse_name(data, offset + pos)?;
        pos += key_consumed;

        // 화이트스페이스 건너뜀
        let ws2 = skip_whitespace_and_comments(data, offset + pos) - (offset + pos);
        pos += ws2;

        // value
        let (val, val_consumed) = parse_object_with_depth(data, offset + pos, depth)?;
        pos += val_consumed;

        entries.push((key_bytes, val));
    }

    Ok((PdfDict(entries), pos))
}

/// 깊이 제한 포함 내부 파싱 진입점.
///
/// `parse_object`는 이 함수를 depth=0으로 호출한다.
/// Array/Dictionary 진입 시 depth+1을 전달한다.
pub(crate) fn parse_object_with_depth(
    data: &[u8],
    offset: usize,
    depth: usize,
) -> Result<(PdfObject, usize), ParseError> {
    let pos = skip_whitespace_and_comments(data, offset);
    let ws = pos - offset;

    if pos >= data.len() {
        return Err(ParseError::InvalidObject {
            offset: pos,
            reason: "unexpected end of data".to_string(),
        });
    }

    let (obj, consumed) = match data[pos] {
        b't' | b'f' => {
            let (b, c) = parse_boolean(data, pos)?;
            (PdfObject::Boolean(b), c)
        }
        b'n' => {
            let c = parse_null(data, pos)?;
            (PdfObject::Null, c)
        }
        b'(' => {
            let (bytes, c) = parse_literal_string(data, pos)?;
            (PdfObject::LiteralString(bytes), c)
        }
        b'<' if data.get(pos + 1) == Some(&b'<') => {
            // Dictionary or Stream
            if depth >= MAX_OBJECT_DEPTH {
                return Err(ParseError::ObjectTooDeep {
                    offset: pos,
                    max_depth: MAX_OBJECT_DEPTH,
                });
            }
            let (dict, dict_c) = parse_dictionary(data, pos, depth + 1)?;

            // Lookahead: dict 뒤에 "stream" 키워드가 오면 스트림 파싱으로 전환
            let after_dict = pos + dict_c;
            let stream_pos = skip_whitespace_and_comments(data, after_dict);
            let stream_ws = stream_pos - after_dict;

            if data
                .get(stream_pos..)
                .is_some_and(|s| s.starts_with(b"stream"))
            {
                let (stream_obj, stream_c) = parse_stream(data, stream_pos, dict)?;
                (PdfObject::Stream(stream_obj), dict_c + stream_ws + stream_c)
            } else {
                (PdfObject::Dictionary(dict), dict_c)
            }
        }
        b'<' => {
            let (bytes, c) = parse_hex_string(data, pos)?;
            (PdfObject::HexString(bytes), c)
        }
        b'/' => {
            let (bytes, c) = parse_name(data, pos)?;
            (PdfObject::Name(bytes), c)
        }
        b'[' => {
            // Array
            if depth >= MAX_OBJECT_DEPTH {
                return Err(ParseError::ObjectTooDeep {
                    offset: pos,
                    max_depth: MAX_OBJECT_DEPTH,
                });
            }
            let (items, c) = parse_array(data, pos, depth + 1)?;
            (PdfObject::Array(items), c)
        }
        b'0'..=b'9' | b'+' | b'-' | b'.' => {
            // Reference 우선 시도 (숫자 시작 시)
            if data[pos].is_ascii_digit() {
                if let Some((id, c)) = try_parse_reference(data, pos) {
                    (PdfObject::Reference(id), c)
                } else {
                    parse_number(data, pos)?
                }
            } else {
                parse_number(data, pos)?
            }
        }
        b => {
            return Err(ParseError::InvalidObject {
                offset: pos,
                reason: format!("unsupported object start byte: {b:#04x} ({:?})", b as char),
            });
        }
    };

    Ok((obj, ws + consumed))
}

/// `data[offset..]`에서 PDF 객체 하나를 파싱한다.
///
/// **Checkpoint D 구현**: Boolean, Null, Integer, Real, LiteralString, HexString,
/// Name, Reference, Array, Dictionary, Stream 지원.
///
/// 선행 화이트스페이스와 주석은 자동으로 건너뛴다.
///
/// # 반환
///
/// `Ok((object, consumed))` — `consumed`는 `offset`부터 소비된 총 바이트 수
/// (선행 화이트스페이스 포함).
///
/// # 에러
///
/// - [`ParseError::InvalidObject`] — 예상치 못한 토큰
/// - [`ParseError::ObjectTooDeep`] — 중첩 깊이 초과
/// - [`ParseError::MalformedStream`] — 스트림 구조 오류
pub fn parse_object(data: &[u8], offset: usize) -> Result<(PdfObject, usize), ParseError> {
    parse_object_with_depth(data, offset, 0)
}

/// `data[offset..]`에서 PDF 스트림 `stream ... endstream` 부분을 파싱한다.
///
/// `offset`은 `stream` 키워드 첫 바이트를 가리킨다. 딕셔너리는 호출자가
/// 이미 파싱해 `dict`로 전달한다.
///
/// - `stream` 키워드 뒤 EOL: `\n` 또는 `\r\n` 만 허용 (`\r` 단독 불허). ISO 32000 §7.3.8
/// - `/Length` 누락·비정수·음수·간접 참조 → `MalformedStream`
/// - 필터 디코딩은 Task #5에서 처리. raw bytes만 반환.
/// - `endstream` 전 선택적 공백은 스킵 (스펙 §7.3.8 "should be an end-of-line … before endstream")
///
/// ISO 32000-1 §7.3.8
pub(crate) fn parse_stream(
    data: &[u8],
    offset: usize,
    dict: PdfDict,
) -> Result<(PdfStream, usize), ParseError> {
    // "stream" 키워드 확인
    if !data[offset..].starts_with(b"stream") {
        return Err(ParseError::MalformedStream {
            offset,
            reason: format!(
                "expected 'stream' keyword, found {:?}",
                peek_str(&data[offset..], 8)
            ),
        });
    }
    let after_kw = offset + 6; // "stream" 다음

    // EOL: \n 또는 \r\n. \r 단독 불허. (ISO 32000 §7.3.8)
    let eol_len = match data.get(after_kw) {
        Some(b'\n') => 1,
        Some(b'\r') if data.get(after_kw + 1) == Some(&b'\n') => 2,
        Some(b'\r') => {
            return Err(ParseError::MalformedStream {
                offset: after_kw,
                reason: "stream keyword followed by CR alone (must be LF or CRLF)".to_string(),
            });
        }
        _ => {
            return Err(ParseError::MalformedStream {
                offset: after_kw,
                reason: "stream keyword not followed by EOL (LF or CRLF required)".to_string(),
            });
        }
    };

    let stream_start = after_kw + eol_len;

    // /Length 키 조회 — get()은 첫 번째 값 반환 (ISO 32000 §7.3.7 스펙 권장)
    let length: usize = match dict.get(b"Length") {
        None => {
            return Err(ParseError::MalformedStream {
                offset,
                reason: "missing /Length in stream dictionary".to_string(),
            });
        }
        Some(PdfObject::Integer(n)) if *n >= 0 => *n as usize,
        Some(PdfObject::Integer(_)) => {
            return Err(ParseError::MalformedStream {
                offset,
                reason: "/Length is negative".to_string(),
            });
        }
        Some(PdfObject::Reference(_)) => {
            return Err(ParseError::MalformedStream {
                offset,
                reason: "/Length is an indirect reference (not yet supported; Task #7)".to_string(),
            });
        }
        Some(_) => {
            return Err(ParseError::MalformedStream {
                offset,
                reason: "/Length is not an integer".to_string(),
            });
        }
    };

    // raw bytes 추출
    let stream_end = stream_start + length;
    if stream_end > data.len() {
        return Err(ParseError::MalformedStream {
            offset,
            reason: format!(
                "/Length {length} exceeds available data ({})",
                data.len() - stream_start
            ),
        });
    }
    let raw_data = data[stream_start..stream_end].to_vec();

    // "endstream" 앞 선택적 공백 스킵 (spec: "should be an end-of-line … before endstream")
    let mut pos = stream_end;
    while pos < data.len() && matches!(data[pos], b'\n' | b'\r' | b' ' | b'\t') {
        pos += 1;
    }

    // "endstream" 키워드 확인
    if !data[pos..].starts_with(b"endstream") {
        return Err(ParseError::MalformedStream {
            offset: pos,
            reason: "missing endstream keyword".to_string(),
        });
    }
    pos += 9; // "endstream"

    Ok((
        PdfStream {
            dict,
            data: raw_data,
        },
        pos - offset,
    ))
}

/// `data[offset..]`에서 간접 객체 `N G obj ... endobj`를 파싱한다.
///
/// `XrefEntry::InUse { offset, .. }`의 `offset`을 `as usize`로 변환하여 넘긴다.
///
/// # 범위 검사
///
/// - 객체 번호: u32 범위 초과 → [`ParseError::InvalidObject`]
///   (try_parse_reference와 달리 폴백 없음 — 헤더 컨텍스트에서는 명백한 오류)
/// - 세대 번호: u16 범위 초과 → [`ParseError::InvalidObject`]
///
/// # 에러
///
/// - [`ParseError::InvalidObject`] — `N G obj` 구조 파싱 실패
/// - [`ParseError::MissingEndobj`] — `endobj` 키워드 없음
/// - `parse_object`의 에러 전파
///
/// ISO 32000-1 §7.3.10
pub fn parse_indirect_object(
    data: &[u8],
    offset: usize,
) -> Result<(IndirectObject, usize), ParseError> {
    let mut cur = skip_whitespace_and_comments(data, offset);

    // 객체 번호 (u32)
    if cur >= data.len() || !data[cur].is_ascii_digit() {
        return Err(ParseError::InvalidObject {
            offset: cur,
            reason: format!(
                "expected object number, found {:?}",
                peek_str(&data[cur..], 8)
            ),
        });
    }
    let num_start = cur;
    while cur < data.len() && data[cur].is_ascii_digit() {
        cur += 1;
    }
    let number_u64 = std::str::from_utf8(&data[num_start..cur])
        .unwrap()
        .parse::<u64>()
        .map_err(|_| ParseError::InvalidObject {
            offset: num_start,
            reason: "object number overflow".to_string(),
        })?;
    if number_u64 > u32::MAX as u64 {
        return Err(ParseError::InvalidObject {
            offset: num_start,
            reason: format!("object number {number_u64} exceeds u32::MAX"),
        });
    }
    let number = number_u64 as u32;

    // 화이트스페이스 필수
    if cur >= data.len() || !is_whitespace(data[cur]) {
        return Err(ParseError::InvalidObject {
            offset: cur,
            reason: "expected whitespace after object number".to_string(),
        });
    }
    cur = skip_whitespace_and_comments(data, cur);

    // 세대 번호 (u16)
    if cur >= data.len() || !data[cur].is_ascii_digit() {
        return Err(ParseError::InvalidObject {
            offset: cur,
            reason: format!(
                "expected generation number, found {:?}",
                peek_str(&data[cur..], 8)
            ),
        });
    }
    let gen_start = cur;
    while cur < data.len() && data[cur].is_ascii_digit() {
        cur += 1;
    }
    let generation_u64 = std::str::from_utf8(&data[gen_start..cur])
        .unwrap()
        .parse::<u64>()
        .map_err(|_| ParseError::InvalidObject {
            offset: gen_start,
            reason: "generation number overflow".to_string(),
        })?;
    if generation_u64 > u16::MAX as u64 {
        return Err(ParseError::InvalidObject {
            offset: gen_start,
            reason: format!("generation number {generation_u64} exceeds u16::MAX"),
        });
    }
    let generation = generation_u64 as u16;

    // 화이트스페이스 필수
    if cur >= data.len() || !is_whitespace(data[cur]) {
        return Err(ParseError::InvalidObject {
            offset: cur,
            reason: "expected whitespace after generation number".to_string(),
        });
    }
    cur = skip_whitespace_and_comments(data, cur);

    // "obj" 키워드
    if !data[cur..].starts_with(b"obj") {
        return Err(ParseError::InvalidObject {
            offset: cur,
            reason: format!(
                "expected 'obj' keyword, found {:?}",
                peek_str(&data[cur..], 8)
            ),
        });
    }
    cur += 3;
    // "obj" 다음이 이름 문자이면 다른 토큰 (예: "objFoo")
    if cur < data.len() && is_name_char(data[cur]) {
        return Err(ParseError::InvalidObject {
            offset: cur - 3,
            reason: format!(
                "'obj' followed by name char: {:?}",
                peek_str(&data[cur..], 4)
            ),
        });
    }

    // 객체 값 파싱
    let (object, obj_c) = parse_object_with_depth(data, cur, 0)?;
    cur += obj_c;

    // 화이트스페이스 건너뜀
    cur = skip_whitespace_and_comments(data, cur);

    // "endobj" 키워드
    if !data[cur..].starts_with(b"endobj") {
        return Err(ParseError::MissingEndobj { offset: cur });
    }
    cur += 6;

    Ok((
        IndirectObject {
            id: ObjectId { number, generation },
            object,
        },
        cur - offset,
    ))
}
