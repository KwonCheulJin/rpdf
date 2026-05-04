//! rpdf-render — PDF 렌더링 레이어
//!
//! pdfium-render를 통해 PDF 페이지를 이미지로 렌더링한다.
//! 도메인 타입은 rpdf-core에서 가져온다.
//!
//! # 현재 상태
//!
//! Task #11: pdfium 환경 구축 완료.
//! Task #12에서 PNG 출력 기능을 추가한다.

pub use pdfium_render::prelude::Pdfium;

#[cfg(test)]
mod tests {
    use pdfium_render::prelude::*;

    /// pdfium 동적 라이브러리가 런타임에 정상 로딩되는지 검증한다.
    ///
    /// `PDFIUM_DYNAMIC_LIB_PATH` 환경변수에 libpdfium이 있는 디렉터리 경로를 설정해야 한다.
    /// 예: `export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib`
    #[test]
    fn pdfium_dynamic_links() {
        let lib_path = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
            .expect("PDFIUM_DYNAMIC_LIB_PATH not set — run scripts/fetch-pdfium.sh first");
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib_path))
            .expect("pdfium dynamic link failed");
    }
}
