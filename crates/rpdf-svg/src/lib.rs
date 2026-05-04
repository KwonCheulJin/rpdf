mod path;
mod state;
mod text;

use rpdf_core::types::{ContentStreamOperator, Page, PdfObject};

use path::PathBuilder;
use state::{Color, StateStack};
use text::TextState;

/// A4 기본 크기 (pt 단위).
const A4_WIDTH: f64 = 595.0;
const A4_HEIGHT: f64 = 842.0;

/// `Page` IR을 SVG 문자열로 렌더링한다.
///
/// - media_box가 없으면 A4 크기(595 × 842 pt)를 기본값으로 사용한다.
/// - 지원하지 않는 연산자는 SVG 주석으로 기록하고 계속 진행한다.
/// - 빈 content 페이지도 유효한 `<svg>` 루트를 반환한다 (에러 아님).
pub fn render_page_svg(page: &Page) -> String {
    let (w, h) = viewport(page);
    let body = render_body(page);

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n<g transform=\"matrix(1 0 0 -1 0 {h})\">\n{body}</g>\n</svg>",
        w = fmt_f64(w),
        h = fmt_f64(h),
    )
}

/// 페이지에서 뷰포트 크기(w, h)를 결정한다.
fn viewport(page: &Page) -> (f64, f64) {
    match page.media_box() {
        Some([x0, y0, x1, y1]) => ((x1 - x0).abs(), (y1 - y0).abs()),
        None => (A4_WIDTH, A4_HEIGHT),
    }
}

/// content stream 연산자 시퀀스를 SVG 요소 문자열로 변환한다.
fn render_body(page: &Page) -> String {
    let mut out = String::new();
    let mut stack = StateStack::new();
    let mut path = PathBuilder::new();
    let mut text_state: Option<TextState> = None;
    // cm이 q/Q 없이 단독 등장 시 닫아야 할 <g> 개수 추적
    let mut loose_cm_depth: usize = 0;

    for op in page.content() {
        match &op.operator {
            // ── 그래픽 상태 ─────────────────────────────────────
            ContentStreamOperator::SaveState => {
                // 단독 cm이 열려 있으면 먼저 닫기 (cm → q 패턴 대응)
                for _ in 0..loose_cm_depth {
                    out.push_str("</g>\n");
                }
                loose_cm_depth = 0;
                stack.push();
                out.push_str("<g>\n");
            }
            ContentStreamOperator::RestoreState => {
                stack.pop();
                // loose cm 닫기
                for _ in 0..loose_cm_depth {
                    out.push_str("</g>\n");
                }
                loose_cm_depth = 0;
                out.push_str("</g>\n");
            }
            ContentStreamOperator::ConcatMatrix => {
                if let Some([a, b, c, d, e, f]) = extract_matrix(&op.operands) {
                    out.push_str(&format!(
                        "<g transform=\"matrix({} {} {} {} {} {})\">\n",
                        fmt_f64(a),
                        fmt_f64(b),
                        fmt_f64(c),
                        fmt_f64(d),
                        fmt_f64(e),
                        fmt_f64(f),
                    ));
                    loose_cm_depth += 1;
                } else {
                    out.push_str("<!-- unsupported: ConcatMatrix (invalid operands) -->\n");
                }
            }
            ContentStreamOperator::SetLineWidth => {
                if let Some(w) = extract_f64(&op.operands, 0) {
                    stack.set_line_width(w);
                }
            }

            // ── 색상 ─────────────────────────────────────────
            ContentStreamOperator::SetFillRGB => {
                if let (Some(r), Some(g), Some(b)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                ) {
                    stack.set_fill_color(Color::from_pdf_floats(r, g, b));
                } else {
                    out.push_str("<!-- unsupported: SetFillRGB (operands 부족) -->\n");
                }
            }
            ContentStreamOperator::SetStrokeRGB => {
                if let (Some(r), Some(g), Some(b)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                ) {
                    stack.set_stroke_color(Color::from_pdf_floats(r, g, b));
                } else {
                    out.push_str("<!-- unsupported: SetStrokeRGB (operands 부족) -->\n");
                }
            }
            ContentStreamOperator::SetFillGray => {
                if let Some(g) = extract_f64(&op.operands, 0) {
                    stack.set_fill_color(Color::from_pdf_floats(g, g, g));
                }
            }
            ContentStreamOperator::SetStrokeGray => {
                if let Some(g) = extract_f64(&op.operands, 0) {
                    stack.set_stroke_color(Color::from_pdf_floats(g, g, g));
                }
            }

            // ── 경로 구성 ────────────────────────────────────
            ContentStreamOperator::MoveTo => {
                if let (Some(x), Some(y)) =
                    (extract_f64(&op.operands, 0), extract_f64(&op.operands, 1))
                {
                    path.move_to(x, y);
                }
            }
            ContentStreamOperator::LineTo => {
                if let (Some(x), Some(y)) =
                    (extract_f64(&op.operands, 0), extract_f64(&op.operands, 1))
                {
                    path.line_to(x, y);
                }
            }
            ContentStreamOperator::CurveTo => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x3), Some(y3)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                    extract_f64(&op.operands, 3),
                    extract_f64(&op.operands, 4),
                    extract_f64(&op.operands, 5),
                ) {
                    path.curve_to(x1, y1, x2, y2, x3, y3);
                }
            }
            ContentStreamOperator::CurveToV => {
                if let (Some(x2), Some(y2), Some(x3), Some(y3)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                    extract_f64(&op.operands, 3),
                ) {
                    path.curve_to_v(x2, y2, x3, y3);
                }
            }
            ContentStreamOperator::CurveToY => {
                if let (Some(x1), Some(y1), Some(x3), Some(y3)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                    extract_f64(&op.operands, 3),
                ) {
                    path.curve_to_y(x1, y1, x3, y3);
                }
            }
            ContentStreamOperator::ClosePath => {
                path.close_path();
            }
            ContentStreamOperator::Rect => {
                if let (Some(x), Some(y), Some(w), Some(h)) = (
                    extract_f64(&op.operands, 0),
                    extract_f64(&op.operands, 1),
                    extract_f64(&op.operands, 2),
                    extract_f64(&op.operands, 3),
                ) {
                    path.rect(x, y, w, h);
                }
            }

            // ── 경로 그리기 ──────────────────────────────────
            ContentStreamOperator::Stroke => {
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_stroke(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::CloseStroke => {
                path.close_path();
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_stroke(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::Fill | ContentStreamOperator::FillObsolete => {
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_fill(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::FillEvenOdd => {
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_fill(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::FillStroke | ContentStreamOperator::FillStrokeEvenOdd => {
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_fill_stroke(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::CloseFillStroke
            | ContentStreamOperator::CloseFillStrokeEvenOdd => {
                path.close_path();
                if !path.is_empty() {
                    let element = std::mem::take(&mut path).finish_fill_stroke(stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::EndPath => {
                // 경로 버리기 — 요소 미생성
                path = PathBuilder::new();
            }

            // ── 텍스트 ───────────────────────────────────────
            ContentStreamOperator::BeginText => {
                text_state = Some(TextState::new());
            }
            ContentStreamOperator::EndText => {
                text_state = None;
            }
            ContentStreamOperator::SetTextMatrix => {
                if let Some(ts) = &mut text_state
                    && let (Some(_a), Some(_b), Some(_c), Some(_d), Some(e), Some(f)) = (
                        extract_f64(&op.operands, 0),
                        extract_f64(&op.operands, 1),
                        extract_f64(&op.operands, 2),
                        extract_f64(&op.operands, 3),
                        extract_f64(&op.operands, 4),
                        extract_f64(&op.operands, 5),
                    )
                {
                    ts.set_matrix(e, f);
                }
            }
            ContentStreamOperator::MoveText => {
                if let Some(ts) = &mut text_state
                    && let (Some(tx), Some(ty)) =
                        (extract_f64(&op.operands, 0), extract_f64(&op.operands, 1))
                {
                    ts.move_by(tx, ty);
                }
            }
            ContentStreamOperator::MoveTextSetLeading => {
                if let Some(ts) = &mut text_state
                    && let (Some(tx), Some(ty)) =
                        (extract_f64(&op.operands, 0), extract_f64(&op.operands, 1))
                {
                    ts.move_by(tx, ty);
                }
            }
            ContentStreamOperator::ShowText => {
                if let Some(ts) = &text_state
                    && let Some(bytes) = op.operands.first().and_then(|o| o.as_string_bytes())
                {
                    let text = String::from_utf8_lossy(bytes).into_owned();
                    let element = ts.show_text(&text, stack.current());
                    out.push_str(&element);
                    out.push('\n');
                }
            }
            ContentStreamOperator::ShowTextAdjusted => {
                if let Some(ts) = &text_state
                    && let Some(PdfObject::Array(items)) = op.operands.first()
                {
                    for item in items {
                        if let Some(bytes) = item.as_string_bytes() {
                            let text = String::from_utf8_lossy(bytes).into_owned();
                            let element = ts.show_text(&text, stack.current());
                            out.push_str(&element);
                            out.push('\n');
                        }
                    }
                }
            }
            ContentStreamOperator::MoveShowText => {
                if let Some(ts) = &mut text_state {
                    ts.move_by(0.0, -stack.current().line_width);
                    if let Some(bytes) = op.operands.first().and_then(|o| o.as_string_bytes()) {
                        let text = String::from_utf8_lossy(bytes).into_owned();
                        let element = ts.show_text(&text, stack.current());
                        out.push_str(&element);
                        out.push('\n');
                    }
                }
            }

            // ── 지원하지 않는 연산자 ─────────────────────────
            other => {
                out.push_str(&format!("<!-- unsupported: {} -->\n", other.pdf_keyword()));
            }
        }
    }

    // 루프 종료 후 닫지 않은 loose cm <g> 닫기
    for _ in 0..loose_cm_depth {
        out.push_str("</g>\n");
    }

    out
}

/// `PdfObject` 슬라이스에서 인덱스 위치의 f64 값을 추출한다.
///
/// `Integer` → f64 변환도 지원한다.
fn extract_f64(operands: &[PdfObject], index: usize) -> Option<f64> {
    match operands.get(index)? {
        PdfObject::Real(v) => Some(*v),
        PdfObject::Integer(n) => Some(*n as f64),
        _ => None,
    }
}

/// `PdfObject` 슬라이스에서 6개 f64 값을 추출한다 (행렬 변환용).
fn extract_matrix(operands: &[PdfObject]) -> Option<[f64; 6]> {
    if operands.len() < 6 {
        return None;
    }
    Some([
        extract_f64(operands, 0)?,
        extract_f64(operands, 1)?,
        extract_f64(operands, 2)?,
        extract_f64(operands, 3)?,
        extract_f64(operands, 4)?,
        extract_f64(operands, 5)?,
    ])
}

/// f64 값을 SVG 속성용 문자열로 변환한다.
fn fmt_f64(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.6}", v);
        s.trim_end_matches('0').to_string()
    }
}
