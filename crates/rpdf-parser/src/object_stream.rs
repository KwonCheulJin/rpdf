//! PDF 1.5+ 객체 스트림(`/Type /ObjStm`) 파싱.
//!
//! ObjStm은 여러 간접 객체를 하나의 압축 스트림 안에 묶어 저장한다(ISO 32000 §7.5.7).
//! `parse_object_stream`은 스트림을 디코딩해 `ParsedObjectStream`으로 반환하고,
//! `ParsedObjectStream::get`은 객체 번호로 개별 객체를 조회한다.

use rpdf_core::types::PdfObject;

use crate::ParseError;

/// ObjStm 파싱 결과. 객체 번호 → `PdfObject` 매핑.
///
/// `objects`는 스트림 헤더에 선언된 순서대로 `(obj_num, PdfObject)` 쌍을 보관한다.
#[derive(Debug, Clone)]
pub struct ParsedObjectStream {
    /// ObjStm이 포함하는 객체 목록 `(obj_num, object)`.
    pub objects: Vec<(u32, PdfObject)>,
}

impl ParsedObjectStream {
    /// `obj_num`에 해당하는 `PdfObject`를 반환한다.
    ///
    /// 존재하지 않으면 `None`. `XrefTable::get()`과 일관된 시그니처.
    ///
    /// **ObjStmObjNumMismatch 정책**: xref 번호와 헤더 번호가 다를 때 xref 우선 +
    /// `tracing::warn` 경고. `ObjStmObjNumMismatch` 에러 변형은 미발생이며
    /// 향후 strict 모드 옵션 도입 시 활용 예약.
    pub fn get(&self, obj_num: u32) -> Option<&PdfObject> {
        self.objects
            .iter()
            .find(|(num, _)| *num == obj_num)
            .map(|(_, obj)| obj)
    }
}

/// ObjStm 간접 객체를 파싱해 객체 목록을 반환한다.
///
/// `offset`은 xref table에서 읽은 ObjStm 객체의 파일 오프셋.
/// 반환된 `ParsedObjectStream.objects`는 `(obj_num, PdfObject)` 쌍 벡터.
#[allow(dead_code)] // Checkpoint B에서 연결됨
pub(crate) fn parse_object_stream(
    _data: &[u8],
    _offset: u64,
) -> Result<ParsedObjectStream, ParseError> {
    todo!("Checkpoint B: ObjStm 딕셔너리 파싱 + 헤더 추출")
}
