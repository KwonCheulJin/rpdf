//! D-2 사전 확인: 실제 PDF에서 page 1의 content stream 추출 후 parse_content_stream 적용.
use std::env;
use std::fs;

use rpdf_core::types::{PdfObject, XrefEntry, XrefTable};
use rpdf_parser::{
    find_eof, parse_content_stream, parse_header, parse_indirect_object, parse_object_stream,
    parse_startxref, parse_xref,
};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/fw4-2024.pdf".to_string());
    let data = fs::read(&path).expect("파일 읽기 실패");
    println!("파일: {path} ({} bytes)", data.len());

    // 1. 헤더
    let header = parse_header(&data).expect("헤더 파싱 실패");
    println!("PDF 버전: {:?}", header.version);

    // 2. xref + trailer
    let eof_offset = find_eof(&data).expect("EOF 없음");
    let xref_offset = parse_startxref(&data, eof_offset).expect("startxref 없음");
    let parsed_xref = parse_xref(&data, xref_offset).expect("xref 파싱 실패");
    println!("xref 엔트리 수: {}", parsed_xref.table.len());

    let root_num = parsed_xref.trailer.root.number;

    // 3. Catalog → Pages → page[0] → /Contents
    let catalog = resolve_dict(&data, &parsed_xref.table, root_num);

    let pages_num = get_ref(&catalog, b"Pages", "Catalog./Pages");
    let pages_dict = resolve_dict(&data, &parsed_xref.table, pages_num);

    let kids = match pages_dict.get(b"Kids").expect("/Kids 없음") {
        PdfObject::Array(arr) => arr.clone(),
        other => panic!("/Kids가 Array가 아님: {other:?}"),
    };

    let page_num = match &kids[0] {
        PdfObject::Reference(id) => id.number,
        other => panic!("kids[0]가 Reference가 아님: {other:?}"),
    };

    let page_dict = resolve_dict(&data, &parsed_xref.table, page_num);

    let contents_ref = page_dict.get(b"Contents").expect("page에 /Contents 없음");
    let contents_num = match contents_ref {
        PdfObject::Reference(id) => id.number,
        PdfObject::Array(arr) => match &arr[0] {
            PdfObject::Reference(id) => id.number,
            _ => panic!("contents array[0]가 Reference가 아님"),
        },
        _ => panic!("/Contents 형식 미지원"),
    };

    // 4. stream 객체 → 압축 해제
    let contents_obj = resolve_object(&data, &parsed_xref.table, contents_num);
    let stream = match &contents_obj {
        PdfObject::Stream(s) => s,
        _ => panic!("contents가 Stream이 아님"),
    };

    println!("content stream 길이(raw): {} bytes", stream.data.len());

    let filter = stream.dict.get(b"Filter");
    let stream_data = if matches!(filter, Some(PdfObject::Name(n)) if n == b"FlateDecode") {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(stream.data.as_slice());
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).expect("FlateDecode 실패");
        out
    } else {
        stream.data.clone()
    };

    println!(
        "content stream 길이(decompressed): {} bytes",
        stream_data.len()
    );
    let preview_len = 300.min(stream_data.len());
    println!(
        "처음 {preview_len}바이트:\n{}",
        String::from_utf8_lossy(&stream_data[..preview_len])
    );

    // 5. parse_content_stream 적용
    match parse_content_stream(&stream_data) {
        Ok(ops) => {
            println!("\n총 연산자 수: {}", ops.len());
            println!("처음 20개 연산자:");
            for (i, op) in ops.iter().take(20).enumerate() {
                println!(
                    "  [{i:2}] {:?} (피연산자 {}개)",
                    op.operator,
                    op.operands.len()
                );
            }
        }
        Err(e) => {
            println!("\nparse_content_stream 에러: {e:?}");
        }
    }
}

fn resolve_object(data: &[u8], table: &XrefTable, obj_num: u32) -> PdfObject {
    let entry = table
        .get(obj_num)
        .unwrap_or_else(|| panic!("obj#{obj_num} xref 없음"));
    match entry {
        XrefEntry::InUse { offset, .. } => {
            let (indirect, _) = parse_indirect_object(data, *offset as usize)
                .unwrap_or_else(|e| panic!("obj#{obj_num} 파싱 실패: {e}"));
            indirect.object
        }
        XrefEntry::Free { .. } => panic!("obj#{obj_num}이 free entry"),
        XrefEntry::Compressed { obj_stm_num, .. } => {
            let stm_entry = table
                .get(*obj_stm_num)
                .unwrap_or_else(|| panic!("ObjStm obj#{obj_stm_num} xref 없음"));
            let stm_offset = match stm_entry {
                XrefEntry::InUse { offset, .. } => *offset,
                _ => panic!("ObjStm이 InUse가 아님"),
            };
            let parsed_stm = parse_object_stream(data, stm_offset)
                .unwrap_or_else(|e| panic!("ObjStm 파싱 실패: {e}"));
            parsed_stm
                .get(obj_num)
                .cloned()
                .unwrap_or_else(|| panic!("obj#{obj_num} ObjStm에서 못 찾음"))
        }
    }
}

fn resolve_dict(data: &[u8], table: &XrefTable, obj_num: u32) -> rpdf_core::types::PdfDict {
    match resolve_object(data, table, obj_num) {
        PdfObject::Dictionary(d) => d,
        other => panic!("obj#{obj_num}이 Dictionary가 아님: {other:?}"),
    }
}

fn get_ref(dict: &rpdf_core::types::PdfDict, key: &[u8], ctx: &str) -> u32 {
    match dict
        .get(key)
        .unwrap_or_else(|| panic!("{ctx}: key {key:?} 없음"))
    {
        PdfObject::Reference(id) => id.number,
        other => panic!("{ctx}: {key:?} → {other:?} (Reference 아님)"),
    }
}
