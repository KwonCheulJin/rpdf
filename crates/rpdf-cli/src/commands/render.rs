use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// `rpdf render` 실행 파라미터.
pub struct RenderParams {
    /// 렌더링할 PDF 파일 경로.
    pub file: PathBuf,
    /// 출력 파일 경로. `None`이면 자동 결정.
    pub output: Option<PathBuf>,
    /// 0-based 페이지 인덱스.
    pub page: u16,
    /// 해상도 배율 (PNG 전용, 1.0 = 72 DPI 기준).
    pub scale: f32,
    /// SVG 출력 모드. `true`면 pdfium 불필요.
    pub svg: bool,
}

/// `rpdf render` 서브커맨드를 실행한다.
///
/// `--svg` 미지정: `PDFIUM_DYNAMIC_LIB_PATH` 환경변수 필요.
/// `--svg` 지정: `rpdf_parser::load_document()` → `rpdf_svg::render_page_svg()` → SVG 파일 저장.
pub fn run(params: RenderParams) -> Result<()> {
    if params.svg {
        run_svg(params)
    } else {
        run_png(params)
    }
}

fn run_png(params: RenderParams) -> Result<()> {
    use rpdf_render::render_page;

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

fn run_svg(params: RenderParams) -> Result<()> {
    use rpdf_svg::render_page_svg;

    let data = std::fs::read(&params.file)
        .with_context(|| format!("파일을 읽을 수 없습니다: {}", params.file.display()))?;

    let doc = rpdf_parser::load_document(&data)
        .with_context(|| format!("PDF 파싱 실패: {}", params.file.display()))?;

    let page_index = params.page as usize;
    let page = doc.pages().get(page_index).with_context(|| {
        format!(
            "페이지 {}를 찾을 수 없습니다 (총 {} 페이지)",
            page_index,
            doc.page_count()
        )
    })?;

    let svg_content = render_page_svg(page);

    let output_path = match params.output {
        Some(p) => p,
        None => {
            let stem = params
                .file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            PathBuf::from(format!("{}_p{}.svg", stem, params.page))
        }
    };

    std::fs::write(&output_path, &svg_content)
        .with_context(|| format!("SVG 저장 실패: {}", output_path.display()))?;

    println!("{}", output_path.display());

    Ok(())
}
