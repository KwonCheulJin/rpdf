/// CQRS 커맨드 실행 중 발생할 수 있는 오류.
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    /// 커맨드 실행 중 오류. 실패 원인 메시지를 포함한다.
    #[error("실행 중 오류: {0}")]
    ExecutionFailed(String),
    /// Undo 중 오류. 실패 원인 메시지를 포함한다.
    #[error("Undo 중 오류: {0}")]
    UndoFailed(String),
    /// `undo_stack`이 비어 있을 때 undo를 호출한 경우.
    #[error("Undo할 커맨드 없음")]
    NothingToUndo,
    /// `redo_stack`이 비어 있을 때 redo를 호출한 경우.
    #[error("Redo할 커맨드 없음")]
    NothingToRedo,
}
