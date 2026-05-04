# Task #19: DeletePagesCommand 구현

**이슈**: #36  
**브랜치**: `local/task19`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: Task #18 완료 (`RotatePageCommand`)

---

## 목표

`rpdf-edit` 크레이트에 `DeletePagesCommand`를 추가한다.  
지정한 페이지 인덱스 목록을 Document에서 제거하고, Undo 시 원래 순서대로 복원한다.

---

## 설계 결정

### 위치

`crates/rpdf-edit/src/commands/delete.rs` 신규 파일.  
`mod.rs`에서 `pub mod delete; pub use delete::DeletePagesCommand;` 추가.

### DeletePagesCommand 구조체

```rust
pub struct DeletePagesCommand {
    indices: Vec<usize>,
    snapshot: Mutex<Option<Vec<(usize, Page)>>>,
}
```

- `indices`: 삭제할 0-based 페이지 인덱스 목록. execute 시 중복 제거 + 정렬 처리.
- `snapshot`: execute 시 `(original_index, Page 클론)` 쌍을 저장. undo 시 복원에 사용.  
  `Mutex<Option<...>>`으로 `Command: Send + Sync` 경계 충족. `None`이면 execute 없이 undo 호출 → `UndoFailed` 반환.

**생성자:**
```rust
pub fn new(indices: Vec<usize>) -> Self {
    Self { indices, snapshot: Mutex::new(None) }
}
```

### execute 로직

```rust
fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 0. 이중 실행 방어: snapshot이 Some이면 → ExecutionFailed("DeletePagesCommand already executed")
    // 1. indices.sort_unstable(); indices.dedup(); (sort 먼저 — dedup은 인접 중복만 제거)
    // 2. 빈 indices → *self.snapshot.lock().unwrap() = Some(vec![]) 후 Ok 반환 (no-op)
    // 3. 범위 검증: any index >= doc.pages.len() →
    //    ExecutionFailed("page index out of bounds: {i} (document has {len} pages)")
    // 4. Move 시맨틱스: sorted_desc.iter().map(|&i| (i, doc.pages.remove(i))).collect()
    //    → sort_by_key ascending → *self.snapshot.lock().unwrap() = Some(snapshot)
    //    (clone() 없음 — remove()가 Page 소유권 반환)
    // 5. 남은 페이지 index 필드 재정렬: doc.pages[i].index = i (for all i)
}
```

**역순 제거 이유**: 오름차순 인덱스를 정순으로 제거하면 뒤 인덱스가 shift된다.  
역순(내림차순)으로 제거하면 앞쪽 인덱스가 유효하게 유지된다.

### undo 로직

```rust
fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 1. snapshot.take(): self.snapshot.lock().unwrap().take()
    //    None → UndoFailed("undo called before execute")
    //    (take()로 소유권 이동 + snapshot을 None으로 리셋 — 이중 undo 방지)
    // 2. snapshot 엔트리를 original_index 오름차순으로 순회:
    //    doc.pages.insert(original_index, page)  (by value, clone() 없음)
    // 3. 전체 페이지 index 필드 재정렬: doc.pages[i].index = i (for all i)
}
```

**오름차순 삽입 이유**: 인덱스 1과 3을 복원할 때, 1 먼저 삽입하면 2(기존)가 올바른 위치에 있고, 3 삽입 시 정확히 원래 자리에 들어간다.

### Page.index 재정렬 정책

execute 후와 undo 후 모두 `doc.pages[i].index = i`로 전체 재정렬한다.  
`Page.index`는 "현재 문서에서의 위치"를 나타내므로, 페이지 목록이 바뀔 때마다 유효해야 한다.

### 에러 처리

| 조건 | 에러 변형 |
|------|---------|
| `any index >= doc.pages.len()` | `CommandError::ExecutionFailed("page index out of bounds: {i} (document has {len} pages)")` |
| `indices` 비어있음 | 성공 (no-op). `snapshot = Some(vec![])` |
| execute 없이 undo 호출 (snapshot None) | `CommandError::UndoFailed("undo called before execute")` |
| snapshot이 Some인 상태에서 execute 재호출 | `CommandError::ExecutionFailed("DeletePagesCommand already executed")` |

> 중복 인덱스는 에러가 아니라 silent dedup으로 처리한다 (사용자 편의).  
> 전체 페이지 삭제(`indices`가 모든 인덱스 포함)는 허용한다 — 빈 Document가 결과.

---

## 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (delete 모듈 추가)
├── delete.rs   ← 신규
├── error.rs    (변경 없음)
├── rotate.rs   (변경 없음)
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

`mod.rs` 변경:
```rust
mod delete;   // 신규
mod error;
mod rotate;
mod stack;
mod traits;

pub use delete::DeletePagesCommand;   // 신규
pub use error::CommandError;
pub use rotate::RotatePageCommand;
pub use stack::CommandStack;
pub use traits::{Command, Query};
```

---

## 테스트 전략

위치: `delete.rs` 하단 `#[cfg(test)] mod tests {}`  
테스트 헬퍼: `RotatePageCommand`와 동일한 `make_doc(pages: usize, rotations: &[i32])` 재작성.

| # | 테스트명 | 검증 내용 |
|---|---------|---------|
| 1 | `delete_single_page` | 3-page doc, index 1 삭제 → 2페이지, 나머지 index 재정렬 |
| 2 | `delete_multiple_pages` | 5-page doc, [1, 3] 삭제 → 3페이지 |
| 3 | `delete_first_page` | index 0 삭제 → 앞 페이지 제거 |
| 4 | `delete_last_page` | 마지막 index 삭제 |
| 5 | `undo_restores_deleted_pages` | execute → undo → 원래 페이지 수·순서 복원 |
| 6 | `execute_undo_redo_via_stack` | CommandStack 통해 execute → undo → redo 라운드트립 |
| 7 | `page_index_out_of_bounds` | 존재하지 않는 index → ExecutionFailed |
| 8 | `duplicate_indices_deduplicated` | [1, 1, 2] → [1, 2] 처리, 2페이지 삭제 |
| 9 | `empty_indices_is_noop` | [] 삭제 → 변경 없음, undo 후에도 동일 |
| 10 | `delete_all_pages` | 전체 삭제 → 빈 Document, undo 후 복원 |
| 11 | `undo_before_execute_fails` | execute 없이 undo → UndoFailed("undo called before execute") |
| 12 | `page_indices_consistent_after_execute` | 삭제 후 남은 pages의 index 필드가 0,1,2... |
| 13 | `page_indices_consistent_after_undo` | undo 후 모든 pages의 index 필드가 0,1,2... |
| 14 | `partial_out_of_bounds_is_atomic` | new(vec![0, 5]) on 2-page doc → ExecutionFailed, doc 변경 없음 (원자성 보장) |
| 15 | `double_execute_fails` | execute() 후 동일 인스턴스에 execute() 재호출 → ExecutionFailed("already executed") |

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `indices = []` | no-op. `snapshot = Some(vec![])`. undo도 no-op |
| 중복 인덱스 `[2, 2, 2]` | dedup → `[2]`, 페이지 1개 삭제 |
| 전체 페이지 삭제 | 허용. `doc.pages = []`. undo 후 복원 |
| 1페이지 문서에서 index 0 삭제 | 허용. 빈 Document |
| out-of-bounds 인덱스 하나라도 포함 | 전체 취소. 아무 페이지도 삭제 안 함 |

---

## NOT in scope

| 항목 | 이유 |
|------|------|
| PDF 내부 참조 정리 (`/Dests`, `/Outlines`, `/Annots`) | 삭제된 페이지를 향하는 링크·북마크는 dangling 상태가 됨. Issue #22 Serializer 구현 시 처리 |
| O(N) 파티션 알고리즘 최적화 | YAGNI. 수백 페이지 규모에서 O(k·N)이면 충분. 성능 문제 발생 시 TODO |
| PageId 기반 안정적 참조 | v0.3 범위 밖 (Task #17 설계에서 DEFER 결정) |

## 의존성

- `rpdf_core::types::document::{Document, Page}` — `Page: Clone` 필요 (기존 `#[derive(Clone)]` 확인됨)
- `rpdf_edit::commands::{Command, CommandError, CommandStack}` — 기존
- `std::sync::Mutex` — 표준 라이브러리

신규 외부 의존성 없음.

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `delete.rs` 생성 + `mod.rs` 등록 | `cargo build -p rpdf-edit` 통과 |
| CP-2 | `execute` + `undo` 구현 | `cargo build -p rpdf-edit` 통과 |
| CP-3 | 15개 단위 테스트 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `DeletePagesCommand`가 `rpdf_edit::commands::DeletePagesCommand`로 공개 API 노출됨
2. execute/undo 대칭성 보장 (`CommandStack` 라운드트립 테스트)
3. 역순 제거 + 오름차순 삽입으로 올바른 페이지 순서 복원
4. `Page.index` 재정렬 (`execute` 후, `undo` 후 모두)
5. 15개 단위 테스트 통과
6. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
7. 공개 API `///` 문서 주석 + `# Examples` 섹션
8. 에러 메시지에 현재 상태 포함 (`has {len} pages`)
9. 이중 execute 방어 (`"DeletePagesCommand already executed"`)
10. Move 시맨틱스 — clone() 없이 `Vec::remove()` 반환값 직접 snapshot 저장

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Outside Voice | Gemini CLI | Independent 2nd opinion | 1 | issues_found | 5 points: 2 adopted (Move+guard), 1 deferred (O(n)), 1 NOT in scope (refs), 1 false positive |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | **CLEAR** | 3 issues found → all resolved (sort/dedup, test 14, test 15) |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | N/A (no UI) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

**VERDICT: ENG REVIEW CLEAR — 구현 시작 가능.**
