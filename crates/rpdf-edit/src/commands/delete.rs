use std::sync::Mutex;

use rpdf_core::types::document::{Document, Page};

use crate::commands::{Command, CommandError};

/// 지정한 페이지 인덱스 목록을 Document에서 제거하는 커맨드.
///
/// `indices`는 0-based 페이지 인덱스 목록이다. execute 시 중복 제거 + 정렬 후 처리된다.
/// Undo 시 원래 순서대로 페이지를 복원하며, `Page.index` 필드도 재정렬된다.
///
/// # Examples
///
/// ```rust
/// use rpdf_edit::commands::DeletePagesCommand;
///
/// let cmd = DeletePagesCommand::new(vec![0, 2]);
/// ```
pub struct DeletePagesCommand {
    indices: Vec<usize>,
    snapshot: Mutex<Option<Vec<(usize, Page)>>>,
}

impl DeletePagesCommand {
    /// 새 `DeletePagesCommand`를 생성한다.
    ///
    /// # Arguments
    /// * `indices` - 삭제할 0-based 페이지 인덱스 목록. 중복 허용 (자동 dedup 처리됨).
    pub fn new(indices: Vec<usize>) -> Self {
        Self {
            indices,
            snapshot: Mutex::new(None),
        }
    }
}

impl Command for DeletePagesCommand {
    fn name(&self) -> &'static str {
        "DeletePagesCommand"
    }

    fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
        // 0. 이중 실행 방어
        if self.snapshot.lock().unwrap().is_some() {
            return Err(CommandError::ExecutionFailed(
                "DeletePagesCommand already executed".to_string(),
            ));
        }

        // 1. 정렬 후 dedup (dedup은 인접 중복만 제거하므로 sort 먼저)
        let mut sorted = self.indices.clone();
        sorted.sort_unstable();
        sorted.dedup();

        // 2. 빈 indices → no-op
        if sorted.is_empty() {
            *self.snapshot.lock().unwrap() = Some(vec![]);
            return Ok(());
        }

        // 3. 범위 검증 (원자성: 하나라도 범위 초과 시 전체 취소)
        let len = doc.pages.len();
        for &i in &sorted {
            if i >= len {
                return Err(CommandError::ExecutionFailed(format!(
                    "page index out of bounds: {i} (document has {len} pages)"
                )));
            }
        }

        // 4. 역순 제거: 내림차순 정렬된 인덱스로 remove — Move 시맨틱스, clone 없음
        let mut sorted_desc = sorted.clone();
        sorted_desc.sort_unstable_by(|a, b| b.cmp(a));

        let mut removed: Vec<(usize, Page)> = sorted_desc
            .iter()
            .map(|&i| (i, doc.pages.remove(i)))
            .collect();

        // 오름차순 재정렬 (undo 시 insert 순서 보장)
        removed.sort_by_key(|&(i, _)| i);

        *self.snapshot.lock().unwrap() = Some(removed);

        // 5. 남은 페이지 index 재정렬
        for (i, page) in doc.pages.iter_mut().enumerate() {
            page.index = i;
        }

        Ok(())
    }

    fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
        // 1. snapshot.take() — None이면 execute 전 undo 호출 오류
        let snapshot =
            self.snapshot.lock().unwrap().take().ok_or_else(|| {
                CommandError::UndoFailed("undo called before execute".to_string())
            })?;

        // 2. 오름차순으로 삽입 (original_index 순서대로 삽입해야 올바른 위치에 들어감)
        for (original_index, page) in snapshot {
            doc.pages.insert(original_index, page);
        }

        // 3. 전체 페이지 index 재정렬
        for (i, page) in doc.pages.iter_mut().enumerate() {
            page.index = i;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rpdf_core::types::document::{Document, Page};

    use super::*;
    use crate::commands::CommandStack;

    fn make_doc(pages: usize, rotations: &[i32]) -> Document {
        let page_vec = (0..pages)
            .map(|i| Page {
                index: i,
                content: vec![],
                resources: None,
                media_box: None,
                crop_box: None,
                rotation: rotations.get(i).copied().unwrap_or(0),
            })
            .collect();
        Document {
            pages: page_vec,
            metadata: None,
        }
    }

    #[test]
    fn delete_single_page() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![1]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        assert_eq!(doc.pages[0].index, 0);
        assert_eq!(doc.pages[1].index, 1);
    }

    #[test]
    fn delete_multiple_pages() {
        let mut doc = make_doc(5, &[]);
        let cmd = DeletePagesCommand::new(vec![1, 3]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
    }

    #[test]
    fn delete_first_page() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![0]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        assert_eq!(doc.pages[0].index, 0);
    }

    #[test]
    fn delete_last_page() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![2]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        assert_eq!(doc.pages[1].index, 1);
    }

    #[test]
    fn undo_restores_deleted_pages() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![1]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
        assert_eq!(doc.pages[0].index, 0);
        assert_eq!(doc.pages[1].index, 1);
        assert_eq!(doc.pages[2].index, 2);
    }

    #[test]
    fn execute_undo_redo_via_stack() {
        let mut doc = make_doc(3, &[]);
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(DeletePagesCommand::new(vec![1])), &mut doc)
            .unwrap();
        assert_eq!(doc.pages.len(), 2);

        stack.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);

        stack.redo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn page_index_out_of_bounds() {
        let mut doc = make_doc(2, &[]);
        let cmd = DeletePagesCommand::new(vec![5]);
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("page index out of bounds"));
            assert!(msg.contains("document has 2 pages"));
        }
    }

    #[test]
    fn duplicate_indices_deduplicated() {
        let mut doc = make_doc(4, &[]);
        let cmd = DeletePagesCommand::new(vec![1, 1, 2]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn empty_indices_is_noop() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
    }

    #[test]
    fn delete_all_pages() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![0, 1, 2]);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 0);
        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 3);
        for i in 0..3 {
            assert_eq!(doc.pages[i].index, i);
        }
    }

    #[test]
    fn undo_before_execute_fails() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![0]);
        let err = cmd.undo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::UndoFailed(_)));
        if let CommandError::UndoFailed(msg) = err {
            assert!(msg.contains("undo called before execute"));
        }
    }

    #[test]
    fn page_indices_consistent_after_execute() {
        let mut doc = make_doc(5, &[]);
        let cmd = DeletePagesCommand::new(vec![1, 3]);
        cmd.execute(&mut doc).unwrap();
        for (i, page) in doc.pages.iter().enumerate() {
            assert_eq!(page.index, i);
        }
    }

    #[test]
    fn page_indices_consistent_after_undo() {
        let mut doc = make_doc(5, &[]);
        let cmd = DeletePagesCommand::new(vec![1, 3]);
        cmd.execute(&mut doc).unwrap();
        cmd.undo(&mut doc).unwrap();
        for (i, page) in doc.pages.iter().enumerate() {
            assert_eq!(page.index, i);
        }
    }

    #[test]
    fn partial_out_of_bounds_is_atomic() {
        let mut doc = make_doc(2, &[]);
        let cmd = DeletePagesCommand::new(vec![0, 5]);
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        // doc 변경 없음 (원자성 보장)
        assert_eq!(doc.pages.len(), 2);
        assert_eq!(doc.pages[0].index, 0);
        assert_eq!(doc.pages[1].index, 1);
    }

    #[test]
    fn double_execute_fails() {
        let mut doc = make_doc(3, &[]);
        let cmd = DeletePagesCommand::new(vec![1]);
        cmd.execute(&mut doc).unwrap();
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("already executed"));
        }
    }
}
