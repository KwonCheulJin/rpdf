use std::path::Path;

use rpdf_core::types::Page;
use rpdf_svg::{RenderOptions, render_page_svg, render_page_svg_with_options};

fn examples_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("examples")
        .join(name)
}

fn load_first_page(pdf_name: &str) -> Page {
    let data = std::fs::read(examples_path(pdf_name)).expect("PDF 파일 읽기 실패");
    let doc = rpdf_parser::load_document(&data).expect("PDF 파싱 실패");
    doc.pages().first().expect("페이지 없음").clone()
}

fn empty_page() -> Page {
    Page {
        index: 0,
        content: vec![],
        resources: None,
        media_box: None,
        crop_box: None,
        rotation: 0,
    }
}

// IT-S1: pdfjs-basicapi.pdf 첫 페이지 → 결과에 "<svg" 포함
#[test]
fn it_s1_render_basicapi_contains_svg_open() {
    let page = load_first_page("pdfjs-basicapi.pdf");
    let svg = render_page_svg(&page);
    assert!(
        svg.contains("<svg"),
        "SVG 루트 태그 없음:\n{}",
        &svg[..svg.len().min(500)]
    );
}

// IT-S2: pdfjs-basicapi.pdf → 결과에 "</svg>" 포함 (태그 닫힘)
#[test]
fn it_s2_render_basicapi_contains_svg_close() {
    let page = load_first_page("pdfjs-basicapi.pdf");
    let svg = render_page_svg(&page);
    assert!(svg.contains("</svg>"), "SVG 닫힘 태그 없음");
}

// IT-S3: media_box 없는 빈 Page → 에러 없이 유효한 <svg> 반환
#[test]
fn it_s3_empty_page_without_media_box_returns_valid_svg() {
    let page = empty_page();
    let svg = render_page_svg(&page);
    assert!(svg.contains("<svg"), "빈 페이지 SVG 루트 없음: {}", svg);
    assert!(svg.contains("</svg>"), "빈 페이지 SVG 닫힘 없음: {}", svg);
    // A4 기본값 검증
    assert!(svg.contains("width=\"595\""), "A4 width 없음: {}", svg);
    assert!(svg.contains("height=\"842\""), "A4 height 없음: {}", svg);
}

// IT-S4: fw4-2024.pdf 첫 페이지도 유효한 SVG 반환
#[test]
fn it_s4_render_fw4_first_page() {
    let page = load_first_page("fw4-2024.pdf");
    let svg = render_page_svg(&page);
    assert!(svg.contains("<svg"), "fw4 SVG 루트 없음");
    assert!(svg.contains("</svg>"), "fw4 SVG 닫힘 없음");
}

// IT-S5: Y축 반전 변환 그룹 존재 확인
#[test]
fn it_s5_y_flip_transform_present() {
    let page = empty_page();
    let svg = render_page_svg(&page);
    assert!(
        svg.contains("matrix(1 0 0 -1 0"),
        "Y축 반전 transform 없음: {}",
        svg
    );
}

// IT-D1: debug_overlay: true → id="debug-overlay" 포함
#[test]
fn it_d1_debug_overlay_true_contains_overlay_group() {
    let page = load_first_page("pdfjs-basicapi.pdf");
    let opts = RenderOptions {
        debug_overlay: true,
    };
    let svg = render_page_svg_with_options(&page, &opts);
    assert!(
        svg.contains("id=\"debug-overlay\""),
        "debug-overlay 그룹 없음: {}",
        &svg[..svg.len().min(500)]
    );
}

// IT-D2: RenderOptions::default() → id="debug-overlay" 미포함
#[test]
fn it_d2_debug_overlay_default_no_overlay_group() {
    let page = load_first_page("pdfjs-basicapi.pdf");
    let opts = RenderOptions::default();
    let svg = render_page_svg_with_options(&page, &opts);
    assert!(
        !svg.contains("id=\"debug-overlay\""),
        "기본 옵션인데 debug-overlay 그룹이 있음"
    );
}

// IT-D3: render_page_svg() 와 render_page_svg_with_options(default) 결과 동일
#[test]
fn it_d3_render_page_svg_equiv_default_options() {
    let page = load_first_page("pdfjs-basicapi.pdf");
    let svg_legacy = render_page_svg(&page);
    let svg_opts = render_page_svg_with_options(&page, &RenderOptions::default());
    assert_eq!(
        svg_legacy, svg_opts,
        "render_page_svg()와 render_page_svg_with_options(default) 결과가 다름"
    );
}
