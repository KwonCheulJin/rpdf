use std::collections::HashSet;

use js_sys::Error as JsError;
use rpdf_core::types::document::Document;
use rpdf_edit::commands::{CommandStack, DeletePagesCommand, RotatePageCommand};
use rpdf_serializer::{PageSource, load_document_tracked, serialize_document};
use wasm_bindgen::prelude::*;

// ============================================================
// 내부 헬퍼 — JsValue 없는 플레인 Rust (네이티브 단위 테스트 가능)
// ============================================================

/// 0-based 페이지 인덱스가 유효한지 검증한다.
fn validate_page_index(idx: usize, count: usize) -> Result<(), String> {
    if idx >= count {
        return Err(format!("페이지 인덱스 범위 초과: {idx}"));
    }
    Ok(())
}

/// 회전각이 유효한지 검증한다. 0, 90, 180, 270만 허용한다.
fn validate_degrees(degrees: i32) -> Result<(), String> {
    match degrees {
        0 | 90 | 180 | 270 => Ok(()),
        _ => Err(format!(
            "유효하지 않은 회전각: {degrees} (0/90/180/270만 허용)"
        )),
    }
}

/// 삭제 인덱스 목록이 유효한지 검증한다.
///
/// `page_count`는 현재 문서 페이지 수이다.
fn validate_delete_indices(indices: &[usize], page_count: usize) -> Result<(), String> {
    if indices.is_empty() {
        return Err("삭제할 페이지 목록이 비어있습니다".to_string());
    }
    for &i in indices {
        if i >= page_count {
            return Err(format!("페이지 인덱스 범위 초과: {i}"));
        }
    }
    Ok(())
}

/// 삭제 후 남은 sources를 반환한다.
///
/// `deleted_indices`는 0-based 인덱스 목록이다.
fn compute_new_sources(sources: &[PageSource], deleted_indices: &[usize]) -> Vec<PageSource> {
    let idx_set: HashSet<usize> = deleted_indices.iter().copied().collect();
    sources
        .iter()
        .enumerate()
        .filter(|(i, _)| !idx_set.contains(i))
        .map(|(_, s)| PageSource {
            bytes: s.bytes.clone(),
            page_index: s.page_index,
        })
        .collect()
}

// ============================================================
// 직렬화 구조체 — page_info 반환용
// ============================================================

#[derive(serde::Serialize)]
struct PageInfo {
    index: usize,
    rotation: i32,
    media_box: Option<[f64; 4]>,
    crop_box: Option<[f64; 4]>,
}

// ============================================================
// wasm_bindgen 구조체
// ============================================================

/// JS/TypeScript에서 PDF를 파싱·편집·저장하는 WASM API.
#[wasm_bindgen]
pub struct PdfDocument {
    doc: Document,
    sources: Vec<PageSource>,
    stack: CommandStack,
    sources_undo: Vec<Vec<PageSource>>,
    sources_redo: Vec<Vec<PageSource>>,
}

#[wasm_bindgen]
impl PdfDocument {
    /// PDF 바이트에서 `PdfDocument`를 생성한다.
    ///
    /// # Errors
    ///
    /// 파싱 실패 시 JsValue 에러를 반환한다.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<PdfDocument, JsValue> {
        let (doc, sources) =
            load_document_tracked(data).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        })
    }

    /// 문서의 페이지 수를 반환한다.
    pub fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    /// 지정한 0-based 인덱스 페이지의 정보를 JsValue로 반환한다.
    ///
    /// 반환 형식: `{ index, rotation, media_box, crop_box }`
    ///
    /// # Errors
    ///
    /// - 인덱스가 범위를 초과하면 JsValue 에러를 반환한다.
    pub fn page_info(&self, index: usize) -> Result<JsValue, JsValue> {
        validate_page_index(index, self.doc.page_count())
            .map_err(|e| JsValue::from(JsError::new(&e)))?;

        let page = &self.doc.pages[index];
        let info = PageInfo {
            index: page.index,
            rotation: page.rotation,
            media_box: page.media_box,
            crop_box: page.crop_box,
        };
        serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from(JsError::new(&e.to_string())))
    }

    /// 현재 문서 상태를 PDF 바이트로 직렬화해 반환한다.
    ///
    /// # Errors
    ///
    /// 직렬화 실패 시 JsValue 에러를 반환한다.
    pub fn save(&self) -> Result<Vec<u8>, JsValue> {
        serialize_document(&self.doc, &self.sources)
            .map_err(|e| JsValue::from(JsError::new(&e.to_string())))
    }

    /// 지정한 0-based 인덱스 페이지를 degrees만큼 회전한다.
    ///
    /// `degrees`는 0, 90, 180, 270만 허용한다.
    ///
    /// # Errors
    ///
    /// - 인덱스 범위 초과 또는 유효하지 않은 각도 시 JsValue 에러를 반환한다.
    pub fn rotate_page(&mut self, index: usize, degrees: i32) -> Result<(), JsValue> {
        validate_page_index(index, self.doc.page_count())
            .map_err(|e| JsValue::from(JsError::new(&e)))?;
        validate_degrees(degrees).map_err(|e| JsValue::from(JsError::new(&e)))?;

        let cmd = Box::new(RotatePageCommand::new(index, degrees));
        // rotate_page는 sources를 변경하지 않으므로 new_sources = None
        self.execute_cmd(cmd, None)
    }

    /// 지정한 0-based 인덱스 목록에 해당하는 페이지를 삭제한다.
    ///
    /// `indices`는 JS `Uint32Array`와 대응하는 `Vec<u32>`이다.
    ///
    /// # Errors
    ///
    /// - 빈 목록, 인덱스 범위 초과 시 JsValue 에러를 반환한다.
    pub fn delete_pages(&mut self, indices: Vec<u32>) -> Result<(), JsValue> {
        let indices_usize: Vec<usize> = indices.iter().map(|&i| i as usize).collect();

        validate_delete_indices(&indices_usize, self.doc.page_count())
            .map_err(|e| JsValue::from(JsError::new(&e)))?;

        let new_sources = compute_new_sources(&self.sources, &indices_usize);
        let cmd = Box::new(DeletePagesCommand::new(indices_usize));
        self.execute_cmd(cmd, Some(new_sources))
    }

    /// 마지막 편집을 되돌린다.
    ///
    /// # Errors
    ///
    /// 되돌릴 커맨드가 없으면 JsValue 에러를 반환한다.
    pub fn undo(&mut self) -> Result<(), JsValue> {
        if self.stack.undo_len() == 0 {
            return Err(JsValue::from(JsError::new("되돌릴 커맨드 없음")));
        }

        self.stack
            .undo(&mut self.doc)
            .map_err(|e| JsValue::from(JsError::new(&e.to_string())))?;

        // sources_undo는 CommandStack과 항상 동일 높이 — 반드시 pop 가능
        let prev = self.sources_undo.pop().expect("sources_undo 높이 불일치");
        let current = std::mem::replace(&mut self.sources, prev);
        self.sources_redo.push(current);
        Ok(())
    }

    /// 마지막으로 되돌린 편집을 다시 적용한다.
    ///
    /// # Errors
    ///
    /// 다시 실행할 커맨드가 없으면 JsValue 에러를 반환한다.
    pub fn redo(&mut self) -> Result<(), JsValue> {
        if self.stack.redo_len() == 0 {
            return Err(JsValue::from(JsError::new("다시 실행할 커맨드 없음")));
        }

        self.stack
            .redo(&mut self.doc)
            .map_err(|e| JsValue::from(JsError::new(&e.to_string())))?;

        // sources_redo는 CommandStack redo 스택과 항상 동일 높이 — 반드시 pop 가능
        let next = self.sources_redo.pop().expect("sources_redo 높이 불일치");
        let current = std::mem::replace(&mut self.sources, next);
        self.sources_undo.push(current);
        Ok(())
    }

    /// 현재 undo 스택 크기를 반환한다.
    pub fn undo_len(&self) -> usize {
        self.stack.undo_len()
    }

    /// 현재 redo 스택 크기를 반환한다.
    pub fn redo_len(&self) -> usize {
        self.stack.redo_len()
    }
}

impl PdfDocument {
    /// 커맨드를 실행하고 sources 스냅샷을 항상 push한다.
    ///
    /// execute 성공 후 현재 sources를 sources_undo에 항상 push해
    /// CommandStack과 sources_undo의 높이를 일치시킨다.
    /// `new_sources`가 Some이면 sources를 new_sources로 교체한다.
    /// execute 실패 시에는 스냅샷을 push하지 않아 탈동기화를 방지한다.
    fn execute_cmd(
        &mut self,
        cmd: Box<dyn rpdf_edit::commands::Command>,
        new_sources: Option<Vec<PageSource>>,
    ) -> Result<(), JsValue> {
        // ⚠️ execute 성공 후에만 sources_undo push — 실패 시 스냅샷 탈동기화 방지
        self.stack
            .execute(cmd, &mut self.doc)
            .map_err(|e| JsValue::from(JsError::new(&e.to_string())))?;

        // 항상 push → CommandStack과 sources_undo 높이 일치 보장
        self.sources_undo.push(self.sources.clone());
        self.sources_redo.clear();
        if let Some(s) = new_sources {
            self.sources = s;
        }
        Ok(())
    }
}

// ============================================================
// 단위 테스트 — 네이티브 cargo test (JsValue 없는 내부 헬퍼 + PDF fixture)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // PDF fixture: samples/trad-xref-basicapi.pdf (단순 1페이지 PDF)
    const FIXTURE_BYTES: &[u8] = include_bytes!("../../../samples/trad-xref-basicapi.pdf");

    // PDF fixture: samples/xref-stream-doc-13-pages.pdf (다중 페이지 PDF)
    const MULTI_PAGE_FIXTURE: &[u8] =
        include_bytes!("../../../samples/xref-stream-doc-13-pages.pdf");

    // ── 내부 헬퍼 테스트 ──────────────────────────────────────

    #[test]
    fn validate_page_index_valid() {
        assert!(validate_page_index(0, 5).is_ok());
        assert!(validate_page_index(4, 5).is_ok());
    }

    #[test]
    fn validate_page_index_out_of_bounds() {
        let err = validate_page_index(5, 5).unwrap_err();
        assert!(err.contains("페이지 인덱스 범위 초과: 5"));
    }

    #[test]
    fn validate_degrees_valid() {
        for d in [0, 90, 180, 270] {
            assert!(validate_degrees(d).is_ok());
        }
    }

    #[test]
    fn validate_degrees_invalid() {
        let err = validate_degrees(45).unwrap_err();
        assert!(err.contains("유효하지 않은 회전각: 45"));
    }

    #[test]
    fn compute_new_sources_removes_correct() {
        let (_, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let original_len = sources.len();
        let new_sources = compute_new_sources(&sources, &[0, 2]);
        assert_eq!(new_sources.len(), original_len - 2);
    }

    // ── UT-01: new() — 유효한 PDF bytes → page_count > 0 ─────

    #[test]
    fn ut_01_new_valid_pdf() {
        let (doc, _) = load_document_tracked(FIXTURE_BYTES).unwrap();
        assert!(doc.page_count() > 0);
    }

    // ── UT-02: new() — 빈 bytes → Err ────────────────────────

    #[test]
    fn ut_02_new_empty_bytes() {
        let result = load_document_tracked(&[]);
        assert!(result.is_err());
    }

    // ── UT-03: rotate_page — 유효 인덱스·각도 → undo_len 1 증가

    #[test]
    fn ut_03_rotate_page_valid() {
        let (doc, sources) = load_document_tracked(FIXTURE_BYTES).unwrap();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        assert_eq!(pdf.undo_len(), 0);
        pdf.rotate_page(0, 90).expect("rotate_page should succeed");
        assert_eq!(pdf.undo_len(), 1);
    }

    // ── UT-04: rotate_page — 범위 초과 → Err ─────────────────
    // 내부 헬퍼를 직접 테스트 (JsError::new는 비-wasm 타겟에서 호출 불가)

    #[test]
    fn ut_04_rotate_page_out_of_bounds() {
        let (doc, _) = load_document_tracked(FIXTURE_BYTES).unwrap();
        // validate_page_index가 에러를 반환함을 검증
        let result = validate_page_index(9999, doc.page_count());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("페이지 인덱스 범위 초과: 9999"));
    }

    // ── UT-05: rotate_page — 유효하지 않은 각도 → Err ────────
    // 내부 헬퍼를 직접 테스트 (JsError::new는 비-wasm 타겟에서 호출 불가)

    #[test]
    fn ut_05_rotate_page_invalid_degrees() {
        let result = validate_degrees(45);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("유효하지 않은 회전각: 45"));
    }

    // ── UT-06: delete_pages — 유효 인덱스 → page_count 감소 ──

    #[test]
    fn ut_06_delete_pages_valid() {
        let (doc, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let original_count = doc.page_count();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        pdf.delete_pages(vec![0])
            .expect("delete_pages should succeed");
        assert_eq!(pdf.page_count(), original_count - 1);
    }

    // ── UT-07: delete_pages — 빈 목록 → Err ──────────────────

    #[test]
    fn ut_07_delete_pages_empty() {
        let result = validate_delete_indices(&[], 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("비어있습니다"));
    }

    // ── UT-08: undo → undo_len 감소, redo_len 증가, sources.len() == page_count()

    #[test]
    fn ut_08_undo_sources_consistency() {
        let (doc, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        pdf.delete_pages(vec![0]).unwrap();
        let undo_len_before = pdf.undo_len();
        pdf.undo().unwrap();
        assert_eq!(pdf.undo_len(), undo_len_before - 1);
        assert_eq!(pdf.redo_len(), 1);
        assert_eq!(pdf.sources.len(), pdf.page_count());
    }

    // ── UT-09: redo → redo_len 감소, undo_len 증가, sources.len() == page_count()

    #[test]
    fn ut_09_redo_sources_consistency() {
        let (doc, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        pdf.delete_pages(vec![0]).unwrap();
        pdf.undo().unwrap();
        pdf.redo().unwrap();
        assert_eq!(pdf.redo_len(), 0);
        assert_eq!(pdf.sources.len(), pdf.page_count());
    }

    // ── UT-10: save — 유효 문서 → bytes 비어있지 않음 ─────────

    #[test]
    fn ut_10_save_non_empty() {
        let (doc, sources) = load_document_tracked(FIXTURE_BYTES).unwrap();
        let pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        let bytes = pdf.save().expect("save should succeed");
        assert!(!bytes.is_empty());
    }

    // ── UT-11: delete_pages → undo → save → 성공 (sources 복원 검증) ─

    #[test]
    fn ut_11_delete_undo_save() {
        let (doc, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let original_count = doc.page_count();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };
        pdf.delete_pages(vec![0]).unwrap();
        assert_eq!(pdf.page_count(), original_count - 1);
        assert_eq!(pdf.sources.len(), pdf.page_count());

        pdf.undo().unwrap();
        assert_eq!(pdf.page_count(), original_count);
        assert_eq!(pdf.sources.len(), pdf.page_count());

        let bytes = pdf.save().expect("save after undo should succeed");
        assert!(!bytes.is_empty());
    }

    // ── UT-12: page_info — rotation/media_box 필드 포함 ───────

    #[test]
    fn ut_12_page_info_fields() {
        let (doc, _sources) = load_document_tracked(FIXTURE_BYTES).unwrap();
        // page_info는 내부 헬퍼로 직접 테스트
        let page = &doc.pages[0];
        let info = PageInfo {
            index: page.index,
            rotation: page.rotation,
            media_box: page.media_box,
            crop_box: page.crop_box,
        };
        // rotation 필드가 유효한 값인지 확인
        assert!(matches!(info.rotation, 0 | 90 | 180 | 270));
        // index 필드 확인
        assert_eq!(info.index, 0);
    }

    // ── UT-13: 혼합 커맨드 — rotate_page → delete_pages → undo → undo
    //          sources.len() == doc.page_count() 일관성 검증

    #[test]
    fn ut_13_mixed_commands_undo_consistency() {
        let (doc, sources) = load_document_tracked(MULTI_PAGE_FIXTURE).unwrap();
        let original_count = doc.page_count();
        let mut pdf = PdfDocument {
            doc,
            sources,
            stack: CommandStack::new(50),
            sources_undo: Vec::new(),
            sources_redo: Vec::new(),
        };

        // 1단계: rotate_page (sources 변경 없음)
        pdf.rotate_page(0, 90).expect("rotate_page should succeed");
        assert_eq!(pdf.sources.len(), pdf.page_count());

        // 2단계: delete_pages (sources 1 감소)
        pdf.delete_pages(vec![1]).expect("delete_pages should succeed");
        assert_eq!(pdf.page_count(), original_count - 1);
        assert_eq!(pdf.sources.len(), pdf.page_count());

        // 3단계: undo delete_pages → sources 복원
        pdf.undo().expect("undo delete should succeed");
        assert_eq!(pdf.page_count(), original_count);
        assert_eq!(pdf.sources.len(), pdf.page_count());

        // 4단계: undo rotate_page → sources 여전히 일치
        pdf.undo().expect("undo rotate should succeed");
        assert_eq!(pdf.page_count(), original_count);
        assert_eq!(pdf.sources.len(), pdf.page_count());
    }
}
