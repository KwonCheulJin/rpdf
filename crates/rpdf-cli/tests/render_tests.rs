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
