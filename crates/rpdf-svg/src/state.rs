/// RGB 색상 (0~255 범위).
///
/// PDF SetFillRGB / SetStrokeRGB 피연산자(0.0~1.0)에서 변환.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// PDF float 피연산자(0.0~1.0)에서 Color를 생성한다.
    ///
    /// 입력이 [0.0, 1.0] 범위를 벗어나면 0 또는 255로 클램프한다.
    pub fn from_pdf_floats(r: f64, g: f64, b: f64) -> Self {
        let clamp = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self {
            r: clamp(r),
            g: clamp(g),
            b: clamp(b),
        }
    }

    /// SVG `fill` / `stroke` 속성용 문자열 `"rgb(r,g,b)"` 반환.
    pub fn to_svg_string(self) -> String {
        format!("rgb({},{},{})", self.r, self.g, self.b)
    }
}

/// 단일 그래픽 상태 스냅샷.
///
/// `SaveState(q)` 시 Rust 스택에 push, `RestoreState(Q)` 시 pop.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsState {
    pub fill_color: Color,
    pub stroke_color: Color,
    pub line_width: f64,
}

impl GraphicsState {
    /// PDF 초기 기본값으로 그래픽 상태를 생성한다.
    pub fn new() -> Self {
        Self {
            fill_color: Color::default(),
            stroke_color: Color::default(),
            line_width: 1.0,
        }
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

/// 그래픽 상태 스택.
///
/// `q`/`Q` 연산자에 대응하는 push/pop을 관리한다.
/// 언더플로(초과 Q)는 무시하고 계속 진행한다.
pub struct StateStack {
    current: GraphicsState,
    stack: Vec<GraphicsState>,
}

impl StateStack {
    /// 초기 상태로 스택을 생성한다.
    pub fn new() -> Self {
        Self {
            current: GraphicsState::new(),
            stack: Vec::new(),
        }
    }

    /// 현재 그래픽 상태를 반환한다.
    pub fn current(&self) -> &GraphicsState {
        &self.current
    }

    /// 현재 그래픽 상태를 스택에 저장한다 (`q`).
    pub fn push(&mut self) {
        self.stack.push(self.current.clone());
    }

    /// 스택에서 이전 그래픽 상태를 복원한다 (`Q`).
    ///
    /// 스택이 비어 있으면 (언더플로) 아무것도 하지 않는다.
    pub fn pop(&mut self) {
        if let Some(saved) = self.stack.pop() {
            self.current = saved;
        }
    }

    /// 현재 fill 색상을 설정한다.
    pub fn set_fill_color(&mut self, color: Color) {
        self.current.fill_color = color;
    }

    /// 현재 stroke 색상을 설정한다.
    pub fn set_stroke_color(&mut self, color: Color) {
        self.current.stroke_color = color;
    }

    /// 현재 선 너비를 설정한다.
    pub fn set_line_width(&mut self, width: f64) {
        self.current.line_width = width;
    }
}

impl Default for StateStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    // ST-1: GraphicsState::new() 기본값
    #[test]
    fn st1_default_state() {
        let gs = GraphicsState::new();
        assert_eq!(gs.fill_color, Color { r: 0, g: 0, b: 0 });
        assert_eq!(gs.stroke_color, Color { r: 0, g: 0, b: 0 });
        assert_eq!(gs.line_width, 1.0);
    }

    // ST-2: Color::from_pdf_floats() 반올림
    #[test]
    fn st2_color_from_pdf_floats_rounds() {
        let c = Color::from_pdf_floats(1.0, 0.5, 0.0);
        assert_eq!(c.r, 255);
        // 0.5 * 255 = 127.5 → 반올림 128
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 0);
    }

    // ST-3: Color::to_svg_string() 형식
    #[test]
    fn st3_color_to_svg_string() {
        let c = Color {
            r: 255,
            g: 128,
            b: 0,
        };
        assert_eq!(c.to_svg_string(), "rgb(255,128,0)");
    }

    // ST-4: StateStack push/pop 색상 복원
    #[test]
    fn st4_state_stack_push_pop_restores_color() {
        let mut stack = StateStack::new();
        stack.set_fill_color(Color { r: 255, g: 0, b: 0 });
        stack.push();
        stack.set_fill_color(Color { r: 0, g: 255, b: 0 });
        assert_eq!(stack.current().fill_color, Color { r: 0, g: 255, b: 0 });
        stack.pop();
        assert_eq!(stack.current().fill_color, Color { r: 255, g: 0, b: 0 });
    }

    // ST-5: StateStack 언더플로 무시
    #[test]
    fn st5_state_stack_underflow_ignored() {
        let mut stack = StateStack::new();
        stack.set_fill_color(Color {
            r: 100,
            g: 100,
            b: 100,
        });
        // pop without push — should not panic
        stack.pop();
        // color unchanged
        assert_eq!(
            stack.current().fill_color,
            Color {
                r: 100,
                g: 100,
                b: 100
            }
        );
    }

    // ST-6: StateStack 선폭 복원
    #[test]
    fn st6_state_stack_restores_line_width() {
        let mut stack = StateStack::new();
        stack.set_line_width(3.0);
        stack.push();
        stack.set_line_width(7.5);
        assert_eq!(stack.current().line_width, 7.5);
        stack.pop();
        assert_eq!(stack.current().line_width, 3.0);
    }

    // ST-7: Color 범위 클램프
    #[test]
    fn st7_color_clamps_out_of_range() {
        let c = Color::from_pdf_floats(-0.5, 2.0, 0.5);
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 128);
    }
}
