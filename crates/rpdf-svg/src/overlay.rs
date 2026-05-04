/// 디버그 오버레이 SVG 문자열을 생성한다.
///
/// 반환값은 `<g id="debug-overlay">...</g>` 요소 전체 문자열.
/// PDF 좌표계(좌하단 기준)를 SVG 좌표계(좌상단 기준)로 변환해 그리드·경계·원점을 배치한다.
pub(crate) fn build_overlay(w: f64, h: f64) -> String {
    let mut out = String::new();
    out.push_str("<g id=\"debug-overlay\">\n");

    // 1. 페이지 경계 사각형
    out.push_str(&format!(
        "<rect x=\"0.5\" y=\"0.5\" width=\"{}\" height=\"{}\" \
         fill=\"none\" stroke=\"rgba(0,0,255,0.6)\" \
         stroke-width=\"1.5\" stroke-dasharray=\"6 3\"/>\n",
        w - 1.0,
        h - 1.0,
    ));

    // 2. 좌표 그리드 (100pt 간격)
    // x: 100, 200, ... (w 미만)
    let mut x = 100.0_f64;
    while x < w {
        let svg_x = x;
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" \
             stroke=\"rgba(128,128,128,0.3)\" stroke-width=\"0.5\"/>\n",
            svg_x, svg_x, h,
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"9\" fill=\"rgba(0,0,200,0.7)\" \
             font-family=\"monospace\">{}</text>\n",
            svg_x + 3.0,
            h - 4.0,
            x as u32,
        ));
        x += 100.0;
    }

    // y: 100, 200, ... (h 미만). SVG y 좌표 = h - pdf_y
    let mut y = 100.0_f64;
    while y < h {
        let svg_y = h - y;
        out.push_str(&format!(
            "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" \
             stroke=\"rgba(128,128,128,0.3)\" stroke-width=\"0.5\"/>\n",
            svg_y, w, svg_y,
        ));
        out.push_str(&format!(
            "<text x=\"4\" y=\"{}\" font-size=\"9\" fill=\"rgba(0,0,200,0.7)\" \
             font-family=\"monospace\">{}</text>\n",
            svg_y - 3.0,
            y as u32,
        ));
        y += 100.0;
    }

    // 3. 원점 마커. PDF (0,0) = SVG (0, h)
    out.push_str(&format!(
        "<circle cx=\"0\" cy=\"{}\" r=\"5\" \
         fill=\"rgba(255,0,0,0.7)\" stroke=\"none\"/>\n",
        h,
    ));
    out.push_str(&format!(
        "<text x=\"7\" y=\"{}\" font-size=\"10\" fill=\"rgba(200,0,0,0.9)\" \
         font-family=\"monospace\">(0,0)</text>\n",
        h - 4.0,
    ));

    out.push_str("</g>\n");
    out
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn build_overlay_contains_rect() {
        let result = build_overlay(595.0, 842.0);
        assert!(result.contains("<rect"), "경계 사각형 없음");
    }

    #[test]
    fn build_overlay_contains_origin_label() {
        let result = build_overlay(595.0, 842.0);
        assert!(result.contains("(0,0)"), "원점 레이블 없음");
    }

    #[test]
    fn build_overlay_contains_grid_label() {
        let result = build_overlay(595.0, 842.0);
        assert!(result.contains(">100<"), "그리드 레이블 없음");
    }

    #[test]
    fn build_overlay_contains_debug_overlay_id() {
        let result = build_overlay(595.0, 842.0);
        assert!(
            result.contains("id=\"debug-overlay\""),
            "debug-overlay id 없음"
        );
    }

    #[test]
    fn build_overlay_small_page_no_grid_lines() {
        let result = build_overlay(50.0, 80.0);
        assert!(
            !result.contains("<line"),
            "소형 페이지에 그리드 선이 있으면 안 됨: {}",
            result
        );
    }

    #[test]
    fn build_overlay_small_page_has_rect() {
        let result = build_overlay(50.0, 80.0);
        assert!(result.contains("<rect"), "소형 페이지 경계 사각형 없음");
    }
}
