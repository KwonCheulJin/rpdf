use crate::commands::error::CommandError;
use rpdf_core::types::document::Document;

/// 문서를 변경하는 커맨드 트레이트.
///
/// 각 커맨드는 `execute`로 변경을 적용하고, `undo`로 이전 상태를 복원한다.
/// `CommandStack`과 함께 사용하여 undo/redo 기능을 제공한다.
pub trait Command: Send + Sync {
    /// 커맨드를 실행해 `doc`를 변경한다.
    fn execute(&self, doc: &mut Document) -> Result<(), CommandError>;

    /// 커맨드 실행 이전 상태로 `doc`를 복원한다.
    fn undo(&self, doc: &mut Document) -> Result<(), CommandError>;

    /// 커맨드의 이름을 반환한다. 로깅·디버깅 용도.
    fn name(&self) -> &'static str;
}

/// 문서를 읽는 쿼리 트레이트.
///
/// 문서를 변경하지 않고 정보를 추출한다.
pub trait Query {
    /// 쿼리 결과 타입.
    type Output;

    /// `doc`에서 정보를 읽어 결과를 반환한다.
    fn execute(&self, doc: &Document) -> Result<Self::Output, CommandError>;
}
