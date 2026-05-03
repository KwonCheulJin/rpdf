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

// ─── IT-6: xref 스트림 PDF — fw4-2024.pdf 전체 파이프라인 + Catalog 검증 ──────

#[test]
fn it6_xref_stream_fw4_full_pipeline() {
    // fw4-2024.pdf (PDF 1.7): xref stream 전용 PDF.
    // parse_trailer는 XrefStreamUnsupported, parse_xref는 전체 chain 파싱 성공.
    // Root(3540 0 R)는 InUse 엔트리, /Type /Catalog 확인.
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");

    let eof_offset = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof_offset).unwrap();

    // 역방향 탐색은 XrefStreamUnsupported 반환 (예상된 동작)
    assert!(
        matches!(
            parse_trailer(data, eof_offset).unwrap_err(),
            ParseError::XrefStreamUnsupported { .. }
        ),
        "parse_trailer가 XrefStreamUnsupported를 반환해야 함"
    );

    // parse_xref: xref 스트림 chain 전체 파싱 성공
    let parsed = parse_xref(data, xref_offset).unwrap();

    assert!(!parsed.table.is_empty(), "xref 테이블이 비어있음");
    // fw4-2024.pdf: 마지막 xref stream(208493) → /Prev 116 → 첫 xref stream
    assert!(
        parsed.sections.len() >= 2,
        "/Prev chain이 있으므로 섹션 수 >= 2: got {}",
        parsed.sections.len()
    );

    // Root(3540 0 R)가 InUse 엔트리이어야 함
    let root_id = parsed.trailer.root;
    let root_entry = parsed
        .table
        .get(root_id.number)
        .expect("root가 xref 테이블에 없음");
    let root_offset = match root_entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        XrefEntry::Compressed { .. } => {
            panic!("fw4-2024.pdf root는 InUse 엔트리여야 함 (Compressed가 아님)")
        }
        other => panic!("예상치 못한 root 엔트리 타입: {other:?}"),
    };

    // Catalog indirect object 파싱 및 /Type /Catalog 검증
    let (root_indirect, _) = parse_indirect_object(data, root_offset).unwrap();
    assert_eq!(root_indirect.id, root_id);
    let dict = root_indirect
        .object
        .as_dict()
        .expect("Catalog은 딕셔너리여야 함");
    let type_val = dict.get(b"Type").expect("/Type 키 누락");
    assert!(
        matches!(type_val, PdfObject::Name(n) if n == b"Catalog"),
        "/Type이 /Catalog가 아님: {type_val:?}"
    );
}

// ─── IT-7: xref 스트림 PDF — irs-f1040.pdf 전체 파이프라인 ────────────────────

#[test]
fn it7_xref_stream_irs_f1040_full_pipeline() {
    // irs-f1040.pdf (PDF 1.7): fw4-2024.pdf와 동일 생성기, 독립적 파일.
    // Root(2399 0 R)는 InUse 엔트리, /Type /Catalog 확인.
    let data = include_bytes!("../../../../examples/irs-f1040.pdf");

    let eof_offset = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof_offset).unwrap();

    // 역방향 탐색은 XrefStreamUnsupported 반환 (예상된 동작)
    assert!(
        matches!(
            parse_trailer(data, eof_offset).unwrap_err(),
            ParseError::XrefStreamUnsupported { .. }
        ),
        "parse_trailer가 XrefStreamUnsupported를 반환해야 함"
    );

    // parse_xref: xref 스트림 chain 전체 파싱 성공
    let parsed = parse_xref(data, xref_offset).unwrap();

    assert!(!parsed.table.is_empty(), "xref 테이블이 비어있음");
    assert!(
        parsed.sections.len() >= 2,
        "/Prev chain이 있으므로 섹션 수 >= 2: got {}",
        parsed.sections.len()
    );

    // Root(2399 0 R)가 InUse 엔트리이어야 함
    let root_id = parsed.trailer.root;
    let root_entry = parsed
        .table
        .get(root_id.number)
        .expect("root가 xref 테이블에 없음");
    let root_offset = match root_entry {
        XrefEntry::InUse { offset, .. } => *offset as usize,
        XrefEntry::Compressed { .. } => {
            panic!("irs-f1040.pdf root는 InUse 엔트리여야 함 (Compressed가 아님)")
        }
        other => panic!("예상치 못한 root 엔트리 타입: {other:?}"),
    };

    // Catalog indirect object 파싱 및 /Type /Catalog 검증
    let (root_indirect, _) = parse_indirect_object(data, root_offset).unwrap();
    assert_eq!(root_indirect.id, root_id);
    let dict = root_indirect
        .object
        .as_dict()
        .expect("Catalog은 딕셔너리여야 함");
    let type_val = dict.get(b"Type").expect("/Type 키 누락");
    assert!(
        matches!(type_val, PdfObject::Name(n) if n == b"Catalog"),
        "/Type이 /Catalog가 아님: {type_val:?}"
    );
}

// ─── IT-8: hybrid chain — 전통 xref + xref 스트림 혼합 (합성) ──────────────────

/// 합성 hybrid PDF를 생성한다.
///
/// 구조:
/// - 베이스: obj 1(Catalog) + obj 2(Pages) + 전통 xref 섹션
/// - 증분 업데이트: obj 3(xref 스트림) → /Prev로 베이스 xref 가리킴
///
/// parse_xref_chain이 스트림→전통 순서로 두 형식을 모두 처리하는지 검증한다.
fn make_hybrid_pdf_for_it8() -> (Vec<u8>, u64) {
    let mut buf: Vec<u8> = Vec::new();

    buf.extend_from_slice(b"%PDF-1.5\n");

    // obj 1: Catalog
    let obj1_offset = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    // obj 2: Pages
    let obj2_offset = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [] /Count 0>>\nendobj\n");

    // 전통 xref 섹션 (obj 0–2)
    let xref_trad_offset = buf.len() as u64;
    buf.extend_from_slice(b"xref\n0 3\n");
    buf.extend_from_slice(b"0000000000 65535 f\r\n");
    buf.extend_from_slice(format!("{:010} 00000 n\r\n", obj1_offset).as_bytes());
    buf.extend_from_slice(format!("{:010} 00000 n\r\n", obj2_offset).as_bytes());
    buf.extend_from_slice(b"trailer\n<</Size 3 /Root 1 0 R>>\n");
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_trad_offset).as_bytes());

    // xref 스트림 (증분 업데이트): obj 3 자체를 참조하는 InUse 엔트리
    // obj3_offset = 현재 buf.len() — 이 값이 스트림 바디에 들어간다.
    let obj3_offset = buf.len() as u64;

    // W=[1,4,1], Index=[3,1], no Filter — row_size=6, 1개 엔트리
    let mut stream_body = vec![0u8; 6];
    stream_body[0] = 1; // type = InUse
    stream_body[1..5].copy_from_slice(&(obj3_offset as u32).to_be_bytes());
    stream_body[5] = 0; // gen = 0

    let dict = format!(
        "3 0 obj\n<</Type /XRef /Size 4 /Root 1 0 R /Prev {} /W [1 4 1] /Index [3 1] /Length {}>>\nstream\n",
        xref_trad_offset,
        stream_body.len()
    );
    buf.extend_from_slice(dict.as_bytes());
    buf.extend_from_slice(&stream_body);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", obj3_offset).as_bytes());

    (buf, obj3_offset)
}

#[test]
fn it8_hybrid_chain_traditional_then_stream() {
    // 합성 hybrid PDF: 전통 xref 섹션 위에 xref 스트림 증분 업데이트를 얹은 구조.
    // parse_xref_chain이 스트림→전통 순서로 두 형식 모두를 처리하는지 검증한다.
    let (data, xref_stream_offset) = make_hybrid_pdf_for_it8();

    // find_eof + parse_startxref로 xref 스트림 오프셋 재확인
    let eof_offset = find_eof(&data).unwrap();
    let detected_offset = parse_startxref(&data, eof_offset).unwrap();
    assert_eq!(
        detected_offset, xref_stream_offset,
        "parse_startxref가 마지막 startxref를 반환해야 함"
    );

    let parsed = parse_xref(&data, xref_stream_offset).unwrap();

    // chain이 xref 스트림 + 전통 xref 두 섹션을 모두 순회했음
    assert_eq!(
        parsed.sections.len(),
        2,
        "hybrid chain은 정확히 2개 섹션: got {}",
        parsed.sections.len()
    );

    // 두 섹션 오프셋 확인
    assert_eq!(
        parsed.sections[0].offset, xref_stream_offset,
        "첫 섹션 = xref 스트림"
    );

    // obj 1, 2(전통 xref), obj 3(스트림)가 모두 테이블에 있어야 함
    assert!(
        matches!(parsed.table.get(1), Some(XrefEntry::InUse { .. })),
        "obj 1이 InUse 엔트리여야 함"
    );
    assert!(
        matches!(parsed.table.get(2), Some(XrefEntry::InUse { .. })),
        "obj 2가 InUse 엔트리여야 함"
    );
    assert!(
        matches!(parsed.table.get(3), Some(XrefEntry::InUse { .. })),
        "obj 3(xref 스트림 자체)이 InUse 엔트리여야 함"
    );

    // trailer.root = 1 0 R (xref 스트림의 trailer가 권위 있는 소스)
    assert_eq!(
        parsed.trailer.root,
        ObjectId {
            number: 1,
            generation: 0
        }
    );
}
