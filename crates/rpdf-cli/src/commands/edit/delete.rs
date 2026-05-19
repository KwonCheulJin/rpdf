use std::path::PathBuf;

use anyhow::Result;
use rpdf_edit::commands::{Command, DeletePagesCommand};
use rpdf_serializer::{load_document_tracked, serialize_document};

use super::parse_page_list;

/// 지정한 페이지들을 삭제한다.
///
/// `pages`는 "2,4,6" 형태의 1-based 페이지 번호 목록이다.
/// 내부에서 0-based 인덱스로 변환 후 sort+dedup을 적용한다.
pub fn run(input: PathBuf, pages: String, output: PathBuf) -> Result<()> {
    let data = std::fs::read(&input)
        .map_err(|e| anyhow::anyhow!("파일을 읽을 수 없습니다: {} ({e})", input.display()))?;
    let (mut doc, sources) = load_document_tracked(&data)
        .map_err(|e| anyhow::anyhow!("PDF 파싱 실패: {} ({e})", input.display()))?;

    let indices = parse_page_list(&pages)?;

    DeletePagesCommand::new(indices.clone()).execute(&mut doc)?;

    // sources 동기화: retain 패턴 사용 (오름차순 remove() 절대 금지)
    let indices_set: std::collections::HashSet<usize> = indices.into_iter().collect();
    let synced_sources: Vec<_> = sources
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !indices_set.contains(i))
        .map(|(_, s)| s)
        .collect();

    let bytes = serialize_document(&doc, &synced_sources)?;
    std::fs::write(&output, bytes)
        .map_err(|e| anyhow::anyhow!("파일 쓰기 실패: {} ({e})", output.display()))?;

    Ok(())
}
