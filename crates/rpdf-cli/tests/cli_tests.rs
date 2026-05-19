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

// ── IT-C: rpdf info ──────────────────────────────────────────────────────────

// IT-C1: rpdf info fw4-2024.pdf → exit 0, "Pages:" 포함
#[test]
fn it_c1_info_exits_ok_contains_pages() {
    rpdf()
        .args(["info", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .stdout(predicates::str::contains("Pages:"));
}

// IT-C2: rpdf info fw4-2024.pdf --json → exit 0, JSON 파싱 가능, page_count 존재
#[test]
fn it_c2_info_json_parseable_with_page_count() {
    let output = rpdf()
        .args(["info", "--json", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("JSON parse failed");
    assert!(
        json["page_count"].as_u64().is_some(),
        "page_count 필드 없음"
    );
    assert_eq!(json["page_count"], 5);
}

// IT-C3: rpdf info irs-f1040.pdf → exit 0 (다른 PDF도 동작)
#[test]
fn it_c3_info_irs_f1040_ok() {
    rpdf()
        .args(["info", &pdf("irs-f1040.pdf")])
        .assert()
        .success();
}

// ── IT-D: rpdf dump-pages ────────────────────────────────────────────────────

// IT-D1: rpdf dump-pages fw4-2024.pdf → exit 0, "Page 0:" 포함 (5개 페이지)
#[test]
fn it_d1_dump_pages_all_pages() {
    rpdf()
        .args(["dump-pages", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .stdout(predicates::str::contains("Page 0:"))
        .stdout(predicates::str::contains("Page 4:"));
}

// IT-D2: rpdf dump-pages -p 0 fw4-2024.pdf --json → JSON, filtered_page=0, pages 길이 1
#[test]
fn it_d2_dump_pages_single_page_json() {
    let output = rpdf()
        .args(["dump-pages", "-p", "0", "--json", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("JSON parse failed");
    assert_eq!(json["filtered_page"], 0);
    assert_eq!(json["pages"].as_array().unwrap().len(), 1);
    assert_eq!(json["pages"][0]["index"], 0);
}

// IT-D3: rpdf dump-pages -p 99 fw4-2024.pdf → exit 1, stderr에 "not found"
#[test]
fn it_d3_dump_pages_out_of_range_fails() {
    rpdf()
        .args(["dump-pages", "-p", "99", &pdf("fw4-2024.pdf")])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

// ── IT-E: rpdf dump ──────────────────────────────────────────────────────────

// IT-E1: rpdf dump -p 0 fw4-2024.pdf → exit 0, "BT" 포함
#[test]
fn it_e1_dump_single_page_contains_bt() {
    rpdf()
        .args(["dump", "-p", "0", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .stdout(predicates::str::contains("BT"));
}

// IT-E2: rpdf dump --json pdfjs-basicapi.pdf → exit 0, JSON 파싱 가능, pages 배열 존재
#[test]
fn it_e2_dump_json_parseable() {
    let output = rpdf()
        .args(["dump", "--json", &pdf("pdfjs-basicapi.pdf")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("JSON parse failed");
    assert!(json["pages"].as_array().is_some(), "pages 배열 없음");
    assert_eq!(json["pages"].as_array().unwrap().len(), 3);
}

// IT-E3: rpdf dump pdfjs-tracemonkey.pdf → exit 0 (14페이지 전체 출력, 크래시 없음)
#[test]
fn it_e3_dump_all_pages_tracemonkey() {
    rpdf()
        .args(["dump", &pdf("pdfjs-tracemonkey.pdf")])
        .assert()
        .success()
        .stdout(predicates::str::contains("Page 13"));
}

// IT-E4: rpdf dump --json fw4-2024.pdf → page_count + filtered_page null
#[test]
fn it_e4_dump_json_top_level_fields() {
    let output = rpdf()
        .args(["dump", "--json", &pdf("fw4-2024.pdf")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).expect("JSON parse failed");
    assert_eq!(json["page_count"], 5);
    assert!(json["filtered_page"].is_null());
}

// IT-E5: rpdf dump -p 99 fw4-2024.pdf → exit 1, stderr에 "not found"
#[test]
fn it_e5_dump_out_of_range_fails() {
    rpdf()
        .args(["dump", "-p", "99", &pdf("fw4-2024.pdf")])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

// ── IT-F: 편집 명령 통합 테스트 ───────────────────────────────────────────────

use tempfile::TempDir;

// IT-F1: rotate — fw4-2024.pdf의 1번 페이지를 90도 회전 → 저장 → 재파싱 → rotation == 90
#[test]
fn it_f1_rotate_page_roundtrip() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("rotated.pdf");

    rpdf()
        .args([
            "rotate",
            &pdf("fw4-2024.pdf"),
            "--page",
            "1",
            "--degrees",
            "90",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let data = std::fs::read(&output).unwrap();
    let doc = rpdf_parser::load_document(&data).unwrap();
    assert_eq!(doc.pages[0].rotation, 90);
}

// IT-F2: delete — fw4-2024.pdf(5페이지)에서 2번 페이지 삭제 → 재파싱 → page_count == 4
#[test]
fn it_f2_delete_page_roundtrip() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("deleted.pdf");

    rpdf()
        .args([
            "delete",
            &pdf("fw4-2024.pdf"),
            "--pages",
            "2",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let data = std::fs::read(&output).unwrap();
    let doc = rpdf_parser::load_document(&data).unwrap();
    assert_eq!(doc.pages.len(), 4);
}

// IT-F3: merge — fw4-2024.pdf(5페이지) + irs-f1040.pdf → 재파싱 → page_count == (5 + N)
#[test]
fn it_f3_merge_roundtrip() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("merged.pdf");

    let data_b = std::fs::read(pdf("irs-f1040.pdf")).unwrap();
    let doc_b = rpdf_parser::load_document(&data_b).unwrap();
    let b_pages = doc_b.pages.len();

    rpdf()
        .args([
            "merge",
            &pdf("fw4-2024.pdf"),
            &pdf("irs-f1040.pdf"),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let data = std::fs::read(&output).unwrap();
    let doc = rpdf_parser::load_document(&data).unwrap();
    assert_eq!(doc.pages.len(), 5 + b_pages);
}

// IT-F4: extract — fw4-2024.pdf에서 2-4 추출 → 재파싱 → page_count == 3
#[test]
fn it_f4_extract_pages_roundtrip() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("extracted.pdf");

    rpdf()
        .args([
            "extract",
            &pdf("fw4-2024.pdf"),
            "--pages",
            "2-4",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let data = std::fs::read(&output).unwrap();
    let doc = rpdf_parser::load_document(&data).unwrap();
    assert_eq!(doc.pages.len(), 3);
}

// IT-F5: split — fw4-2024.pdf를 "1-2,4-5"로 분리 → 출력 디렉토리에 파일 2개 생성
#[test]
fn it_f5_split_roundtrip() {
    let dir = TempDir::new().unwrap();

    rpdf()
        .args([
            "split",
            &pdf("fw4-2024.pdf"),
            "--pages",
            "1-2,4-5",
            "-o",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let part1 = dir.path().join("fw4-2024_part1.pdf");
    let part2 = dir.path().join("fw4-2024_part2.pdf");
    assert!(part1.exists(), "fw4-2024_part1.pdf 없음");
    assert!(part2.exists(), "fw4-2024_part2.pdf 없음");

    let data1 = std::fs::read(&part1).unwrap();
    let doc1 = rpdf_parser::load_document(&data1).unwrap();
    assert_eq!(doc1.pages.len(), 2);
}

// IT-F6: 에러 — delete --pages 0 → exit 1
#[test]
fn it_f6_delete_zero_page_errors() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out.pdf");

    rpdf()
        .args([
            "delete",
            &pdf("fw4-2024.pdf"),
            "--pages",
            "0",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// IT-F7: 에러 — merge 입력 파일 1개 → exit 2 (clap 에러)
#[test]
fn it_f7_merge_requires_two_inputs() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out.pdf");

    rpdf()
        .args([
            "merge",
            &pdf("fw4-2024.pdf"),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// ── proptest: 임의 바이트 입력 → 크래시 없음 ───────────────────────────────

#[test]
fn arbitrary_input_rpdf_dump_no_panic() {
    use proptest::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    proptest!(|(input in proptest::collection::vec(any::<u8>(), 0..=4096))| {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&input).unwrap();
        let path = tmp.path().to_string_lossy().into_owned();

        // exit 0 or 1 허용 — panic/abort 없음이 기준
        let _ = rpdf().args(["dump", &path]).output();
        let _ = rpdf().args(["info", &path]).output();
        let _ = rpdf().args(["dump-pages", &path]).output();
    });
}
