use std::sync::Arc;

/// 단일 페이지의 원본 PDF 출처를 기록하는 직렬화 힌트.
///
/// `load_document_tracked`가 반환하는 `Vec<PageSource>`는
/// `Document.pages`와 1:1 대응한다: `sources[i]` → `pages[i]`.
///
/// 커맨드 실행 후 호출자가 `sources`를 `doc.pages`와 동기화할 책임을 진다.
/// - `RotatePageCommand`: 변경 없음 (page 순서 동일)
/// - `DeletePagesCommand(indices: [1, 3])`: `[s0, s2, s4]` — 삭제 인덱스 제거
/// - `ExtractPagesCommand(1-5)`: `sources[0..=4]` — 범위 슬라이스
/// - `SplitCommand`: 각 결과 doc의 page 원본 인덱스로 sources 슬라이스
/// - `MergeCommand`: `a_sources + b_sources + ...` 순서로 연결
#[derive(Clone)]
pub struct PageSource {
    /// 원본 PDF 바이트. 동일한 파일에서 로드된 여러 PageSource는 같은 Arc를 공유한다.
    pub bytes: Arc<Vec<u8>>,
    /// 원본 PDF에서의 0-based 페이지 인덱스.
    pub page_index: usize,
}
