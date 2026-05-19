mod delete;
mod error;
mod extract;
mod merge;
mod rotate;
mod split;
mod stack;
mod traits;

pub use delete::DeletePagesCommand;
pub use error::CommandError;
pub use extract::ExtractPagesCommand;
pub use merge::MergeCommand;
pub use rotate::RotatePageCommand;
pub use split::SplitCommand;
pub use stack::CommandStack;
pub use traits::{Command, Query};

use rpdf_core::types::document::Page;

/// 페이지 목록의 index 필드를 현재 위치(0-based)에 맞게 재정렬한다.
pub(crate) fn reindex_pages(pages: &mut [Page]) {
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i;
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use rpdf_core::types::document::{Document, Page};

    pub fn make_doc(pages: usize, rotations: &[i32]) -> Document {
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
}
