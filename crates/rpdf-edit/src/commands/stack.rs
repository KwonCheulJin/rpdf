use rpdf_core::types::document::Document;

use crate::commands::error::CommandError;
use crate::commands::traits::Command;

/// undo/redo 기록을 관리하는 커맨드 스택.
///
/// `max_depth`를 초과하면 가장 오래된 항목을 제거한다.
/// `max_depth = 0`으로 생성하면 내부적으로 1로 강제된다.
pub struct CommandStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
}

impl CommandStack {
    /// 새 커맨드 스택을 생성한다.
    ///
    /// `max_depth = 0`이면 1로 강제한다.
    pub fn new(max_depth: usize) -> Self {
        let effective_depth = if max_depth == 0 { 1 } else { max_depth };
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: effective_depth,
        }
    }

    /// 커맨드를 실행하고 undo 스택에 push한다.
    ///
    /// 실행 성공 후 redo 스택을 비운다. undo 스택이 `max_depth`를 초과하면
    /// 가장 오래된 항목(index 0)을 제거한다.
    pub fn execute(
        &mut self,
        cmd: Box<dyn Command>,
        doc: &mut Document,
    ) -> Result<(), CommandError> {
        cmd.execute(doc)?;
        self.redo_stack.clear();
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
        Ok(())
    }

    /// 마지막으로 실행한 커맨드를 되돌린다.
    ///
    /// undo 실패 시 pop한 커맨드를 다시 undo 스택에 push해 원자성을 보장한다.
    pub fn undo(&mut self, doc: &mut Document) -> Result<(), CommandError> {
        let cmd = self.undo_stack.pop().ok_or(CommandError::NothingToUndo)?;
        match cmd.undo(doc) {
            Ok(()) => {
                self.redo_stack.push(cmd);
                Ok(())
            }
            Err(e) => {
                // undo 실패 시 스택 상태를 복원해 원자성을 보장한다.
                self.undo_stack.push(cmd);
                Err(e)
            }
        }
    }

    /// 마지막으로 되돌린 커맨드를 다시 실행한다.
    ///
    /// redo는 `cmd.execute(doc)`를 재호출하고 undo 스택에 push한다.
    pub fn redo(&mut self, doc: &mut Document) -> Result<(), CommandError> {
        let cmd = self.redo_stack.pop().ok_or(CommandError::NothingToRedo)?;
        cmd.execute(doc)?;
        self.undo_stack.push(cmd);
        Ok(())
    }

    /// 현재 undo 스택 크기를 반환한다.
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    /// 현재 redo 스택 크기를 반환한다.
    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rpdf_core::types::document::{Document, DocumentMetadata, Page};

    use super::*;

    fn make_doc() -> Document {
        Document {
            pages: vec![Page {
                index: 0,
                content: vec![],
                resources: None,
                media_box: None,
                crop_box: None,
                rotation: 0,
            }],
            metadata: Some(DocumentMetadata {
                title: None,
                ..Default::default()
            }),
        }
    }

    struct ToggleTitleCommand {
        // Command: Send + Sync 조건 충족을 위해 Mutex를 사용한다.
        prev_title: Mutex<Option<Vec<u8>>>,
    }

    impl ToggleTitleCommand {
        fn new() -> Self {
            Self {
                prev_title: Mutex::new(None),
            }
        }
    }

    impl Command for ToggleTitleCommand {
        fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
            let current = doc.metadata.as_ref().and_then(|m| m.title.clone());
            *self.prev_title.lock().unwrap() = current;
            if let Some(meta) = doc.metadata.as_mut() {
                meta.title = Some(b"test".to_vec());
            }
            Ok(())
        }

        fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
            let prev = self.prev_title.lock().unwrap().take();
            if let Some(meta) = doc.metadata.as_mut() {
                meta.title = prev;
            }
            Ok(())
        }

        fn name(&self) -> &'static str {
            "toggle_title"
        }
    }

    struct FailingUndoCommand;

    impl Command for FailingUndoCommand {
        fn execute(&self, _doc: &mut Document) -> Result<(), CommandError> {
            Ok(())
        }

        fn undo(&self, _doc: &mut Document) -> Result<(), CommandError> {
            Err(CommandError::UndoFailed("의도적 실패".to_string()))
        }

        fn name(&self) -> &'static str {
            "failing_undo"
        }
    }

    fn title_of(doc: &Document) -> Option<Vec<u8>> {
        doc.metadata.as_ref().and_then(|m| m.title.clone())
    }

    #[test]
    fn execute_undo_redo_roundtrip() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        assert_eq!(title_of(&doc), Some(b"test".to_vec()));

        stack.undo(&mut doc).unwrap();
        assert_eq!(title_of(&doc), None);

        stack.redo(&mut doc).unwrap();
        assert_eq!(title_of(&doc), Some(b"test".to_vec()));
    }

    #[test]
    fn undo_moves_to_redo_stack() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        assert_eq!(stack.redo_len(), 0);

        stack.undo(&mut doc).unwrap();
        assert_eq!(stack.redo_len(), 1);
    }

    #[test]
    fn execute_clears_redo_stack() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        stack.undo(&mut doc).unwrap();
        assert_eq!(stack.redo_len(), 1);

        // 새 execute → redo 스택 비워짐
        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        assert_eq!(stack.redo_len(), 0);
    }

    #[test]
    fn undo_on_empty_stack_returns_nothing_to_undo() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        let err = stack.undo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::NothingToUndo));
    }

    #[test]
    fn redo_on_empty_stack_returns_nothing_to_redo() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        let err = stack.redo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::NothingToRedo));
    }

    #[test]
    fn max_depth_drops_oldest_entry() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(3);

        for _ in 0..4 {
            stack
                .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
                .unwrap();
        }
        assert_eq!(stack.undo_len(), 3);
    }

    #[test]
    fn redo_then_undo_roundtrip() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        // execute → undo → redo → undo 4단계 라운드트립
        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        assert_eq!(title_of(&doc), Some(b"test".to_vec()));

        stack.undo(&mut doc).unwrap();
        assert_eq!(title_of(&doc), None);

        stack.redo(&mut doc).unwrap();
        assert_eq!(title_of(&doc), Some(b"test".to_vec()));

        stack.undo(&mut doc).unwrap();
        assert_eq!(title_of(&doc), None);
    }

    #[test]
    fn undo_failure_preserves_undo_stack_len() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(FailingUndoCommand), &mut doc)
            .unwrap();
        let len_before = stack.undo_len();

        let err = stack.undo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::UndoFailed(_)));
        // 실패 원자성: undo 실패 후 스택 크기가 변경 전과 동일해야 한다
        assert_eq!(stack.undo_len(), len_before);
    }

    #[test]
    fn max_depth_one_keeps_only_latest() {
        let mut doc = make_doc();
        let mut stack = CommandStack::new(1);

        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        stack
            .execute(Box::new(ToggleTitleCommand::new()), &mut doc)
            .unwrap();
        assert_eq!(stack.undo_len(), 1);
    }
}
