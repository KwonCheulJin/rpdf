use std::sync::Mutex;

use rpdf_core::types::document::Document;

use crate::commands::{Command, CommandError};

/// PDF 페이지를 상대적인 각도만큼 회전시키는 커맨드.
///
/// `degrees`는 상대 회전값으로 90의 배수만 허용한다(양수: 시계방향, 음수: 반시계방향).
/// `/Rotate` 값은 PDF 스펙(ISO 32000)에 따라 0·90·180·270으로 정규화된다.
///
/// # Examples
///
/// ```rust
/// use rpdf_edit::commands::RotatePageCommand;
///
/// let cmd = RotatePageCommand::new(0, 90);
/// ```
pub struct RotatePageCommand {
    page_index: usize,
    degrees: i32,
    prev_rotation: Mutex<Option<i32>>,
}

impl RotatePageCommand {
    /// 새 `RotatePageCommand`를 생성한다.
    ///
    /// # Arguments
    /// * `page_index` - 0-based 페이지 인덱스
    /// * `degrees` - 상대 회전값 (90의 배수; 양수=시계방향, 음수=반시계방향)
    pub fn new(page_index: usize, degrees: i32) -> Self {
        Self {
            page_index,
            degrees,
            prev_rotation: Mutex::new(None),
        }
    }
}

impl Command for RotatePageCommand {
    fn name(&self) -> &'static str {
        "RotatePageCommand"
    }

    fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
        if self.page_index >= doc.pages.len() {
            return Err(CommandError::ExecutionFailed(format!(
                "page index out of bounds: {} (document has {} pages)",
                self.page_index,
                doc.pages.len()
            )));
        }
        if self.degrees % 90 != 0 {
            return Err(CommandError::ExecutionFailed(format!(
                "degrees must be a multiple of 90, got {}; valid: 90, 180, 270, -90, ...",
                self.degrees
            )));
        }
        let current_rotation = doc.pages[self.page_index].rotation;
        *self.prev_rotation.lock().unwrap() = Some(current_rotation);
        let new_rotation = (current_rotation + self.degrees).rem_euclid(360);
        doc.pages[self.page_index].rotation = new_rotation;
        Ok(())
    }

    fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
        if self.page_index >= doc.pages.len() {
            return Err(CommandError::UndoFailed(format!(
                "page index out of bounds during undo: {}",
                self.page_index
            )));
        }
        let prev =
            self.prev_rotation.lock().unwrap().ok_or_else(|| {
                CommandError::UndoFailed("undo called before execute".to_string())
            })?;
        doc.pages[self.page_index].rotation = prev;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandStack;
    use crate::commands::test_utils::make_doc;

    #[test]
    fn rotate_90_forward() {
        let mut doc = make_doc(1, &[0]);
        let cmd = RotatePageCommand::new(0, 90);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 90);
    }

    #[test]
    fn rotate_180() {
        let mut doc = make_doc(1, &[90]);
        let cmd = RotatePageCommand::new(0, 180);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 270);
    }

    #[test]
    fn rotate_wraps_at_360() {
        let mut doc = make_doc(1, &[270]);
        let cmd = RotatePageCommand::new(0, 90);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 0);
    }

    #[test]
    fn rotate_negative_degrees() {
        let mut doc = make_doc(1, &[0]);
        let cmd = RotatePageCommand::new(0, -90);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 270);
    }

    #[test]
    fn undo_restores_original() {
        let mut doc = make_doc(1, &[90]);
        let cmd = RotatePageCommand::new(0, 90);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 180);
        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 90);
    }

    #[test]
    fn execute_undo_redo_via_stack() {
        let mut doc = make_doc(1, &[0]);
        let mut stack = CommandStack::new(10);

        stack
            .execute(Box::new(RotatePageCommand::new(0, 90)), &mut doc)
            .unwrap();
        assert_eq!(doc.pages[0].rotation, 90);

        stack.undo(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 0);

        stack.redo(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 90);
    }

    #[test]
    fn invalid_degrees_not_multiple_of_90() {
        let mut doc = make_doc(1, &[0]);
        let cmd = RotatePageCommand::new(0, 45);
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
    }

    #[test]
    fn page_index_out_of_bounds() {
        let mut doc = make_doc(1, &[0]);
        let cmd = RotatePageCommand::new(5, 90);
        let err = cmd.execute(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
    }

    #[test]
    fn zero_degrees_is_noop() {
        let mut doc = make_doc(1, &[180]);
        let cmd = RotatePageCommand::new(0, 0);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 180);
    }

    #[test]
    fn rotate_720_is_noop() {
        let mut doc = make_doc(1, &[270]);
        let cmd = RotatePageCommand::new(0, 720);
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.pages[0].rotation, 270);
    }

    #[test]
    fn undo_before_execute_fails() {
        let mut doc = make_doc(1, &[0]);
        let cmd = RotatePageCommand::new(0, 90);
        let err = cmd.undo(&mut doc).unwrap_err();
        assert!(matches!(err, CommandError::UndoFailed(_)));
        if let CommandError::UndoFailed(msg) = err {
            assert!(msg.contains("undo called before execute"));
        }
    }
}
