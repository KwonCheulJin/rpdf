//! PDF content stream 파서 — 연산자 시퀀스 토큰화 + 분류.
//!
//! **범위 외 (의미 해석 X)**:
//! - 폰트 매핑 / 텍스트 렌더링 (Task #8, v0.2)
//! - 좌표 변환 누적 (v0.2)
//! - 색상 모델 해석 (v0.2)
use crate::error::ParseError;
use crate::objects::{parse_object, skip_whitespace_and_comments};
use rpdf_core::types::{ContentStreamOperation, ContentStreamOperator, PdfObject};

// content stream 키워드 문자: ASCII 그래픽, 일부 구두점 허용 (*, ')
fn is_keyword_char(b: u8) -> bool {
    b.is_ascii_alphabetic() || matches!(b, b'*' | b'\'' | b'"')
}

/// 내부 토큰 — B 단계에서 keyword bytes를 보관하고 C 단계에서 enum으로 변환.
enum Token {
    Operand(PdfObject),
    Keyword(Vec<u8>),
}

/// `data[pos..]`에서 다음 토큰 하나를 읽는다.
///
/// 반환: `Ok(Some((token, next_pos)))` 또는 입력 끝이면 `Ok(None)`.
fn next_token(data: &[u8], pos: usize) -> Result<Option<(Token, usize)>, ParseError> {
    let pos = skip_whitespace_and_comments(data, pos);
    if pos >= data.len() {
        return Ok(None);
    }

    let b = data[pos];

    // keyword 토큰: 알파벳 또는 ' " 로 시작
    if b.is_ascii_alphabetic() || b == b'\'' || b == b'"' {
        let end = data[pos..]
            .iter()
            .position(|&c| !is_keyword_char(c))
            .map(|n| pos + n)
            .unwrap_or(data.len());
        let keyword = data[pos..end].to_vec();
        return Ok(Some((Token::Keyword(keyword), end)));
    }

    // 피연산자: parse_object로 파싱.
    // parse_object는 (obj, bytes_consumed_from_pos)를 반환하므로 절대 위치 = pos + consumed.
    match parse_object(data, pos) {
        Ok((obj, consumed)) => Ok(Some((Token::Operand(obj), pos + consumed))),
        Err(_) => Err(ParseError::MalformedContentStream {
            offset: pos,
            reason: format!("예상치 못한 바이트: 0x{:02X} ({:?})", b, char::from(b)),
        }),
    }
}

/// `data` 전체를 content stream으로 파싱한다.
///
/// 피연산자는 `parse_object`로 파싱된다. 연산자 키워드는 ASCII 이름
/// 토큰으로 인식된 후 `ContentStreamOperator`로 분류된다.
///
/// q/Q 깊이 검증:
/// - `Q` 만났을 때 depth == 0 → `UnbalancedGraphicsState { offset, depth: -1 }` 즉시 에러
/// - 파싱 완료 후 depth > 0 → `UnbalancedGraphicsState { offset: data.len(), depth }` 에러
///
/// 알 수 없는 키워드는 `Unknown(bytes)` 변형으로 보존 (에러 아님).
///
/// ISO 32000-1 §7.8.2
pub fn parse_content_stream(data: &[u8]) -> Result<Vec<ContentStreamOperation>, ParseError> {
    let mut ops: Vec<ContentStreamOperation> = Vec::new();
    let mut operands: Vec<PdfObject> = Vec::new();
    let mut depth: i32 = 0;
    let mut pos = 0;

    loop {
        // BI 특수 처리: 인라인 이미지
        let ws_pos = skip_whitespace_and_comments(data, pos);
        if ws_pos < data.len()
            && data[ws_pos..].starts_with(b"BI")
            && data[ws_pos + 2..]
                .first()
                .map(|&b| !is_keyword_char(b))
                .unwrap_or(true)
        {
            let (op, next_pos) = parse_inline_image(data, ws_pos)?;
            ops.push(op);
            operands.clear(); // BI 앞에 쌓인 피연산자는 무시 (스펙상 없어야 함)
            pos = next_pos;
            continue;
        }

        match next_token(data, pos)? {
            None => break,
            Some((Token::Operand(obj), next_pos)) => {
                operands.push(obj);
                pos = next_pos;
            }
            Some((Token::Keyword(keyword), next_pos)) => {
                // keyword_start = next_pos - keyword.len() (next_token이 keyword 끝 위치 반환)
                let keyword_start = next_pos - keyword.len();
                pos = next_pos;
                let operator = keyword_to_operator(&keyword);

                // q/Q 깊이 추적
                match &operator {
                    ContentStreamOperator::SaveState => depth += 1,
                    ContentStreamOperator::RestoreState => {
                        if depth == 0 {
                            return Err(ParseError::UnbalancedGraphicsState {
                                offset: keyword_start,
                                depth: -1,
                            });
                        }
                        depth -= 1;
                    }
                    _ => {}
                }

                ops.push(ContentStreamOperation::new(
                    operator,
                    std::mem::take(&mut operands),
                ));
            }
        }
    }

    // 파싱 완료 후 depth > 0 검증
    if depth > 0 {
        return Err(ParseError::UnbalancedGraphicsState {
            offset: data.len(),
            depth,
        });
    }

    Ok(ops)
}

/// `data[bi_offset..]`에서 인라인 이미지 `BI...ID...EI`를 파싱한다.
///
/// `bi_offset`은 `BI` 키워드의 시작 위치.
///
/// 구조:
/// - `BI` 키워드
/// - 0개 이상의 key(Name) + value(PdfObject) 쌍
/// - `ID` 키워드 + 공백 1바이트
/// - raw image bytes
/// - 공백 1바이트 + `EI` 키워드
///
/// ISO 32000-1 §8.9.7
fn parse_inline_image(
    data: &[u8],
    bi_offset: usize,
) -> Result<(ContentStreamOperation, usize), ParseError> {
    // BI 키워드 스킵 (bi_offset 이미 'B' 위치)
    let mut pos = bi_offset + 2; // "BI" 다음

    // key-value 쌍 파싱: Name + PdfObject 반복, ID 키워드 만날 때까지
    let mut dict_operands: Vec<PdfObject> = Vec::new();
    loop {
        let ws_pos = skip_whitespace_and_comments(data, pos);
        if ws_pos >= data.len() {
            return Err(ParseError::MalformedInlineImage {
                offset: bi_offset,
                reason: "BI 이후 ID 키워드 없이 데이터 끝".to_string(),
            });
        }

        // ID 키워드 감지 (뒤에 키워드 문자 없어야 함)
        if data[ws_pos..].starts_with(b"ID")
            && data[ws_pos + 2..]
                .first()
                .map(|&b| !is_keyword_char(b))
                .unwrap_or(true)
        {
            pos = ws_pos + 2; // "ID" 다음
            break;
        }

        // key: Name 객체 (/ 로 시작)
        if data[ws_pos] != b'/' {
            return Err(ParseError::MalformedInlineImage {
                offset: ws_pos,
                reason: format!(
                    "인라인 이미지 dict key가 Name이 아님: 0x{:02X}",
                    data[ws_pos]
                ),
            });
        }
        let (key_obj, key_consumed) =
            parse_object(data, ws_pos).map_err(|_| ParseError::MalformedInlineImage {
                offset: ws_pos,
                reason: "인라인 이미지 dict key 파싱 실패".to_string(),
            })?;
        dict_operands.push(key_obj);
        pos = ws_pos + key_consumed;

        // value: 임의 PdfObject
        let (val_obj, val_consumed) =
            parse_object(data, pos).map_err(|_| ParseError::MalformedInlineImage {
                offset: pos,
                reason: "인라인 이미지 dict value 파싱 실패".to_string(),
            })?;
        dict_operands.push(val_obj);
        pos += val_consumed;
    }

    // ID 다음 공백 1바이트 스킵 (스펙: "followed by a single white-space character")
    if pos < data.len() && is_pdf_whitespace(data[pos]) {
        pos += 1;
    }

    // raw image bytes 수집: 공백 + EI 패턴 탐색
    // 스펙: "The EI operator shall be preceded by a single white-space character"
    // EI 뒤에 키워드 문자가 없어야 함 (연산자 경계)
    let image_data_start = pos;
    loop {
        if pos >= data.len() {
            return Err(ParseError::MalformedInlineImage {
                offset: bi_offset,
                reason: "인라인 이미지 EI 키워드 없이 데이터 끝".to_string(),
            });
        }

        // 현재 바이트가 whitespace이고 다음에 EI가 오는지 확인
        if is_pdf_whitespace(data[pos])
            && data[pos + 1..].starts_with(b"EI")
            && data[pos + 3..]
                .first()
                .map(|&b| !is_keyword_char(b))
                .unwrap_or(true)
        {
            let image_data = data[image_data_start..pos].to_vec();
            let next_pos = pos + 3; // whitespace + "EI"
            let op = ContentStreamOperation::inline_image(dict_operands, image_data);
            return Ok((op, next_pos));
        }

        pos += 1;
    }
}

fn is_pdf_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' | b'\x00')
}

/// PDF 키워드 바이트를 `ContentStreamOperator`로 변환한다.
///
/// 알 수 없는 키워드는 `Unknown(bytes)` 반환 (에러 아님).
fn keyword_to_operator(keyword: &[u8]) -> ContentStreamOperator {
    match keyword {
        // ── Text 객체
        b"BT" => ContentStreamOperator::BeginText,
        b"ET" => ContentStreamOperator::EndText,
        // ── Text 상태
        b"Tc" => ContentStreamOperator::SetCharSpacing,
        b"Tw" => ContentStreamOperator::SetWordSpacing,
        b"Tz" => ContentStreamOperator::SetHorizontalScale,
        b"TL" => ContentStreamOperator::SetLeading,
        b"Tf" => ContentStreamOperator::SetFont,
        b"Tr" => ContentStreamOperator::SetRenderingMode,
        b"Ts" => ContentStreamOperator::SetTextRise,
        // ── Text 위치
        b"Td" => ContentStreamOperator::MoveText,
        b"TD" => ContentStreamOperator::MoveTextSetLeading,
        b"Tm" => ContentStreamOperator::SetTextMatrix,
        b"T*" => ContentStreamOperator::MoveToNextLine,
        // ── Text 표시
        b"Tj" => ContentStreamOperator::ShowText,
        b"TJ" => ContentStreamOperator::ShowTextAdjusted,
        b"'" => ContentStreamOperator::MoveShowText,
        b"\"" => ContentStreamOperator::MoveSetShowText,
        // ── 그래픽 상태
        b"q" => ContentStreamOperator::SaveState,
        b"Q" => ContentStreamOperator::RestoreState,
        b"cm" => ContentStreamOperator::ConcatMatrix,
        b"w" => ContentStreamOperator::SetLineWidth,
        b"J" => ContentStreamOperator::SetLineCap,
        b"j" => ContentStreamOperator::SetLineJoin,
        b"M" => ContentStreamOperator::SetMiterLimit,
        b"d" => ContentStreamOperator::SetDashPattern,
        b"i" => ContentStreamOperator::SetFlatness,
        b"gs" => ContentStreamOperator::SetGraphicsState,
        b"ri" => ContentStreamOperator::SetRenderingIntent,
        // ── 경로 구성
        b"m" => ContentStreamOperator::MoveTo,
        b"l" => ContentStreamOperator::LineTo,
        b"c" => ContentStreamOperator::CurveTo,
        b"v" => ContentStreamOperator::CurveToV,
        b"y" => ContentStreamOperator::CurveToY,
        b"h" => ContentStreamOperator::ClosePath,
        b"re" => ContentStreamOperator::Rect,
        // ── 경로 그리기
        b"S" => ContentStreamOperator::Stroke,
        b"s" => ContentStreamOperator::CloseStroke,
        b"f" => ContentStreamOperator::Fill,
        b"F" => ContentStreamOperator::FillObsolete,
        b"f*" => ContentStreamOperator::FillEvenOdd,
        b"B" => ContentStreamOperator::FillStroke,
        b"B*" => ContentStreamOperator::FillStrokeEvenOdd,
        b"b" => ContentStreamOperator::CloseFillStroke,
        b"b*" => ContentStreamOperator::CloseFillStrokeEvenOdd,
        b"n" => ContentStreamOperator::EndPath,
        // ── 클리핑
        b"W" => ContentStreamOperator::Clip,
        b"W*" => ContentStreamOperator::ClipEvenOdd,
        // ── 색상
        b"CS" => ContentStreamOperator::SetStrokeColorSpace,
        b"cs" => ContentStreamOperator::SetFillColorSpace,
        b"SC" => ContentStreamOperator::SetStrokeColor,
        b"SCN" => ContentStreamOperator::SetStrokeColorN,
        b"sc" => ContentStreamOperator::SetFillColor,
        b"scn" => ContentStreamOperator::SetFillColorN,
        b"G" => ContentStreamOperator::SetStrokeGray,
        b"g" => ContentStreamOperator::SetFillGray,
        b"RG" => ContentStreamOperator::SetStrokeRGB,
        b"rg" => ContentStreamOperator::SetFillRGB,
        b"K" => ContentStreamOperator::SetStrokeCMYK,
        b"k" => ContentStreamOperator::SetFillCMYK,
        // ── XObject / 셰이딩
        b"Do" => ContentStreamOperator::InvokeXObject,
        b"sh" => ContentStreamOperator::Shading,
        // ── 인라인 이미지 (D-1에서 처리, 여기에 도달하면 Unknown)
        b"ID" | b"EI" => ContentStreamOperator::Unknown(keyword.to_vec()),
        // ── 마킹된 콘텐츠
        b"MP" => ContentStreamOperator::MarkedContentPoint,
        b"DP" => ContentStreamOperator::MarkedContentPointProp,
        b"BMC" => ContentStreamOperator::BeginMarkedContent,
        b"BDC" => ContentStreamOperator::BeginMarkedContentProp,
        b"EMC" => ContentStreamOperator::EndMarkedContent,
        // ── 호환성
        b"BX" => ContentStreamOperator::BeginCompatibility,
        b"EX" => ContentStreamOperator::EndCompatibility,
        // ── 알 수 없는 연산자
        _ => ContentStreamOperator::Unknown(keyword.to_vec()),
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use rpdf_core::types::PdfObject;

    // ── Checkpoint B 단위 테스트 ────────────────────────────────────

    #[test]
    fn empty_input_returns_empty() {
        assert!(parse_content_stream(b"").unwrap().is_empty());
    }

    #[test]
    fn single_operand_plus_keyword() {
        let ops = parse_content_stream(b"12 w").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operator, ContentStreamOperator::SetLineWidth);
        assert_eq!(ops[0].operands, vec![PdfObject::Integer(12)]);
    }

    #[test]
    fn multiple_operands_plus_keyword() {
        let ops = parse_content_stream(b"1 0 0 1 50 700 cm").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operands.len(), 6);
    }

    #[test]
    fn keyword_with_no_operands() {
        let ops = parse_content_stream(b"BT").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
        assert!(ops[0].operands.is_empty());
    }

    #[test]
    fn comment_is_skipped() {
        // % 주석 줄 사이에 연산자가 있어도 정상 파싱
        let ops = parse_content_stream(b"% comment\nBT\n% another\nET").unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
        assert_eq!(ops[1].operator, ContentStreamOperator::EndText);
    }

    #[test]
    fn multiple_operations_in_sequence() {
        let ops = parse_content_stream(b"BT\n/F1 12 Tf\n72 720 Td\nET").unwrap();
        assert_eq!(ops.len(), 4);
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
        assert_eq!(ops[1].operator, ContentStreamOperator::SetFont);
        assert_eq!(ops[2].operator, ContentStreamOperator::MoveText);
        assert_eq!(ops[3].operator, ContentStreamOperator::EndText);
    }

    #[test]
    fn array_operand_for_tj() {
        // TJ의 배열 인자: [(Hello)10(World)]
        let ops = parse_content_stream(b"[(Hello)10(World)] TJ").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operator, ContentStreamOperator::ShowTextAdjusted);
        assert_eq!(ops[0].operands.len(), 1);
        matches!(ops[0].operands[0], PdfObject::Array(_));
    }

    #[test]
    fn operands_only_no_keyword_returns_empty() {
        // 피연산자만 있고 키워드 없음 → 남은 스택 무시
        let ops = parse_content_stream(b"1 2 3").unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn invalid_byte_returns_error() {
        let err = parse_content_stream(b"!invalid").unwrap_err();
        assert!(matches!(err, ParseError::MalformedContentStream { .. }));
    }

    #[test]
    fn balanced_q_q_succeeds() {
        let ops = parse_content_stream(b"q Q").unwrap();
        assert_eq!(ops.len(), 2);
    }

    // ── Checkpoint C 단위 테스트 ────────────────────────────────────

    #[test]
    fn text_operators_classified() {
        let ops = parse_content_stream(b"BT ET").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
        assert_eq!(ops[1].operator, ContentStreamOperator::EndText);
    }

    #[test]
    fn show_text_and_adjusted() {
        let ops = parse_content_stream(b"(Hi) Tj [(x)10] TJ").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::ShowText);
        assert_eq!(ops[1].operator, ContentStreamOperator::ShowTextAdjusted);
    }

    #[test]
    fn move_show_text_operators() {
        let ops = parse_content_stream(b"(Hi) ' 0 0 (Hi) \"").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::MoveShowText);
        assert_eq!(ops[1].operator, ContentStreamOperator::MoveSetShowText);
    }

    #[test]
    fn graphics_state_operators() {
        let ops = parse_content_stream(b"q Q 1 0 0 1 0 0 cm /GS1 gs").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::SaveState);
        assert_eq!(ops[1].operator, ContentStreamOperator::RestoreState);
        assert_eq!(ops[2].operator, ContentStreamOperator::ConcatMatrix);
        assert_eq!(ops[3].operator, ContentStreamOperator::SetGraphicsState);
    }

    #[test]
    fn path_operators_classified() {
        let ops = parse_content_stream(b"0 0 m 10 10 l h re f* W*").unwrap();
        // m, l, h requires operands but keyword_to_operator is pure
        // Just check operator types
        assert_eq!(ops[0].operator, ContentStreamOperator::MoveTo);
        assert_eq!(ops[1].operator, ContentStreamOperator::LineTo);
        assert_eq!(ops[2].operator, ContentStreamOperator::ClosePath);
        // re needs 4 operands; without them it's still classified
        // f*, W*
        let fstar_idx = ops
            .iter()
            .position(|o| o.operator == ContentStreamOperator::FillEvenOdd);
        assert!(fstar_idx.is_some());
        let wstar_idx = ops
            .iter()
            .position(|o| o.operator == ContentStreamOperator::ClipEvenOdd);
        assert!(wstar_idx.is_some());
    }

    #[test]
    fn color_operators_classified() {
        let ops = parse_content_stream(b"1 0 0 RG 0.5 g 0 0 0 1 k").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::SetStrokeRGB);
        assert_eq!(ops[1].operator, ContentStreamOperator::SetFillGray);
        assert_eq!(ops[2].operator, ContentStreamOperator::SetFillCMYK);
    }

    #[test]
    fn xobject_operator() {
        let ops = parse_content_stream(b"/Image1 Do").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::InvokeXObject);
    }

    #[test]
    fn marked_content_operators() {
        let ops = parse_content_stream(b"/Tag BMC EMC").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginMarkedContent);
        assert_eq!(ops[1].operator, ContentStreamOperator::EndMarkedContent);
    }

    #[test]
    fn compatibility_operators() {
        let ops = parse_content_stream(b"BX EX").unwrap();
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginCompatibility);
        assert_eq!(ops[1].operator, ContentStreamOperator::EndCompatibility);
    }

    #[test]
    fn unknown_keyword_preserved() {
        let ops = parse_content_stream(b"xyz").unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0].operator,
            ContentStreamOperator::Unknown(b"xyz".to_vec())
        );
    }

    // ── Checkpoint D-1 단위 테스트 ────────────────────────────────────

    #[test]
    fn inline_image_basic() {
        // BI /W 10 /H 5 /CS /G /BPC 8 ID <10*5=50 bytes> EI
        let mut data = b"BI /W 10 /H 5 /CS /G /BPC 8\nID ".to_vec();
        data.extend_from_slice(&[0xABu8; 50]); // 50 raw bytes
        data.extend_from_slice(b"\nEI");
        let ops = parse_content_stream(&data).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operator, ContentStreamOperator::InlineImage);
        // dict: /W 10 /H 5 /CS /G /BPC 8 → 4쌍 = 8개 피연산자
        assert_eq!(ops[0].operands.len(), 8);
        assert_eq!(ops[0].inline_data.as_ref().unwrap().len(), 50);
    }

    #[test]
    fn inline_image_without_id_returns_error() {
        let data = b"BI /W 10\n";
        let err = parse_content_stream(data).unwrap_err();
        assert!(matches!(err, ParseError::MalformedInlineImage { .. }));
    }

    #[test]
    fn inline_image_without_ei_returns_error() {
        let mut data = b"BI /W 10 ID ".to_vec();
        data.extend_from_slice(&[0xABu8; 20]);
        let err = parse_content_stream(&data).unwrap_err();
        assert!(matches!(err, ParseError::MalformedInlineImage { .. }));
    }

    #[test]
    fn inline_image_ei_in_data_without_preceding_whitespace_is_not_ei() {
        // raw bytes에 EI가 있어도 앞에 공백이 없으면 EI로 인식하지 않음
        let mut data = b"BI /W 2 /H 1 ID ".to_vec();
        data.extend_from_slice(b"xEI"); // 공백 없이 EI → EI 아님
        data.extend_from_slice(b"\nEI"); // 진짜 EI
        let ops = parse_content_stream(&data).unwrap();
        assert_eq!(ops.len(), 1);
        // inline_data에는 'xEI'가 포함되어야 함
        let img_data = ops[0].inline_data.as_ref().unwrap();
        assert!(img_data.contains(&b'x'));
    }

    #[test]
    fn q_q_balanced_succeeds() {
        let ops = parse_content_stream(b"q q Q Q").unwrap();
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn q_q_unbalanced_excess_q_at_end() {
        let err = parse_content_stream(b"q q Q").unwrap_err();
        match err {
            ParseError::UnbalancedGraphicsState { depth, .. } => assert_eq!(depth, 1),
            other => panic!("expected UnbalancedGraphicsState, got {other:?}"),
        }
    }

    #[test]
    fn q_unmatched_restore_first() {
        let err = parse_content_stream(b"Q q Q").unwrap_err();
        match err {
            ParseError::UnbalancedGraphicsState { depth, offset } => {
                assert_eq!(depth, -1);
                assert_eq!(offset, 0); // Q는 위치 0
            }
            other => panic!("expected UnbalancedGraphicsState, got {other:?}"),
        }
    }

    #[test]
    fn integrated_text_path_color_stream() {
        let stream = b"\
BT\n\
  /F1 12 Tf\n\
  72 720 Td\n\
  (Hello) Tj\n\
ET\n\
q\n\
  1 0 0 RG\n\
  100 100 m\n\
  200 200 l\n\
  S\n\
Q\n\
";
        let ops = parse_content_stream(stream).unwrap();
        // BT, Tf, Td, Tj, ET, q, RG, m, l, S, Q = 11개
        assert_eq!(ops.len(), 11);
        assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
        assert_eq!(ops[4].operator, ContentStreamOperator::EndText);
        assert_eq!(ops[5].operator, ContentStreamOperator::SaveState);
        assert_eq!(ops[10].operator, ContentStreamOperator::RestoreState);
    }

    #[test]
    fn empty_bt_et_pair() {
        let ops = parse_content_stream(b"BT ET").unwrap();
        assert_eq!(ops.len(), 2);
        assert!(ops[0].operands.is_empty());
        assert!(ops[1].operands.is_empty());
    }
}
