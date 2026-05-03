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
        // BI 특수 처리: 인라인 이미지 (D-1에서 구현)
        let ws_pos = skip_whitespace_and_comments(data, pos);
        if ws_pos < data.len()
            && data[ws_pos..].starts_with(b"BI")
            && data[ws_pos + 2..]
                .first()
                .map(|&b| !is_keyword_char(b))
                .unwrap_or(true)
        {
            // D-1 이전까지 BI를 만나면 임시로 에러
            return Err(ParseError::MalformedContentStream {
                offset: ws_pos,
                reason: "인라인 이미지(BI)는 Checkpoint D-1 이후 지원".to_string(),
            });
        }

        match next_token(data, pos)? {
            None => break,
            Some((Token::Operand(obj), next_pos)) => {
                operands.push(obj);
                pos = next_pos;
            }
            Some((Token::Keyword(keyword), next_pos)) => {
                pos = next_pos;
                let operator = keyword_to_operator(&keyword);

                // q/Q 깊이 추적
                match &operator {
                    ContentStreamOperator::SaveState => depth += 1,
                    ContentStreamOperator::RestoreState => {
                        if depth == 0 {
                            return Err(ParseError::UnbalancedGraphicsState {
                                offset: skip_whitespace_and_comments(data, pos)
                                    .saturating_sub(keyword.len()),
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
}
