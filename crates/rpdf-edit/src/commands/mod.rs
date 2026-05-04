mod delete;
mod error;
mod merge;
mod rotate;
mod stack;
mod traits;

pub use delete::DeletePagesCommand;
pub use error::CommandError;
pub use merge::MergeCommand;
pub use rotate::RotatePageCommand;
pub use stack::CommandStack;
pub use traits::{Command, Query};

use rpdf_core::types::document::Page;

/// 페이지 목록의 index 필드를 현재 위치(0-based)에 맞게 재정렬한다.
pub(crate) fn reindex_pages(pages: &mut [Page]) {
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i;
    }
}
