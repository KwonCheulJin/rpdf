use crate::state::GraphicsState;

/// 경로 구성 연산자 시퀀스를 SVG `d` 속성 문자열로 빌드한다.
///
/// `MoveTo`, `LineTo`, `CurveTo`, `CurveToV`, `CurveToY`, `ClosePath`, `Rect`
/// 연산자를 호출 순서대로 추가하다가 `finish_stroke` / `finish_fill` /
/// `finish_fill_stroke`로 `<path>` 요소 문자열을 완성한다.
#[derive(Debug, Default)]
pub struct PathBuilder {
    segments: Vec<String>,
    /// 현재 점 (CurveToV 첫 제어점 계산에 필요).
    current_x: f64,
    current_y: f64,
}

impl PathBuilder {
    /// 새 경로 빌더를 생성한다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 경로가 비어 있는지 반환한다.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// MoveTo `m` 연산자.
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.segments
            .push(format!("M {} {}", fmt_f64(x), fmt_f64(y)));
        self.current_x = x;
        self.current_y = y;
    }

    /// LineTo `l` 연산자.
    pub fn line_to(&mut self, x: f64, y: f64) {
        self.segments
            .push(format!("L {} {}", fmt_f64(x), fmt_f64(y)));
        self.current_x = x;
        self.current_y = y;
    }

    /// CurveTo `c` 연산자 (cubic Bézier, 6개 좌표).
    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.segments.push(format!(
            "C {} {} {} {} {} {}",
            fmt_f64(x1),
            fmt_f64(y1),
            fmt_f64(x2),
            fmt_f64(y2),
            fmt_f64(x3),
            fmt_f64(y3),
        ));
        self.current_x = x3;
        self.current_y = y3;
    }

    /// CurveToV `v` 연산자 (첫 제어점 = 현재 점).
    pub fn curve_to_v(&mut self, x2: f64, y2: f64, x3: f64, y3: f64) {
        let cx = self.current_x;
        let cy = self.current_y;
        self.segments.push(format!(
            "C {} {} {} {} {} {}",
            fmt_f64(cx),
            fmt_f64(cy),
            fmt_f64(x2),
            fmt_f64(y2),
            fmt_f64(x3),
            fmt_f64(y3),
        ));
        self.current_x = x3;
        self.current_y = y3;
    }

    /// CurveToY `y` 연산자 (두 번째 제어점 = 끝 점).
    pub fn curve_to_y(&mut self, x1: f64, y1: f64, x3: f64, y3: f64) {
        self.segments.push(format!(
            "C {} {} {} {} {} {}",
            fmt_f64(x1),
            fmt_f64(y1),
            fmt_f64(x3),
            fmt_f64(y3),
            fmt_f64(x3),
            fmt_f64(y3),
        ));
        self.current_x = x3;
        self.current_y = y3;
    }

    /// ClosePath `h` 연산자.
    pub fn close_path(&mut self) {
        self.segments.push("Z".to_string());
    }

    /// Rect `re` 연산자.
    ///
    /// 음수 너비/높이는 절댓값으로 처리한다.
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let w = w.abs();
        let h = h.abs();
        self.segments.push(format!(
            "M {} {} h {} v {} h -{} Z",
            fmt_f64(x),
            fmt_f64(y),
            fmt_f64(w),
            fmt_f64(h),
            fmt_f64(w),
        ));
        self.current_x = x;
        self.current_y = y;
    }

    /// Stroke 연산자 → `<path>` 요소 문자열 반환.
    pub fn finish_stroke(self, state: &GraphicsState) -> String {
        let d = self.build_d();
        format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}"/>"#,
            d,
            state.stroke_color.to_svg_string(),
            fmt_f64(state.line_width),
        )
    }

    /// Fill / FillObsolete / FillEvenOdd 연산자 → `<path>` 요소 문자열 반환.
    pub fn finish_fill(self, state: &GraphicsState) -> String {
        let d = self.build_d();
        format!(
            r#"<path d="{}" fill="{}" stroke="none"/>"#,
            d,
            state.fill_color.to_svg_string(),
        )
    }

    /// FillStroke / FillStrokeEvenOdd 연산자 → `<path>` 요소 문자열 반환.
    pub fn finish_fill_stroke(self, state: &GraphicsState) -> String {
        let d = self.build_d();
        format!(
            r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
            d,
            state.fill_color.to_svg_string(),
            state.stroke_color.to_svg_string(),
            fmt_f64(state.line_width),
        )
    }

    fn build_d(&self) -> String {
        self.segments.join(" ")
    }
}

/// f64 값을 SVG 경로용 문자열로 변환한다.
///
/// 소수점이 필요 없으면 정수로, 필요하면 최대 6자리 소수점으로 출력한다.
fn fmt_f64(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        // 후행 0 제거
        let s = format!("{:.6}", v);
        s.trim_end_matches('0').to_string()
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use crate::state::{Color, GraphicsState};

    fn black_state() -> GraphicsState {
        GraphicsState::new()
    }

    fn red_fill_state() -> GraphicsState {
        let mut gs = GraphicsState::new();
        gs.fill_color = Color { r: 255, g: 0, b: 0 };
        gs
    }

    // PT-1: MoveTo + LineTo + Stroke → <path d="M ... L ..." fill="none" .../>
    #[test]
    fn pt1_move_line_stroke() {
        let mut b = PathBuilder::new();
        b.move_to(10.0, 20.0);
        b.line_to(100.0, 200.0);
        let svg = b.finish_stroke(&black_state());
        assert!(svg.contains(r#"d="M 10 20 L 100 200""#), "d 속성: {}", svg);
        assert!(svg.contains(r#"fill="none""#), "fill 속성: {}", svg);
        assert!(
            svg.contains(r#"stroke="rgb(0,0,0)""#),
            "stroke 속성: {}",
            svg
        );
    }

    // PT-2: CurveToV → C {cur_x} {cur_y} {x2} {y2} {x3} {y3}
    #[test]
    fn pt2_curve_to_v_uses_current_point() {
        let mut b = PathBuilder::new();
        b.move_to(50.0, 50.0);
        b.curve_to_v(80.0, 90.0, 120.0, 130.0);
        let svg = b.finish_stroke(&black_state());
        assert!(
            svg.contains("C 50 50 80 90 120 130"),
            "CurveToV d 속성: {}",
            svg
        );
    }

    // PT-3: CurveToY → C {x1} {y1} {x3} {y3} {x3} {y3}
    #[test]
    fn pt3_curve_to_y_second_control_is_end() {
        let mut b = PathBuilder::new();
        b.move_to(0.0, 0.0);
        b.curve_to_y(30.0, 40.0, 70.0, 80.0);
        let svg = b.finish_stroke(&black_state());
        assert!(
            svg.contains("C 30 40 70 80 70 80"),
            "CurveToY d 속성: {}",
            svg
        );
    }

    // PT-4: Rect + Fill
    #[test]
    fn pt4_rect_fill() {
        let mut b = PathBuilder::new();
        b.rect(10.0, 20.0, 50.0, 30.0);
        let svg = b.finish_fill(&red_fill_state());
        assert!(
            svg.contains(r#"d="M 10 20 h 50 v 30 h -50 Z""#),
            "Rect d 속성: {}",
            svg
        );
        assert!(svg.contains(r#"fill="rgb(255,0,0)""#), "fill 색상: {}", svg);
        assert!(svg.contains(r#"stroke="none""#), "stroke 속성: {}", svg);
    }

    // PT-5: Rect 음수 크기 → 절댓값
    #[test]
    fn pt5_rect_negative_size_uses_abs() {
        let mut b = PathBuilder::new();
        b.rect(10.0, 20.0, -50.0, -30.0);
        let svg = b.finish_fill(&black_state());
        assert!(svg.contains("h 50 v 30 h -50"), "절댓값 사용 확인: {}", svg);
    }

    // PT-6: FillStroke → fill + stroke 속성 모두 포함
    #[test]
    fn pt6_fill_stroke() {
        let mut b = PathBuilder::new();
        b.move_to(0.0, 0.0);
        b.line_to(100.0, 0.0);
        let svg = b.finish_fill_stroke(&black_state());
        assert!(svg.contains(r#"fill="rgb(0,0,0)""#), "fill: {}", svg);
        assert!(svg.contains(r#"stroke="rgb(0,0,0)""#), "stroke: {}", svg);
    }

    // PT-7: ClosePath → Z
    #[test]
    fn pt7_close_path() {
        let mut b = PathBuilder::new();
        b.move_to(0.0, 0.0);
        b.line_to(10.0, 0.0);
        b.close_path();
        let svg = b.finish_stroke(&black_state());
        assert!(svg.contains("Z"), "ClosePath Z: {}", svg);
    }

    // PT-8: SetFillRGB 반영 (색상 주입 검증)
    #[test]
    fn pt8_set_fill_rgb_reflected() {
        let mut gs = GraphicsState::new();
        gs.fill_color = Color::from_pdf_floats(1.0, 0.0, 0.0);
        let mut b = PathBuilder::new();
        b.rect(0.0, 0.0, 10.0, 10.0);
        let svg = b.finish_fill(&gs);
        assert!(svg.contains(r#"fill="rgb(255,0,0)""#), "RGB 반영: {}", svg);
    }
}
