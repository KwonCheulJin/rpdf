use rpdf_core::types::{ObjectId, PdfObject, XrefEntry};
use rpdf_parser::{
    ParseError, find_eof, parse_header, parse_indirect_object, parse_startxref, parse_trailer,
    parse_xref,
};

// ─── IT-1: 표준 PDF 1.4 — 4개 함수 전체 연동 + Catalog 파싱 ────────────────

#[test]
fn it1_standard_pdf_all_four_functions() {
    let data = include_bytes!("../../../../examples/pdfjs-tracemonkey.pdf");

    let eof_offset = find_eof(data).unwrap();
    let header = parse_header(data).unwrap();
    let xref_offset = parse_startxref(data, eof_offset).unwrap();
    let parsed = parse_trailer(data, eof_offset).unwrap();

    assert_eq!(header.version.major(), 1);
    assert_eq!(header.version.minor(), 4);
    assert_eq!(parsed.trailer.size, 997);
    assert_eq!(
        parsed.trailer.root,
        ObjectId {
            number: 995,
            generation: 0
        }
    );
    assert_eq!(
        parsed.trailer.info,
        Some(ObjectId {
            number: 996,
            generation: 0
        })
    );
    assert_eq!(parsed.trailer.prev, None);
    assert_eq!(parsed.xref_offset, xref_offset);
    assert_eq!(parsed.xref_offset, 996213);

    let parsed_xref = parse_xref(data, xref_offset).unwrap();
    assert!(!parsed_xref.table.is_empty(), "xref 항목이 비어있음");
    let root_num = parsed_xref.trailer.root.number;
    assert!(
        parsed_xref.table.get(root_num).is_some(),
        "/Root 객체 {root_num}이 xref에 없음"
    );

    // Catalog indirect object 파싱 및 검증
    let root_id = parsed_xref.trailer.root;
    let entry = parsed_xref.table.get(root_id.number).expect("root in xref");
    let offset = match entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        _ => panic!("root must be in-use"),
    };
    let (indirect, _) = parse_indirect_object(data, offset).unwrap();
    assert_eq!(indirect.id, root_id);
    let dict = indirect.object.as_dict().expect("catalog must be dict");
    let type_value = dict.get(b"Type").expect("/Type key");
    assert!(
        matches!(type_value, PdfObject::Name(name) if name == b"Catalog"),
        "/Type is not /Catalog: {type_value:?}"
    );
}

// ─── IT-2: 헤더 오프셋 != 0 ──────────────────────────────────────────────────

#[test]
fn it2_header_at_nonzero_offset() {
    // %PDF-1.4 앞에 512 바이트 정크 데이터
    let mut data = vec![b'X'; 512];
    data.extend_from_slice(b"%PDF-1.4\n");
    data.extend_from_slice(b"trailer\n<<\n/Size 1\n/Root 1 0 R\n>>\nstartxref\n0\n%%EOF\n");

    let header = parse_header(&data).unwrap();
    assert_eq!(header.byte_offset, 512);
    assert_eq!(header.version.major(), 1);
    assert_eq!(header.version.minor(), 4);

    // startxref = 0 이면 parse_xref가 오프셋 0에서 xref 키워드를 찾지 못한다.
    assert!(matches!(
        parse_xref(&data, 0).unwrap_err(),
        ParseError::InvalidXrefAtOffset { offset: 0, .. }
    ));
}

// ─── IT-3: 점진적 업데이트 — 마지막 trailer 사용 + Catalog 파싱 ──────────────

#[test]
fn it3_incremental_update_uses_last_trailer() {
    // pdfjs-annotation-border.pdf 는 두 개의 trailer 섹션을 포함한다.
    // 마지막 trailer 에는 /Prev 가 있고, startxref 값이 89371 이다.
    let data = include_bytes!("../../../../examples/pdfjs-annotation-border.pdf");

    let eof_offset = find_eof(data).unwrap();
    let parsed = parse_trailer(data, eof_offset).unwrap();

    // 마지막 trailer: /Size 35, /Root 1 0 R, /Info 8 0 R, /Prev 84248
    assert_eq!(parsed.trailer.size, 35);
    assert_eq!(
        parsed.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
    assert_eq!(
        parsed.trailer.info,
        Some(ObjectId {
            number: 8,
            generation: 0
        })
    );
    assert_eq!(parsed.trailer.prev, Some(84248));
    assert_eq!(parsed.xref_offset, 89371);

    // /Prev chain 순회: 두 xref 섹션을 모두 읽어 병합한다.
    let parsed_xref = parse_xref(data, parsed.xref_offset).unwrap();
    assert!(!parsed_xref.table.is_empty(), "xref 항목이 비어있음");
    assert!(
        parsed_xref.sections.len() >= 2,
        "/Prev chain이 있으므로 섹션이 2개 이상이어야 함: got {}",
        parsed_xref.sections.len()
    );
    let root_num = parsed_xref.trailer.root.number;
    assert!(
        parsed_xref.table.get(root_num).is_some(),
        "/Root 객체 {root_num}이 xref에 없음"
    );

    // Catalog indirect object 파싱 및 검증
    let root_id = parsed_xref.trailer.root;
    let entry = parsed_xref.table.get(root_id.number).expect("root in xref");
    let offset = match entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        _ => panic!("root must be in-use"),
    };
    let (indirect, _) = parse_indirect_object(data, offset).unwrap();
    assert_eq!(indirect.id, root_id);
    let dict = indirect.object.as_dict().expect("catalog must be dict");
    let type_value = dict.get(b"Type").expect("/Type key");
    assert!(
        matches!(type_value, PdfObject::Name(name) if name == b"Catalog"),
        "/Type is not /Catalog: {type_value:?}"
    );
}

// ─── IT-4: 파일 잘림 — MissingEof 반환 ──────────────────────────────────────

#[test]
fn it4_truncated_file_returns_missing_eof() {
    // 완전한 PDF 끝에서 %%EOF 를 잘라낸다
    let base = b"trailer\n<<\n/Size 5\n/Root 1 0 R\n>>\nstartxref\n100\n";
    // %%EOF 없이 끝나는 데이터
    assert!(matches!(
        find_eof(base).unwrap_err(),
        ParseError::MissingEof
    ));
}

// ─── IT-5: /Info 객체 파싱 + 메타데이터 문자열 검증 ──────────────────────────

#[test]
fn it5_info_object_id_extracted() {
    // pdfjs-tracemonkey.pdf: /Info 996 0 R
    // IT-1 과 동일 파일이지만, /Info 추출에만 초점을 맞춘 명시적 검증.
    //
    // 주의: ObjectId 추출만 확인. 실제 Info 딕셔너리의 제목/작성자 문자열 디코딩은
    // Task #7 이후에서 다룬다 (UTF-16BE BOM 처리 필요).
    let data = include_bytes!("../../../../examples/pdfjs-tracemonkey.pdf");

    let eof_offset = find_eof(data).unwrap();
    let parsed = parse_trailer(data, eof_offset).unwrap();

    assert_eq!(
        parsed.trailer.info,
        Some(ObjectId {
            number: 996,
            generation: 0
        })
    );

    let xref_offset = parse_startxref(data, eof_offset).unwrap();
    let parsed_xref = parse_xref(data, xref_offset).unwrap();
    let info_id = parsed_xref.trailer.info.unwrap();
    assert!(
        parsed_xref.table.get(info_id.number).is_some(),
        "/Info 객체 {}이 xref에 없음",
        info_id.number
    );

    // Catalog indirect object 파싱
    let root_id = parsed_xref.trailer.root;
    let root_entry = parsed_xref.table.get(root_id.number).expect("root in xref");
    let root_offset = match root_entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        _ => panic!("root must be in-use"),
    };
    let (root_indirect, _) = parse_indirect_object(data, root_offset).unwrap();
    assert_eq!(root_indirect.id, root_id);
    let catalog = root_indirect
        .object
        .as_dict()
        .expect("catalog must be dict");
    let type_value = catalog.get(b"Type").expect("/Type key");
    assert!(
        matches!(type_value, PdfObject::Name(name) if name == b"Catalog"),
        "/Type is not /Catalog: {type_value:?}"
    );

    // /Info indirect object 파싱 + 메타데이터 문자열 raw bytes 검증
    let info_entry = parsed_xref.table.get(info_id.number).expect("info in xref");
    let info_offset = match info_entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        _ => panic!("info must be in-use"),
    };
    let (info_indirect, _) = parse_indirect_object(data, info_offset).unwrap();
    assert_eq!(info_indirect.id, info_id);
    let info_dict = info_indirect.object.as_dict().expect("info must be dict");

    // /Producer 는 이 파일에 확실히 존재 (pdfTeX 생성). /Title·/Author 는 없어 가드 불필요.
    // (인코딩 해석은 Task #7 영역 — raw bytes 형식 확인만 수행)
    let producer = info_dict
        .get(b"Producer")
        .or_else(|| info_dict.get(b"Creator"))
        .expect("/Producer 또는 /Creator가 Info dict에 없음");
    assert!(
        matches!(
            producer,
            PdfObject::LiteralString(_) | PdfObject::HexString(_)
        ),
        "/Producer 가 문자열이 아님: {producer:?}"
    );
    let bytes = producer.as_string_bytes().unwrap();
    assert!(!bytes.is_empty(), "/Producer raw bytes가 비어있음");
}

// ─── IT-6: xref 스트림 PDF — parse_trailer·parse_xref 양쪽 모두 XrefStreamUnsupported ──

#[test]
fn it6_xref_stream_returns_unsupported_error() {
    // fw4-2024.pdf (PDF 1.7) 은 xref stream 방식을 사용한다.
    // parse_trailer(역방향)와 parse_xref(순방향) 두 경로 모두 동일 에러를 반환해야 한다.
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");

    let eof_offset = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof_offset).unwrap();

    assert!(
        matches!(
            parse_trailer(data, eof_offset).unwrap_err(),
            ParseError::XrefStreamUnsupported { xref_offset: _ }
        ),
        "parse_trailer가 XrefStreamUnsupported를 반환해야 함"
    );
    assert!(
        matches!(
            parse_xref(data, xref_offset).unwrap_err(),
            ParseError::XrefStreamUnsupported { xref_offset: _ }
        ),
        "parse_xref가 XrefStreamUnsupported를 반환해야 함"
    );
}
