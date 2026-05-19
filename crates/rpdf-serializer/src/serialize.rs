use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use rpdf_core::types::document::Document;
use rpdf_parser::{ParseError, load_document};

use crate::error::SerializeError;
use crate::types::PageSource;

/// PDF 바이트에서 `Document`와 `PageSource` 목록을 함께 반환한다.
///
/// `Document.pages[i]`에 대응하는 원본 출처가 `sources[i]`에 담겨있다.
/// 커맨드 실행 후에는 호출자가 `sources`를 `pages`와 동기화해야 한다.
///
/// # Errors
///
/// - [`ParseError`] — rpdf-parser 파싱 실패 시
pub fn load_document_tracked(data: &[u8]) -> Result<(Document, Vec<PageSource>), ParseError> {
    let doc = load_document(data)?;
    // 한 번만 clone해 모든 PageSource가 공유하도록 Arc로 감싼다.
    let bytes_arc = Arc::new(data.to_vec());
    let sources = doc
        .pages
        .iter()
        .map(|p| PageSource {
            bytes: Arc::clone(&bytes_arc),
            page_index: p.index,
        })
        .collect();
    Ok((doc, sources))
}

/// `Document` IR을 PDF 바이트로 직렬화한다.
///
/// `sources[i]`는 `doc.pages[i]`의 원본 출처여야 한다.
/// `sources.len() != doc.pages.len()`이면 에러를 반환한다.
///
/// # 소스 동일성 판별
///
/// `Arc::ptr_eq`로 포인터를 비교해 단일 소스(같은 파일)와
/// 다중 소스(MergeCommand 결과)를 구분한다.
///
/// # Errors
///
/// - [`SerializeError::EmptyDocument`] — pages가 비어있음
/// - [`SerializeError::SourceLengthMismatch`] — sources와 pages 개수 불일치
/// - [`SerializeError::LoadSource`] — lopdf가 source_bytes 로드 실패
/// - [`SerializeError::PageOutOfBounds`] — source.page_index가 원본 페이지 수 초과
/// - [`SerializeError::Save`] — lopdf save_to 실패
pub fn serialize_document(
    doc: &Document,
    sources: &[PageSource],
) -> Result<Vec<u8>, SerializeError> {
    // 1. pages 비어있으면 즉시 에러
    if doc.pages.is_empty() {
        return Err(SerializeError::EmptyDocument);
    }

    // 2. sources.len() != pages.len() → SourceLengthMismatch
    if sources.len() != doc.pages.len() {
        return Err(SerializeError::SourceLengthMismatch {
            sources: sources.len(),
            pages: doc.pages.len(),
        });
    }

    // 3. 소스 그룹핑: Arc 포인터로 동일 소스 식별
    // 첫 번째 소스 기준으로, 다른 포인터가 하나라도 있으면 다중 소스
    let is_multi_source = sources
        .iter()
        .any(|s| !Arc::ptr_eq(&s.bytes, &sources[0].bytes));

    if is_multi_source {
        serialize_multi_source(doc, sources)
    } else {
        serialize_single_source(doc, sources)
    }
}

/// 단일 소스 경로: 모든 sources[i].bytes가 동일한 Arc 포인터를 가리키는 경우.
fn serialize_single_source(
    doc: &Document,
    sources: &[PageSource],
) -> Result<Vec<u8>, SerializeError> {
    let bytes = &sources[0].bytes;

    // 4a. lopdf로 로드
    let mut lopdf_doc = lopdf::Document::load_mem(bytes).map_err(SerializeError::LoadSource)?;

    // 4b. 삭제 전에 page_oid_map 구성 (ObjectId는 delete 후에도 변경되지 않음)
    // lopdf.get_pages() → BTreeMap<u32, ObjectId> (u32: 1-based)
    let lopdf_pages: BTreeMap<u32, lopdf::ObjectId> = lopdf_doc.get_pages();
    let total_page_count = lopdf_pages.len();

    // src_page_index(0-based) → ObjectId 매핑
    let page_oid_map: HashMap<usize, lopdf::ObjectId> = lopdf_pages
        .into_iter()
        .map(|(one_based, oid)| ((one_based - 1) as usize, oid))
        .collect();

    // 4b. PageOutOfBounds 검증
    for source in sources {
        if source.page_index >= total_page_count {
            return Err(SerializeError::PageOutOfBounds {
                idx: source.page_index,
                count: total_page_count,
            });
        }
    }

    // 4c. 유지할 원본 페이지 번호(1-based) 집합
    let keep_set: HashSet<u32> = sources.iter().map(|s| (s.page_index + 1) as u32).collect();

    // 4d. 삭제할 페이지 번호(1-based) 목록
    let delete_list: Vec<u32> = (1..=(total_page_count as u32))
        .filter(|n| !keep_set.contains(n))
        .collect();

    // 4e. 페이지 삭제
    if !delete_list.is_empty() {
        lopdf_doc.delete_pages(&delete_list);
    }

    // 4f. rotation 항상 적용 (rotation == 0이어도 /Rotate 0을 명시적으로 설정)
    // ObjectId는 delete_pages 후에도 변경되지 않으므로 pre-build 매핑 재사용 가능
    apply_rotation(doc, sources, &page_oid_map, &mut lopdf_doc);

    // 4g. 직렬화
    save_to_vec(lopdf_doc)
}

/// 다중 소스 경로: Arc 포인터가 다른 sources가 있는 경우 (MergeCommand 결과).
///
/// lopdf 공식 merge 예제 패턴을 따른다:
/// 1. 각 소스 Document를 renumber_objects_with로 ID 충돌 방지
/// 2. 모든 objects를 수집 (Catalog/Pages는 특별 처리)
/// 3. 통합 Catalog/Pages 구성 + 각 Page에 Parent 설정
/// 4. 단일 lopdf Document로 저장
fn serialize_multi_source(
    doc: &Document,
    sources: &[PageSource],
) -> Result<Vec<u8>, SerializeError> {
    // a. 소스별로 고유한 Arc 포인터 목록 수집 (순서 보존)
    let mut seen_ptrs: Vec<*const Vec<u8>> = Vec::new();
    let mut unique_arcs: Vec<Arc<Vec<u8>>> = Vec::new();
    for source in sources {
        let ptr = Arc::as_ptr(&source.bytes);
        if !seen_ptrs.contains(&ptr) {
            seen_ptrs.push(ptr);
            unique_arcs.push(Arc::clone(&source.bytes));
        }
    }

    // b. 각 고유 소스별 lopdf Document 로드 + page ObjectId 매핑 구성
    struct SourceEntry {
        lopdf_doc: lopdf::Document,
        /// 0-based page_index → ObjectId (renumber 후 갱신됨)
        page_oid_map: HashMap<usize, lopdf::ObjectId>,
        total_pages: usize,
    }

    let mut entries: Vec<(*const Vec<u8>, SourceEntry)> = Vec::new();
    for arc_bytes in &unique_arcs {
        let lopdf_doc = lopdf::Document::load_mem(arc_bytes).map_err(SerializeError::LoadSource)?;

        let lopdf_pages: BTreeMap<u32, lopdf::ObjectId> = lopdf_doc.get_pages();
        let total_pages = lopdf_pages.len();
        let page_oid_map: HashMap<usize, lopdf::ObjectId> = lopdf_pages
            .into_iter()
            .map(|(one_based, oid)| ((one_based - 1) as usize, oid))
            .collect();

        entries.push((
            Arc::as_ptr(arc_bytes),
            SourceEntry {
                lopdf_doc,
                page_oid_map,
                total_pages,
            },
        ));
    }

    // PageOutOfBounds 검증
    for source in sources {
        let ptr = Arc::as_ptr(&source.bytes);
        if let Some((_, entry)) = entries.iter().find(|(p, _)| *p == ptr)
            && source.page_index >= entry.total_pages
        {
            return Err(SerializeError::PageOutOfBounds {
                idx: source.page_index,
                count: entry.total_pages,
            });
        }
    }

    // c. renumber_objects_with로 ID 충돌 방지
    // 첫 번째 소스의 max_id 이후부터 두 번째 소스 ID 시작
    // renumber 후 ObjectId가 변경되므로 page_oid_map을 반드시 재계산한다.
    let mut max_id = entries[0].1.lopdf_doc.max_id;
    for (_, entry) in entries.iter_mut().skip(1) {
        entry.lopdf_doc.renumber_objects_with(max_id + 1);
        max_id = entry.lopdf_doc.max_id;

        // renumber 후 ObjectId 변경 → 매핑 재계산
        let new_pages: BTreeMap<u32, lopdf::ObjectId> = entry.lopdf_doc.get_pages();
        entry.page_oid_map = new_pages
            .into_iter()
            .map(|(one_based, oid)| ((one_based - 1) as usize, oid))
            .collect();
    }

    // d. 결과 Document 구성 (lopdf 공식 merge 패턴)
    // 첫 번째 소스의 버전을 사용해 새 Document 생성
    let pdf_version = entries[0].1.lopdf_doc.version.clone();
    let mut merged = lopdf::Document::with_version(pdf_version);

    // doc.pages 순서대로 유지할 Page ObjectId 목록 구성
    // 각 source.page_index → 해당 소스의 renumber된 ObjectId
    let ordered_page_ids: Vec<lopdf::ObjectId> = sources
        .iter()
        .map(|source| {
            let ptr = Arc::as_ptr(&source.bytes);
            let (_, entry) = entries.iter().find(|(p, _)| *p == ptr).unwrap();
            *entry.page_oid_map.get(&source.page_index).unwrap()
        })
        .collect();

    // e. 모든 소스의 objects를 merged에 수집
    // Catalog, Pages 타입은 스킵하고 별도 처리
    // Page 타입은 그대로 수집 (Parent는 나중에 업데이트)
    let mut catalog_obj: Option<(lopdf::ObjectId, lopdf::Object)> = None;
    let mut pages_obj: Option<(lopdf::ObjectId, lopdf::Object)> = None;

    for (_, entry) in &entries {
        for (oid, obj) in &entry.lopdf_doc.objects {
            let type_name = obj.type_name().unwrap_or(b"");
            match type_name {
                b"Catalog" => {
                    // 첫 번째 Catalog만 유지
                    if catalog_obj.is_none() {
                        catalog_obj = Some((*oid, obj.clone()));
                    }
                }
                b"Pages" => {
                    // 첫 번째 Pages만 유지 (Kids, Count는 나중에 재구성)
                    if pages_obj.is_none() {
                        pages_obj = Some((*oid, obj.clone()));
                    }
                }
                b"Outlines" | b"Outline" => {
                    // Merge 시 Outlines 미지원 (v0.3 알려진 한계)
                }
                _ => {
                    merged.objects.insert(*oid, obj.clone());
                }
            }
        }
    }

    let (catalog_id, catalog_object) = catalog_obj.ok_or_else(|| {
        SerializeError::Save(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no Catalog found in source documents",
        ))
    })?;
    let (pages_id, pages_object) = pages_obj.ok_or_else(|| {
        SerializeError::Save(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no Pages root found in source documents",
        ))
    })?;

    // f. 각 Page에 Parent를 통합 pages_id로 설정
    for page_oid in &ordered_page_ids {
        if let Some(lopdf::Object::Dictionary(dict)) = merged.objects.get_mut(page_oid) {
            dict.set("Parent", lopdf::Object::Reference(pages_id));
        }
    }

    // g. Pages 딕셔너리 재구성: /Kids, /Count 업데이트
    if let lopdf::Object::Dictionary(mut pages_dict) = pages_object {
        let kids: Vec<lopdf::Object> = ordered_page_ids
            .iter()
            .map(|&oid| lopdf::Object::Reference(oid))
            .collect();
        pages_dict.set("Kids", lopdf::Object::Array(kids));
        pages_dict.set(
            "Count",
            lopdf::Object::Integer(ordered_page_ids.len() as i64),
        );
        // Outlines 제거 (merge 시 미지원)
        pages_dict.remove(b"Outlines");
        merged
            .objects
            .insert(pages_id, lopdf::Object::Dictionary(pages_dict));
    }

    // h. Catalog 삽입 + trailer /Root 설정
    if let lopdf::Object::Dictionary(mut cat_dict) = catalog_object {
        cat_dict.set("Pages", lopdf::Object::Reference(pages_id));
        cat_dict.remove(b"Outlines");
        merged
            .objects
            .insert(catalog_id, lopdf::Object::Dictionary(cat_dict));
    }
    merged
        .trailer
        .set("Root", lopdf::Object::Reference(catalog_id));

    // i. max_id 갱신 + renumber
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();

    // j. renumber 후 페이지 ObjectId 매핑 재계산 (rotation 적용을 위해)
    let renumbered_pages: BTreeMap<u32, lopdf::ObjectId> = merged.get_pages();
    // renumber 후 페이지 순서 보존: ordered_page_ids의 순서가 get_pages() 순서와 일치해야 한다.
    // get_pages()는 1-based 순서로 반환하므로 doc.pages 순서와 대응
    let renumbered_oids: Vec<lopdf::ObjectId> = renumbered_pages.values().cloned().collect();

    // k. rotation 항상 적용
    for (i, page) in doc.pages.iter().enumerate() {
        if let Some(&oid) = renumbered_oids.get(i) {
            set_rotation(&mut merged, oid, page.rotation);
        }
    }

    // l. 직렬화
    save_to_vec(merged)
}

/// 단일 page object의 /Rotate 값을 설정한다.
///
/// rotation == 0이어도 반드시 /Rotate 0을 명시적으로 설정한다.
/// 원본이 /Rotate 90이었다가 0으로 복원하는 경우를 처리하기 위함.
fn set_rotation(lopdf_doc: &mut lopdf::Document, oid: lopdf::ObjectId, rotation: i32) {
    if let Ok(lopdf::Object::Dictionary(dict)) = lopdf_doc.get_object_mut(oid) {
        dict.set("Rotate", lopdf::Object::Integer(rotation as i64));
    }
}

/// doc.pages와 sources를 zip하여 각 페이지 ObjectId에 rotation을 적용한다.
fn apply_rotation(
    doc: &Document,
    sources: &[PageSource],
    page_oid_map: &HashMap<usize, lopdf::ObjectId>,
    lopdf_doc: &mut lopdf::Document,
) {
    for (page, source) in doc.pages.iter().zip(sources.iter()) {
        if let Some(&oid) = page_oid_map.get(&source.page_index) {
            set_rotation(lopdf_doc, oid, page.rotation);
        }
    }
}

/// lopdf Document를 Vec<u8>으로 직렬화한다.
fn save_to_vec(mut lopdf_doc: lopdf::Document) -> Result<Vec<u8>, SerializeError> {
    let mut out = Vec::new();
    lopdf_doc.save_to(&mut out)?;
    Ok(out)
}
