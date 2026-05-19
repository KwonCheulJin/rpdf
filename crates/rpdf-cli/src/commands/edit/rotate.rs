use std::path::PathBuf;

use anyhow::Result;
use rpdf_edit::commands::{Command, RotatePageCommand};
use rpdf_serializer::{load_document_tracked, serialize_document};

/// 지정한 페이지를 회전시킨다.
///
/// `page`는 1-based 페이지 번호이며, 내부에서 0-based 인덱스로 변환한다.
/// `degrees`는 90의 배수여야 하며, 양수는 시계방향 회전이다.
pub fn run(input: PathBuf, page: usize, degrees: i32, output: PathBuf) -> Result<()> {
    let data = std::fs::read(&input)
        .map_err(|e| anyhow::anyhow!("파일을 읽을 수 없습니다: {} ({e})", input.display()))?;
    let (mut doc, sources) = load_document_tracked(&data)
        .map_err(|e| anyhow::anyhow!("PDF 파싱 실패: {} ({e})", input.display()))?;

    if page == 0 {
        anyhow::bail!("페이지 번호는 1부터 시작합니다 (0 입력됨)");
    }
    let page_index = page - 1;

    RotatePageCommand::new(page_index, degrees).execute(&mut doc)?;

    let bytes = serialize_document(&doc, &sources)?;
    std::fs::write(&output, bytes)
        .map_err(|e| anyhow::anyhow!("파일 쓰기 실패: {} ({e})", output.display()))?;

    Ok(())
}
