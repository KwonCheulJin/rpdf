//! rpdf-render — PDF 렌더링 레이어
//!
//! pdfium-render를 통해 PDF 페이지를 이미지로 렌더링한다.

use std::path::Path;

use image::DynamicImage;
use pdfium_render::prelude::*;

/// 렌더링 과정에서 발생할 수 있는 에러.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("pdfium 라이브러리 로딩 실패: {0}")]
    LibraryLoad(String),
    #[error("PDF 파일 열기 실패: {0}")]
    FileOpen(String),
    #[error("페이지 {0} 접근 실패")]
    PageAccess(u16),
    #[error("렌더링 실패: {0}")]
    Render(String),
    #[error("scale은 0보다 커야 합니다: {0}")]
    InvalidScale(f32),
}

/// PDF 단일 페이지를 PNG DynamicImage로 렌더링한다.
///
/// - `lib_path`: pdfium 동적 라이브러리가 있는 디렉터리 경로 (`PDFIUM_DYNAMIC_LIB_PATH`)
/// - `pdf_path`: 렌더링할 PDF 파일 경로
/// - `page_index`: 0-based 페이지 인덱스
/// - `scale`: 해상도 배율 (1.0 = 72 DPI 기준, 2.0 = ~144 DPI)
pub fn render_page(
    lib_path: &Path,
    pdf_path: &Path,
    page_index: u16,
    scale: f32,
) -> Result<DynamicImage, RenderError> {
    if scale <= 0.0 {
        return Err(RenderError::InvalidScale(scale));
    }

    let pdfium = load_pdfium(lib_path)?;

    let doc = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| RenderError::FileOpen(e.to_string()))?;

    let page = doc
        .pages()
        .get(page_index as PdfPageIndex)
        .map_err(|_| RenderError::PageAccess(page_index))?;

    let width = (page.width().value * scale) as Pixels;
    let height = (page.height().value * scale) as Pixels;

    let config = PdfRenderConfig::new()
        .set_target_width(width)
        .set_target_height(height);

    let image = page
        .render_with_config(&config)
        .map_err(|e| RenderError::Render(e.to_string()))?
        .as_image()
        .map_err(|e| RenderError::Render(e.to_string()))?;

    Ok(image)
}

/// pdfium 동적 라이브러리를 로딩하여 `Pdfium` 인스턴스를 반환한다.
///
/// `Pdfium`은 내부적으로 전역 `OnceCell`을 사용한다. 라이브러리가 이미 초기화된 경우
/// `Pdfium::default()`로 기존 바인딩을 재사용한다.
fn load_pdfium(lib_path: &Path) -> Result<Pdfium, RenderError> {
    match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(lib_path)) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => {
            // 이미 초기화된 경우: 기존 전역 바인딩이 있으므로 Pdfium::default()로 재사용한다.
            // Pdfium::default()는 내부적으로 이미 초기화된 경우를 감지하고 안전하게 반환한다.
            Ok(Pdfium::default())
        }
        Err(e) => Err(RenderError::LibraryLoad(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use pdfium_render::prelude::*;

    /// pdfium 동적 라이브러리가 런타임에 정상 로딩되는지 검증한다.
    ///
    /// `PDFIUM_DYNAMIC_LIB_PATH` 환경변수에 libpdfium이 있는 디렉터리 경로를 설정해야 한다.
    /// 예: `export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib`
    #[test]
    #[ignore = "requires PDFIUM_DYNAMIC_LIB_PATH — run scripts/fetch-pdfium.sh first"]
    fn pdfium_dynamic_links() {
        let lib_path = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
            .expect("PDFIUM_DYNAMIC_LIB_PATH not set — run scripts/fetch-pdfium.sh first");
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path))
            .expect("pdfium dynamic link failed");
    }
}
