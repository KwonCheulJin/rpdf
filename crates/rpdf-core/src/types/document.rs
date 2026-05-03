use crate::types::{ContentStreamOperation, PdfDict};

/// PDF 문서 최상위 구조. `load_document`의 출력.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Document {
    pub pages: Vec<Page>,
    pub metadata: Option<DocumentMetadata>,
}

impl Document {
    /// 페이지 순서 보장된 슬라이스를 반환한다.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// 페이지 수를 반환한다.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// 문서 메타데이터를 반환한다. `/Info` 없으면 `None`.
    pub fn metadata(&self) -> Option<&DocumentMetadata> {
        self.metadata.as_ref()
    }
}

/// 페이지 단위 구조. 의미 해석은 v0.2.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Page {
    /// 0-based 페이지 인덱스 (page tree 순회 순서).
    pub index: usize,
    /// pre-parsed content stream 연산자 시퀀스.
    /// `ContentStreamOperation`은 `serde::Serialize`를 구현하지 않으므로 직렬화 제외.
    #[serde(skip)]
    pub content: Vec<ContentStreamOperation>,
    /// /Resources (상속 포함). None이면 빈 리소스.
    /// `PdfDict`는 `serde::Serialize`를 구현하지 않으므로 직렬화 제외.
    #[serde(skip)]
    pub resources: Option<PdfDict>,
    /// /MediaBox [x0, y0, x1, y1] (상속 포함).
    pub media_box: Option<[f64; 4]>,
    /// /CropBox [x0, y0, x1, y1] (상속 포함).
    pub crop_box: Option<[f64; 4]>,
    /// /Rotate (상속 포함, 기본값 0). 유효값: 0, 90, 180, 270.
    pub rotation: i32,
}

impl Page {
    /// pre-parsed content stream 연산자 시퀀스를 반환한다.
    pub fn content(&self) -> &[ContentStreamOperation] {
        &self.content
    }

    /// `/Resources` 딕셔너리를 반환한다. 상속 포함, 없으면 `None`.
    pub fn resources(&self) -> Option<&PdfDict> {
        self.resources.as_ref()
    }

    /// `/MediaBox [x0, y0, x1, y1]`를 반환한다. 상속 포함, 없으면 `None`.
    pub fn media_box(&self) -> Option<[f64; 4]> {
        self.media_box
    }

    /// `/CropBox [x0, y0, x1, y1]`를 반환한다. 상속 포함, 없으면 `None`.
    pub fn crop_box(&self) -> Option<[f64; 4]> {
        self.crop_box
    }

    /// `/Rotate`를 반환한다. 상속 포함, 기본값 0.
    pub fn rotation(&self) -> i32 {
        self.rotation
    }
}

/// /Info 딕셔너리에서 추출한 메타데이터. 모든 필드 Optional.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DocumentMetadata {
    pub title: Option<Vec<u8>>,
    pub author: Option<Vec<u8>>,
    pub subject: Option<Vec<u8>>,
    pub creator: Option<Vec<u8>>,
    pub producer: Option<Vec<u8>>,
    pub creation_date: Option<Vec<u8>>,
    pub modification_date: Option<Vec<u8>>,
}
