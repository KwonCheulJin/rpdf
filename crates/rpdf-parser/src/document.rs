//! Document IR — `load_document` 진입점.
//!
//! Task #2~7에서 구축한 파서들을 단일 진입점으로 통합한다.

use std::collections::HashMap;

use rpdf_core::types::{Document, DocumentMetadata, ObjectId, Page, PdfDict, PdfObject};

use crate::xref_stream::decompress_flate;
use crate::{
    ParseError, ParsedObjectStream, find_eof, parse_content_stream, parse_indirect_object,
    parse_object_stream, parse_startxref, parse_xref,
};
use rpdf_core::types::XrefTable;

/// Reference chain 최대 허용 깊이. Task #4 MAX_OBJECT_DEPTH와 일관.
const MAX_RESOLVE_DEPTH: usize = 50;

/// PDF 파일 바이트 슬라이스에서 `Document`를 파싱한다.
///
/// Task #2~7에서 구축한 모든 파서를 통합한 단일 진입점이다.
///
/// # Errors
///
/// - [`ParseError::MissingEof`] — `%%EOF` 마커 없음
/// - [`ParseError::MissingStartXref`] — `startxref` 키워드 없음
/// - [`ParseError::MalformedCatalog`] — Catalog 구조 오류
/// - [`ParseError::MalformedPageTree`] — 페이지 트리 구조 오류
/// - [`ParseError::MalformedPageContents`] — 페이지 콘텐츠 구조 오류
/// - [`ParseError::ReferenceCycle`] / [`ParseError::ReferenceTooDeep`] — 참조 오류
pub fn load_document(data: &[u8]) -> Result<Document, ParseError> {
    // 1. EOF 탐색 + xref 파싱
    let eof_offset = find_eof(data)?;
    let xref_offset = parse_startxref(data, eof_offset)?;
    let parsed_xref = parse_xref(data, xref_offset)?;
    let xref = &parsed_xref.table;
    let trailer = &parsed_xref.trailer;

    // 2. ObjStm 로컬 캐시 (단일 load 범위 내 최적화)
    let mut stm_cache: HashMap<u32, ParsedObjectStream> = HashMap::new();

    // 3. Catalog → Pages 트리 순회
    let catalog_dict = find_catalog(data, xref, &mut stm_cache, trailer.root)?;

    let pages_ref = match catalog_dict.get(b"Pages") {
        Some(PdfObject::Reference(id)) => *id,
        Some(_) => {
            return Err(ParseError::MalformedCatalog {
                reason: "/Pages is not a reference".into(),
            });
        }
        None => {
            return Err(ParseError::MalformedCatalog {
                reason: "/Pages key missing".into(),
            });
        }
    };

    let pages_obj = get_object(data, xref, pages_ref, &mut stm_cache)?;
    let pages_dict = match pages_obj {
        PdfObject::Dictionary(d) => d,
        _ => {
            return Err(ParseError::MalformedCatalog {
                reason: "/Pages is not a dictionary".into(),
            });
        }
    };

    let inherited = InheritedPageAttrs::default();
    let mut counter: usize = 0;
    let pages = collect_pages(
        data,
        xref,
        &mut stm_cache,
        pages_dict,
        &inherited,
        &mut counter,
    )?;

    // 4. /Info → DocumentMetadata (선택)
    let metadata = trailer
        .info
        .and_then(|info_id| extract_metadata(data, xref, &mut stm_cache, info_id).ok());

    Ok(Document { pages, metadata })
}

/// `get_object_inner`의 공개 래퍼. chain을 비어있는 Vec으로 초기화한다.
pub(crate) fn get_object(
    data: &[u8],
    xref: &XrefTable,
    obj_id: ObjectId,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
) -> Result<PdfObject, ParseError> {
    get_object_inner(data, xref, obj_id, stm_cache, &mut Vec::new())
}

/// Reference chain을 따라 최종 `PdfObject`를 반환한다.
///
/// cycle 감지는 obj_num만 비교한다 (generation 무시).
/// 동일 obj_num이 chain에 두 번 나타나면 generation 차이와 무관하게
/// `ReferenceCycle` 에러로 보고.
fn get_object_inner(
    data: &[u8],
    xref: &XrefTable,
    obj_id: ObjectId,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    chain: &mut Vec<u32>,
) -> Result<PdfObject, ParseError> {
    if chain.len() >= MAX_RESOLVE_DEPTH {
        return Err(ParseError::ReferenceTooDeep {
            max_depth: MAX_RESOLVE_DEPTH,
        });
    }
    if chain.contains(&obj_id.number) {
        return Err(ParseError::ReferenceCycle { obj_id });
    }
    chain.push(obj_id.number);

    let entry = xref
        .get(obj_id.number)
        .ok_or(ParseError::ReferenceNotFound { obj_id })?;

    let obj = match entry {
        rpdf_core::types::XrefEntry::InUse { offset, .. } => {
            let (indirect, _) = parse_indirect_object(data, *offset as usize)?;
            indirect.object
        }
        rpdf_core::types::XrefEntry::Compressed {
            obj_stm_num,
            index: _,
        } => {
            let stm_num = *obj_stm_num;
            // stm_cache에 없으면 파싱 후 삽입
            if let std::collections::hash_map::Entry::Vacant(e) = stm_cache.entry(stm_num) {
                let stm_entry = xref.get(stm_num).ok_or(ParseError::ReferenceNotFound {
                    obj_id: ObjectId::new(stm_num, 0),
                })?;
                let stm_offset = match stm_entry {
                    rpdf_core::types::XrefEntry::InUse { offset, .. } => *offset,
                    _ => {
                        return Err(ParseError::ReferenceNotFound {
                            obj_id: ObjectId::new(stm_num, 0),
                        });
                    }
                };
                let parsed_stm = parse_object_stream(data, stm_offset)?;
                e.insert(parsed_stm);
            }
            // ParsedObjectStream::get은 obj_num으로 조회한다.
            stm_cache
                .get(&stm_num)
                .unwrap()
                .get(obj_id.number)
                .cloned()
                .ok_or(ParseError::ReferenceNotFound { obj_id })?
        }
        rpdf_core::types::XrefEntry::Free { .. } => {
            return Err(ParseError::ReferenceNotFound { obj_id });
        }
    };

    let result = match obj {
        PdfObject::Reference(next_id) => get_object_inner(data, xref, next_id, stm_cache, chain)?,
        other => other,
    };

    chain.pop();
    Ok(result)
}

/// `/Root`를 따라 Catalog 딕셔너리를 찾는다.
///
/// # Errors
///
/// - [`ParseError::MalformedCatalog`] — /Type /Catalog 없음 또는 딕셔너리 아님
fn find_catalog(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    root_id: ObjectId,
) -> Result<PdfDict, ParseError> {
    let obj = get_object(data, xref, root_id, stm_cache)?;
    let dict = match obj {
        PdfObject::Dictionary(d) => d,
        _ => {
            return Err(ParseError::MalformedCatalog {
                reason: "root object is not a dictionary".into(),
            });
        }
    };

    // /Type /Catalog 확인
    match dict.get(b"Type") {
        Some(PdfObject::Name(n)) if n.as_slice() == b"Catalog" => {}
        Some(_) => {
            return Err(ParseError::MalformedCatalog {
                reason: "/Type is not /Catalog".into(),
            });
        }
        None => {
            return Err(ParseError::MalformedCatalog {
                reason: "/Type key missing in root object".into(),
            });
        }
    }

    Ok(dict)
}

/// Page tree를 재귀 순회해 `Page` 목록을 수집한다.
///
/// `inherited`는 부모 노드에서 전달된 상속 속성이다.
/// `counter`는 0-based 페이지 인덱스 카운터.
fn collect_pages(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    node_dict: PdfDict,
    inherited: &InheritedPageAttrs,
    counter: &mut usize,
) -> Result<Vec<Page>, ParseError> {
    let node_inherited = merge_inherited(inherited, &node_dict);

    match (node_dict.get(b"Type"), node_dict.get(b"Kids")) {
        (Some(t), _) if t == &PdfObject::Name(b"Pages".to_vec()) => {
            // Pages 노드: /Kids 순회
            collect_kids(data, xref, stm_cache, &node_dict, &node_inherited, counter)
        }
        (Some(t), _) if t == &PdfObject::Name(b"Page".to_vec()) => {
            // Page 노드
            let page = build_page(data, xref, stm_cache, node_dict, &node_inherited, *counter)?;
            *counter += 1;
            Ok(vec![page])
        }
        (Some(_), _) => Err(ParseError::MalformedPageTree {
            reason: "unknown /Type".into(),
        }),
        // /Type 없음 → /Kids 유무로 추론
        (None, Some(_)) => {
            collect_kids(data, xref, stm_cache, &node_dict, &node_inherited, counter)
        }
        (None, None) => {
            let page = build_page(data, xref, stm_cache, node_dict, &node_inherited, *counter)?;
            *counter += 1;
            Ok(vec![page])
        }
    }
}

/// Pages 노드의 /Kids 배열을 순회한다.
fn collect_kids(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    node_dict: &PdfDict,
    node_inherited: &InheritedPageAttrs,
    counter: &mut usize,
) -> Result<Vec<Page>, ParseError> {
    let kids = match node_dict.get(b"Kids") {
        Some(PdfObject::Array(arr)) => arr.clone(),
        Some(_) => {
            return Err(ParseError::MalformedPageTree {
                reason: "/Kids is not an array".into(),
            });
        }
        None => {
            return Err(ParseError::MalformedPageTree {
                reason: "/Kids key missing in Pages node".into(),
            });
        }
    };

    let mut pages = Vec::new();
    for kid in kids {
        let kid_id = match kid {
            PdfObject::Reference(id) => id,
            _ => {
                return Err(ParseError::MalformedPageTree {
                    reason: "/Kids array contains non-reference".into(),
                });
            }
        };
        let kid_obj = get_object(data, xref, kid_id, stm_cache)?;
        let kid_dict = match kid_obj {
            PdfObject::Dictionary(d) => d,
            _ => {
                return Err(ParseError::MalformedPageTree {
                    reason: "kid object is not a dictionary".into(),
                });
            }
        };
        let mut child_pages =
            collect_pages(data, xref, stm_cache, kid_dict, node_inherited, counter)?;
        pages.append(&mut child_pages);
    }
    Ok(pages)
}

/// Page tree 노드에서 상속 가능한 4속성을 추출한다.
fn extract_heritable(dict: &PdfDict) -> InheritedPageAttrs {
    InheritedPageAttrs {
        resources: dict.get(b"Resources").and_then(|v| match v {
            PdfObject::Dictionary(d) => Some(d.clone()),
            _ => None,
        }),
        media_box: dict
            .get(b"MediaBox")
            .and_then(|v| v.as_array())
            .and_then(parse_rect),
        crop_box: dict
            .get(b"CropBox")
            .and_then(|v| v.as_array())
            .and_then(parse_rect),
        rotation: dict
            .get(b"Rotate")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32),
    }
}

/// 부모 상속값과 현재 딕셔너리를 병합한다.
///
/// child_dict에 속성이 있으면 child 우선, 없으면 parent에서 상속.
fn merge_inherited(parent: &InheritedPageAttrs, child_dict: &PdfDict) -> InheritedPageAttrs {
    let child_extracted = extract_heritable(child_dict);
    InheritedPageAttrs {
        resources: child_extracted
            .resources
            .or_else(|| parent.resources.clone()),
        media_box: child_extracted.media_box.or(parent.media_box),
        crop_box: child_extracted.crop_box.or(parent.crop_box),
        rotation: child_extracted.rotation.or(parent.rotation),
    }
}

/// `[f64; 4]` rect 배열을 파싱한다. 배열 원소가 Integer이면 f64로 변환.
fn parse_rect(arr: &[PdfObject]) -> Option<[f64; 4]> {
    if arr.len() != 4 {
        return None;
    }
    let mut result = [0.0f64; 4];
    for (i, obj) in arr.iter().enumerate() {
        result[i] = match obj {
            PdfObject::Real(f) => *f,
            PdfObject::Integer(n) => *n as f64,
            _ => return None,
        };
    }
    Some(result)
}

/// Page 딕셔너리에서 `Page` 구조체를 빌드한다.
fn build_page(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    page_dict: PdfDict,
    inherited: &InheritedPageAttrs,
    index: usize,
) -> Result<Page, ParseError> {
    let page_inherited = merge_inherited(inherited, &page_dict);

    let raw_content = merge_contents(data, xref, stm_cache, &page_dict)?;
    let content = if raw_content.is_empty() {
        Vec::new()
    } else {
        parse_content_stream(&raw_content).map_err(|e| ParseError::MalformedPageContents {
            reason: e.to_string(),
        })?
    };

    Ok(Page {
        index,
        content,
        resources: page_inherited.resources,
        media_box: page_inherited.media_box,
        crop_box: page_inherited.crop_box,
        rotation: page_inherited.rotation.unwrap_or(0),
    })
}

/// 페이지의 /Contents를 합성해 raw bytes를 반환한다.
///
/// - /Contents 없음 → `Ok(vec![])`
/// - Reference → get_object → Stream → data
/// - Array → 각 항목 Reference → get_object → Stream.data → 순서대로 연결
/// - FlateDecode 스트림 → 압축 해제 후 반환
/// - 다른 필터 → `InvalidContentStreamFilter`
fn merge_contents(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    page_dict: &PdfDict,
) -> Result<Vec<u8>, ParseError> {
    let contents_val = match page_dict.get(b"Contents") {
        None => return Ok(Vec::new()),
        Some(v) => v.clone(),
    };

    // Reference 또는 Array를 처리
    match contents_val {
        PdfObject::Reference(id) => {
            let obj = get_object(data, xref, id, stm_cache)?;
            extract_stream_data(obj, id)
        }
        PdfObject::Array(arr) => {
            let mut combined = Vec::new();
            for item in arr {
                let item_id = match item {
                    PdfObject::Reference(id) => id,
                    _ => {
                        return Err(ParseError::MalformedPageContents {
                            reason: "/Contents array contains non-reference".into(),
                        });
                    }
                };
                let obj = get_object(data, xref, item_id, stm_cache)?;
                let chunk = extract_stream_data(obj, item_id)?;
                combined.extend_from_slice(&chunk);
            }
            Ok(combined)
        }
        _ => Err(ParseError::MalformedPageContents {
            reason: "/Contents is not a reference or array".into(),
        }),
    }
}

/// `PdfObject::Stream`에서 raw bytes를 추출하고 필터를 처리한다.
///
/// - 필터 없음 → raw data 그대로 반환
/// - `/FlateDecode` → 압축 해제 후 반환
/// - 그 외 → `InvalidContentStreamFilter`
fn extract_stream_data(obj: PdfObject, obj_id: ObjectId) -> Result<Vec<u8>, ParseError> {
    let stream = match obj {
        PdfObject::Stream(s) => s,
        _ => {
            return Err(ParseError::MalformedPageContents {
                reason: format!("Contents object {} is not a stream", obj_id),
            });
        }
    };

    match stream.dict.get(b"Filter") {
        None => Ok(stream.data),
        Some(PdfObject::Name(name)) if name.as_slice() == b"FlateDecode" => {
            // 오프셋은 진단 목적으로만 사용 (정확한 값 없음, 0 사용)
            decompress_flate(&stream.data, 0).map_err(|e| ParseError::MalformedPageContents {
                reason: format!("FlateDecode 압축 해제 실패: {e}"),
            })
        }
        Some(PdfObject::Name(name)) => Err(ParseError::InvalidContentStreamFilter {
            filter: String::from_utf8_lossy(name).into_owned(),
        }),
        Some(PdfObject::Array(arr)) => {
            // 배열 필터: 단일 FlateDecode인 경우만 처리
            if arr.len() == 1 {
                match &arr[0] {
                    PdfObject::Name(name) if name.as_slice() == b"FlateDecode" => {
                        decompress_flate(&stream.data, 0).map_err(|e| {
                            ParseError::MalformedPageContents {
                                reason: format!("FlateDecode 압축 해제 실패: {e}"),
                            }
                        })
                    }
                    PdfObject::Name(name) => Err(ParseError::InvalidContentStreamFilter {
                        filter: String::from_utf8_lossy(name).into_owned(),
                    }),
                    _ => Err(ParseError::MalformedPageContents {
                        reason: "/Filter array contains non-name".into(),
                    }),
                }
            } else {
                Err(ParseError::InvalidContentStreamFilter {
                    filter: format!("multiple filters ({})", arr.len()),
                })
            }
        }
        Some(_) => Err(ParseError::MalformedPageContents {
            reason: "/Filter value is not a name or array".into(),
        }),
    }
}

/// /Info 딕셔너리에서 메타데이터를 추출한다.
fn extract_metadata(
    data: &[u8],
    xref: &XrefTable,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
    info_id: ObjectId,
) -> Result<DocumentMetadata, ParseError> {
    let obj = get_object(data, xref, info_id, stm_cache)?;
    let dict = match obj {
        PdfObject::Dictionary(d) => d,
        _ => return Ok(DocumentMetadata::default()),
    };

    let extract_str = |key: &[u8]| -> Option<Vec<u8>> {
        dict.get(key)
            .and_then(|v| v.as_string_bytes())
            .map(|b| b.to_vec())
    };

    Ok(DocumentMetadata {
        title: extract_str(b"Title"),
        author: extract_str(b"Author"),
        subject: extract_str(b"Subject"),
        creator: extract_str(b"Creator"),
        producer: extract_str(b"Producer"),
        creation_date: extract_str(b"CreationDate"),
        modification_date: extract_str(b"ModDate"),
    })
}

/// Page tree에서 상속 가능한 4속성 컨텍스트.
#[derive(Debug, Clone, Default)]
struct InheritedPageAttrs {
    resources: Option<PdfDict>,
    media_box: Option<[f64; 4]>,
    crop_box: Option<[f64; 4]>,
    rotation: Option<i32>,
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use rpdf_core::types::{XrefEntry, XrefTable};

    // ── 테스트 헬퍼 ─────────────────────────────────────────────────────────────

    /// 간단한 간접 객체 바이트를 만든다.
    fn make_indirect(obj_num: u32, generation: u16, body: &str) -> Vec<u8> {
        format!("{obj_num} {generation} obj\n{body}\nendobj").into_bytes()
    }

    fn make_xref_inuse(offset: u64) -> XrefEntry {
        XrefEntry::InUse {
            offset,
            generation: 0,
        }
    }

    // ── Checkpoint B 단위 테스트 ─────────────────────────────────────────────

    // B-1. InUse 경로 정상 → PdfObject 반환
    #[test]
    fn b1_inuse_returns_object() {
        let data = make_indirect(1, 0, "42");
        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(0));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        assert_eq!(obj, PdfObject::Integer(42));
    }

    // B-2. Free → ReferenceNotFound
    #[test]
    fn b2_free_entry_returns_not_found() {
        let data = b"";
        let mut xref = XrefTable::new();
        xref.insert_if_absent(
            1,
            XrefEntry::Free {
                next_free_obj_num: 0,
                generation: 65535,
            },
        );
        let mut cache = HashMap::new();
        let err = get_object(data, &xref, ObjectId::new(1, 0), &mut cache).unwrap_err();
        assert!(
            matches!(err, ParseError::ReferenceNotFound { .. }),
            "expected ReferenceNotFound, got {err:?}"
        );
    }

    // B-3. 존재하지 않는 obj_id → ReferenceNotFound
    #[test]
    fn b3_missing_entry_returns_not_found() {
        let data = b"";
        let xref = XrefTable::new();
        let mut cache = HashMap::new();
        let err = get_object(data, &xref, ObjectId::new(99, 0), &mut cache).unwrap_err();
        assert!(
            matches!(err, ParseError::ReferenceNotFound { .. }),
            "expected ReferenceNotFound, got {err:?}"
        );
    }

    // B-4. Reference 체인 2단계 해소 (A→B→Integer)
    #[test]
    fn b4_reference_chain_two_steps() {
        // obj 1: Reference to obj 2
        // obj 2: integer 99
        let mut data = Vec::new();
        let offset1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "2 0 R"));
        let offset2 = data.len();
        data.extend_from_slice(b" "); // separator
        data.extend_from_slice(&make_indirect(2, 0, "99"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(offset1 as u64));
        xref.insert_if_absent(2, make_xref_inuse((offset2 + 1) as u64));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        assert_eq!(obj, PdfObject::Integer(99));
    }

    // B-5. Reference cycle (A→B→A) → ReferenceCycle
    #[test]
    fn b5_reference_cycle_detected() {
        let mut data = Vec::new();
        let offset1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "2 0 R"));
        let offset2 = data.len();
        data.extend_from_slice(&make_indirect(2, 0, "1 0 R"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(offset1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(offset2 as u64));
        let mut cache = HashMap::new();
        let err = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap_err();
        assert!(
            matches!(err, ParseError::ReferenceCycle { .. }),
            "expected ReferenceCycle, got {err:?}"
        );
    }

    // B-6. 깊이 51단 체인 → ReferenceTooDeep
    #[test]
    fn b6_deep_chain_returns_too_deep() {
        // 51개 객체가 체인을 이룬다. obj 1 → obj 2 → ... → obj 51 → obj 52(integer)
        // MAX_RESOLVE_DEPTH = 50 → chain.len() = 50일 때 에러 발생
        let total = 52usize;
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for i in 1..total {
            offsets.push(data.len());
            let body = format!("{} 0 R", i + 1);
            data.extend_from_slice(&make_indirect(i as u32, 0, &body));
        }
        // 마지막 obj: integer
        offsets.push(data.len());
        data.extend_from_slice(&make_indirect(total as u32, 0, "999"));

        let mut xref = XrefTable::new();
        for (i, &off) in offsets.iter().enumerate() {
            xref.insert_if_absent((i + 1) as u32, make_xref_inuse(off as u64));
        }
        let mut cache = HashMap::new();
        let err = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap_err();
        assert!(
            matches!(err, ParseError::ReferenceTooDeep { .. }),
            "expected ReferenceTooDeep, got {err:?}"
        );
    }

    // ── Checkpoint C 단위 테스트 ─────────────────────────────────────────────

    // C-1. 단순 1페이지 구조
    #[test]
    fn c1_single_page_structure() {
        // Catalog(obj1) → Pages(obj2) → Page(obj3)
        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "<< /Type /Catalog /Pages 2 0 R >>"));
        let off2 = data.len();
        data.extend_from_slice(&make_indirect(
            2,
            0,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        ));
        let off3 = data.len();
        data.extend_from_slice(&make_indirect(
            3,
            0,
            "<< /Type /Page /MediaBox [0 0 612 792] >>",
        ));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(off2 as u64));
        xref.insert_if_absent(3, make_xref_inuse(off3 as u64));
        let mut cache = HashMap::new();

        let catalog = find_catalog(&data, &xref, &mut cache, ObjectId::new(1, 0)).unwrap();
        let pages_ref = match catalog.get(b"Pages") {
            Some(PdfObject::Reference(id)) => *id,
            _ => panic!("no /Pages"),
        };
        let pages_obj = get_object(&data, &xref, pages_ref, &mut cache).unwrap();
        let pages_dict = pages_obj.as_dict().unwrap().clone();
        let mut counter = 0;
        let pages = collect_pages(
            &data,
            &xref,
            &mut cache,
            pages_dict,
            &InheritedPageAttrs::default(),
            &mut counter,
        )
        .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].index, 0);
    }

    // C-2. 2단계 중첩 Pages
    #[test]
    fn c2_nested_pages_two_levels() {
        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "<< /Type /Catalog /Pages 2 0 R >>"));
        let off2 = data.len();
        data.extend_from_slice(&make_indirect(
            2,
            0,
            "<< /Type /Pages /Kids [3 0 R] /Count 2 >>",
        ));
        let off3 = data.len();
        data.extend_from_slice(&make_indirect(
            3,
            0,
            "<< /Type /Pages /Kids [4 0 R 5 0 R] /Count 2 >>",
        ));
        let off4 = data.len();
        data.extend_from_slice(&make_indirect(4, 0, "<< /Type /Page >>"));
        let off5 = data.len();
        data.extend_from_slice(&make_indirect(5, 0, "<< /Type /Page >>"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(off2 as u64));
        xref.insert_if_absent(3, make_xref_inuse(off3 as u64));
        xref.insert_if_absent(4, make_xref_inuse(off4 as u64));
        xref.insert_if_absent(5, make_xref_inuse(off5 as u64));
        let mut cache = HashMap::new();

        let catalog = find_catalog(&data, &xref, &mut cache, ObjectId::new(1, 0)).unwrap();
        let pages_ref = match catalog.get(b"Pages") {
            Some(PdfObject::Reference(id)) => *id,
            _ => panic!("no /Pages"),
        };
        let pages_dict = get_object(&data, &xref, pages_ref, &mut cache)
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        let mut counter = 0;
        let pages = collect_pages(
            &data,
            &xref,
            &mut cache,
            pages_dict,
            &InheritedPageAttrs::default(),
            &mut counter,
        )
        .unwrap();
        assert_eq!(pages.len(), 2);
    }

    // C-3. /Type /Catalog 없는 root → MalformedCatalog
    #[test]
    fn c3_missing_catalog_type_returns_error() {
        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "<< /Pages 2 0 R >>"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        let mut cache = HashMap::new();
        let err = find_catalog(&data, &xref, &mut cache, ObjectId::new(1, 0)).unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedCatalog { .. }),
            "expected MalformedCatalog, got {err:?}"
        );
    }

    // C-4. /Kids 없는 Pages → MalformedPageTree
    #[test]
    fn c4_pages_without_kids_returns_error() {
        let mut data = Vec::new();
        let off = data.len();
        data.extend_from_slice(&make_indirect(2, 0, "<< /Type /Pages /Count 0 >>"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(2, make_xref_inuse(off as u64));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(2, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();
        let mut counter = 0;
        let err = collect_pages(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            &mut counter,
        )
        .unwrap_err();
        assert!(
            matches!(err, ParseError::MalformedPageTree { .. }),
            "expected MalformedPageTree, got {err:?}"
        );
    }

    // C-5. 빈 /Kids → 빈 Vec 반환 (에러 아님)
    #[test]
    fn c5_empty_kids_returns_empty_vec() {
        let mut data = Vec::new();
        let off = data.len();
        data.extend_from_slice(&make_indirect(2, 0, "<< /Type /Pages /Kids [] /Count 0 >>"));

        let mut xref = XrefTable::new();
        xref.insert_if_absent(2, make_xref_inuse(off as u64));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(2, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();
        let mut counter = 0;
        let pages = collect_pages(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            &mut counter,
        )
        .unwrap();
        assert!(pages.is_empty());
    }

    // ── Checkpoint D 단위 테스트 ─────────────────────────────────────────────

    // D-1. 자기 /MediaBox 있는 Page → 자기 값 사용
    #[test]
    fn d1_page_own_mediabox_used() {
        let page_dict_bytes = b"<< /Type /Page /MediaBox [0 0 612 792] >>";
        let data = format!(
            "1 0 obj\n{}\nendobj",
            std::str::from_utf8(page_dict_bytes).unwrap()
        )
        .into_bytes();
        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(0));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();

        // parent inherited에 다른 MediaBox
        let inherited = InheritedPageAttrs {
            media_box: Some([0.0, 0.0, 100.0, 100.0]),
            ..Default::default()
        };
        let page = build_page(&data, &xref, &mut cache, dict, &inherited, 0).unwrap();
        // 자기 값 [0,0,612,792] 사용
        assert_eq!(page.media_box(), Some([0.0, 0.0, 612.0, 792.0]));
    }

    // D-2. 자기 /MediaBox 없는 Page + 부모 /MediaBox → 상속값 사용
    #[test]
    fn d2_page_inherits_mediabox() {
        let data = make_indirect(1, 0, "<< /Type /Page >>");
        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(0));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();

        let inherited = InheritedPageAttrs {
            media_box: Some([0.0, 0.0, 595.0, 842.0]),
            ..Default::default()
        };
        let page = build_page(&data, &xref, &mut cache, dict, &inherited, 0).unwrap();
        assert_eq!(page.media_box(), Some([0.0, 0.0, 595.0, 842.0]));
    }

    // D-3. /Rotate 없는 Page → rotation = 0
    #[test]
    fn d3_page_default_rotation_is_zero() {
        let data = make_indirect(1, 0, "<< /Type /Page >>");
        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(0));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();
        let page = build_page(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            0,
        )
        .unwrap();
        assert_eq!(page.rotation(), 0);
    }

    // D-4. Integer MediaBox → f64 변환
    #[test]
    fn d4_integer_mediabox_converted_to_f64() {
        let arr = vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(612),
            PdfObject::Integer(792),
        ];
        let result = parse_rect(&arr).unwrap();
        assert_eq!(result, [0.0, 0.0, 612.0, 792.0]);
    }

    // ── Checkpoint E 단위 테스트 ─────────────────────────────────────────────

    // E-1. /Contents 없는 Page → content = vec![]
    #[test]
    fn e1_page_without_contents_has_empty_content() {
        let data = make_indirect(1, 0, "<< /Type /Page >>");
        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(0));
        let mut cache = HashMap::new();
        let obj = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj.as_dict().unwrap().clone();
        let page = build_page(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            0,
        )
        .unwrap();
        assert!(page.content().is_empty());
    }

    // E-2. 단일 /Contents (Reference → Stream) → 정상 파싱
    #[test]
    fn e2_single_contents_reference_parsed() {
        // page: { /Type /Page /Contents 2 0 R }
        // obj 2: stream { BT ET }
        let stream_body = b"BT ET";
        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "<< /Type /Page /Contents 2 0 R >>"));
        let off2 = data.len();
        let stream_def = format!(
            "2 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj",
            stream_body.len(),
            std::str::from_utf8(stream_body).unwrap()
        );
        data.extend_from_slice(stream_def.as_bytes());

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(off2 as u64));
        let mut cache = HashMap::new();
        let obj1 = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj1.as_dict().unwrap().clone();
        let page = build_page(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            0,
        )
        .unwrap();
        assert!(!page.content().is_empty(), "content should have operations");
    }

    // E-3. /Contents 배열 2개 → 두 stream 합쳐서 파싱
    #[test]
    fn e3_contents_array_two_streams_merged() {
        let stream1 = b"BT";
        let stream2 = b" ET";
        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(
            1,
            0,
            "<< /Type /Page /Contents [2 0 R 3 0 R] >>",
        ));
        let off2 = data.len();
        data.extend_from_slice(
            format!(
                "2 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj",
                stream1.len(),
                std::str::from_utf8(stream1).unwrap()
            )
            .as_bytes(),
        );
        let off3 = data.len();
        data.extend_from_slice(
            format!(
                "3 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj",
                stream2.len(),
                std::str::from_utf8(stream2).unwrap()
            )
            .as_bytes(),
        );

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(off2 as u64));
        xref.insert_if_absent(3, make_xref_inuse(off3 as u64));
        let mut cache = HashMap::new();
        let obj1 = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj1.as_dict().unwrap().clone();
        let page = build_page(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            0,
        )
        .unwrap();
        // BT ET → 2개 연산자
        assert_eq!(page.content().len(), 2);
    }

    // E-4. FlateDecode /Contents → 해제 후 파싱
    #[test]
    fn e4_flatedecode_contents_decompressed() {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        let plain = b"BT ET";
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(plain).unwrap();
        let compressed = enc.finish().unwrap();

        let mut data = Vec::new();
        let off1 = data.len();
        data.extend_from_slice(&make_indirect(1, 0, "<< /Type /Page /Contents 2 0 R >>"));
        let off2 = data.len();
        data.extend_from_slice(
            format!(
                "2 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n",
                compressed.len()
            )
            .as_bytes(),
        );
        data.extend_from_slice(&compressed);
        data.extend_from_slice(b"\nendstream\nendobj");

        let mut xref = XrefTable::new();
        xref.insert_if_absent(1, make_xref_inuse(off1 as u64));
        xref.insert_if_absent(2, make_xref_inuse(off2 as u64));
        let mut cache = HashMap::new();
        let obj1 = get_object(&data, &xref, ObjectId::new(1, 0), &mut cache).unwrap();
        let dict = obj1.as_dict().unwrap().clone();
        let page = build_page(
            &data,
            &xref,
            &mut cache,
            dict,
            &InheritedPageAttrs::default(),
            0,
        )
        .unwrap();
        // BT ET → 2개 연산자
        assert_eq!(page.content().len(), 2);
    }
}
