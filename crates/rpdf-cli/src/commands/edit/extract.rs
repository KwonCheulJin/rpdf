use std::path::PathBuf;

use anyhow::Result;
use rpdf_edit::commands::{ExtractPagesCommand, Query};
use rpdf_serializer::{load_document_tracked, serialize_document};

use super::parse_single_range;

/// 지정한 페이지 범위를 새 PDF로 추출한다.
///
/// `pages`는 "2-4" 또는 "5" 형태의 1-based 페이지 범위이다.
/// ExtractPagesCommand에 1-based 값을 그대로 전달한다.
pub fn run(input: PathBuf, pages: String, output: PathBuf) -> Result<()> {
    let data = std::fs::read(&input)
        .map_err(|e| anyhow::anyhow!("파일을 읽을 수 없습니다: {} ({e})", input.display()))?;
    let (doc, sources) = load_document_tracked(&data)
        .map_err(|e| anyhow::anyhow!("PDF 파싱 실패: {} ({e})", input.display()))?;

    let (start, end) = parse_single_range(&pages)?;

    let new_doc = ExtractPagesCommand::new(start, end)?.execute(&doc)?;

    // 1-based → 0-based 슬라이싱 (PageSource는 Clone 미구현이므로 into_iter로 소비)
    let sub_sources: Vec<_> = sources
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i >= start - 1 && *i < end)
        .map(|(_, s)| s)
        .collect();

    let bytes = serialize_document(&new_doc, &sub_sources)?;
    std::fs::write(&output, bytes)
        .map_err(|e| anyhow::anyhow!("파일 쓰기 실패: {} ({e})", output.display()))?;

    Ok(())
}
