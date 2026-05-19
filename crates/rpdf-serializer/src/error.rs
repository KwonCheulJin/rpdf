/// `serialize_document` 호출 시 발생 가능한 에러.
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    /// pages가 비어있음.
    #[error("document has no pages")]
    EmptyDocument,

    /// sources와 pages 개수 불일치.
    #[error("sources length {sources} != pages length {pages}")]
    SourceLengthMismatch { sources: usize, pages: usize },

    /// lopdf가 source_bytes 로드 실패 (이중 파서 비호환).
    #[error("failed to load source PDF (lopdf incompatible): {0}")]
    LoadSource(lopdf::Error),

    /// source.page_index가 원본 페이지 수 초과.
    #[error("source_page_index {idx} out of bounds (source has {count} pages)")]
    PageOutOfBounds { idx: usize, count: usize },

    /// lopdf save_to 실패.
    #[error("lopdf save failed: {0}")]
    Save(#[from] std::io::Error),
}
