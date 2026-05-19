use std::path::PathBuf;

use anyhow::{Result, bail};
use rpdf_edit::commands::{Command, MergeCommand};
use rpdf_serializer::{load_document_tracked, serialize_document};

/// 여러 PDF 파일을 하나의 PDF로 합친다.
///
/// 첫 번째 파일을 기준 Document로 사용하고,
/// 나머지 파일의 페이지를 순서대로 뒤에 추가한다.
pub fn run(inputs: Vec<PathBuf>, output: PathBuf) -> Result<()> {
    if inputs.len() < 2 {
        bail!("merge 명령은 최소 2개의 입력 파일이 필요합니다");
    }

    let mut docs_and_sources = inputs
        .iter()
        .map(|path| {
            let data = std::fs::read(path).map_err(|e| {
                anyhow::anyhow!("파일을 읽을 수 없습니다: {} ({e})", path.display())
            })?;
            load_document_tracked(&data)
                .map_err(|e| anyhow::anyhow!("PDF 파싱 실패: {} ({e})", path.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    // 첫 번째 doc을 기준으로 설정
    let (mut base_doc, base_sources) = docs_and_sources.remove(0);
    let mut all_sources = base_sources;

    // 나머지 doc의 sources를 수집
    let (merge_docs, extra_sources_list): (Vec<_>, Vec<_>) = docs_and_sources.into_iter().unzip();

    MergeCommand::new(merge_docs).execute(&mut base_doc)?;

    for extra_sources in extra_sources_list {
        all_sources.extend(extra_sources);
    }

    let bytes = serialize_document(&base_doc, &all_sources)?;
    std::fs::write(&output, bytes)
        .map_err(|e| anyhow::anyhow!("파일 쓰기 실패: {} ({e})", output.display()))?;

    Ok(())
}
