use rpdf_parser::load_document;
use serde::Serialize;

// ────────────────────────────────────────────────────────────
// 스냅샷 타입 — content stream 전체 제외, 메타데이터+페이지 메타만
// ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ParseResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<DocSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct DocSnapshot {
    page_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<MetaSnapshot>,
    pages: Vec<PageSnapshot>,
}

#[derive(Serialize)]
struct MetaSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    producer: Option<String>,
}

#[derive(Serialize)]
struct PageSnapshot {
    index: usize,
    media_box: Option<[f64; 4]>,
    rotation: i32,
    op_count: usize,
}

fn bytes_to_string(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn parse_to_snapshot(bytes: &[u8]) -> ParseResult {
    match load_document(bytes) {
        Ok(doc) => {
            let metadata = doc.metadata().map(|m| MetaSnapshot {
                title: m.title.as_deref().map(bytes_to_string),
                author: m.author.as_deref().map(bytes_to_string),
                producer: m.producer.as_deref().map(bytes_to_string),
            });
            let pages = doc
                .pages()
                .iter()
                .map(|p| PageSnapshot {
                    index: p.index,
                    media_box: p.media_box,
                    rotation: p.rotation,
                    op_count: p.content().len(),
                })
                .collect();
            ParseResult {
                doc: Some(DocSnapshot {
                    page_count: doc.page_count(),
                    metadata,
                    pages,
                }),
                error: None,
            }
        }
        Err(e) => ParseResult {
            doc: None,
            error: Some(format!("{e:?}")),
        },
    }
}

fn load_sample(filename: &str) -> Vec<u8> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest}/../../samples/{filename}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("샘플 파일 없음 ({path}): {e}"))
}

fn load_sample_large(filename: &str) -> Vec<u8> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest}/../../samples/large/{filename}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("대용량 샘플 없음 ({path}): {e}"))
}

// ────────────────────────────────────────────────────────────
// 스냅샷 테스트 매크로
// ────────────────────────────────────────────────────────────

macro_rules! snapshot_sample {
    ($test_name:ident, $filename:expr) => {
        #[test]
        fn $test_name() {
            let bytes = load_sample($filename);
            insta::assert_yaml_snapshot!(parse_to_snapshot(&bytes));
        }
    };
}

macro_rules! snapshot_large_sample {
    ($test_name:ident, $filename:expr) => {
        #[test]
        #[cfg_attr(not(feature = "samples-large"), ignore)]
        fn $test_name() {
            let bytes = load_sample_large($filename);
            insta::assert_yaml_snapshot!(parse_to_snapshot(&bytes));
        }
    };
}

// ────────────────────────────────────────────────────────────
// T1~T8: 전통 xref
// ────────────────────────────────────────────────────────────
snapshot_sample!(t1_trad_xref_basicapi, "trad-xref-basicapi.pdf");
snapshot_sample!(t2_trad_xref_tracemonkey, "trad-xref-tracemonkey.pdf");
snapshot_sample!(t3_trad_xref_canvas, "trad-xref-canvas.pdf");
snapshot_sample!(t4_trad_xref_cmyk_jpeg, "trad-xref-cmyk-jpeg.pdf");
snapshot_sample!(t5_trad_xref_attachment, "trad-xref-attachment.pdf");
snapshot_sample!(t6_trad_xref_find_all, "trad-xref-find-all.pdf");
snapshot_sample!(t7_trad_xref_issue1155r, "trad-xref-issue1155r.pdf");
snapshot_sample!(t8_trad_xref_issue1293r, "trad-xref-issue1293r.pdf");

// ────────────────────────────────────────────────────────────
// S1~S8: xref stream
// ────────────────────────────────────────────────────────────
snapshot_sample!(s1_xref_stream_doc_13_pages, "xref-stream-doc-13-pages.pdf");
snapshot_sample!(s2_xref_stream_extract_link, "xref-stream-extract-link.pdf");
snapshot_sample!(
    s3_xref_stream_form_two_pages,
    "xref-stream-form-two-pages.pdf"
);
snapshot_sample!(s4_xref_stream_zapfdingbats, "xref-stream-zapfdingbats.pdf");
snapshot_sample!(s5_xref_stream_irs_f1040nr, "xref-stream-irs-f1040nr.pdf");
snapshot_sample!(s6_xref_stream_irs_f1040es, "xref-stream-irs-f1040es.pdf");
snapshot_sample!(s7_xref_stream_irs_f1120, "xref-stream-irs-f1120.pdf");
snapshot_sample!(s8_xref_stream_irs_f941, "xref-stream-irs-f941.pdf");

// ────────────────────────────────────────────────────────────
// M1~M4: 다국어/유니코드
// ────────────────────────────────────────────────────────────
snapshot_sample!(m1_multilang_french, "multilang-french-diacritics.pdf");
snapshot_sample!(m2_multilang_german, "multilang-german-umlaut.pdf");
snapshot_sample!(m3_multilang_arabic, "multilang-arabic-cidfont.pdf");
snapshot_sample!(m4_multilang_korean, "multilang-korean-metadata.pdf");

// ────────────────────────────────────────────────────────────
// B1~B2: 손상 (ParseError 검증)
// ────────────────────────────────────────────────────────────
snapshot_sample!(b1_broken_missing_trailer, "broken-missing-trailer.pdf");
snapshot_sample!(b2_broken_bad_xref_offset, "broken-bad-xref-offset.pdf");

// ────────────────────────────────────────────────────────────
// N1~N2: 비표준
// ────────────────────────────────────────────────────────────
snapshot_sample!(
    n1_nonstandard_helloworld_bad,
    "nonstandard-helloworld-bad.pdf"
);
snapshot_sample!(
    n2_nonstandard_bad_page_labels,
    "nonstandard-bad-page-labels.pdf"
);

// ────────────────────────────────────────────────────────────
// X1~X4: 특수 케이스
// ────────────────────────────────────────────────────────────
snapshot_sample!(x1_special_acroform_calc, "special-acroform-calc.pdf");
snapshot_sample!(x2_special_irs_f4868, "special-irs-f4868.pdf");
snapshot_sample!(x3_special_irs_f1099msc, "special-irs-f1099msc.pdf");
snapshot_sample!(
    x4_special_unicode_en_cidfont,
    "special-unicode-en-cidfont.pdf"
);

// ────────────────────────────────────────────────────────────
// L1~L2: 대용량 (samples-large feature 없으면 ignore)
// ────────────────────────────────────────────────────────────
snapshot_large_sample!(l1_large_pdfjs_9279, "large-pdfjs-9279.pdf");
snapshot_large_sample!(l2_large_pdfjs_issue12841, "large-pdfjs-issue12841.pdf");
