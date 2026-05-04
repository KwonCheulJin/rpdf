use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use rpdf_render::render_page;

/// `rpdf render` 실행 파라미터.
pub struct RenderParams {
    /// 렌더링할 PDF 파일 경로.
    pub file: PathBuf,
    /// 출력 PNG 파일 경로. `None`이면 `{pdf_stem}_p{page}.png` (현재 디렉터리).
    pub output: Option<PathBuf>,
    /// 0-based 페이지 인덱스.
    pub page: u16,
    /// 해상도 배율 (1.0 = 72 DPI 기준).
    pub scale: f32,
}

/// `rpdf render` 서브커맨드를 실행한다.
///
/// `PDFIUM_DYNAMIC_LIB_PATH` 환경변수가 설정되어야 한다.
pub fn run(params: RenderParams) -> Result<()> {
    let lib_path = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
        .context("PDFIUM_DYNAMIC_LIB_PATH 환경변수가 설정되지 않았습니다. scripts/fetch-pdfium.sh를 실행하고 환경변수를 설정하세요.")?;

    let output_path = match params.output {
        Some(p) => p,
        None => {
            let stem = params
                .file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            PathBuf::from(format!("{}_p{}.png", stem, params.page))
        }
    };

    if params.scale <= 0.0 {
        bail!("scale은 0보다 커야 합니다: {}", params.scale);
    }

    let image = render_page(
        &std::path::PathBuf::from(&lib_path),
        &params.file,
        params.page,
        params.scale,
    )
    .with_context(|| format!("렌더링 실패: {}", params.file.display()))?;

    image
        .save(&output_path)
        .with_context(|| format!("PNG 저장 실패: {}", output_path.display()))?;

    println!("{}", output_path.display());

    Ok(())
}
