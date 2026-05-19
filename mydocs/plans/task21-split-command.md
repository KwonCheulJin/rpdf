# Task #21: SplitCommand 구현

**이슈**: #40  
**브랜치**: `local/task21`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: Task #20 완료 (`MergeCommand`)

---

## 목표

`rpdf-edit` 크레이트에 `SplitCommand`를 추가한다.  
페이지 범위 명세(`"1-3,5,7-10"`)를 파싱해, 원본 Document에서 각 범위에 해당하는 페이지를  
추출한 새 Document 목록(`Vec<Document>`)을 반환한다.  
원본 Document는 **변경하지 않는다**.

---

## 설계 결정

### `Query` 트레이트 구현 (Command 아님)

`Command` 트레이트는 `&mut Document`를 변경하는 연산 전용이다.  
`SplitCommand`는 원본을 변경하지 않고 새 Document를 생성하므로 `Query` 트레이트가 적합하다.

```rust
pub trait Query {
    type Output;
    fn execute(&self, doc: &Document) -> Result<Self::Output, CommandError>;
}
```

`SplitCommand`는 `Query`를 구현하며 `type Output = Vec<Document>`로 선언한다.  
Undo 불필요 — 원본이 변경되지 않으므로 복원할 것이 없다.  
`CommandStack`에 push하지 않는다.

### 페이지 범위 명세 형식

- **1-based** 페이지 번호 (사용자 친화적)
- `"N"` → N번 페이지 단독 (1장짜리 Document)
- `"N-M"` → N~M번 페이지 (N, M 포함, 1-based)
- `"1-3,5,7-10"` → 세 개의 범위 → 세 개의 Document 반환
- 내부적으로 0-based로 변환하여 저장

### 위치

`crates/rpdf-edit/src/commands/split.rs` 신규 파일.  
`mod.rs`에서 `mod split; pub use split::SplitCommand;` 추가.

### SplitCommand 구조체

```rust
pub struct SplitCommand {
    ranges: Vec<PageRange>,
}

struct PageRange {
    start: usize, // 0-based, inclusive
    end: usize,   // 0-based, inclusive
}
```

- `ranges`: 파싱 완료된 0-based 범위 목록. `new()` 시점에 검증.
- 범위 내용이 유효하더라도 실제 페이지 수 초과 여부는 `execute()` 시점에 검증.

### 생성자

```rust
pub fn new(spec: &str) -> Result<Self, CommandError>
```

- 파싱 전 `spec.split(',').map(str::trim)` — 각 토큰의 앞뒤 공백 제거 (`"1-3, 5"` → 정상 파싱)
- 파싱 실패 → `CommandError::ExecutionFailed("invalid range spec: ...")`
- 빈 문자열 → `CommandError::ExecutionFailed("range spec must not be empty")`
- `"3-1"` (start > end) → `CommandError::ExecutionFailed("invalid range: start > end")`
- `"0"` 또는 `"0-3"` (0-based 입력 거부) → `CommandError::ExecutionFailed("page numbers are 1-based")`
- 성공 시 `Ok(Self { ranges })`

### execute 로직

```rust
fn execute(&self, doc: &Document) -> Result<Vec<Document>, CommandError> {
    // 0. doc.pages.is_empty() → ExecutionFailed("document has no pages")
    // 1. ranges가 비어 있으면 → Ok(vec![]) (no-op)
    // 2. 각 range에 대해:
    //    a. range.end >= doc.pages.len() → ExecutionFailed("page index out of bounds")
    //    b. doc.pages[range.start..=range.end]을 clone해 pages 수집
    //    c. reindex_pages(&mut pages) — 각 출력 doc 내에서 0부터 재정렬
    //    d. Document { pages, metadata: doc.metadata.clone() } 생성
    // 3. Ok(result)
}
```

**메타데이터 정책**: 원본 Document의 `metadata`를 각 출력 Document에 그대로 복사한다.  
"분할된 각 문서는 원본의 메타데이터를 상속" 정책.  
(v0.4 이후 필요 시 호출자가 재지정 가능)

### Page.index 재정렬 정책

각 출력 Document 내에서 `pages[i].index = i` (0-based) 재정렬.  
`super::reindex_pages(&mut pages)` 사용 (DRY, mod.rs의 pub(crate) 헬퍼).

### 에러 처리

| 조건 | 에러 | 발생 위치 |
|------|------|---------|
| 빈 spec | `ExecutionFailed("range spec must not be empty")` | `new()` |
| 숫자 파싱 실패 | `ExecutionFailed("invalid range spec: ...")` | `new()` |
| 0 포함 (1-based 위반) | `ExecutionFailed("page numbers are 1-based, got 0")` | `new()` |
| start > end | `ExecutionFailed("invalid range N-M: start > end")` | `new()` |
| 페이지 범위 초과 | `ExecutionFailed("page index out of bounds: ...")` | `execute()` |
| 0-page Document | `ExecutionFailed("document has no pages")` | `execute()` |

---

## 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (split 모듈 추가)
├── delete.rs   (변경 없음)
├── error.rs    (변경 없음)
├── merge.rs    (변경 없음)
├── rotate.rs   (변경 없음)
├── split.rs    ← 신규
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

`mod.rs` 변경:
```rust
mod delete;
mod error;
mod merge;
mod rotate;
mod split;   // 신규
mod stack;
mod traits;

pub use delete::DeletePagesCommand;
pub use error::CommandError;
pub use merge::MergeCommand;
pub use rotate::RotatePageCommand;
pub use split::SplitCommand;   // 신규
pub use stack::CommandStack;
pub use traits::{Command, Query};

use rpdf_core::types::document::Page;

pub(crate) fn reindex_pages(pages: &mut [Page]) {
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i;
    }
}
```

---

## 테스트 전략

위치: `split.rs` 하단 `#[cfg(test)] mod tests {}`  
테스트 헬퍼: `make_doc(pages: usize, rotations: &[i32]) -> Document` — 기존 패턴과 동일.

| # | 테스트명 | 검증 내용 |
|---|---------|---------|
| 1 | `split_single_range` | `"2-4"`, 5-page doc → 1개 doc, 3페이지 |
| 2 | `split_multiple_ranges` | `"1-2,4-5"`, 5-page doc → 2개 doc, 각 2페이지 |
| 3 | `split_single_page_spec` | `"3"`, 5-page doc → 1개 doc, 1페이지 |
| 4 | `split_entire_document` | `"1-5"`, 5-page doc → 1개 doc, 5페이지 |
| 5 | `split_preserves_page_content` | rotation·media_box 원본 값 보존 |
| 6 | `split_page_indices_reindexed` | 각 출력 doc의 pages[i].index = i |
| 7 | `split_metadata_copied` | 원본 metadata가 각 출력 doc에 복사됨 |
| 8 | `split_out_of_bounds` | `"1-10"`, 5-page doc → `ExecutionFailed` |
| 9 | `split_invalid_spec_letters` | `"abc"` → `new()` 에러 |
| 10 | `split_invalid_spec_empty` | `""` → `new()` 에러 |
| 11 | `split_range_start_greater_than_end` | `"3-1"` → `new()` 에러 |
| 12 | `split_on_empty_document` | 0-page doc → `execute()` 에러 |
| 13 | `split_range_order_preserved` | 범위 내 페이지 순서(rotation 값)가 출력 doc에서 유지됨 |
| 14 | `split_original_doc_unchanged` | execute 후 원본 doc.pages.len() 변화 없음 |
| 15 | `split_zero_page_number` | `"0"` 또는 `"0-3"` → `new()` 에러 (`"page numbers are 1-based"`) |
| 16 | `split_spec_with_spaces` | `"1-3, 5"` (콤마 뒤 공백) → 정상 파싱, 2개 doc |

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `spec = ""` | `new()` 에러 |
| 페이지 수 0인 doc | `execute()` → `ExecutionFailed("document has no pages")` |
| 범위 초과 | `execute()` → `ExecutionFailed` (원자성: 첫 범위 초과 즉시 중단) |
| `"5-5"` (단일 페이지 범위) | 허용. 1-page Document 반환 |
| ranges 비어있음 | `Ok(vec![])` — no-op |
| 중복 범위 `"1-3,1-3"` | 허용. 동일 페이지 두 번 추출, 2개 doc 반환 |

---

## NOT in scope

| 항목 | 이유 |
|------|------|
| 범위 명세의 중복/겹침 검증 | YAGNI. 호출자 책임 |
| `CommandStack` 등록 | Query는 undo 불필요, stack에 push 안 함 |
| 스트리밍/대용량 최적화 | YAGNI |
| 출력 Document 메타데이터 개별 지정 | YAGNI. 원본 복사 정책으로 충분 |

---

## 의존성

- `rpdf_core::types::document::{Document, Page}` — `Document: Clone`, `Page: Clone` 확인됨
- `rpdf_edit::commands::{Query, CommandError}` — 기존
- 신규 외부 의존성 없음

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `split.rs` 생성 + `mod.rs` 등록 | `cargo build -p rpdf-edit` 통과 |
| CP-2 | `new()` 파싱 + `execute()` 구현 | `cargo build -p rpdf-edit` 통과 |
| CP-3 | 16개 단위 테스트 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `SplitCommand`가 `rpdf_edit::commands::SplitCommand`로 공개 API 노출됨
2. `Query` 트레이트 구현, `type Output = Vec<Document>`
3. 범위 명세 파싱을 `new()` 시점에 처리 (parse-then-validate)
4. 각 출력 Document의 `Page.index` 재정렬 (0-based)
5. 원본 Document 변경 없음 (`split_original_doc_unchanged` 테스트로 검증)
6. 16개 단위 테스트 통과 (test #15: `split_zero_page_number`, test #16: `split_spec_with_spaces` 포함)
7. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
8. 공개 API `///` 문서 주석 + `# Examples` 섹션
9. `new()` 문서에 1-based 페이지 번호 명시
10. 원자성: 첫 번째 out-of-bounds 범위 발견 시 즉시 에러 반환 (부분 결과 없음)

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Outside Voice | Gemini CLI (폴백: Claude subagent) | Independent 2nd opinion | 1 | CLEAR | 5건 중 1 false positive, 4건 반영 (execute 0-page check, trim, test #15, test #16) |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | Outside voice 4건 계획서 반영 완료 |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (no UI) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |
