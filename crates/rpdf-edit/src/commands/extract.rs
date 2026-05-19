use rpdf_core::types::document::Document;

use crate::commands::{CommandError, Query, reindex_pages};

/// 단일 페이지 범위를 추출해 새 Document를 반환하는 쿼리.
///
/// 1-based 시작/끝 페이지 번호로 범위를 지정한다.
/// 원본 Document는 변경하지 않는다.
///
/// # Examples
///
/// ```rust
/// use rpdf_edit::commands::{ExtractPagesCommand, Query};
/// use rpdf_core::types::document::{Document, Page};
///
/// let pages = (0..5)
///     .map(|i| Page {
///         index: i,
///         content: vec![],
///         resources: None,
///         media_box: None,
///         crop_box: None,
///         rotation: 0,
///     })
///     .collect();
/// let doc = Document { pages, metadata: None };
///
/// // 1-based 페이지 번호: 2~4번 페이지 추출 → 3페이지 Document
/// let cmd = ExtractPagesCommand::new(2, 4).unwrap();
/// let result = cmd.execute(&doc).unwrap();
/// assert_eq!(result.pages.len(), 3);
/// ```
#[derive(Debug)]
pub struct ExtractPagesCommand {
    start: usize, // 0-based, inclusive
    end: usize,   // 0-based, inclusive
}

impl ExtractPagesCommand {
    /// 1-based 시작/끝 페이지 번호로 `ExtractPagesCommand`를 생성한다.
    ///
    /// # Arguments
    /// * `start_page` - 추출 시작 페이지 (1-based, inclusive)
    /// * `end_page` - 추출 끝 페이지 (1-based, inclusive)
    ///
    /// # Errors
    ///
    /// - `start_page == 0` 또는 `end_page == 0`: `ExecutionFailed("page numbers are 1-based, got 0")`
    /// - `start_page > end_page`: `ExecutionFailed("invalid range N-M: start > end")`
    pub fn new(start_page: usize, end_page: usize) -> Result<Self, CommandError> {
        if start_page == 0 || end_page == 0 {
            return Err(CommandError::ExecutionFailed(
                "page numbers are 1-based, got 0".to_string(),
            ));
        }
        if start_page > end_page {
            return Err(CommandError::ExecutionFailed(format!(
                "invalid range {start_page}-{end_page}: start > end"
            )));
        }
        Ok(Self {
            start: start_page - 1,
            end: end_page - 1,
        })
    }
}

impl Query for ExtractPagesCommand {
    type Output = Document;

    fn execute(&self, doc: &Document) -> Result<Document, CommandError> {
        if doc.pages.is_empty() {
            return Err(CommandError::ExecutionFailed(
                "document has no pages".to_string(),
            ));
        }
        if self.end >= doc.pages.len() {
            return Err(CommandError::ExecutionFailed(format!(
                "page index out of bounds: {}",
                self.end
            )));
        }
        let mut pages = doc.pages[self.start..=self.end].to_vec();
        reindex_pages(&mut pages);
        Ok(Document {
            pages,
            metadata: doc.metadata.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use rpdf_core::types::document::DocumentMetadata;

    use super::super::test_utils::make_doc;
    use super::*;

    #[test]
    fn extract_basic_range() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(2, 4).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages.len(), 3);
    }

    #[test]
    fn extract_single_page() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(3, 3).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages.len(), 1);
    }

    #[test]
    fn extract_entire_document() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(1, 5).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages.len(), 5);
    }

    #[test]
    fn extract_preserves_page_content() {
        let doc = make_doc(3, &[90, 180, 270]);
        let cmd = ExtractPagesCommand::new(1, 3).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages[0].rotation, 90);
        assert_eq!(result.pages[1].rotation, 180);
        assert_eq!(result.pages[2].rotation, 270);
    }

    #[test]
    fn extract_page_indices_reindexed() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(2, 4).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages[0].index, 0);
        assert_eq!(result.pages[1].index, 1);
        assert_eq!(result.pages[2].index, 2);
    }

    #[test]
    fn extract_metadata_copied() {
        let mut doc = make_doc(3, &[]);
        doc.metadata = Some(DocumentMetadata {
            title: Some(b"Test".to_vec()),
            ..Default::default()
        });
        let cmd = ExtractPagesCommand::new(1, 2).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert!(result.metadata.is_some());
        let meta = result.metadata.as_ref().unwrap();
        assert_eq!(meta.title, Some(b"Test".to_vec()));
    }

    #[test]
    fn extract_out_of_bounds_end() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(1, 10).unwrap();
        let err = cmd.execute(&doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("page index out of bounds"));
        }
    }

    #[test]
    fn extract_start_out_of_bounds() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(8, 10).unwrap();
        let err = cmd.execute(&doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
    }

    #[test]
    fn extract_zero_start_page() {
        let err = ExtractPagesCommand::new(0, 3).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("1-based"));
        }
    }

    #[test]
    fn extract_zero_end_page() {
        let err = ExtractPagesCommand::new(1, 0).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("1-based"));
        }
    }

    #[test]
    fn extract_start_greater_than_end() {
        let err = ExtractPagesCommand::new(4, 2).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("start > end"));
        }
    }

    #[test]
    fn extract_on_empty_document() {
        let doc = make_doc(0, &[]);
        let cmd = ExtractPagesCommand::new(1, 1).unwrap();
        let err = cmd.execute(&doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("document has no pages"));
        }
    }

    #[test]
    fn extract_original_doc_unchanged() {
        let doc = make_doc(5, &[]);
        let original_len = doc.pages.len();
        let cmd = ExtractPagesCommand::new(1, 3).unwrap();
        cmd.execute(&doc).unwrap();
        assert_eq!(doc.pages.len(), original_len);
    }

    #[test]
    fn extract_last_page_boundary() {
        let doc = make_doc(5, &[]);
        let cmd = ExtractPagesCommand::new(5, 5).unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].index, 0);
    }
}
