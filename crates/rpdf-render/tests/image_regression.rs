//! samples/ 28개 PDF에 대한 이미지 회귀 테스트.
//!
//! 실행 조건: `PDFIUM_DYNAMIC_LIB_PATH` 환경변수가 설정되어 있어야 한다.
//! 미설정 시 전체 테스트가 skip된다.

mod snapshot_utils;

use std::path::{Path, PathBuf};

use image::DynamicImage;
use snapshot_utils::{DIFF_THRESHOLD, diff_image, normalized_diff};

const BROKEN_PDFS: &[&str] = &["broken-bad-xref-offset.pdf", "broken-missing-trailer.pdf"];

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("samples")
}

fn snapshots_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(os_name())
}

fn diff_output_dir() -> PathBuf {
    match std::env::var("RPDF_DIFF_DIR") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from("/tmp/rpdf-diff"),
    }
}

fn os_name() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "windows"
    }
}

fn is_broken(filename: &str) -> bool {
    BROKEN_PDFS.contains(&filename)
}

/// UPDATE_SNAPSHOTS 또는 linux 첫 실행 자동 적용 여부를 반환한다.
fn should_update_snapshots() -> bool {
    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        return true;
    }
    // linux CI 첫 실행: 스냅샷 없을 때 자동 생성
    cfg!(target_os = "linux")
}

fn save_snapshot(path: &Path, img: &DynamicImage) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("스냅샷 디렉터리 생성 실패");
    }
    img.save(path).expect("스냅샷 저장 실패");
}

fn save_diff(stem: &str, diff: &DynamicImage) -> PathBuf {
    let dir = diff_output_dir();
    std::fs::create_dir_all(&dir).expect("diff 디렉터리 생성 실패");
    let path = dir.join(format!("{stem}_diff.png"));
    diff.save(&path).expect("diff 이미지 저장 실패");
    path
}

fn run_single_regression(lib_path: &Path, pdf_path: &Path, stem: &str, filename: &str) {
    let result = rpdf_render::render_page(lib_path, pdf_path, 0, 1.0);

    let rendered = match result {
        Err(_) if is_broken(filename) => {
            // broken PDF 렌더링 실패 → 예상된 에러, 통과
            return;
        }
        Err(e) => panic!("렌더링 실패 ({stem}): {e}"),
        Ok(img) => img,
    };

    let snapshot_path = snapshots_dir().join(format!("{stem}_p0.png"));

    if !snapshot_path.exists() {
        if should_update_snapshots() {
            save_snapshot(&snapshot_path, &rendered);
            // 스냅샷 생성 완료 → 비교 없이 통과
            return;
        } else {
            panic!(
                "기준 PNG 없음: {}\nUPDATE_SNAPSHOTS=1 환경변수를 설정해 스냅샷을 생성하세요.",
                snapshot_path.display()
            );
        }
    }

    if should_update_snapshots() {
        save_snapshot(&snapshot_path, &rendered);
        return;
    }

    let baseline = image::open(&snapshot_path)
        .unwrap_or_else(|e| panic!("스냅샷 로딩 실패 ({}): {e}", snapshot_path.display()));

    let diff = normalized_diff(&baseline, &rendered);

    if diff > DIFF_THRESHOLD {
        let diff_img = diff_image(&baseline, &rendered);
        let diff_path = save_diff(stem, &diff_img);
        panic!(
            "이미지 회귀 감지 ({stem}): normalized_diff={diff:.6} > threshold={DIFF_THRESHOLD}\ndiff 이미지: {}",
            diff_path.display()
        );
    }
}

#[test]
fn image_regression_all_samples() {
    let lib_path = match std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            // PDFIUM_DYNAMIC_LIB_PATH 미설정 → 전체 skip
            return;
        }
    };

    let samples = samples_dir();
    let mut pdf_files: Vec<_> = std::fs::read_dir(&samples)
        .unwrap_or_else(|e| panic!("samples 디렉터리 읽기 실패: {e}"))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "pdf")
                .unwrap_or(false)
        })
        .collect();

    pdf_files.sort_by_key(|e| e.file_name());

    let mut failures: Vec<String> = Vec::new();

    for entry in &pdf_files {
        let pdf_path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy().to_string();
        let stem = pdf_path.file_stem().unwrap().to_string_lossy().to_string();

        let result = std::panic::catch_unwind(|| {
            run_single_regression(&lib_path, &pdf_path, &stem, &filename_str);
        });

        if let Err(e) = result {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                (*s).to_string()
            } else {
                format!("알 수 없는 패닉 ({stem})")
            };
            failures.push(msg);
        }
    }

    if !failures.is_empty() {
        panic!(
            "이미지 회귀 테스트 실패 ({}/{}개):\n\n{}",
            failures.len(),
            pdf_files.len(),
            failures.join("\n\n---\n\n")
        );
    }
}
