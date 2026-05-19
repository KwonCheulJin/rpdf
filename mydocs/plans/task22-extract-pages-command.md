# Task #22: ExtractPagesCommand 구현

**이슈**: #42  
**브랜치**: `local/task22`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: Task #21 완료 (`SplitCommand`)

---

## 목표

`rpdf-edit` 크레이트에 `ExtractPagesCommand`를 추가한다.  
`SplitCommand`의 간소화 버전으로, 연속된 단일 페이지 범위를 추출해 새 `Document` 하나를 반환한다.  
원본 Document는 **변경하지 않는다**.

---

## 설계 결정

### `Query` 트레이트 구현 (Command 아님)

`ExtractPagesCommand`는 원본 Document를 변경하지 않으므로 `Query` 트레이트를 구현한다.

```rust
pub trait Query {
    type Output;
    fn execute(&self, doc: &Document) -> Result<Self::Output, CommandError>;
}
```

`ExtractPagesCommand`는 `type Output = Document` — 단일 Document 반환.  
`CommandStack`에 push하지 않는다.

### SplitCommand와의 차이

| 항목 | SplitCommand | ExtractPagesCommand |
|------|-------------|---------------------|
| 입력 형식 | 문자열 명세 (`"1-3,5,7-10"`) | 1-based 시작/끝 페이지 번호 |
| 범위 수 | 다수 범위 허용 | 단일 범위만 |
| 출력 타입 | `Vec<Document>` | `Document` |

`ExtractPagesCommand`는 명시적 정수 인터페이스를 사용한다 — 더 단순하고, 문자열 파싱 오류가 없다.  
문자열 명세가 필요하면 `SplitCommand`를 사용한다.

### 생성자 인터페이스

```rust
pub fn new(start_page: usize, end_page: usize) -> Result<Self, CommandError>
```

- 1-based 페이지 번호 입력 (사용자 친화적, SplitCommand 정책과 일관)
- 내부에서 0-based로 변환하여 저장
- `start_page == 0` 또는 `end_page == 0` → `ExecutionFailed("page numbers are 1-based, got 0")`
- `start_page > end_page` → `ExecutionFailed("invalid range N-M: start > end")`
- 성공 시 `Ok(Self { start: start_page - 1, end: end_page - 1 })`

### 구조체

```rust
pub struct ExtractPagesCommand {
    start: usize, // 0-based, inclusive
    end: usize,   // 0-based, inclusive
}
```

### execute 로직

```rust
fn execute(&self, doc: &Document) -> Result<Document, CommandError> {
    // 1. doc.pages.is_empty() → ExecutionFailed("document has no pages")
    // 2. self.end >= doc.pages.len() → ExecutionFailed("page index out of bounds: {end}")
    // 3. doc.pages[self.start..=self.end] clone → pages
    // 4. reindex_pages(&mut pages)
    // 5. Ok(Document { pages, metadata: doc.metadata.clone() })
}
```

**메타데이터 정책**: 원본 Document의 `metadata`를 출력 Document에 그대로 복사한다.  
(SplitCommand와 동일한 정책 — 일관성 유지)

### 에러 처리

| 조건 | 에러 | 발생 위치 |
|------|------|---------|
| `start_page == 0` | `ExecutionFailed("page numbers are 1-based, got 0")` | `new()` |
| `end_page == 0` | `ExecutionFailed("page numbers are 1-based, got 0")` | `new()` |
| `start_page > end_page` | `ExecutionFailed("invalid range N-M: start > end")` | `new()` |
| 0-page Document | `ExecutionFailed("document has no pages")` | `execute()` |
| `end >= doc.pages.len()` | `ExecutionFailed("page index out of bounds: {end}")` | `execute()` |

> **교차 검증**: 에러 표의 5개 항목이 execute() 로직 설명의 단계 1~2에 모두 대응된다.  
> - 에러 항목 1~3 → `new()` 단계  
> - 에러 항목 4 → 로직 단계 1  
> - 에러 항목 5 → 로직 단계 2

---

## 위치

`crates/rpdf-edit/src/commands/extract.rs` 신규 파일.  
`mod.rs`에서 `mod extract; pub use extract::ExtractPagesCommand;` 추가.

### mod.rs 변경

```rust
mod delete;
mod error;
mod extract;   // 신규
mod merge;
mod rotate;
mod split;
mod stack;
mod traits;

pub use delete::DeletePagesCommand;
pub use error::CommandError;
pub use extract::ExtractPagesCommand;  // 신규
pub use merge::MergeCommand;
pub use rotate::RotatePageCommand;
pub use split::SplitCommand;
pub use stack::CommandStack;
pub use traits::{Command, Query};
```

---

## 테스트 전략

### make_doc 헬퍼 통합 (DRY 리팩터링)

`make_doc(pages: usize, rotations: &[i32]) -> Document` 헬퍼가 rotate.rs, delete.rs, merge.rs, split.rs에 이미 4번 중복되어 있다. CLAUDE.md DRY 룰("세 번 등장하면 추출 검토") 기준 초과. 이 PR에서 `mod.rs`에 공유 헬퍼를 이동하고 각 파일이 이를 사용하도록 수정한다.

```rust
// commands/mod.rs 에 추가
#[cfg(test)]
pub(crate) mod test_utils {
    use rpdf_core::types::document::{Document, Page};

    pub fn make_doc(pages: usize, rotations: &[i32]) -> Document {
        let page_vec = (0..pages)
            .map(|i| Page {
                index: i,
                content: vec![],
                resources: None,
                media_box: None,
                crop_box: None,
                rotation: rotations.get(i).copied().unwrap_or(0),
            })
            .collect();
        Document { pages: page_vec, metadata: None }
    }
}
```

각 테스트 파일은 `use super::super::test_utils::make_doc;` (또는 동등한 경로)로 임포트.

### 테스트 목록

위치: `extract.rs` 하단 `#[cfg(test)] mod tests {}`

| # | 테스트명 | 검증 내용 |
|---|---------|---------|
| 1 | `extract_basic_range` | `new(2, 4)`, 5-page doc → 3페이지 doc |
| 2 | `extract_single_page` | `new(3, 3)`, 5-page doc → 1페이지 doc |
| 3 | `extract_entire_document` | `new(1, 5)`, 5-page doc → 5페이지 doc |
| 4 | `extract_preserves_page_content` | rotation·media_box 원본 값 보존 |
| 5 | `extract_page_indices_reindexed` | 출력 doc의 pages[i].index = i |
| 6 | `extract_metadata_copied` | 원본 metadata가 출력 doc에 복사됨 |
| 7 | `extract_out_of_bounds_end` | `new(1, 10)`, 5-page doc → `ExecutionFailed` |
| 8 | `extract_start_out_of_bounds` | `new(8, 10)`, 5-page doc → `ExecutionFailed` |
| 9 | `extract_zero_start_page` | `new(0, 3)` → `new()` 에러 (`"1-based"`) |
| 10 | `extract_zero_end_page` | `new(1, 0)` → `new()` 에러 (`"1-based"`) |
| 11 | `extract_start_greater_than_end` | `new(4, 2)` → `new()` 에러 (`"start > end"`) |
| 12 | `extract_on_empty_document` | 0-page doc + `new(1, 1)` → `execute()` 에러 |
| 13 | `extract_original_doc_unchanged` | execute 후 원본 doc.pages.len() 변화 없음 |
| 14 | `extract_last_page_boundary` | `new(5, 5)`, 5-page doc → 1페이지 doc (0-based end == len-1 경계) |

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `start_page == end_page` | 허용. 1-page Document 반환 |
| 0-page doc | `execute()` → `ExecutionFailed("document has no pages")` |
| `end >= doc.pages.len()` | `execute()` → `ExecutionFailed` |
| `start_page == 1, end_page == doc.pages.len()` | 전체 추출. 원본 복사본 반환 |

---

## NOT in scope

| 항목 | 이유 |
|------|------|
| 문자열 명세 파싱 (`"3-7"`) | SplitCommand 담당. YAGNI |
| 비연속 범위 지원 | SplitCommand 담당. YAGNI |
| `CommandStack` 등록 | Query는 undo 불필요 |
| 출력 Document 메타데이터 개별 지정 | YAGNI. 원본 복사 정책으로 충분 |
| Outlines/Bookmarks/Annotations 내부 참조 업데이트 | 추출 후 제거된 페이지를 가리키는 Bookmark 등은 dangling 상태가 됨. v0.4 이후 full PDF rewrite 모드에서 처리. SplitCommand도 동일한 한계. |

---

## 의존성

- `rpdf_core::types::document::{Document, Page}` — `Clone` 확인됨
- `rpdf_edit::commands::{Query, CommandError}` — 기존
- `super::reindex_pages` — 기존 `pub(crate)` 헬퍼
- 신규 외부 의존성 없음

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `extract.rs` 생성 + `mod.rs` 등록 | `cargo build -p rpdf-edit` 통과 |
| CP-2 | `new()` + `execute()` 구현 | `cargo build -p rpdf-edit` 통과 |
| CP-3 | 14개 단위 테스트 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `ExtractPagesCommand`가 `rpdf_edit::commands::ExtractPagesCommand`로 공개 API 노출됨
2. `Query` 트레이트 구현, `type Output = Document`
3. 1-based 시작/끝 페이지 번호를 `new()` 시점에 검증
4. 출력 Document의 `Page.index` 재정렬 (0-based)
5. 원본 Document 변경 없음 (`extract_original_doc_unchanged` 테스트로 검증)
6. 14개 단위 테스트 통과 (테스트 #14: `extract_last_page_boundary` 포함)
7. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
8. 공개 API `///` 문서 주석 + `# Examples` 섹션
9. `new()` 문서에 1-based 페이지 번호 명시

---

## Implementation Tasks

- [ ] **T1 (P2, human: ~30min / CC: ~5min)** — rpdf-edit/commands — `make_doc` 테스트 헬퍼를 `mod.rs` `test_utils`로 통합
  - Surfaced by: Code Quality — rotate.rs, delete.rs, merge.rs, split.rs, extract.rs 5개 파일 중복 (DRY 임계값 초과)
  - Files: `commands/mod.rs`, `commands/rotate.rs`, `commands/delete.rs`, `commands/merge.rs`, `commands/split.rs`, `commands/extract.rs`
  - Verify: `cargo test -p rpdf-edit` 통과
- [ ] **T2 (P2, human: ~5min / CC: ~1min)** — rpdf-edit/commands/extract.rs — `extract_last_page_boundary` 테스트 추가
  - Surfaced by: Test Review — `new(n, n)` on n-page doc (0-based end == len-1) 경계값 미검증
  - Files: `crates/rpdf-edit/src/commands/extract.rs`
  - Verify: `cargo test -p rpdf-edit extract_last_page_boundary` 통과
- [ ] **T3 (P3, human: ~5min / CC: ~1min)** — 계획서 NOT in scope 업데이트 (이미 완료됨)

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Outside Voice | Gemini CLI (폴백: Claude subagent) | Independent 2nd opinion | 1 | CLEAR | 6건 중 2 false positive, 1 scope-out, 3건 반영 (make_doc 통합, last-page boundary 테스트, NOT in scope 명시) |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 3건 발견 → 계획서 반영 완료 |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (no UI) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

**VERDICT:** ENG CLEARED — 구현 시작 가능.
