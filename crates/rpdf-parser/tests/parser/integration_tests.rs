use rpdf_core::types::ObjectId;
use rpdf_parser::{ParseError, find_eof, parse_header, parse_startxref, parse_trailer};

// ─── IT-1: 표준 PDF 1.4 — 4개 함수 전체 연동 ───────────────────────────────

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
}

// ─── IT-3: 점진적 업데이트 — 마지막 trailer 사용 ─────────────────────────────

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

// ─── IT-5: /Info 필드 ObjectId 추출 ─────────────────────────────────────────

#[test]
fn it5_info_object_id_extracted() {
    // pdfjs-tracemonkey.pdf: /Info 996 0 R
    // IT-1 과 동일 파일이지만, /Info 추출에만 초점을 맞춘 명시적 검증.
    //
    // 주의: ObjectId 추출만 확인. 실제 Info 딕셔너리의 제목/작성자 문자열 디코딩은
    // Task #5 이후에서 다룬다 (UTF-16BE BOM 처리 필요).
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
}

// ─── IT-6: xref 스트림 PDF — XrefStreamUnsupported 반환 ──────────────────────

#[test]
fn it6_xref_stream_returns_unsupported_error() {
    // fw4-2024.pdf (PDF 1.7) 은 xref stream 방식을 사용한다.
    // trailer 키워드가 없으므로 XrefStreamUnsupported 에러를 반환해야 한다.
    let data = include_bytes!("../../../../examples/fw4-2024.pdf");

    let eof_offset = find_eof(data).unwrap();
    assert!(matches!(
        parse_trailer(data, eof_offset).unwrap_err(),
        ParseError::XrefStreamUnsupported
    ));
}
