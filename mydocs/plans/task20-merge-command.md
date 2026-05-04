# Task #20: MergeCommand 구현

**이슈**: #38  
**브랜치**: `local/task20`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: Task #19 완료 (`DeletePagesCommand`)

---

## 목표

`rpdf-edit` 크레이트에 `MergeCommand`를 추가한다.  
하나 이상의 소스 Document 페이지를 대상 Document 뒤에 순서대로 추가하고,  
Undo 시 추가된 페이지를 제거해 원래 상태로 복원한다.

---

## 설계 결정

### IR 수준 단순 합산 방침

현재 `Page.resources`는 상속이 해결된(materialized) 자기완결 딕셔너리다.  
서로 다른 Document에서 온 두 Page는 별개의 `resources: Option<PdfDict>`를 가지므로,  
IR 수준에서 리소스 이름 충돌이 발생하지 않는다.

리소스 prefix 재작성(중복 폰트/이미지 이름 처리)은  
**Task #22 Serializer**가 PDF 바이트를 생성할 때의 문제다.  
MergeCommand는 IR 수준에서 단순 페이지 연결만 담당한다.

### 위치

`crates/rpdf-edit/src/commands/merge.rs` 신규 파일.  
`mod.rs`에서 `pub mod merge; pub use merge::MergeCommand;` 추가.

### MergeCommand 구조체

```rust
pub struct MergeCommand {
    sources: Vec<Document>,
    snapshot: Mutex<Option<usize>>,
}
```

- `sources`: 합산할 소스 Document 목록. execute 시 각 Document의 pages를 순서대로 클론해 대상 doc에 append.
- `snapshot`: execute 전 `doc.pages.len()`을 저장. undo 시 truncate 기준.  
  `Mutex<Option<usize>>`으로 `Command: Send + Sync` 경계 충족.  
  `None`이면 execute 없이 undo 호출 → `UndoFailed` 반환.

**생성자:**
```rust
pub fn new(sources: Vec<Document>) -> Self {
    Self { sources, snapshot: Mutex::new(None) }
}
```

### execute 로직

```rust
fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 0. 이중 실행 방어: snapshot이 Some이면 → ExecutionFailed("MergeCommand already executed")
    // 1. original_len = doc.pages.len() 저장
    // 2. sources가 비어 있으면 → *self.snapshot.lock().unwrap() = Some(original_len) 후 Ok (no-op)
    // 3. 각 source document의 pages를 순서대로 clone + append:
    //    for source in &self.sources { doc.pages.extend(source.pages.iter().cloned()); }
    // 4. 전체 index 재정렬: doc.pages[i].index = i (for all i)
    // 5. *self.snapshot.lock().unwrap() = Some(original_len)
}
```

**source.pages 순서 보장**: `sources` 벡터 순서대로 각 Document의 pages[0..n-1]이 추가된다.  
소스 내 페이지 순서, 소스 간 순서 모두 보존된다.

### undo 로직

```rust
fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 1. snapshot.take(): self.snapshot.lock().unwrap().take()
    //    None → UndoFailed("undo called before execute")
    // 2. doc.pages.truncate(original_len)
    // 3. 전체 index 재정렬: doc.pages[i].index = i (for all i)
}
```

**`truncate` 기반 undo 이유**: append는 항상 뒤에 붙이므로 `truncate(original_len)`만으로 원래 상태 복원 가능.  
DeletePagesCommand처럼 페이지별 스냅샷이 필요 없다 (`Page` clone 없음, `original_len: usize`만 저장).

### Page.index 재정렬 정책

execute 후와 undo 후 모두 `doc.pages[i].index = i`로 전체 재정렬.

### 메타데이터 처리 방침

대상 Document의 `metadata`는 변경하지 않는다.  
소스 Document의 metadata는 무시된다.  
"합쳐진 문서의 메타데이터는 첫 번째 문서(대상)의 것" 정책.

### 에러 처리

| 조건 | 에러 변형 |
|------|---------|
| snapshot이 Some인 상태에서 execute 재호출 | `CommandError::ExecutionFailed("MergeCommand already executed")` |
| execute 없이 undo 호출 (snapshot None) | `CommandError::UndoFailed("undo called before execute")` |
| sources 비어있음 | 성공 (no-op). `snapshot = Some(original_len)` |
| source Document가 0페이지 | 해당 source 건너뜀 (no-op for that source). 정상 진행 |

---

## 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (merge 모듈 추가 + reindex_pages 헬퍼 추가)
├── delete.rs   (reindex_pages 헬퍼로 교체 — 인라인 루프 제거)
├── error.rs    (변경 없음)
├── merge.rs    ← 신규
├── rotate.rs   (변경 없음)
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

### reindex_pages 헬퍼 추출 (DRY)

`mod.rs` 또는 `merge.rs` 상단에 크레이트 전용 헬퍼 함수를 추가한다:

```rust
pub(crate) fn reindex_pages(pages: &mut [Page]) {
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i;
    }
}
```

- `delete.rs`의 두 인라인 루프를 `reindex_pages(&mut doc.pages)` 호출로 교체
- `merge.rs`의 두 인라인 루프를 `reindex_pages(&mut doc.pages)` 호출로 교체
- 위치: `mod.rs`에 `pub(crate) fn reindex_pages` 추가 (모든 command 모듈이 `use super::*`나 `use crate::commands::` 로 접근 가능)

`mod.rs` 변경:
```rust
mod delete;
mod error;
mod merge;   // 신규
mod rotate;
mod stack;
mod traits;

pub use delete::DeletePagesCommand;
pub use error::CommandError;
pub use merge::MergeCommand;   // 신규
pub use rotate::RotatePageCommand;
pub use stack::CommandStack;
pub use traits::{Command, Query};

use rpdf_core::types::document::Page;

/// 페이지 목록의 index 필드를 현재 위치(0-based)에 맞게 재정렬한다.
pub(crate) fn reindex_pages(pages: &mut [Page]) {
    for (i, page) in pages.iter_mut().enumerate() {
        page.index = i;
    }
}
```

---

## 테스트 전략

위치: `merge.rs` 하단 `#[cfg(test)] mod tests {}`  
테스트 헬퍼: `make_doc(pages: usize, rotations: &[i32]) -> Document` — rotate.rs·delete.rs와 동일.

| # | 테스트명 | 검증 내용 |
|---|---------|---------|
| 1 | `merge_single_source` | 3-page target + 2-page source → 5페이지 |
| 2 | `merge_multiple_sources` | 2-page target + [3-page, 1-page] sources → 6페이지 |
| 3 | `merge_empty_sources_is_noop` | sources=[] → 변경 없음 |
| 4 | `merge_empty_source_document` | sources=[0-page doc] → 변경 없음 |
| 5 | `merge_into_empty_target` | target 0페이지 + 3-page source → 3페이지, **undo 후 0페이지 복원 포함** |
| 6 | `undo_restores_original_pages` | execute → undo → 원래 페이지 수·순서 복원 |
| 7 | `execute_undo_redo_via_stack` | CommandStack 통해 execute → undo → redo 라운드트립 |
| 8 | `page_indices_consistent_after_merge` | merge 후 doc.pages[i].index = i (for all i) |
| 9 | `page_indices_consistent_after_undo` | undo 후 doc.pages[i].index = i (for all i) |
| 10 | `double_execute_fails` | execute() 후 재호출 → ExecutionFailed("already executed") |
| 11 | `undo_before_execute_fails` | execute 없이 undo → UndoFailed("undo called before execute") |
| 12 | `merge_preserves_page_content` | 합산된 page의 rotation·media_box 등 원본 값 보존 |
| 13 | `merge_source_page_order_preserved` | 3-page source의 페이지 순서(0,1,2)가 target 뒤에 그대로 유지됨 |

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `sources = []` | no-op. `snapshot = Some(original_len)`. undo도 no-op |
| source Document가 0페이지 | 해당 source 건너뜀. 다른 source는 정상 처리 |
| target이 0페이지 | 허용. source pages가 index 0부터 시작하게 됨 |
| 매우 큰 source | no limit. 메모리는 호출자 책임 |

---

## NOT in scope

| 항목 | 이유 |
|------|------|
| 리소스 이름 prefix 재작성 | IR 수준 불필요. Serializer(Task #22) 담당 |
| 소스 metadata 병합 | YAGNI. 단순 정책(target 우선) 적용 |
| 페이지 삽입 위치 지정 | append-only. SplitInsertCommand 등 별도 Task |
| 진행 콜백 / 대용량 최적화 | YAGNI |

---

## 의존성

- `rpdf_core::types::document::{Document, Page}` — `Document: Clone`, `Page: Clone` 확인됨
- `rpdf_edit::commands::{Command, CommandError, CommandStack}` — 기존
- `std::sync::Mutex` — 표준 라이브러리

신규 외부 의존성 없음.

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `merge.rs` 생성 + `mod.rs` 등록 | `cargo build -p rpdf-edit` 통과 |
| CP-2 | `execute` + `undo` 구현 | `cargo build -p rpdf-edit` 통과 |
| CP-3 | 13개 단위 테스트 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `MergeCommand`가 `rpdf_edit::commands::MergeCommand`로 공개 API 노출됨
2. execute/undo 대칭성 보장 (`CommandStack` 라운드트립 테스트)
3. 소스 페이지 순서 보존 (소스 내 순서, 소스 간 순서 모두)
4. `Page.index` 재정렬 (`execute` 후, `undo` 후 모두)
5. 13개 단위 테스트 통과
6. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
7. 공개 API `///` 문서 주석 + `# Examples` 섹션
8. 이중 execute 방어 (`"MergeCommand already executed"`)
9. `original_len: usize`만 snapshot으로 저장 — Page clone 최소화
10. `new()` 문서에 소유권 이동 명시: "sources의 소유권을 커맨드가 가져감. 호출자가 재사용하려면 `doc.clone()` 후 전달"

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Outside Voice | Gemini CLI (폴백: Claude subagent) | Independent 2nd opinion | 1 | issues_found | 6 points: 2 resolved (소유권 문서, test #5 확장), 3 false positive (redo, empty slice, delete.rs), 1 confirmed process note |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | **CLEAR** | 2 issues found → all resolved (reindex_pages DRY 추출, test #5 undo 확장) |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (no UI) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

**VERDICT: ENG REVIEW CLEAR — 구현 시작 가능.**
