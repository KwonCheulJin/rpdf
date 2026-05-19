use std::path::Path;
use std::sync::Arc;

use rpdf_edit::commands::{
    Command, DeletePagesCommand, ExtractPagesCommand, MergeCommand, Query, RotatePageCommand,
    SplitCommand,
};
use rpdf_serializer::{PageSource, SerializeError, load_document_tracked, serialize_document};

/// 테스트 픽스처 파일을 읽어 바이트로 반환한다.
fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {} 읽기 실패: {}", name, e))
}

// --- CP-B: 에러 테스트 ---

#[test]
fn serialize_empty_doc_error() {
    use rpdf_core::types::document::Document;

    let doc = Document {
        pages: vec![],
        metadata: None,
    };
    let sources: Vec<PageSource> = vec![];
    let err = serialize_document(&doc, &sources).unwrap_err();
    assert!(matches!(err, SerializeError::EmptyDocument), "got: {err}");
}

#[test]
fn serialize_source_length_mismatch_error() {
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (doc, sources) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 3);

    // sources를 2개만 전달 (pages는 3개)
    let truncated: Vec<PageSource> = sources.into_iter().take(2).collect();
    let err = serialize_document(&doc, &truncated).unwrap_err();
    assert!(
        matches!(
            err,
            SerializeError::SourceLengthMismatch {
                sources: 2,
                pages: 3
            }
        ),
        "got: {err}"
    );
}

#[test]
fn serialize_page_out_of_bounds_error() {
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (doc, _) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 3);

    // page_index가 원본 페이지 수(3)를 초과하는 PageSource 수동 구성
    let fake_source = PageSource {
        bytes: Arc::new(bytes),
        page_index: 99, // out of bounds
    };
    let sources = vec![fake_source];

    // doc을 1페이지짜리로 만들어야 SourceLengthMismatch를 피함
    use rpdf_core::types::document::{Document, Page};
    let single_page_doc = Document {
        pages: vec![Page {
            index: 0,
            content: vec![],
            resources: None,
            media_box: None,
            crop_box: None,
            rotation: 0,
        }],
        metadata: None,
    };
    let err = serialize_document(&single_page_doc, &sources).unwrap_err();
    assert!(
        matches!(err, SerializeError::PageOutOfBounds { idx: 99, .. }),
        "got: {err}"
    );
}

// --- CP-C: roundtrip + rotation 테스트 ---

#[test]
fn serialize_basic_roundtrip() {
    // pdfjs-basicapi.pdf: 3페이지
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (doc, sources) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 3, "픽스처는 3페이지여야 한다");

    let out = serialize_document(&doc, &sources).unwrap();
    assert!(!out.is_empty(), "직렬화 결과가 비어있으면 안 됨");

    // re-parse
    let (doc2, _) = load_document_tracked(&out).unwrap();
    assert_eq!(doc2.pages.len(), 3, "re-parse 후 페이지 수 동일해야 한다");
}

#[test]
fn serialize_after_rotate() {
    // 0→90 rotation 후 serialize → re-parse → rotation == 90
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (mut doc, sources) = load_document_tracked(&bytes).unwrap();

    let cmd = RotatePageCommand::new(0, 90);
    cmd.execute(&mut doc).unwrap();
    assert_eq!(doc.pages[0].rotation, 90);

    let out = serialize_document(&doc, &sources).unwrap();
    let (doc2, _) = load_document_tracked(&out).unwrap();
    assert_eq!(
        doc2.pages[0].rotation, 90,
        "re-parse 후 rotation == 90이어야 한다"
    );
}

#[test]
fn serialize_after_rotate_to_zero() {
    // rotation 90→0 후 serialize → re-parse → rotation == 0
    // D4: rotation == 0이어도 /Rotate 0을 명시적으로 설정하는 경로 검증
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (mut doc, sources) = load_document_tracked(&bytes).unwrap();

    // 먼저 90도로 설정
    let cmd_90 = RotatePageCommand::new(0, 90);
    cmd_90.execute(&mut doc).unwrap();
    assert_eq!(doc.pages[0].rotation, 90);

    let out_90 = serialize_document(&doc, &sources).unwrap();
    let (doc_90, sources_90) = load_document_tracked(&out_90).unwrap();
    assert_eq!(doc_90.pages[0].rotation, 90);

    // 다시 90도 추가 회전 → 총 180도
    // 또는 명시적으로 rotation을 0으로 설정해 re-serialize
    // 계획서 D4: "수동으로 page.rotation=0인 Page 구성 후 serialize"
    let mut doc_zero = doc_90;
    doc_zero.pages[0].rotation = 0;

    let out_zero = serialize_document(&doc_zero, &sources_90).unwrap();
    let (doc_re, _) = load_document_tracked(&out_zero).unwrap();
    assert_eq!(
        doc_re.pages[0].rotation, 0,
        "re-parse 후 rotation == 0이어야 한다 (rotation=0도 명시적으로 덮어씀)"
    );
}

// --- CP-D: delete/extract/split 테스트 ---

#[test]
fn serialize_after_delete() {
    // pdfjs-basicapi.pdf: 3페이지 → 1페이지 삭제 → 2페이지
    let bytes = fixture("pdfjs-basicapi.pdf");
    let (mut doc, mut sources) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 3);

    // 0-based index 1 삭제
    let cmd = DeletePagesCommand::new(vec![1]);
    cmd.execute(&mut doc).unwrap();
    assert_eq!(doc.pages.len(), 2);

    // sources 동기화: index 1 제거
    sources.remove(1);
    assert_eq!(sources.len(), 2);

    let out = serialize_document(&doc, &sources).unwrap();
    let (doc2, _) = load_document_tracked(&out).unwrap();
    assert_eq!(doc2.pages.len(), 2, "삭제 후 re-parse 페이지 수 == 2");
}

#[test]
fn serialize_after_extract() {
    // fw4-2024.pdf: 5페이지 → 1-3 extract → 3페이지
    let bytes = fixture("fw4-2024.pdf");
    let (doc, sources) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 5, "fw4-2024.pdf는 5페이지여야 한다");

    let cmd = ExtractPagesCommand::new(1, 3).unwrap();
    let extracted_doc = cmd.execute(&doc).unwrap();
    assert_eq!(extracted_doc.pages.len(), 3);

    // sources 동기화: pages[0..=2]에 해당하는 sources[0..=2]
    let extracted_sources: Vec<PageSource> = sources.into_iter().take(3).collect();
    assert_eq!(extracted_sources.len(), 3);

    let out = serialize_document(&extracted_doc, &extracted_sources).unwrap();
    let (doc2, _) = load_document_tracked(&out).unwrap();
    assert_eq!(doc2.pages.len(), 3, "extract 후 re-parse 페이지 수 == 3");
}

#[test]
fn serialize_after_split() {
    // fw4-2024.pdf: 5페이지 → "1-2,3-5" split → [2p, 3p]
    let bytes = fixture("fw4-2024.pdf");
    let (doc, sources) = load_document_tracked(&bytes).unwrap();
    assert_eq!(doc.pages.len(), 5);

    let cmd = SplitCommand::new("1-2,3-5").unwrap();
    let split_docs = cmd.execute(&doc).unwrap();
    assert_eq!(split_docs.len(), 2);
    assert_eq!(split_docs[0].pages.len(), 2);
    assert_eq!(split_docs[1].pages.len(), 3);

    // 각 split doc의 sources 동기화
    // split_docs[0]: 원본 pages[0..=1] → sources[0..=1]
    // split_docs[1]: 원본 pages[2..=4] → sources[2..=4]
    let sources0: Vec<PageSource> = sources
        .iter()
        .take(2)
        .map(|s| PageSource {
            bytes: Arc::clone(&s.bytes),
            page_index: s.page_index,
        })
        .collect();
    let sources1: Vec<PageSource> = sources
        .iter()
        .skip(2)
        .map(|s| PageSource {
            bytes: Arc::clone(&s.bytes),
            page_index: s.page_index,
        })
        .collect();

    let out0 = serialize_document(&split_docs[0], &sources0).unwrap();
    let (doc_a, _) = load_document_tracked(&out0).unwrap();
    assert_eq!(doc_a.pages.len(), 2, "split[0] re-parse 페이지 수 == 2");

    let out1 = serialize_document(&split_docs[1], &sources1).unwrap();
    let (doc_b, _) = load_document_tracked(&out1).unwrap();
    assert_eq!(doc_b.pages.len(), 3, "split[1] re-parse 페이지 수 == 3");
}

// --- CP-E: merge 다중 소스 테스트 ---

#[test]
fn serialize_after_merge() {
    // irs-f1040.pdf (2페이지) + pdfjs-basicapi.pdf (3페이지) = 5페이지
    let bytes_a = fixture("irs-f1040.pdf");
    let bytes_b = fixture("pdfjs-basicapi.pdf");

    let (mut doc_a, sources_a) = load_document_tracked(&bytes_a).unwrap();
    let (doc_b, sources_b) = load_document_tracked(&bytes_b).unwrap();

    assert_eq!(doc_a.pages.len(), 2, "irs-f1040.pdf는 2페이지여야 한다");
    assert_eq!(
        doc_b.pages.len(),
        3,
        "pdfjs-basicapi.pdf는 3페이지여야 한다"
    );

    let cmd = MergeCommand::new(vec![doc_b.clone()]);
    cmd.execute(&mut doc_a).unwrap();
    assert_eq!(doc_a.pages.len(), 5);

    // sources 동기화: a_sources + b_sources 순서로 연결
    let merged_sources: Vec<PageSource> = sources_a.into_iter().chain(sources_b).collect();
    assert_eq!(merged_sources.len(), 5);

    let out = serialize_document(&doc_a, &merged_sources).unwrap();
    let (doc_merged, _) = load_document_tracked(&out).unwrap();
    assert_eq!(
        doc_merged.pages.len(),
        5,
        "merge 후 re-parse 페이지 수 == 5"
    );
}
