use rpdf_core::types::{ContentStreamOperator, ObjectId, PdfObject, XrefEntry};
use rpdf_parser::{
    ParseError, find_eof, parse_content_stream, parse_header, parse_indirect_object,
    parse_object_stream, parse_startxref, parse_trailer, parse_xref,
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

// ─── IT-9: 합성 ObjStm PDF — Compressed 엔트리 해소 + PdfObject 검증 ──────────

/// FlateDecode 압축 ObjStm이 포함된 합성 PDF를 생성한다.
///
/// 구조:
/// - obj 1 (Catalog, InUse)
/// - obj 2 (Pages, InUse)
/// - obj 10 (ObjStm, InUse): Page1(obj3)·Page2(obj4) 두 객체 포함, FlateDecode
/// - xref stream (obj 5): /Index [0 5 10 1]
///   - obj 0: Free
///   - obj 1,2: InUse
///   - obj 3,4: Compressed → ObjStm 10, index 0/1
///   - obj 10: InUse (ObjStm 자체)
fn make_pdf_with_objstm() -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.5\n");

    // obj 1: Catalog
    let obj1_offset = buf.len() as u64;
    buf.extend_from_slice(b"1 0 obj\n<</Type /Catalog /Pages 2 0 R>>\nendobj\n");

    // obj 2: Pages
    let obj2_offset = buf.len() as u64;
    buf.extend_from_slice(b"2 0 obj\n<</Type /Pages /Kids [3 0 R 4 0 R] /Count 2>>\nendobj\n");

    // ObjStm 본문 (비압축): "3 0 4 B2_len\n<Page1><Page2>"
    let page1_bytes = b"<</Type /Page /Parent 2 0 R>>";
    let page2_bytes = b"<</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]>>";
    let hdr_offset1: usize = 0;
    let hdr_offset2: usize = page1_bytes.len();
    let header_str = format!("3 {} 4 {}\n", hdr_offset1, hdr_offset2);
    let first = header_str.len();
    let mut plain_body: Vec<u8> = header_str.into_bytes();
    plain_body.extend_from_slice(page1_bytes);
    plain_body.extend_from_slice(page2_bytes);

    // FlateDecode 압축
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&plain_body).unwrap();
    let compressed = enc.finish().unwrap();

    // obj 10: ObjStm
    let obj10_offset = buf.len() as u64;
    let objstm_header = format!(
        "10 0 obj\n<</Type /ObjStm /N 2 /First {} /Filter /FlateDecode /Length {}>>\nstream\n",
        first,
        compressed.len()
    );
    buf.extend_from_slice(objstm_header.as_bytes());
    buf.extend_from_slice(&compressed);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    // xref stream (obj 5): W=[1,4,2], /Index [0 5 10 1]
    // row_size = 1+4+2 = 7 bytes
    // entries: obj0(Free), obj1(InUse), obj2(InUse), obj3(Compressed), obj4(Compressed)
    //          obj10(InUse)
    let xref_offset = buf.len() as u64;

    let mut stream_body: Vec<u8> = Vec::new();
    let mut push_row = |t: u8, b1: u32, b2: u16| {
        stream_body.push(t);
        stream_body.extend_from_slice(&b1.to_be_bytes());
        stream_body.extend_from_slice(&b2.to_be_bytes());
    };
    push_row(0, 0, 65535); // obj 0: Free
    push_row(1, obj1_offset as u32, 0); // obj 1: InUse
    push_row(1, obj2_offset as u32, 0); // obj 2: InUse
    push_row(2, 10, 0); // obj 3: Compressed → ObjStm 10, index 0
    push_row(2, 10, 1); // obj 4: Compressed → ObjStm 10, index 1
    push_row(1, obj10_offset as u32, 0); // obj 10: InUse

    let xref_dict = format!(
        "5 0 obj\n<</Type /XRef /Size 11 /Root 1 0 R /W [1 4 2] /Index [0 5 10 1] /Length {}>>\nstream\n",
        stream_body.len()
    );
    buf.extend_from_slice(xref_dict.as_bytes());
    buf.extend_from_slice(&stream_body);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());

    buf
}

#[test]
fn it9_objstm_compressed_entry_resolved_via_parse_object_stream() {
    let data = make_pdf_with_objstm();

    // (a) parse_xref 성공
    let eof = find_eof(&data).unwrap();
    let xref_offset = parse_startxref(&data, eof).unwrap();
    let parsed = parse_xref(&data, xref_offset).unwrap();

    // (b) obj 3, obj 4가 Compressed 엔트리인지 확인
    let entry3 = parsed.table.get(3).expect("obj 3 in xref");
    let entry4 = parsed.table.get(4).expect("obj 4 in xref");

    let (obj_stm_num3, idx3) = match entry3 {
        XrefEntry::Compressed { obj_stm_num, index } => (*obj_stm_num, *index),
        other => panic!("obj 3은 Compressed여야 함: {other:?}"),
    };
    let (obj_stm_num4, idx4) = match entry4 {
        XrefEntry::Compressed { obj_stm_num, index } => (*obj_stm_num, *index),
        other => panic!("obj 4는 Compressed여야 함: {other:?}"),
    };

    assert_eq!(obj_stm_num3, 10, "obj 3의 ObjStm은 obj 10이어야 함");
    assert_eq!(obj_stm_num4, 10, "obj 4의 ObjStm은 obj 10이어야 함");
    assert_eq!(idx3, 0, "obj 3의 index는 0이어야 함");
    assert_eq!(idx4, 1, "obj 4의 index는 1이어야 함");

    // (c) ObjStm의 파일 오프셋 조회 (xref에서 obj 10 InUse 엔트리)
    let objstm_entry = parsed.table.get(10).expect("obj 10 (ObjStm) in xref");
    let objstm_file_offset = match objstm_entry {
        XrefEntry::InUse { offset, .. } => *offset,
        other => panic!("ObjStm obj 10은 InUse여야 함: {other:?}"),
    };

    // (d) parse_object_stream 호출
    let objstm = parse_object_stream(&data, objstm_file_offset).unwrap();

    // (e) get(3) → Some(Dictionary)
    let page1 = objstm.get(3).expect("ObjStm에 obj 3이 없음");
    assert!(
        matches!(page1, PdfObject::Dictionary(_)),
        "obj 3은 Dictionary여야 함: {page1:?}"
    );

    // (f) /Type /Page 검증
    let dict1 = page1.as_dict().unwrap();
    let type_val = dict1.get(b"Type").expect("/Type 키 누락");
    assert!(
        matches!(type_val, PdfObject::Name(n) if n == b"Page"),
        "obj 3의 /Type이 /Page가 아님: {type_val:?}"
    );

    // obj 4도 동일하게 검증
    let page2 = objstm.get(4).expect("ObjStm에 obj 4가 없음");
    let dict2 = page2.as_dict().unwrap();
    let type_val2 = dict2.get(b"Type").expect("/Type 키 누락");
    assert!(
        matches!(type_val2, PdfObject::Name(n) if n == b"Page"),
        "obj 4의 /Type이 /Page가 아님: {type_val2:?}"
    );
}

// ─── IT-10: 실제 PDF (fw4-2024.pdf) — Compressed 엔트리 해소 검증 ─────────────

#[test]
fn it10_real_pdf_compressed_entry_resolved_via_parse_object_stream() {
    // fw4-2024.pdf: obj 83 → ObjStm 7, index 0
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");

    let eof = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof).unwrap();
    let parsed = parse_xref(data, xref_offset).unwrap();

    // obj 83이 Compressed 엔트리인지 확인 (D-1 스캔에서 확인됨)
    let entry83 = parsed.table.get(83).expect("obj 83 in xref");
    let (obj_stm_num, idx) = match entry83 {
        XrefEntry::Compressed { obj_stm_num, index } => (*obj_stm_num, *index),
        other => panic!("obj 83은 Compressed여야 함: {other:?}"),
    };
    assert_eq!(obj_stm_num, 7, "obj 83의 ObjStm은 obj 7이어야 함");
    assert_eq!(idx, 0, "obj 83의 index는 0이어야 함");

    // ObjStm(obj 7)의 파일 오프셋 조회
    let objstm_entry = parsed.table.get(7).expect("ObjStm obj 7 in xref");
    let objstm_offset = match objstm_entry {
        XrefEntry::InUse { offset, .. } => *offset,
        other => panic!("ObjStm obj 7은 InUse여야 함: {other:?}"),
    };

    // parse_object_stream 호출
    let objstm = parse_object_stream(data, objstm_offset).unwrap();

    // obj 83 조회 성공
    let obj83 = objstm.get(83).expect("ObjStm에 obj 83이 없음");
    // 딕셔너리 형식이어야 함 (일반적인 PDF 구조 객체)
    assert!(
        matches!(
            obj83,
            PdfObject::Dictionary(_) | PdfObject::Array(_) | PdfObject::Integer(_)
        ),
        "obj 83이 예상 타입이 아님: {obj83:?}"
    );
}

// ─── IT-11: 합성 content stream 통합 테스트 ──────────────────────────────────

#[test]
fn it11_synthetic_content_stream() {
    // 텍스트 + 경로 + 색상 + q/Q + 인라인 이미지 포함 합성 스트림
    let mut stream: Vec<u8> = Vec::new();
    stream.extend_from_slice(
        b"BT\n\
        /F1 12 Tf\n\
        72 720 Td\n\
        (Hello PDF) Tj\n\
        [(multi)10(glyph)] TJ\n\
        ET\n\
        q\n\
        1 0 0 RG\n\
        0.5 g\n\
        100 200 m\n\
        300 400 l\n\
        h\n\
        f*\n\
        0 0 1 0 re\n\
        S\n\
        Q\n\
        /BMCTag BMC\n\
        EMC\n\
        BX\n\
        EX\n",
    );
    // 인라인 이미지 추가
    stream.extend_from_slice(b"BI /W 4 /H 2 /CS /G /BPC 8\nID ");
    stream.extend_from_slice(&[0xAAu8; 8]); // 4*2=8 raw bytes
    stream.extend_from_slice(b"\nEI\n");

    let ops = parse_content_stream(&stream).unwrap();

    // 총 연산자 수 확인:
    // BT Tf Td Tj TJ ET q RG g m l h f* re S Q BMC EMC BX EX InlineImage = 21개
    assert_eq!(ops.len(), 21, "연산자 수 불일치: {ops:?}");

    // 순서 + 분류 검증
    assert_eq!(ops[0].operator, ContentStreamOperator::BeginText);
    assert_eq!(ops[0].operands.len(), 0);

    assert_eq!(ops[1].operator, ContentStreamOperator::SetFont);
    assert_eq!(ops[1].operands.len(), 2); // /F1, 12

    assert_eq!(ops[2].operator, ContentStreamOperator::MoveText);
    assert_eq!(ops[2].operands.len(), 2); // 72, 720

    assert_eq!(ops[3].operator, ContentStreamOperator::ShowText);
    assert_eq!(ops[3].operands.len(), 1); // (Hello PDF)

    assert_eq!(ops[4].operator, ContentStreamOperator::ShowTextAdjusted);
    assert_eq!(ops[4].operands.len(), 1); // [...] array

    assert_eq!(ops[5].operator, ContentStreamOperator::EndText);

    assert_eq!(ops[6].operator, ContentStreamOperator::SaveState);
    assert_eq!(ops[7].operator, ContentStreamOperator::SetStrokeRGB);
    assert_eq!(ops[7].operands.len(), 3); // 1 0 0

    assert_eq!(ops[8].operator, ContentStreamOperator::SetFillGray);
    assert_eq!(ops[8].operands.len(), 1); // 0.5

    assert_eq!(ops[9].operator, ContentStreamOperator::MoveTo);
    assert_eq!(ops[10].operator, ContentStreamOperator::LineTo);
    assert_eq!(ops[11].operator, ContentStreamOperator::ClosePath);
    assert_eq!(ops[12].operator, ContentStreamOperator::FillEvenOdd);
    assert_eq!(ops[13].operator, ContentStreamOperator::Rect);
    assert_eq!(ops[13].operands.len(), 4); // 0 0 1 0
    assert_eq!(ops[14].operator, ContentStreamOperator::Stroke);
    assert_eq!(ops[15].operator, ContentStreamOperator::RestoreState);

    assert_eq!(ops[16].operator, ContentStreamOperator::BeginMarkedContent);
    assert_eq!(ops[16].operands.len(), 1); // /BMCTag

    assert_eq!(ops[17].operator, ContentStreamOperator::EndMarkedContent);
    assert_eq!(ops[18].operator, ContentStreamOperator::BeginCompatibility);
    assert_eq!(ops[19].operator, ContentStreamOperator::EndCompatibility);

    // 인라인 이미지
    assert_eq!(ops[20].operator, ContentStreamOperator::InlineImage);
    let img_data = ops[20].inline_data.as_ref().expect("inline_data 없음");
    assert_eq!(img_data.len(), 8);
    assert_eq!(img_data[0], 0xAA);
    // dict: /W 4 /H 2 /CS /G /BPC 8 → 4쌍 = 8개 피연산자
    assert_eq!(ops[20].operands.len(), 8);
}

// ─── IT-12: 실제 PDF (fw4-2024.pdf) content stream 통합 테스트 ───────────────

/// D-2 사전 확인 결과: fw4-2024.pdf 1페이지 content stream에서
/// parse_content_stream 적용 → 228개 연산자, 첫 연산자 BeginMarkedContentProp.
#[test]
fn it12_real_pdf_fw4_content_stream() {
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");

    // xref 파싱
    let eof_offset = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof_offset).unwrap();
    let parsed_xref = parse_xref(data, xref_offset).unwrap();

    // Catalog → Pages → page[0] → /Contents
    let root_num = parsed_xref.trailer.root.number;
    let catalog = resolve_dict(data, &parsed_xref, root_num);
    let pages_num = match catalog.get(b"Pages").unwrap() {
        PdfObject::Reference(id) => id.number,
        _ => panic!("/Pages Reference 아님"),
    };
    let pages_dict = resolve_dict(data, &parsed_xref, pages_num);
    let kids = match pages_dict.get(b"Kids").unwrap() {
        PdfObject::Array(arr) => arr.clone(),
        _ => panic!("/Kids Array 아님"),
    };
    let page_num = match &kids[0] {
        PdfObject::Reference(id) => id.number,
        _ => panic!("kids[0] Reference 아님"),
    };
    let page_dict = resolve_dict(data, &parsed_xref, page_num);
    let contents_num = match page_dict.get(b"Contents").unwrap() {
        PdfObject::Reference(id) => id.number,
        PdfObject::Array(arr) => match &arr[0] {
            PdfObject::Reference(id) => id.number,
            _ => panic!("contents array[0] Reference 아님"),
        },
        _ => panic!("/Contents 형식 미지원"),
    };

    // content stream 압축 해제
    let contents_obj = resolve_object(data, &parsed_xref, contents_num);
    let stream = match &contents_obj {
        PdfObject::Stream(s) => s,
        _ => panic!("contents가 Stream 아님"),
    };
    let stream_data = if matches!(stream.dict.get(b"Filter"), Some(PdfObject::Name(n)) if n == b"FlateDecode")
    {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut dec = ZlibDecoder::new(stream.data.as_slice());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).unwrap();
        out
    } else {
        stream.data.clone()
    };

    // parse_content_stream 적용 — D-2 사전 확인 값과 일치 여부 검증
    let ops = parse_content_stream(&stream_data).expect("parse_content_stream 실패");
    assert_eq!(ops.len(), 228, "D-2 확인값: 228개");
    assert_eq!(
        ops[0].operator,
        ContentStreamOperator::BeginMarkedContentProp,
        "첫 연산자는 BeginMarkedContentProp"
    );
    assert_eq!(ops[0].operands.len(), 2, "BDC 피연산자 2개 (Tag, dict)");
    assert_eq!(
        ops[11].operator,
        ContentStreamOperator::BeginText,
        "ops[11]은 BeginText"
    );
    assert_eq!(
        ops[17].operator,
        ContentStreamOperator::ShowText,
        "ops[17]은 ShowText"
    );
}

// IT-12 헬퍼: parse_xref 반환값에서 객체를 resolve
fn resolve_object(data: &[u8], parsed: &rpdf_parser::ParsedXref, obj_num: u32) -> PdfObject {
    let entry = parsed
        .table
        .get(obj_num)
        .unwrap_or_else(|| panic!("obj#{obj_num} 없음"));
    match entry {
        XrefEntry::InUse { offset, .. } => {
            let (indirect, _) = parse_indirect_object(data, *offset as usize).unwrap();
            indirect.object
        }
        XrefEntry::Free { .. } => panic!("obj#{obj_num} free"),
        XrefEntry::Compressed { obj_stm_num, .. } => {
            let stm_entry = parsed.table.get(*obj_stm_num).unwrap();
            let stm_off = match stm_entry {
                XrefEntry::InUse { offset, .. } => *offset,
                _ => panic!("ObjStm InUse 아님"),
            };
            let stm = parse_object_stream(data, stm_off).unwrap();
            stm.get(obj_num).cloned().unwrap()
        }
    }
}

fn resolve_dict(
    data: &[u8],
    parsed: &rpdf_parser::ParsedXref,
    obj_num: u32,
) -> rpdf_core::types::PdfDict {
    match resolve_object(data, parsed, obj_num) {
        PdfObject::Dictionary(d) => d,
        other => panic!("obj#{obj_num} Dictionary 아님: {other:?}"),
    }
}
