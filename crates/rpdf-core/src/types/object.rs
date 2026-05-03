use crate::types::ObjectId;

/// PDF 기본 객체 타입 (ISO 32000-1 §7.3).
///
/// 문자열은 raw bytes로 저장한다. 인코딩 해석(PDFDocEncoding, UTF-16BE)은
/// Task #7 Document IR에서 담당한다.
///
/// 스트림 raw bytes는 필터 적용 전 원본 데이터다. 필터 디코딩은 Task #5.
///
/// `LiteralString`과 `HexString`은 모두 raw bytes를 담지만 출처 형식을 보존한다.
/// 편집 후 저장 시 원본 형식을 유지하고, Task #7 인코딩 해석 분기에 활용된다.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    // 스칼라
    /// `null`
    Null,
    /// `true` 또는 `false`
    Boolean(bool),
    /// 정수 (예: `42`, `-3`, `+10`). ISO 32000 §7.3.3.
    Integer(i64),
    /// 실수 (예: `3.14`, `-.5`, `+1.0`). 지수 표기법 미지원. ISO 32000 §7.3.3.
    Real(f64),

    // 문자열류 (raw bytes — 출처 형식 보존)
    /// 괄호 형식 문자열 `(...)`. 이스케이프 시퀀스 처리 후 raw bytes. ISO 32000 §7.3.4.2.
    LiteralString(Vec<u8>),
    /// hex 형식 문자열 `<...>`. hex 디코딩 후 raw bytes. ISO 32000 §7.3.4.3.
    HexString(Vec<u8>),
    /// 이름 객체 `/Foo`. `/` 제외, `#HH` 이스케이프 처리 후 raw bytes. ISO 32000 §7.3.5.
    Name(Vec<u8>),

    // 컨테이너
    /// 배열 `[...]`. ISO 32000 §7.3.6.
    Array(Vec<PdfObject>),
    /// 딕셔너리 `<<...>>`. ISO 32000 §7.3.7.
    Dictionary(PdfDict),

    // 특수 (구조)
    /// 스트림 객체. 헤더 딕셔너리와 raw bytes(필터 미적용). ISO 32000 §7.3.8.
    Stream(PdfStream),
    /// 간접 참조 `N G R`. 해결(resolve)하지 않은 상태. ISO 32000 §7.3.10.
    Reference(ObjectId),
}

impl PdfObject {
    /// `LiteralString` 또는 `HexString`의 raw bytes를 반환한다.
    /// 다른 변형이면 `None`.
    pub fn as_string_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::LiteralString(bytes) | PdfObject::HexString(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// 문자열 출처 형식을 반환한다. 문자열 변형이 아니면 `None`.
    ///
    /// 저장 시 어느 형식으로 쓸지 결정하거나, Task #7에서 인코딩 분기에 활용한다.
    pub fn string_format(&self) -> Option<StringFormat> {
        match self {
            PdfObject::LiteralString(_) => Some(StringFormat::Literal),
            PdfObject::HexString(_) => Some(StringFormat::Hex),
            _ => None,
        }
    }

    /// `Name` 변형의 raw bytes를 반환한다. 다른 변형이면 `None`.
    pub fn as_name_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::Name(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// `Integer` 변형의 값을 반환한다. 다른 변형이면 `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// `Integer` 변형이 음수가 아니면 `u64`로 반환한다.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            PdfObject::Integer(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    /// `Boolean` 변형의 값을 반환한다. 다른 변형이면 `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// `Dictionary` 변형을 반환한다. 다른 변형이면 `None`.
    pub fn as_dict(&self) -> Option<&PdfDict> {
        match self {
            PdfObject::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    /// `Array` 변형을 반환한다. 다른 변형이면 `None`.
    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// 문자열 출처 형식.
///
/// 편집 후 PDF 저장 시 원본 형식을 유지하거나,
/// Task #7에서 인코딩 해석 분기 기준으로 사용한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringFormat {
    /// 괄호 형식 `(...)`
    Literal,
    /// hex 형식 `<...>`
    Hex,
}

/// PDF 딕셔너리 `<< key value ... >>` (ISO 32000-1 §7.3.7).
///
/// 키는 Name raw bytes(`Vec<u8>`), 값은 `PdfObject`.
/// 삽입 순서를 보존하기 위해 `Vec` 사용.
///
/// 중복 키는 PDF 스펙상 비허용이나 실제 파일에서 발생하므로
/// 스펙 기준 조회(`get`)와 처리기 호환 조회(`get_last`) 두 가지를 제공한다.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfDict(pub Vec<(Vec<u8>, PdfObject)>);

impl PdfDict {
    /// 키와 일치하는 **첫 번째** 항목을 반환한다.
    ///
    /// ISO 32000-1 §7.3.7: "If a key is repeated, the first occurrence shall be used."
    pub fn get(&self, key: &[u8]) -> Option<&PdfObject> {
        self.0
            .iter()
            .find(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
    }

    /// 키와 일치하는 **마지막** 항목을 반환한다.
    ///
    /// 일부 PDF 생성기는 마지막 값을 사용하는 동작을 가정한다.
    pub fn get_last(&self, key: &[u8]) -> Option<&PdfObject> {
        self.0
            .iter()
            .rfind(|(k, _)| k.as_slice() == key)
            .map(|(_, v)| v)
    }

    /// 키-값 쌍의 반복자를 반환한다 (삽입 순서).
    pub fn iter(&self) -> impl Iterator<Item = &(Vec<u8>, PdfObject)> {
        self.0.iter()
    }

    /// 딕셔너리가 비어있으면 `true`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 키-값 쌍의 수를 반환한다 (중복 키 포함).
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// PDF 스트림 객체 `<< ... >> stream ... endstream` (ISO 32000-1 §7.3.8).
///
/// `data`는 필터 적용 전 raw bytes다. 필터 디코딩은 Task #5에서 처리한다.
/// 헤더 딕셔너리의 `/Length` 키 값은 `data.len()`과 일치한다.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    /// 스트림 헤더 딕셔너리. `/Length`, `/Filter` 등 메타데이터 포함.
    pub dict: PdfDict,
    /// 필터 미적용 raw bytes.
    pub data: Vec<u8>,
}

/// 간접 객체 컨테이너 `N G obj ... endobj` (ISO 32000-1 §7.3.10).
///
/// PDF 스펙의 "indirect object"는 파일 내 특정 위치에 ID와 함께 저장된 객체다.
/// `PdfObject`의 한 변형이 아니라 객체에 ID와 세대 번호를 부여하는 **최상위 래퍼**다.
///
/// - `PdfObject::Reference(ObjectId)`: 다른 indirect object를 가리키는 참조값
/// - `IndirectObject`: `N G obj ... endobj` 구조 전체를 파싱한 결과
#[derive(Debug, Clone, PartialEq)]
pub struct IndirectObject {
    /// 객체 번호와 세대 번호.
    pub id: ObjectId,
    /// 실제 객체 값.
    pub object: PdfObject,
}
