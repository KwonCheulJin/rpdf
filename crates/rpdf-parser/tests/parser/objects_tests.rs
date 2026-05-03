use rpdf_core::types::{ObjectId, PdfDict, PdfObject, PdfStream};
use rpdf_parser::{ParseError, parse_indirect_object, parse_object};

// 내부 함수 테스트를 위해 crate 내부 직접 접근이 필요하므로
// 아래는 rpdf_parser의 pub(crate) 함수를 통합 테스트에서 직접 접근할 수 없다.
// 공개 parse_object를 통해 간접적으로 검증하는 방식을 사용한다.
// parse_boolean, parse_null, parse_integer, parse_number는
// parse_object를 통해 동작이 검증된다.

// ─── skip_whitespace_and_comments ────────────────────────────────────────────
// parse_object는 선행 화이트스페이스를 건너뛴 뒤 consumed에 포함시킨다.
// consumed 크기로 화이트스페이스 skip 동작을 간접 검증한다.

#[test]
fn skip_ws_empty_input_returns_error() {
    // 빈 입력 → parse_object가 InvalidObject 반환
    assert!(matches!(
        parse_object(b"", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn skip_ws_whitespace_only_returns_error() {
    // 화이트스페이스만 있으면 객체가 없으므로 InvalidObject
    assert!(matches!(
        parse_object(b"   \t\n", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn skip_ws_comment_only_returns_error() {
    // 주석만 있어도 객체가 없으므로 InvalidObject
    assert!(matches!(
        parse_object(b"% this is a comment\n", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn skip_ws_whitespace_then_object_consumes_ws_in_total() {
    // "   true" → consumed = 3(공백) + 4(true) = 7
    let (obj, consumed) = parse_object(b"   true", 0).unwrap();
    assert_eq!(obj, PdfObject::Boolean(true));
    assert_eq!(consumed, 7);
}

#[test]
fn skip_ws_comment_then_object_on_next_line() {
    // "% comment\ntrue" → skip comment + newline, parse true
    let data = b"% comment\ntrue";
    let (obj, consumed) = parse_object(data, 0).unwrap();
    assert_eq!(obj, PdfObject::Boolean(true));
    // "% comment\n" = 10 bytes, "true" = 4 bytes
    assert_eq!(consumed, 14);
}

// ─── parse_boolean ────────────────────────────────────────────────────────────

#[test]
fn parse_boolean_true() {
    let (obj, consumed) = parse_object(b"true", 0).unwrap();
    assert_eq!(obj, PdfObject::Boolean(true));
    assert_eq!(consumed, 4);
}

#[test]
fn parse_boolean_false() {
    let (obj, consumed) = parse_object(b"false", 0).unwrap();
    assert_eq!(obj, PdfObject::Boolean(false));
    assert_eq!(consumed, 5);
}

#[test]
fn parse_boolean_rejects_keyword_followed_by_name_char() {
    // "truez" — 'z'는 name char이므로 boolean 토큰으로 간주되지 않음
    assert!(matches!(
        parse_object(b"truez", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
    assert!(matches!(
        parse_object(b"falseX", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

// ─── parse_null ───────────────────────────────────────────────────────────────

#[test]
fn parse_null_keyword() {
    let (obj, consumed) = parse_object(b"null", 0).unwrap();
    assert_eq!(obj, PdfObject::Null);
    assert_eq!(consumed, 4);
}

#[test]
fn parse_null_rejects_partial_match() {
    // "nul" — null 키워드 불완전
    assert!(matches!(
        parse_object(b"nul", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_null_rejects_keyword_followed_by_name_char() {
    // "nullx" — null 뒤에 name char
    assert!(matches!(
        parse_object(b"nullx", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

// ─── parse_integer ────────────────────────────────────────────────────────────

#[test]
fn parse_integer_positive() {
    let (obj, consumed) = parse_object(b"42", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(42));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_integer_negative() {
    let (obj, consumed) = parse_object(b"-3", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(-3));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_integer_zero() {
    let (obj, consumed) = parse_object(b"0", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(0));
    assert_eq!(consumed, 1);
}

#[test]
fn parse_integer_explicit_positive_sign() {
    let (obj, consumed) = parse_object(b"+10", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(10));
    assert_eq!(consumed, 3);
}

#[test]
fn parse_integer_i64_max() {
    let data = b"9223372036854775807"; // i64::MAX
    let (obj, consumed) = parse_object(data, 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(i64::MAX));
    assert_eq!(consumed, 19);
}

#[test]
fn parse_integer_overflow_returns_error() {
    // i64::MAX + 1
    let data = b"9223372036854775808";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_integer_empty_input_returns_error() {
    assert!(matches!(
        parse_object(b"", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_integer_non_digit_returns_error() {
    // 'a'는 숫자로 시작할 수 없음 (name char이므로 unsupported start byte)
    assert!(matches!(
        parse_object(b"abc", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

// ─── parse_number (Integer/Real 자동 판별) ────────────────────────────────────

#[test]
fn parse_number_integer_via_parse_number() {
    // 소수점 없으면 Integer
    let (obj, consumed) = parse_object(b"42", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(42));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_number_positive_real() {
    let (obj, consumed) = parse_object(b"1.5", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(1.5_f64));
    assert_eq!(consumed, 3);
}

#[test]
fn parse_number_negative_real() {
    let (obj, consumed) = parse_object(b"-1.5", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(-1.5_f64));
    assert_eq!(consumed, 4);
}

#[test]
fn parse_number_zero_real() {
    let (obj, consumed) = parse_object(b"0.0", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(0.0_f64));
    assert_eq!(consumed, 3);
}

#[test]
fn parse_number_no_integer_part() {
    // ".5" — 소수점 앞 숫자 생략 허용 (ISO 32000 §7.3.3)
    let (obj, consumed) = parse_object(b".5", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(0.5_f64));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_number_no_fractional_part() {
    // "3." — 소수점 뒤 숫자 생략 허용
    let (obj, consumed) = parse_object(b"3.", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(3.0_f64));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_number_ignores_exponent_notation() {
    // PDF 스펙은 지수 표기법 미지원: "1e5" → Integer(1), 'e5'는 별도 토큰
    let (obj, consumed) = parse_object(b"1e5", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(1));
    assert_eq!(consumed, 1);
}

#[test]
fn parse_number_signed_real() {
    // "+1.5"
    let (obj, consumed) = parse_object(b"+1.5", 0).unwrap();
    assert_eq!(obj, PdfObject::Real(1.5_f64));
    assert_eq!(consumed, 4);
}

// ─── 회귀 보강: B 단계 누락 ───────────────────────────────────────────────────

#[test]
fn parse_number_rejects_lone_plus_sign() {
    assert!(matches!(
        parse_object(b"+", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_number_rejects_lone_minus_sign() {
    assert!(matches!(
        parse_object(b"-", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_number_rejects_lone_dot() {
    assert!(matches!(
        parse_object(b".", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

// ─── parse_literal_string ─────────────────────────────────────────────────────

#[test]
fn parse_literal_string_simple() {
    let (obj, consumed) = parse_object(b"(hello)", 0).unwrap();
    assert_eq!(obj, PdfObject::LiteralString(b"hello".to_vec()));
    assert_eq!(consumed, 7);
}

#[test]
fn parse_literal_string_nested_parens() {
    // "(a(b)c)" — 중첩 괄호는 그대로 보존
    let (obj, consumed) = parse_object(b"(a(b)c)", 0).unwrap();
    assert_eq!(obj, PdfObject::LiteralString(b"a(b)c".to_vec()));
    assert_eq!(consumed, 7);
}

#[test]
fn parse_literal_string_escape_sequences() {
    // "\n\(\\" → [0x0A, 0x28, 0x5C]
    let (obj, _) = parse_object(b"(\\n\\(\\\\)", 0).unwrap();
    assert_eq!(obj, PdfObject::LiteralString(vec![b'\n', b'(', b'\\']));
}

#[test]
fn parse_literal_string_octal_escape() {
    // "\101" → 'A' (0x41 == 65)
    let (obj, consumed) = parse_object(b"(\\101)", 0).unwrap();
    assert_eq!(obj, PdfObject::LiteralString(vec![0x41]));
    assert_eq!(consumed, 6);
}

#[test]
fn parse_literal_string_eol_normalization() {
    // CR → LF, CRLF → LF (ISO 32000 §7.3.4.2)
    let (obj, _) = parse_object(b"(\r\r\n)", 0).unwrap();
    assert_eq!(obj, PdfObject::LiteralString(vec![b'\n', b'\n']));
}

// ─── parse_hex_string ─────────────────────────────────────────────────────────

#[test]
fn parse_hex_string_even_length() {
    // <4142> → [0x41, 0x42] == "AB"
    let (obj, consumed) = parse_object(b"<4142>", 0).unwrap();
    assert_eq!(obj, PdfObject::HexString(vec![0x41, 0x42]));
    assert_eq!(consumed, 6);
}

#[test]
fn parse_hex_string_odd_length_pads_zero() {
    // <414> → [0x41, 0x40] (마지막 nibble 4에 0 추가)
    let (obj, consumed) = parse_object(b"<414>", 0).unwrap();
    assert_eq!(obj, PdfObject::HexString(vec![0x41, 0x40]));
    assert_eq!(consumed, 5);
}

#[test]
fn parse_hex_string_ignores_whitespace() {
    // <41 42> — 중간 공백 무시
    let (obj, _) = parse_object(b"<41 42>", 0).unwrap();
    assert_eq!(obj, PdfObject::HexString(vec![0x41, 0x42]));
}

#[test]
fn parse_hex_string_invalid_char_returns_error() {
    assert!(matches!(
        parse_object(b"<4G>", 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

// ─── parse_name ──────────────────────────────────────────────────────────────

#[test]
fn parse_name_simple() {
    let (obj, consumed) = parse_object(b"/Type", 0).unwrap();
    assert_eq!(obj, PdfObject::Name(b"Type".to_vec()));
    assert_eq!(consumed, 5);
}

#[test]
fn parse_name_hash_escape() {
    // /F#23ile → "F#ile" (0x23 = '#')
    let (obj, _) = parse_object(b"/F#23ile", 0).unwrap();
    assert_eq!(obj, PdfObject::Name(b"F#ile".to_vec()));
}

#[test]
fn parse_name_empty() {
    // "/" 바로 뒤에 공백 — 빈 이름 허용
    let (obj, consumed) = parse_object(b"/ ", 0).unwrap();
    assert_eq!(obj, PdfObject::Name(b"".to_vec()));
    assert_eq!(consumed, 1);
}

#[test]
fn parse_name_stops_at_delimiter() {
    // "/Foo/Bar" — 첫 이름만 파싱
    let (obj, consumed) = parse_object(b"/Foo/Bar", 0).unwrap();
    assert_eq!(obj, PdfObject::Name(b"Foo".to_vec()));
    assert_eq!(consumed, 4);
}

// ─── parse_reference ─────────────────────────────────────────────────────────

#[test]
fn parse_reference_normal() {
    let (obj, consumed) = parse_object(b"12 0 R", 0).unwrap();
    assert_eq!(
        obj,
        PdfObject::Reference(ObjectId {
            number: 12,
            generation: 0
        })
    );
    assert_eq!(consumed, 6);
}

#[test]
fn parse_reference_no_r_falls_back_to_integer() {
    // "12 0 " — R 없음 → Integer(12)
    let (obj, consumed) = parse_object(b"12 0 ", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(12));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_reference_rect_keyword_not_reference() {
    // "12 0 Rect" — R 다음에 name char이 오면 Reference로 처리하지 않음
    let (obj, consumed) = parse_object(b"12 0 Rect", 0).unwrap();
    assert_eq!(obj, PdfObject::Integer(12));
    assert_eq!(consumed, 2);
}

// ─── parse_array ─────────────────────────────────────────────────────────────

#[test]
fn parse_array_empty() {
    let (obj, consumed) = parse_object(b"[]", 0).unwrap();
    assert_eq!(obj, PdfObject::Array(vec![]));
    assert_eq!(consumed, 2);
}

#[test]
fn parse_array_mixed_types() {
    let (obj, _) = parse_object(b"[1 true /Name]", 0).unwrap();
    assert_eq!(
        obj,
        PdfObject::Array(vec![
            PdfObject::Integer(1),
            PdfObject::Boolean(true),
            PdfObject::Name(b"Name".to_vec()),
        ])
    );
}

#[test]
fn parse_array_nested() {
    let (obj, _) = parse_object(b"[[1 2] [3]]", 0).unwrap();
    assert_eq!(
        obj,
        PdfObject::Array(vec![
            PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(2)]),
            PdfObject::Array(vec![PdfObject::Integer(3)]),
        ])
    );
}

#[test]
fn parse_array_depth_49_succeeds() {
    // 49단계 중첩 배열은 통과해야 함 (최대 50)
    let mut input = String::new();
    for _ in 0..49 {
        input.push('[');
    }
    input.push('1');
    for _ in 0..49 {
        input.push(']');
    }
    let result = parse_object(input.as_bytes(), 0);
    assert!(result.is_ok(), "49단계 중첩은 성공해야 함: {result:?}");
}

// ─── parse_dictionary ────────────────────────────────────────────────────────

#[test]
fn parse_dictionary_empty() {
    let (obj, consumed) = parse_object(b"<<>>", 0).unwrap();
    assert_eq!(obj, PdfObject::Dictionary(PdfDict(vec![])));
    assert_eq!(consumed, 4);
}

#[test]
fn parse_dictionary_simple() {
    let (obj, _) = parse_object(b"<</Type /Page>>", 0).unwrap();
    assert_eq!(
        obj,
        PdfObject::Dictionary(PdfDict(vec![(
            b"Type".to_vec(),
            PdfObject::Name(b"Page".to_vec())
        )]))
    );
}

#[test]
fn parse_dictionary_nested() {
    let input = b"<</Outer <</Inner 1>>>>";
    let (obj, _) = parse_object(input, 0).unwrap();
    match obj {
        PdfObject::Dictionary(dict) => {
            let inner = dict.get(b"Outer").unwrap();
            match inner {
                PdfObject::Dictionary(inner_dict) => {
                    assert_eq!(inner_dict.get(b"Inner"), Some(&PdfObject::Integer(1)));
                }
                _ => panic!("Inner should be dictionary"),
            }
        }
        _ => panic!("Expected dictionary"),
    }
}

#[test]
fn parse_dictionary_duplicate_keys() {
    // 중복 키 허용 — 두 항목 모두 PdfDict에 저장
    let (obj, _) = parse_object(b"<</K 1 /K 2>>", 0).unwrap();
    match obj {
        PdfObject::Dictionary(dict) => {
            assert_eq!(dict.get(b"K"), Some(&PdfObject::Integer(1)));
            assert_eq!(dict.get_last(b"K"), Some(&PdfObject::Integer(2)));
            assert_eq!(dict.len(), 2);
        }
        _ => panic!("Expected dictionary"),
    }
}

// ─── 깊이 제한 ────────────────────────────────────────────────────────────────

#[test]
fn parse_array_depth_51_returns_too_deep() {
    // 51단계 중첩 배열은 ObjectTooDeep 반환
    let mut input = String::new();
    for _ in 0..51 {
        input.push('[');
    }
    input.push('1');
    for _ in 0..51 {
        input.push(']');
    }
    assert!(matches!(
        parse_object(input.as_bytes(), 0).unwrap_err(),
        ParseError::ObjectTooDeep { .. }
    ));
}

// ─── parse_stream (parse_object를 통한 간접 검증) ─────────────────────────────

#[test]
fn parse_stream_normal() {
    // 정상 스트림: Length=5, 데이터="Hello"
    let data = b"<</Length 5>>\nstream\nHello\nendstream";
    let (obj, consumed) = parse_object(data, 0).unwrap();
    match obj {
        PdfObject::Stream(PdfStream { dict, data: raw }) => {
            assert_eq!(dict.get(b"Length"), Some(&PdfObject::Integer(5)));
            assert_eq!(raw, b"Hello");
        }
        _ => panic!("Expected Stream, got {obj:?}"),
    }
    assert_eq!(consumed, data.len());
}

#[test]
fn parse_stream_length_zero() {
    // Length=0 → 빈 스트림
    let data = b"<</Length 0>>\nstream\nendstream";
    let (obj, _) = parse_object(data, 0).unwrap();
    match obj {
        PdfObject::Stream(PdfStream { data: raw, .. }) => {
            assert!(raw.is_empty());
        }
        _ => panic!("Expected Stream"),
    }
}

#[test]
fn parse_stream_missing_length_returns_error() {
    let data = b"<<>>\nstream\nhello\nendstream";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::MalformedStream { .. }
    ));
}

#[test]
fn parse_stream_negative_length_returns_error() {
    let data = b"<</Length -1>>\nstream\nhello\nendstream";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::MalformedStream { .. }
    ));
}

#[test]
fn parse_stream_non_integer_length_returns_error() {
    let data = b"<</Length (foo)>>\nstream\nhello\nendstream";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::MalformedStream { .. }
    ));
}

#[test]
fn parse_stream_crlf_eol_succeeds() {
    // stream 키워드 후 \r\n 허용
    let data = b"<</Length 5>>\nstream\r\nHello\nendstream";
    let (obj, _) = parse_object(data, 0).unwrap();
    match obj {
        PdfObject::Stream(PdfStream { data: raw, .. }) => {
            assert_eq!(raw, b"Hello");
        }
        _ => panic!("Expected Stream"),
    }
}

#[test]
fn parse_stream_lone_cr_eol_returns_error() {
    // stream 키워드 후 \r 단독 → MalformedStream
    let data = b"<</Length 5>>\nstream\rHello\nendstream";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::MalformedStream { .. }
    ));
}

#[test]
fn parse_stream_missing_endstream_returns_error() {
    let data = b"<</Length 5>>\nstream\nHello\n";
    assert!(matches!(
        parse_object(data, 0).unwrap_err(),
        ParseError::MalformedStream { .. }
    ));
}

// ─── parse_indirect_object ───────────────────────────────────────────────────

#[test]
fn parse_indirect_object_normal() {
    let data = b"1 0 obj\ntrue\nendobj";
    let (indirect, consumed) = parse_indirect_object(data, 0).unwrap();
    assert_eq!(indirect.id.number, 1);
    assert_eq!(indirect.id.generation, 0);
    assert_eq!(indirect.object, PdfObject::Boolean(true));
    assert_eq!(consumed, data.len());
}

#[test]
fn parse_indirect_object_generation_max() {
    // generation 65535 (u16::MAX) 허용
    let data = b"1 65535 obj\nnull\nendobj";
    let (indirect, _) = parse_indirect_object(data, 0).unwrap();
    assert_eq!(indirect.id.generation, 65535);
    assert_eq!(indirect.object, PdfObject::Null);
}

#[test]
fn parse_indirect_object_number_zero() {
    // 객체 번호 0 허용 (free 객체 체인의 head)
    let data = b"0 0 obj\n42\nendobj";
    let (indirect, _) = parse_indirect_object(data, 0).unwrap();
    assert_eq!(indirect.id.number, 0);
    assert_eq!(indirect.object, PdfObject::Integer(42));
}

#[test]
fn parse_indirect_object_number_u32_max() {
    // u32::MAX (4294967295) 허용
    let data = b"4294967295 0 obj\nnull\nendobj";
    let (indirect, _) = parse_indirect_object(data, 0).unwrap();
    assert_eq!(indirect.id.number, u32::MAX);
}

#[test]
fn parse_indirect_object_number_exceeds_u32_returns_error() {
    // u32::MAX + 1 → InvalidObject
    let data = b"4294967296 0 obj\nnull\nendobj";
    assert!(matches!(
        parse_indirect_object(data, 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_indirect_object_generation_exceeds_u16_returns_error() {
    // u16::MAX + 1 → InvalidObject
    let data = b"1 65536 obj\nnull\nendobj";
    assert!(matches!(
        parse_indirect_object(data, 0).unwrap_err(),
        ParseError::InvalidObject { .. }
    ));
}

#[test]
fn parse_indirect_object_missing_endobj_returns_error() {
    let data = b"1 0 obj\ntrue\n";
    assert!(matches!(
        parse_indirect_object(data, 0).unwrap_err(),
        ParseError::MissingEndobj { .. }
    ));
}

#[test]
fn parse_indirect_object_with_stream() {
    // 내부 객체가 스트림인 간접 객체
    let data = b"1 0 obj\n<</Length 5>>\nstream\nHello\nendstream\nendobj";
    let (indirect, consumed) = parse_indirect_object(data, 0).unwrap();
    assert_eq!(indirect.id.number, 1);
    match indirect.object {
        PdfObject::Stream(PdfStream { data: raw, .. }) => {
            assert_eq!(raw, b"Hello");
        }
        _ => panic!("Expected Stream inside IndirectObject"),
    }
    assert_eq!(consumed, data.len());
}
