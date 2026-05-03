/// PDF 파싱 중 발생할 수 있는 모든 에러.
///
/// 각 변형은 가능한 한 실패 위치(바이트 오프셋)나 실패 원인(found 문자열)을 포함해
/// 디버깅 시 어느 지점에서 무엇이 잘못됐는지 파악할 수 있게 한다.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// 파일 첫 1KB 내에서 `%PDF-` 시그니처를 찾지 못함.
    #[error("PDF 헤더를 찾을 수 없음: 처음 {searched_bytes}바이트 내에 %PDF- 없음")]
    HeaderNotFound { searched_bytes: usize },

    /// `%PDF-` 뒤에 오는 버전 문자열이 `<digit>.<digit>` 형식이 아님.
    #[error("잘못된 PDF 버전 (오프셋 {offset}): {found:?}")]
    InvalidVersion { offset: usize, found: String },

    /// 파일 끝 탐색 버퍼 내에 `%%EOF` 마커가 없음.
    #[error("%%EOF 마커를 찾을 수 없음")]
    MissingEof,

    /// `%%EOF` 앞에서 `startxref` 키워드를 찾지 못함.
    #[error("startxref 키워드를 찾을 수 없음")]
    MissingStartXref,

    /// `startxref` 다음에 오는 값이 정수로 파싱되지 않음.
    #[error("startxref 값이 유효하지 않음: {found:?}")]
    InvalidStartXref { found: String },

    /// `trailer` 키워드를 찾지 못함.
    /// xref 스트림 형식(PDF 1.5+)의 경우 `XrefStreamUnsupported`가 먼저 반환된다.
    #[error("trailer 딕셔너리를 찾을 수 없음")]
    MissingTrailer,

    /// trailer 딕셔너리에서 필수 키가 누락됨.
    #[error("필수 키 누락: {key}")]
    MissingRequiredKey { key: &'static str },

    /// 간접 참조(`<num> <gen> R`) 파싱 실패.
    #[error("잘못된 간접 참조: {found:?}")]
    InvalidObjectRef { found: String },

    /// xref 스트림 형식(PDF 1.5+) 감지됨. Task #5에서 처리 예정.
    #[error("xref 스트림 형식은 지원되지 않음 (오프셋 {xref_offset}): Task #5에서 처리 예정")]
    XrefStreamUnsupported { xref_offset: u64 },

    /// xref 항목의 20바이트 형식이 올바르지 않음.
    #[error("xref 항목 형식 오류 (오프셋 {offset}): {reason}")]
    MalformedXref { offset: u64, reason: String },

    /// `/Prev` chain에서 이미 방문한 오프셋이 다시 등장함 (순환 참조).
    #[error("xref chain 순환 감지 (오프셋 {offset})")]
    XrefChainCycle { offset: u64 },

    /// `/Prev` chain 깊이가 허용 상한(MAX_XREF_CHAIN_DEPTH)을 초과함.
    #[error("xref chain 깊이 초과: {max_depth}")]
    XrefChainTooDeep { max_depth: usize },

    /// `startxref` 값이 파일 크기를 벗어남.
    #[error("xref 오프셋 {offset}이 파일 크기 {file_size}를 벗어남")]
    XrefOffsetOutOfBounds { offset: u64, file_size: u64 },

    /// 지정된 오프셋에 `xref` 키워드가 없음 (다른 데이터 또는 xref 스트림).
    #[error("오프셋 {offset}에 xref 테이블 없음: {found:?}")]
    InvalidXrefAtOffset { offset: u64, found: String },

    /// trailer 딕셔너리 구조가 손상됨 (예: `<<` 닫힘 없음, 값 타입 불일치 등).
    #[error("trailer 딕셔너리 형식 오류: {reason}")]
    MalformedTrailer { reason: String },

    /// trailer 딕셔너리가 탐색 버퍼(기본 4KB)를 초과함.
    #[error("trailer가 탐색 버퍼({limit_kb}KB)를 초과함")]
    TrailerTooLarge { limit_kb: usize },

    /// 배열 또는 딕셔너리 중첩이 허용 깊이(`MAX_OBJECT_DEPTH`)를 초과함.
    #[error("오프셋 {offset}에서 객체 중첩 깊이 초과: {max_depth}")]
    ObjectTooDeep { offset: usize, max_depth: usize },

    /// 객체 파싱 실패 (예: 예상치 못한 토큰, 잘못된 이름 이스케이프).
    #[error("오프셋 {offset}에서 잘못된 객체: {reason:?}")]
    InvalidObject { offset: usize, reason: String },

    /// 스트림 구조 오류 (`stream` 키워드 없음, `endstream` 없음, `/Length` 없음 또는 간접 참조).
    #[error("스트림 구조 오류 (오프셋 {offset}): {reason}")]
    MalformedStream { offset: usize, reason: String },

    /// `endobj` 키워드가 없음.
    #[error("오프셋 {offset}에서 endobj 없음")]
    MissingEndobj { offset: usize },

    /// 위 변형으로 분류되지 않는 형식 오류.
    #[error("예상치 못한 형식: {0}")]
    UnexpectedFormat(String),
}
