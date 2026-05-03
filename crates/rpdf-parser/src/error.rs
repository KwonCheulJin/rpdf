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

    /// xref 스트림 형식(PDF 1.5+) 감지 시 trailer 파싱 경로에서 반환.
    ///
    /// Task #5 이후 `parse_xref_chain`은 xref 스트림을 직접 처리하므로
    /// 이 변형은 `parse_trailer`(역방향 탐색 경로)에서만 발생한다.
    #[error("xref 스트림 형식 감지 (오프셋 {xref_offset}): trailer 역방향 파싱 경로")]
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

    // ── 객체 스트림 전용 (Task #6) ────────────────────────────────────────────
    /// `/Type /ObjStm` 딕셔너리가 손상됨 (필수 키 누락 또는 잘못된 값).
    #[error("객체 스트림 구조 오류 (오프셋 {offset}): {reason}")]
    MalformedObjStm { offset: u64, reason: String },

    /// `/Extends` 키가 발견됨 — 상속 ObjStm은 v0.1 범위 외.
    #[error("객체 스트림 /Extends 미지원 (오프셋 {offset})")]
    ObjStmExtendsUnsupported { offset: u64 },

    /// `/Filter`가 FlateDecode 외 필터를 지정함.
    #[error("지원하지 않는 ObjStm 필터 (오프셋 {offset}): {filter:?}")]
    InvalidObjStmFilter { offset: u64, filter: String },

    /// 헤더의 `obj_num`이 XrefTable의 키와 불일치.
    ///
    /// **현재 정책**: 발생시키지 않음 — xref 우선 + `tracing::warn` 로그.
    /// 향후 strict 모드 옵션 도입 시 활용 예약.
    #[error("객체 스트림 헤더 번호 불일치: 헤더={header_num}, xref={xref_num}")]
    ObjStmObjNumMismatch { header_num: u32, xref_num: u32 },

    // ── xref 스트림 전용 (Task #5) ────────────────────────────────────────────
    /// xref 스트림 간접 객체가 스트림이 아니거나 딕셔너리 구조가 손상됨.
    /// 발생: parse_xref_stream_dict — /Type /XRef 불일치, stream 키워드 없음 등.
    #[error("xref 스트림 구조 오류 (오프셋 {offset}): {reason}")]
    MalformedXrefStream { offset: u64, reason: String },

    /// xref 스트림 `/W` 배열이 없거나 3개의 비음수 정수로 구성되지 않음.
    /// 발생: parse_xref_stream_dict — /W 누락 또는 원소 수 != 3.
    #[error("xref 스트림 /W 배열 오류 (오프셋 {offset}): {reason}")]
    XrefStreamInvalidW { offset: u64, reason: String },

    /// xref 스트림 `/Index` 배열이 홀수 원소 개수이거나 비정수를 포함함.
    /// 발생: parse_xref_stream_dict — /Index 원소 수가 홀수.
    #[error("xref 스트림 /Index 배열 오류 (오프셋 {offset}): {reason}")]
    XrefStreamInvalidIndex { offset: u64, reason: String },

    /// xref 스트림 `/Filter`가 FlateDecode 이외의 필터를 지정함.
    /// 발생: parse_xref_stream_dict — filter == "LZWDecode" 등.
    #[error("지원하지 않는 xref 스트림 필터 (오프셋 {offset}): {filter:?}")]
    InvalidXrefStreamFilter { offset: u64, filter: String },

    /// `/DecodeParms /Predictor` 값이 지원 범위(1, 10–15) 밖.
    /// 발생: unpredict_png — Predictor=2(TIFF) 또는 알 수 없는 값.
    #[error("지원하지 않는 Predictor 값 (오프셋 {offset}): {value}")]
    UnsupportedPredictor { offset: u64, value: u8 },

    /// zlib/FlateDecode 압축 해제 실패.
    /// 발생: decompress_flate — 손상된 zlib 스트림.
    #[error("xref 스트림 압축 해제 실패 (오프셋 {offset}): {reason}")]
    XrefStreamDecompressError { offset: u64, reason: String },

    /// `/W` 배열 크기 합계(W1+W2+W3)와 스트림 데이터 길이가 맞지 않음.
    /// 발생: decode_entries — 행 경계에서 데이터 잘림.
    #[error(
        "xref 스트림 W 필드 크기 불일치 (오프셋 {offset}): \
         행 크기 {w_total}, 데이터 잔여 {data_len}"
    )]
    XrefStreamWFieldMismatch {
        offset: u64,
        w_total: usize,
        data_len: usize,
    },

    /// xref 스트림 실제 엔트리 수가 `/Index`가 선언한 수와 다름.
    /// 발생: decode_entries — 디코딩 후 엔트리 수 검증.
    #[error(
        "xref 스트림 엔트리 수 불일치 (오프셋 {offset}): \
         /Index 선언 {declared}, 실제 {actual}"
    )]
    XrefStreamEntryCountMismatch {
        offset: u64,
        declared: usize,
        actual: usize,
    },

    /// content stream 구조 오류 (피연산자/연산자 불일치, 예상치 못한 EOF 등).
    /// 발생: parse_content_stream — 토큰 파싱 실패.
    #[error("content stream 파싱 오류 (오프셋 {offset}): {reason}")]
    MalformedContentStream { offset: usize, reason: String },

    /// 인라인 이미지 (BI...ID...EI) 파싱 오류.
    /// 발생: parse_inline_image — ID/EI 키워드 누락 또는 EOF.
    #[error("인라인 이미지 파싱 오류 (오프셋 {offset}): {reason}")]
    MalformedInlineImage { offset: usize, reason: String },

    /// q/Q 상태 스택 불균형.
    ///
    /// - Q 만났을 때 depth == 0 → depth = -1, offset = 해당 Q 위치
    /// - 파싱 완료 후 depth > 0 → offset = data.len()
    ///
    /// 발생: parse_content_stream — q/Q 깊이 추적.
    #[error("graphics state 스택 불균형 (오프셋 {offset}): depth = {depth}")]
    UnbalancedGraphicsState { offset: usize, depth: i32 },
}
