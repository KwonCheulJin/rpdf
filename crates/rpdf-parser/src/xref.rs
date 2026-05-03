use rpdf_core::types::{XrefEntry, XrefTable};

use crate::error::ParseError;
use crate::trailer::PdfTrailer;

/// `parse_xref` 반환값: 병합된 xref 테이블, 권위 있는 trailer, 섹션 메타데이터.
#[derive(Debug, Clone)]
pub struct ParsedXref {
    /// 모든 incremental update 섹션을 병합한 xref 테이블.
    /// 최신 섹션의 엔트리가 우선한다 (`insert_if_absent`).
    pub table: XrefTable,
    /// 가장 최신 섹션의 trailer (/Root, /Info 등의 권위 있는 소스).
    pub trailer: PdfTrailer,
    /// 순회한 각 섹션의 위치와 엔트리 수 (디버그용).
    pub sections: Vec<XrefSectionInfo>,
}

/// 단일 xref 섹션의 메타데이터.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefSectionInfo {
    pub offset: u64,
    pub entry_count: usize,
}

/// PDF 파일에서 xref 테이블과 trailer를 파싱한다.
///
/// `xref_offset`은 `parse_startxref`가 반환한 값을 그대로 전달한다.
/// `/Prev` 포인터를 따라 incremental update chain 전체를 순회하며
/// `XrefTable`에 병합한다.
///
/// # 에러
///
/// - `XrefOffsetOutOfBounds` — `xref_offset >= data.len()`
/// - `XrefStreamUnsupported` — xref 스트림 형식(PDF 1.5+) 감지
/// - `InvalidXrefAtOffset` — 지정 오프셋에 `xref` 키워드 없음
/// - `MalformedXref` — 항목 형식 오류
/// - `XrefChainCycle` — `/Prev` chain에 순환 참조
/// - `XrefChainTooDeep` — chain 깊이 초과
pub fn parse_xref(data: &[u8], xref_offset: u64) -> Result<ParsedXref, ParseError> {
    let _ = (data, xref_offset, XrefEntry::Free { next_free_obj_num: 0, generation: 0 });
    todo!("Checkpoint B에서 구현")
}
