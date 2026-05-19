use rpdf_core::types::document::Document;

use crate::commands::{CommandError, Query, reindex_pages};

/// 페이지 범위를 나타내는 내부 구조체 (0-based, inclusive).
#[derive(Debug)]
struct PageRange {
    start: usize,
    end: usize,
}

/// 지정된 페이지 범위에 따라 Document를 여러 Document로 분리하는 쿼리.
///
/// 1-based 페이지 번호 명세를 파싱해 범위별로 Document를 반환한다.
/// 원본 Document는 변경하지 않는다.
///
/// # Examples
///
/// ```rust
/// use rpdf_edit::commands::{SplitCommand, Query};
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
/// // 1-based 페이지 번호: "1-3,5" → 두 개의 Document (3페이지, 1페이지)
/// let cmd = SplitCommand::new("1-3,5").unwrap();
/// let result = cmd.execute(&doc).unwrap();
/// assert_eq!(result.len(), 2);
/// assert_eq!(result[0].pages.len(), 3);
/// assert_eq!(result[1].pages.len(), 1);
/// ```
#[derive(Debug)]
pub struct SplitCommand {
    ranges: Vec<PageRange>,
}

impl SplitCommand {
    /// 페이지 범위 명세 문자열로 `SplitCommand`를 생성한다.
    ///
    /// # Arguments
    /// * `spec` - 1-based 페이지 번호 명세. 쉼표로 구분된 범위 목록.
    ///   - 단일 페이지: `"3"` → 3번째 페이지만
    ///   - 범위: `"1-3"` → 1~3번째 페이지
    ///   - 복합: `"1-3,5"` → 1~3번째 + 5번째 페이지
    ///   - 공백 허용: `"1-3, 5"` → 정상 파싱
    ///
    /// # Errors
    ///
    /// - 빈 spec: `ExecutionFailed("range spec must not be empty")`
    /// - 숫자 파싱 실패: `ExecutionFailed("invalid range spec: {token}")`
    /// - 0 포함 (1-based 위반): `ExecutionFailed("page numbers are 1-based, got 0")`
    /// - start > end: `ExecutionFailed("invalid range {N}-{M}: start > end")`
    pub fn new(spec: &str) -> Result<Self, CommandError> {
        if spec.is_empty() {
            return Err(CommandError::ExecutionFailed(
                "range spec must not be empty".to_string(),
            ));
        }

        let ranges = spec
            .split(',')
            .map(str::trim)
            .map(parse_range_token)
            .collect::<Result<Vec<PageRange>, CommandError>>()?;

        Ok(Self { ranges })
    }
}

fn parse_range_token(token: &str) -> Result<PageRange, CommandError> {
    if token.contains('-') {
        let parts: Vec<&str> = token.splitn(2, '-').collect();
        let n: u64 = parts[0]
            .parse()
            .map_err(|_| CommandError::ExecutionFailed(format!("invalid range spec: {token}")))?;
        let m: u64 = parts[1]
            .parse()
            .map_err(|_| CommandError::ExecutionFailed(format!("invalid range spec: {token}")))?;

        if n == 0 || m == 0 {
            return Err(CommandError::ExecutionFailed(
                "page numbers are 1-based, got 0".to_string(),
            ));
        }

        if n > m {
            return Err(CommandError::ExecutionFailed(format!(
                "invalid range {n}-{m}: start > end"
            )));
        }

        Ok(PageRange {
            start: (n - 1) as usize,
            end: (m - 1) as usize,
        })
    } else {
        let n: u64 = token
            .parse()
            .map_err(|_| CommandError::ExecutionFailed(format!("invalid range spec: {token}")))?;

        if n == 0 {
            return Err(CommandError::ExecutionFailed(
                "page numbers are 1-based, got 0".to_string(),
            ));
        }

        Ok(PageRange {
            start: (n - 1) as usize,
            end: (n - 1) as usize,
        })
    }
}

impl Query for SplitCommand {
    type Output = Vec<Document>;

    fn execute(&self, doc: &Document) -> Result<Vec<Document>, CommandError> {
        if doc.pages.is_empty() {
            return Err(CommandError::ExecutionFailed(
                "document has no pages".to_string(),
            ));
        }

        if self.ranges.is_empty() {
            return Ok(vec![]);
        }

        // 원자성: 첫 번째 out-of-bounds 범위 발견 시 즉시 에러 반환
        for range in &self.ranges {
            if range.end >= doc.pages.len() {
                return Err(CommandError::ExecutionFailed(format!(
                    "page index out of bounds: {}",
                    range.end
                )));
            }
        }

        let result = self
            .ranges
            .iter()
            .map(|range| {
                let mut pages = doc.pages[range.start..=range.end].to_vec();
                reindex_pages(&mut pages);
                Document {
                    pages,
                    metadata: doc.metadata.clone(),
                }
            })
            .collect();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use rpdf_core::types::document::DocumentMetadata;

    use super::*;
    use crate::commands::test_utils::make_doc;

    #[test]
    fn split_single_range() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("2-4").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pages.len(), 3);
    }

    #[test]
    fn split_multiple_ranges() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("1-2,4-5").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages.len(), 2);
        assert_eq!(result[1].pages.len(), 2);
    }

    #[test]
    fn split_single_page_spec() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("3").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pages.len(), 1);
    }

    #[test]
    fn split_entire_document() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("1-5").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pages.len(), 5);
    }

    #[test]
    fn split_preserves_page_content() {
        let doc = make_doc(3, &[90, 180, 270]);
        let cmd = SplitCommand::new("1-3").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result[0].pages[0].rotation, 90);
        assert_eq!(result[0].pages[1].rotation, 180);
        assert_eq!(result[0].pages[2].rotation, 270);
    }

    #[test]
    fn split_page_indices_reindexed() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("2-4").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result[0].pages[0].index, 0);
        assert_eq!(result[0].pages[1].index, 1);
        assert_eq!(result[0].pages[2].index, 2);
    }

    #[test]
    fn split_metadata_copied() {
        let mut doc = make_doc(3, &[]);
        doc.metadata = Some(DocumentMetadata {
            title: Some(b"Test".to_vec()),
            ..Default::default()
        });
        let cmd = SplitCommand::new("1-2").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert!(result[0].metadata.is_some());
        let meta = result[0].metadata.as_ref().unwrap();
        assert_eq!(meta.title, Some(b"Test".to_vec()));
    }

    #[test]
    fn split_out_of_bounds() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("1-10").unwrap();
        let err = cmd.execute(&doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("page index out of bounds"));
        }
    }

    #[test]
    fn split_invalid_spec_letters() {
        let err = SplitCommand::new("abc").unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
    }

    #[test]
    fn split_invalid_spec_empty() {
        let err = SplitCommand::new("").unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("range spec must not be empty"));
        }
    }

    #[test]
    fn split_range_start_greater_than_end() {
        let err = SplitCommand::new("3-1").unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("start > end"));
        }
    }

    #[test]
    fn split_on_empty_document() {
        let doc = make_doc(0, &[]);
        let cmd = SplitCommand::new("1-3").unwrap();
        let err = cmd.execute(&doc).unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("document has no pages"));
        }
    }

    #[test]
    fn split_range_order_preserved() {
        let doc = make_doc(5, &[90, 180, 270, 0, 0]);
        let cmd = SplitCommand::new("1-3").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result[0].pages[0].rotation, 90);
        assert_eq!(result[0].pages[1].rotation, 180);
        assert_eq!(result[0].pages[2].rotation, 270);
    }

    #[test]
    fn split_original_doc_unchanged() {
        let doc = make_doc(5, &[]);
        let original_len = doc.pages.len();
        let cmd = SplitCommand::new("1-3").unwrap();
        cmd.execute(&doc).unwrap();
        assert_eq!(doc.pages.len(), original_len);
    }

    #[test]
    fn split_zero_page_number() {
        let err = SplitCommand::new("0").unwrap_err();
        assert!(matches!(err, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err {
            assert!(msg.contains("1-based"));
        }

        let err2 = SplitCommand::new("0-3").unwrap_err();
        assert!(matches!(err2, CommandError::ExecutionFailed(_)));
        if let CommandError::ExecutionFailed(msg) = err2 {
            assert!(msg.contains("1-based"));
        }
    }

    #[test]
    fn split_spec_with_spaces() {
        let doc = make_doc(5, &[]);
        let cmd = SplitCommand::new("1-3, 5").unwrap();
        let result = cmd.execute(&doc).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pages.len(), 3);
        assert_eq!(result[1].pages.len(), 1);
    }
}
