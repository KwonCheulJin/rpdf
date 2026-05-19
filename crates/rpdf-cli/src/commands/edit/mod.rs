pub mod delete;
pub mod extract;
pub mod merge;
pub mod rotate;
pub mod split;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// "2,4,6" 형태의 1-based 페이지 목록 문자열을 0-based 인덱스 Vec으로 변환한다.
///
/// 중복 제거와 오름차순 정렬을 적용한다.
///
/// # Errors
///
/// - 빈 문자열이면 에러
/// - 항목이 0이면 에러 (1-based 위반)
/// - 숫자가 아닌 문자 포함 시 에러
pub(super) fn parse_page_list(spec: &str) -> Result<Vec<usize>> {
    if spec.is_empty() {
        bail!("페이지 목록이 비어있습니다");
    }
    let mut indices: Vec<usize> = spec
        .split(',')
        .map(str::trim)
        .map(|token| {
            let n: u64 = token
                .parse()
                .map_err(|_| anyhow::anyhow!("잘못된 페이지 번호: {token}"))?;
            if n == 0 {
                bail!("페이지 번호는 1-based입니다 (0 입력됨)");
            }
            Ok((n - 1) as usize)
        })
        .collect::<Result<Vec<_>>>()?;
    indices.sort_unstable();
    indices.dedup();
    Ok(indices)
}

/// "5-10" 또는 "5" 형태의 1-based 페이지 범위 문자열을 (start, end) 튜플로 변환한다.
///
/// 단일 숫자 "5" → (5, 5). ExtractPagesCommand에 1-based 그대로 전달.
///
/// # Errors
///
/// - start 또는 end가 0이면 에러 (1-based 위반)
/// - start > end이면 에러
/// - 숫자가 아닌 문자 포함 시 에러
pub(super) fn parse_single_range(spec: &str) -> Result<(usize, usize)> {
    if spec.contains('-') {
        let parts: Vec<&str> = spec.splitn(2, '-').collect();
        let start: u64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {spec}"))?;
        let end: u64 = parts[1]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {spec}"))?;
        if start == 0 || end == 0 {
            bail!("페이지 번호는 1-based입니다 (0 입력됨)");
        }
        if start > end {
            bail!("잘못된 범위 {start}-{end}: start > end");
        }
        Ok((start as usize, end as usize))
    } else {
        let n: u64 = spec
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {spec}"))?;
        if n == 0 {
            bail!("페이지 번호는 1-based입니다 (0 입력됨)");
        }
        Ok((n as usize, n as usize))
    }
}

/// split 출력 파일명을 생성한다: `{stem}_part{N}.pdf` (N은 1-based).
///
/// `stem`이 빈 경우 "output"을 fallback으로 사용한다.
pub(super) fn split_output_path(dir: &Path, stem: &str, n: usize) -> PathBuf {
    let stem = if stem.is_empty() { "output" } else { stem };
    dir.join(format!("{stem}_part{n}.pdf"))
}

/// split 명세 문자열을 파싱해 0-based (start, end) 범위 목록을 반환한다.
///
/// SplitCommand의 `ranges` 필드가 private이므로, sources 슬라이싱을 위해
/// CLI 측에서 명세를 직접 재파싱할 때 사용한다.
pub(super) fn parse_ranges_for_split(spec: &str) -> Result<Vec<(usize, usize)>> {
    if spec.is_empty() {
        bail!("range spec must not be empty");
    }
    spec.split(',')
        .map(str::trim)
        .map(|token| {
            if token.contains('-') {
                let parts: Vec<&str> = token.splitn(2, '-').collect();
                let n: u64 = parts[0]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {token}"))?;
                let m: u64 = parts[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {token}"))?;
                if n == 0 || m == 0 {
                    bail!("페이지 번호는 1-based입니다 (0 입력됨)");
                }
                if n > m {
                    bail!("잘못된 범위 {n}-{m}: start > end");
                }
                Ok(((n - 1) as usize, (m - 1) as usize))
            } else {
                let n: u64 = token
                    .parse()
                    .map_err(|_| anyhow::anyhow!("잘못된 범위 명세: {token}"))?;
                if n == 0 {
                    bail!("페이지 번호는 1-based입니다 (0 입력됨)");
                }
                Ok(((n - 1) as usize, (n - 1) as usize))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_page_list 단위 테스트

    #[test]
    fn parse_page_list_normal() {
        let result = parse_page_list("2,4,6").unwrap();
        assert_eq!(result, vec![1, 3, 5]); // 0-based
    }

    #[test]
    fn parse_page_list_dedup() {
        let result = parse_page_list("2,2,4").unwrap();
        assert_eq!(result, vec![1, 3]); // 중복 제거
    }

    #[test]
    fn parse_page_list_zero_error() {
        let err = parse_page_list("0").unwrap_err();
        assert!(err.to_string().contains("1-based"));
    }

    #[test]
    fn parse_page_list_empty_error() {
        let err = parse_page_list("").unwrap_err();
        assert!(err.to_string().contains("비어있습니다"));
    }

    #[test]
    fn parse_page_list_letters_error() {
        let err = parse_page_list("2,abc,4").unwrap_err();
        assert!(err.to_string().contains("잘못된 페이지 번호"));
    }

    // parse_single_range 단위 테스트

    #[test]
    fn parse_single_range_range() {
        let (start, end) = parse_single_range("5-10").unwrap();
        assert_eq!(start, 5);
        assert_eq!(end, 10);
    }

    #[test]
    fn parse_single_range_single() {
        let (start, end) = parse_single_range("5").unwrap();
        assert_eq!(start, 5);
        assert_eq!(end, 5);
    }

    #[test]
    fn parse_single_range_inverted_error() {
        let err = parse_single_range("10-5").unwrap_err();
        assert!(err.to_string().contains("start > end"));
    }

    #[test]
    fn parse_single_range_zero_start_error() {
        let err = parse_single_range("0-5").unwrap_err();
        assert!(err.to_string().contains("1-based"));
    }

    #[test]
    fn parse_single_range_zero_single_error() {
        let err = parse_single_range("0").unwrap_err();
        assert!(err.to_string().contains("1-based"));
    }

    // split_output_path 단위 테스트

    #[test]
    fn split_output_path_normal() {
        let dir = Path::new("/tmp/out");
        let path = split_output_path(dir, "fw4-2024", 1);
        assert_eq!(path, Path::new("/tmp/out/fw4-2024_part1.pdf"));
    }

    #[test]
    fn split_output_path_part2() {
        let dir = Path::new("/tmp/out");
        let path = split_output_path(dir, "fw4-2024", 2);
        assert_eq!(path, Path::new("/tmp/out/fw4-2024_part2.pdf"));
    }

    #[test]
    fn split_output_path_empty_stem_fallback() {
        let dir = Path::new("/tmp/out");
        let path = split_output_path(dir, "", 1);
        assert_eq!(path, Path::new("/tmp/out/output_part1.pdf"));
    }
}
