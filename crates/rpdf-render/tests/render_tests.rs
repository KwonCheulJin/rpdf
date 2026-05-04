use std::path::Path;

fn examples_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("examples")
}

fn lib_path_or_skip() -> Option<std::path::PathBuf> {
    match std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        Ok(p) => Some(std::path::PathBuf::from(p)),
        Err(_) => None,
    }
}

// IT-R1: fw4-2024.pdf 첫 페이지 렌더링 성공 — 이미지 크기 0 이상
#[test]
fn it_r1_render_fw4() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("fw4-2024.pdf");
    let img = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0).expect("렌더링 실패");
    assert!(img.width() > 0 && img.height() > 0, "이미지 크기가 0");
}

// IT-R2: irs-f1040.pdf 첫 페이지 렌더링 성공
#[test]
fn it_r2_render_irs_f1040() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("irs-f1040.pdf");
    let img = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0).expect("렌더링 실패");
    assert!(img.width() > 0 && img.height() > 0);
}

// IT-R3: pdfjs-annotation-border.pdf 첫 페이지 렌더링 성공
#[test]
fn it_r3_render_pdfjs_annotation_border() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("pdfjs-annotation-border.pdf");
    let img = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0).expect("렌더링 실패");
    assert!(img.width() > 0 && img.height() > 0);
}

// IT-R4: pdfjs-basicapi.pdf 첫 페이지 렌더링 성공
#[test]
fn it_r4_render_pdfjs_basicapi() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("pdfjs-basicapi.pdf");
    let img = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0).expect("렌더링 실패");
    assert!(img.width() > 0 && img.height() > 0);
}

// IT-R5: pdfjs-tracemonkey.pdf 첫 페이지 렌더링 성공
#[test]
fn it_r5_render_pdfjs_tracemonkey() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("pdfjs-tracemonkey.pdf");
    let img = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0).expect("렌더링 실패");
    assert!(img.width() > 0 && img.height() > 0);
}

// IT-R6: 존재하지 않는 PDF → RenderError::FileOpen 반환
#[test]
fn it_r6_nonexistent_pdf_returns_file_open_error() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("does_not_exist.pdf");
    let result = rpdf_render::render_page(&lib_path, &pdf, 0, 2.0);
    assert!(
        matches!(result, Err(rpdf_render::RenderError::FileOpen(_))),
        "예상: FileOpen 에러, 실제: {:?}",
        result
    );
}

// IT-R7: 범위 초과 페이지 인덱스 → RenderError::PageAccess 반환
#[test]
fn it_r7_out_of_range_page_returns_page_access_error() {
    let Some(lib_path) = lib_path_or_skip() else {
        return;
    };
    let pdf = examples_dir().join("pdfjs-basicapi.pdf");
    let result = rpdf_render::render_page(&lib_path, &pdf, 9999, 2.0);
    assert!(
        matches!(result, Err(rpdf_render::RenderError::PageAccess(9999))),
        "예상: PageAccess(9999) 에러, 실제: {:?}",
        result
    );
}

// IT-R8: scale <= 0.0 → RenderError::InvalidScale 반환 (pdfium 불필요)
#[test]
fn it_r8_invalid_scale_returns_error() {
    use std::path::PathBuf;
    let dummy_lib = PathBuf::from("/tmp/nonexistent");
    let dummy_pdf = PathBuf::from("/tmp/nonexistent.pdf");
    let result = rpdf_render::render_page(&dummy_lib, &dummy_pdf, 0, 0.0);
    assert!(
        matches!(result, Err(rpdf_render::RenderError::InvalidScale(_))),
        "예상: InvalidScale 에러, 실제: {:?}",
        result
    );
}
