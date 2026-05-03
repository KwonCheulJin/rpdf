use proptest::prelude::*;
use rpdf_parser::load_document;

// ── IT-13 ~ IT-17: load_document 통합 테스트 ───────────────────────────────

/// 파일을 로드해 페이지 수와 MediaBox 유효성을 검증한다.
///
/// 사전 확인 (Checkpoint E 완료 후 scan_page_count 실행 결과):
/// - fw4-2024.pdf:              5페이지
/// - irs-f1040.pdf:             2페이지
/// - pdfjs-basicapi.pdf:        3페이지
/// - pdfjs-tracemonkey.pdf:    14페이지
/// - pdfjs-annotation-border.pdf: 1페이지
fn load_and_check(bytes: &[u8], expected_pages: usize) {
    let doc = load_document(bytes).expect("load_document 실패");
    assert_eq!(doc.page_count(), expected_pages);
    for page in doc.pages() {
        // MediaBox 있으면 유효한 값인지 확인 (x1 > x0, y1 > y0)
        if let Some(mb) = page.media_box() {
            assert!(mb[2] > mb[0] && mb[3] > mb[1], "invalid MediaBox: {:?}", mb);
        }
    }
}

// ── IT-13: fw4-2024.pdf — xref stream + ObjStm ───────────────────────────

#[test]
fn it13_fw4_2024_load_document() {
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");
    load_and_check(data, 5);
}

// ── IT-14: irs-f1040.pdf — xref stream + ObjStm ──────────────────────────

#[test]
fn it14_irs_f1040_load_document() {
    let data = include_bytes!("../../../../examples/irs-f1040.pdf");
    load_and_check(data, 2);
}

// ── IT-15: pdfjs-basicapi.pdf ────────────────────────────────────────────

#[test]
fn it15_pdfjs_basicapi_load_document() {
    let data = include_bytes!("../../../../examples/pdfjs-basicapi.pdf");
    load_and_check(data, 3);
}

// ── IT-16: pdfjs-tracemonkey.pdf — 대용량 ───────────────────────────────

#[test]
fn it16_pdfjs_tracemonkey_load_document() {
    let data = include_bytes!("../../../../examples/pdfjs-tracemonkey.pdf");
    load_and_check(data, 14);
}

// ── IT-17: pdfjs-annotation-border.pdf — 증분 업데이트 ──────────────────

#[test]
fn it17_pdfjs_annotation_border_load_document() {
    let data = include_bytes!("../../../../examples/pdfjs-annotation-border.pdf");
    load_and_check(data, 1);
}

// ── proptest: 임의 입력 패닉 없음 ────────────────────────────────────────

proptest! {
    /// 임의 바이트 입력에 대해 load_document가 패닉을 일으키지 않는다.
    #[test]
    fn arbitrary_input_never_panics_load_document(
        data in proptest::collection::vec(any::<u8>(), 0..65536)
    ) {
        let _ = load_document(&data);
    }
}
