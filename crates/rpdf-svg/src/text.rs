use crate::state::GraphicsState;

/// 텍스트 객체 상태.
///
/// `BeginText(BT)` ~ `EndText(ET)` 범위에서 유효하다.
#[derive(Debug, Clone, Default)]
pub struct TextState {
    /// 현재 텍스트 행렬 원점 X (SetTextMatrix의 e).
    pub tx: f64,
    /// 현재 텍스트 행렬 원점 Y (SetTextMatrix의 f).
    pub ty: f64,
}

impl TextState {
    pub fn new() -> Self {
        Self::default()
    }

    /// SetTextMatrix `Tm` 처리: 텍스트 위치를 (e, f)로 설정한다.
    pub fn set_matrix(&mut self, e: f64, f: f64) {
        self.tx = e;
        self.ty = f;
    }

    /// MoveText `Td` 처리: 현재 위치에 (tx, ty)를 더한다.
    pub fn move_by(&mut self, tx: f64, ty: f64) {
        self.tx += tx;
        self.ty += ty;
    }

    /// ShowText / ShowTextAdjusted 처리 → SVG `<text>` 요소 문자열 반환.
    ///
    /// Y축 반전 보정을 위해 `transform="scale(1,-1) translate(0,-{ty})"` 적용.
    pub fn show_text(&self, text: &str, state: &GraphicsState) -> String {
        // Y축 반전 그룹 안에 있으므로 텍스트가 뒤집히는 것을 다시 역보정한다.
        // translate(0, -ty)는 "부모 transform의 y flip 이후 위치 보정" 역할.
        let ty = self.ty;
        format!(
            r#"<text x="{}" y="{}" fill="{}" transform="scale(1,-1) translate(0,-{})">{}</text>"#,
            fmt_f64(self.tx),
            fmt_f64(ty),
            state.fill_color.to_svg_string(),
            fmt_f64(ty),
            escape_xml(text),
        )
    }
}

/// XML 특수문자를 이스케이프한다.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

#[cfg(test)]
mod internal_tests {
    use super::*;
    use crate::state::GraphicsState;

    fn black_state() -> GraphicsState {
        GraphicsState::new()
    }

    // TX-1: SetTextMatrix → tx, ty 설정
    #[test]
    fn tx1_set_matrix_updates_position() {
        let mut ts = TextState::new();
        ts.set_matrix(100.0, 200.0);
        assert_eq!(ts.tx, 100.0);
        assert_eq!(ts.ty, 200.0);
    }

    // TX-2: MoveText → 위치 누적
    #[test]
    fn tx2_move_text_accumulates() {
        let mut ts = TextState::new();
        ts.set_matrix(10.0, 20.0);
        ts.move_by(5.0, -3.0);
        assert_eq!(ts.tx, 15.0);
        assert_eq!(ts.ty, 17.0);
    }

    // TX-3: ShowText → <text> 요소 포함
    #[test]
    fn tx3_show_text_contains_text_element() {
        let mut ts = TextState::new();
        ts.set_matrix(50.0, 100.0);
        let svg = ts.show_text("Hello", &black_state());
        assert!(svg.contains("<text"), "text 태그: {}", svg);
        assert!(svg.contains("Hello"), "텍스트 내용: {}", svg);
        assert!(svg.contains(r#"x="50""#), "x 속성: {}", svg);
        assert!(svg.contains(r#"y="100""#), "y 속성: {}", svg);
    }

    // TX-4: ShowText → XML 이스케이프
    #[test]
    fn tx4_show_text_escapes_xml() {
        let ts = TextState::new();
        let svg = ts.show_text("<b>&\"test\"</b>", &black_state());
        assert!(svg.contains("&lt;b&gt;"), "lt/gt 이스케이프: {}", svg);
        assert!(svg.contains("&amp;"), "amp 이스케이프: {}", svg);
        assert!(svg.contains("&quot;"), "quot 이스케이프: {}", svg);
    }

    // TX-5: ShowText transform 속성 포함
    #[test]
    fn tx5_show_text_has_transform() {
        let mut ts = TextState::new();
        ts.set_matrix(30.0, 50.0);
        let svg = ts.show_text("X", &black_state());
        assert!(svg.contains("scale(1,-1)"), "scale transform: {}", svg);
        assert!(
            svg.contains("translate(0,-50)"),
            "translate transform: {}",
            svg
        );
    }
}
