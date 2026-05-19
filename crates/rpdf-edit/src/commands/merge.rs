use std::sync::Mutex;

use rpdf_core::types::document::Document;

use crate::commands::{Command, CommandError, reindex_pages};

/// 하나 이상의 소스 Document 페이지를 대상 Document 뒤에 순서대로 추가하는 커맨드.
///
/// 소스 내 페이지 순서와 소스 간 순서 모두 보존된다.
/// Undo 시 추가된 페이지를 제거해 원래 상태로 복원한다.
///
/// # Ownership
///
/// sources의 소유권을 커맨드가 가져감.
/// 호출자가 재사용하려면 `doc.clone()` 후 전달.
///
/// # Examples
///
/// ```rust
/// use rpdf_edit::commands::MergeCommand;
/// use rpdf_core::types::document::Document;
///
/// let source = Document { pages: vec![], metadata: None };
/// let cmd = MergeCommand::new(vec![source]);
/// ```
pub struct MergeCommand {
    sources: Vec<Document>,
    snapshot: Mutex<Option<usize>>,
}

impl MergeCommand {
    /// 새 `MergeCommand`를 생성한다.
    ///
    /// # Arguments
    /// * `sources` - 합산할 소스 Document 목록. 소유권을 커맨드가 가져감.
    ///   호출자가 재사용하려면 `doc.clone()` 후 전달.
    pub fn new(sources: Vec<Document>) -> Self {
        Self {
            sources,
            snapshot: Mutex::new(None),
        }
    }
}

impl Command for MergeCommand {
    fn name(&self) -> &'static str {
        "MergeCommand"
    }

    fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
        // 0. 이중 실행 방어
        if self.snapshot.lock().unwrap().is_some() {
            return Err(CommandError::ExecutionFailed(
                "MergeCommand already executed".to_string(),
            ));
        }

        // 1. execute 전 페이지 수 저장
        let original_len = doc.pages.len();

        // 2. sources 비어 있으면 no-op
        if self.sources.is_empty() {
            *self.snapshot.lock().unwrap() = Some(original_len);
            return Ok(());
        }

        // 3. 각 source document의 pages를 순서대로 clone + append
        for source in &self.sources {
            doc.pages.extend(source.pages.iter().cloned());
        }

        // 4. 전체 페이지 index 재정렬
        reindex_pages(&mut doc.pages);

        // 5. snapshot 저장
        *self.snapshot.lock().unwrap() = Some(original_len);

        Ok(())
    }

    fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
        // 1. snapshot.take() — None이면 execute 전 undo 호출 오류
        let original_len =
            self.snapshot.lock().unwrap().take().ok_or_else(|| {
                CommandError::UndoFailed("undo called before execute".to_string())
            })?;

        // 2. 추가된 페이지 제거
        doc.pages.truncate(original_len);

        // 3. 전체 페이지 index 재정렬
        reindex_pages(&mut doc.pages);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandStack;
    use crate::commands::test_utils::make_doc;

    #[test]
    fn merge_single_source() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 5);
    }

    #[test]
    fn merge_multiple_sources() {
        let mut doc = make_doc(2, &[]);
        let s1 = make_doc(3, &[10, 11, 12]);
        let s2 = make_doc(1, &[20]);
        let cmd = MergeCommand::new(vec![s1, s2]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 6);
        // 소스 간 순서: s1 페이지(index 2,3,4) → s2 페이지(index 5)
        assert_eq!(doc.pages[2].rotation, 10);
        assert_eq!(doc.pages[4].rotation, 12);
        assert_eq!(doc.pages[5].rotation, 20);
    }

    #[test]
    fn merge_empty_sources_is_noop() {
        let mut doc = make_doc(3, &[]);
        let cmd = MergeCommand::new(vec![]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
    }

    #[test]
    fn merge_empty_source_document() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(0, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
    }

    #[test]
    fn merge_into_empty_target() {
        let mut doc = make_doc(0, &[]);
        let source = make_doc(3, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 0);
    }

    #[test]
    fn undo_restores_original_pages() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 5);
        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
        for i in 0..3 {
            assert_eq!(doc.pages[i].index, i);
        }
    }

    #[test]
    fn execute_undo_redo_via_stack() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(MergeCommand::new(vec![source])), &mut doc)
            .unwrap();
        assert_eq!(doc.pages.len(), 5);

        stack.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);

        stack.redo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 5);
    }

    #[test]
    fn page_indices_consistent_after_merge() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        for (i, page) in doc.pages.iter().enumerate() {
            assert_eq!(page.index, i);
        }
    }

    #[test]
    fn page_indices_consistent_after_undo() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        cmd.undo(&mut doc).unwrap();
        for (i, page) in doc.pages.iter().enumerate() {
            assert_eq!(page.index, i);
        }
    }

    #[test]
    fn double_execute_fails() {
        let mut doc = make_doc(3, &[]);
        let source = make_doc(2, &[]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("already executed"));
        }
    }

    #[test]
    fn undo_before_execute_fails() {
        let mut doc = make_doc(3, &[]);
        let cmd = MergeCommand::new(vec![]);
        let err = cmd.undo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::UndoFailed(_)));
        if let CommandError::UndoFailed(msg) = err {
            assert!(msg.contains("undo called before execute"));
        }
    }

    #[test]
    fn merge_preserves_page_content() {
        let mut doc = make_doc(1, &[90]);
        doc.pages[0].media_box = Some([0.0, 0.0, 595.0, 842.0]);

        let mut source = make_doc(1, &[180]);
        source.pages[0].media_box = Some([0.0, 0.0, 210.0, 297.0]);

        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();

        assert_eq!(doc.pages[0].rotation, 90);
        assert_eq!(doc.pages[0].media_box, Some([0.0, 0.0, 595.0, 842.0]));
        assert_eq!(doc.pages[1].rotation, 180);
        assert_eq!(doc.pages[1].media_box, Some([0.0, 0.0, 210.0, 297.0]));
    }

    #[test]
    fn merge_source_page_order_preserved() {
        let mut doc = make_doc(2, &[]);
        let source = make_doc(3, &[90, 180, 270]);
        let cmd = MergeCommand::new(vec![source]);
        cmd.execute(&mut doc).unwrap();

        assert_eq!(doc.pages.len(), 5);
        // 소스 페이지 순서: rotation 90, 180, 270
        assert_eq!(doc.pages[2].rotation, 90);
        assert_eq!(doc.pages[3].rotation, 180);
        assert_eq!(doc.pages[4].rotation, 270);
        // index도 올바르게 재정렬됨
        assert_eq!(doc.pages[2].index, 2);
        assert_eq!(doc.pages[3].index, 3);
        assert_eq!(doc.pages[4].index, 4);
    }
}
