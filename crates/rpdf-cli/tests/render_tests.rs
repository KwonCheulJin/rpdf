use assert_cmd::Command;
use std::path::Path;

fn rpdf() -> Command {
    Command::cargo_bin("rpdf").expect("rpdf binary not found")
}

fn pdf(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("examples")
        .join(name);
    path.to_string_lossy().into_owned()
}

fn has_pdfium() -> bool {
    std::env::var("PDFIUM_DYNAMIC_LIB_PATH").is_ok()
}

// IT-F1: rpdf render pdfjs-basicapi.pdf -o /tmp/out.png → exit 0, 파일 생성
#[test]
fn it_f1_render_creates_output_file() {
    if !has_pdfium() {
        return;
    }
    let out = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .args(["render", &pdf("pdfjs-basicapi.pdf"), "-o", &out_path])
        .assert()
        .success();

    assert!(
        std::fs::metadata(&out_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false),
        "출력 파일이 존재하지 않거나 비어 있음"
    );
}

// IT-F2: rpdf render fw4-2024.pdf -p 2 -o /tmp/out.png → exit 0
#[test]
fn it_f2_render_page_2_success() {
    if !has_pdfium() {
        return;
    }
    let out = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .args(["render", &pdf("fw4-2024.pdf"), "-p", "2", "-o", &out_path])
        .assert()
        .success();
}

// IT-F3: rpdf render --scale 1.0 pdfjs-basicapi.pdf -o /tmp/out.png → exit 0
#[test]
fn it_f3_render_with_scale_option() {
    if !has_pdfium() {
        return;
    }
    let out = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .args([
            "render",
            &pdf("pdfjs-basicapi.pdf"),
            "--scale",
            "1.0",
            "-o",
            &out_path,
        ])
        .assert()
        .success();
}

// IT-F4: 존재하지 않는 PDF → exit 1, stderr 비어 있지 않음
#[test]
fn it_f4_nonexistent_pdf_fails() {
    if !has_pdfium() {
        return;
    }
    rpdf()
        .args([
            "render",
            "/tmp/does_not_exist_xyz.pdf",
            "-o",
            "/tmp/out.png",
        ])
        .assert()
        .failure();
}

// IT-F5: PDFIUM_DYNAMIC_LIB_PATH 미설정 → exit 1
#[test]
fn it_f5_missing_pdfium_env_fails() {
    rpdf()
        .env_remove("PDFIUM_DYNAMIC_LIB_PATH")
        .args(["render", &pdf("pdfjs-basicapi.pdf"), "-o", "/tmp/out.png"])
        .assert()
        .failure();
}

// IT-S4: rpdf render pdfjs-basicapi.pdf --svg → .svg 파일 생성, 내용에 <svg 포함
#[test]
fn it_s4_render_svg_creates_svg_file() {
    let out = tempfile::Builder::new()
        .suffix(".svg")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .args([
            "render",
            &pdf("pdfjs-basicapi.pdf"),
            "--svg",
            "-o",
            &out_path,
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_path).expect("SVG 파일 읽기 실패");
    assert!(
        content.contains("<svg"),
        "SVG 루트 태그 없음: {}",
        &content[..content.len().min(200)]
    );
    assert!(content.contains("</svg>"), "SVG 닫힘 태그 없음");
}

// IT-S5: rpdf render pdfjs-basicapi.pdf --svg (PDFIUM 환경변수 없어도 동작)
#[test]
fn it_s5_render_svg_no_pdfium_needed() {
    let out = tempfile::Builder::new()
        .suffix(".svg")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .env_remove("PDFIUM_DYNAMIC_LIB_PATH")
        .args([
            "render",
            &pdf("pdfjs-basicapi.pdf"),
            "--svg",
            "-o",
            &out_path,
        ])
        .assert()
        .success();
}

// IT-D4: rpdf render pdfjs-basicapi.pdf --svg --debug-overlay → id="debug-overlay" 포함
#[test]
fn it_d4_render_svg_debug_overlay_contains_overlay_group() {
    let out = tempfile::Builder::new()
        .suffix(".svg")
        .tempfile()
        .expect("tempfile 생성 실패");
    let out_path = out.path().to_string_lossy().into_owned();

    rpdf()
        .args([
            "render",
            &pdf("pdfjs-basicapi.pdf"),
            "--svg",
            "--debug-overlay",
            "-o",
            &out_path,
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&out_path).expect("SVG 파일 읽기 실패");
    assert!(
        content.contains("id=\"debug-overlay\""),
        "debug-overlay 그룹 없음: {}",
        &content[..content.len().min(500)]
    );
}

// IT-D5: rpdf render pdfjs-basicapi.pdf --debug-overlay (--svg 없음) → stderr에 Warning: 포함
#[test]
fn it_d5_debug_overlay_without_svg_warns() {
    if !has_pdfium() {
        // PDFIUM 없을 때는 PNG 렌더링 자체가 실패하지만 warning은 출력된다
        rpdf()
            .env_remove("PDFIUM_DYNAMIC_LIB_PATH")
            .args(["render", &pdf("pdfjs-basicapi.pdf"), "--debug-overlay"])
            .assert()
            .stderr(predicates::str::contains("Warning:"));
    } else {
        let out = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .expect("tempfile 생성 실패");
        let out_path = out.path().to_string_lossy().into_owned();

        rpdf()
            .args([
                "render",
                &pdf("pdfjs-basicapi.pdf"),
                "--debug-overlay",
                "-o",
                &out_path,
            ])
            .assert()
            .stderr(predicates::str::contains("Warning:"));
    }
}
