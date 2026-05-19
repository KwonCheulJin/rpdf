use std::path::PathBuf;

use anyhow::{Result, bail};
use rpdf_edit::commands::{Query, SplitCommand};
use rpdf_serializer::{load_document_tracked, serialize_document};

use super::{parse_ranges_for_split, split_output_path};

/// 지정한 페이지 범위별로 PDF를 여러 파일로 분리한다.
///
/// `pages`는 "1-2,4-5" 형태의 1-based 페이지 범위 명세이다.
/// `output`은 출력 파일을 저장할 디렉토리 경로이며, 반드시 존재해야 한다.
pub fn run(input: PathBuf, pages: String, output: PathBuf) -> Result<()> {
    // 출력 디렉토리 존재 검사
    if !output.is_dir() {
        bail!("출력 디렉토리가 존재하지 않습니다: {}", output.display());
    }

    let data = std::fs::read(&input)
        .map_err(|e| anyhow::anyhow!("파일을 읽을 수 없습니다: {} ({e})", input.display()))?;
    let (doc, sources) = load_document_tracked(&data)
        .map_err(|e| anyhow::anyhow!("PDF 파싱 실패: {} ({e})", input.display()))?;

    let cmd = SplitCommand::new(&pages)?;
    let sub_docs = cmd.execute(&doc)?;

    // SplitCommand의 ranges 필드가 private이므로 pages spec을 직접 재파싱해 ranges 추출
    let ranges = parse_ranges_for_split(&pages)?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());

    // PageSource는 Clone 미구현이므로, 각 범위에 해당하는 sources를
    // page_index 기준으로 필터링해 직렬화한다.
    // 단일 소스 파일이므로 page_index로 직접 매핑 가능하다.
    for (i, (sub_doc, (range_start, range_end))) in sub_docs.iter().zip(ranges.iter()).enumerate() {
        // sources는 load_document_tracked가 반환한 원본 파일 기준 0-based 인덱스 목록
        // range_start..=range_end 범위의 page_index를 가진 sources를 필터링
        let sub_sources: Vec<_> = sources
            .iter()
            .filter(|s| s.page_index >= *range_start && s.page_index <= *range_end)
            .map(|s| rpdf_serializer::PageSource {
                bytes: std::sync::Arc::clone(&s.bytes),
                page_index: s.page_index,
            })
            .collect();

        let bytes = serialize_document(sub_doc, &sub_sources)?;
        let out_path = split_output_path(&output, &stem, i + 1);
        std::fs::write(&out_path, bytes)
            .map_err(|e| anyhow::anyhow!("파일 쓰기 실패: {} ({e})", out_path.display()))?;
    }

    Ok(())
}
