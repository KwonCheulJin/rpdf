# Task #19 완료 보고서: DeletePagesCommand 구현

**이슈**: #36  
**브랜치**: `local/task19`  
**마일스톤**: v0.3 — 편집 커맨드  
**완료일**: 2026-05-04

---

## 완료 체크리스트

- [x] `crates/rpdf-edit/src/commands/delete.rs` 신규 파일 생성
- [x] `mod.rs`에 `mod delete` + `pub use delete::DeletePagesCommand` 추가
- [x] `DeletePagesCommand::new(indices)` 공개 생성자 + `///` 문서 주석 + `# Examples` doctest
- [x] `execute` 구현: 이중실행 방어 → sort+dedup → 빈 인덱스 no-op → 범위 검증(원자성) → 역순 제거(Move 시맨틱스) → 오름차순 snapshot 저장 → index 재정렬
- [x] `undo` 구현: `snapshot.take()`(이중 undo 방지) → 오름차순 insert 복원 → index 재정렬
- [x] 15개 단위 테스트 통과
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 전체 통과

---

## 구현 내용

### 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (delete 모듈 등록 + pub use 추가)
├── delete.rs   ← 신규
├── error.rs    (변경 없음)
├── rotate.rs   (변경 없음)
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

### 주요 설계 결정

1. **`Mutex<Option<Vec<(usize, Page)>>>`**: `Command: Send + Sync` 경계 충족. `None` 센티넬로 execute 없이 undo 호출 시 `UndoFailed("undo called before execute")` 반환. `take()`으로 이중 undo 방지.

2. **역순 제거 + 오름차순 복원**:
   - execute: `sorted_desc`로 내림차순 정렬 후 `doc.pages.remove(i)` — 앞쪽 인덱스 shift 방지
   - snapshot을 `sort_by_key(|&(i, _)| i)`로 오름차순 재정렬 저장
   - undo: 오름차순으로 `doc.pages.insert(original_index, page)` — 정확한 원래 위치 복원

3. **Move 시맨틱스**: `doc.pages.remove(i)` 반환값(소유권)을 직접 snapshot에 저장 — `Page::clone()` 없음.

4. **이중 실행 방어**: execute 진입 시 `snapshot.is_some()` 체크 → `ExecutionFailed("DeletePagesCommand already executed")`.

5. **원자성 보장**: 범위 검증이 remove 이전에 완전 완료. 인덱스 하나라도 OOB이면 doc 변경 없이 전체 취소.

6. **에러 메시지에 현재 상태 포함**: `"page index out of bounds: {i} (document has {len} pages)"`.

---

## 품질 게이트

| 명령 | 결과 |
|------|------|
| `cargo test -p rpdf-edit` | 35/35 단위 테스트 + 2개 doctest 통과 |
| `cargo clippy -p rpdf-edit -- -D warnings` | 경고 없음 |
| `cargo fmt --check` | 통과 |

---

## 테스트 커버리지

| # | 테스트명 | 검증 |
|---|---------|------|
| 1 | `delete_single_page` | 3-page doc, index 1 삭제 → 2페이지, index 재정렬 |
| 2 | `delete_multiple_pages` | 5-page doc, [1, 3] 삭제 → 3페이지 |
| 3 | `delete_first_page` | index 0 삭제 → 앞 페이지 제거 |
| 4 | `delete_last_page` | 마지막 index 삭제 |
| 5 | `undo_restores_deleted_pages` | execute → undo → 원래 페이지 수·순서 복원 |
| 6 | `execute_undo_redo_via_stack` | CommandStack 라운드트립 |
| 7 | `page_index_out_of_bounds` | OOB index → ExecutionFailed |
| 8 | `duplicate_indices_deduplicated` | [1, 1, 2] → 2페이지 삭제 |
| 9 | `empty_indices_is_noop` | [] → 변경 없음 |
| 10 | `delete_all_pages` | 전체 삭제 → 빈 Document, undo 후 복원 |
| 11 | `undo_before_execute_fails` | UndoFailed("undo called before execute") |
| 12 | `page_indices_consistent_after_execute` | 삭제 후 남은 pages의 index 0,1,2... |
| 13 | `page_indices_consistent_after_undo` | undo 후 모든 pages의 index 0,1,2... |
| 14 | `partial_out_of_bounds_is_atomic` | vec![0, 5] on 2-page doc → ExecutionFailed, doc 변경 없음 |
| 15 | `double_execute_fails` | execute() 재호출 → ExecutionFailed("already executed") |

---

## 회고 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | `sort_unstable() → dedup()` 순서 — dedup은 인접 중복만 제거 | C: 스킵 | CLAUDE.md 설계 원칙에서 이미 다루는 Rust 관용구 |
| 2 | 역순 제거 + 오름차순 복원 패턴 | C: 완료 보고서 메모 | 계획서에 충분히 설명됨, 별도 CLAUDE.md 규칙 불필요 |
| 3 | `snapshot.take()`로 이중 undo 방지 + `is_some()`으로 이중 execute 방지 | C: 스킵 | `Mutex<Option<T>>` 패턴은 CLAUDE.md에 이미 기록됨 |

모든 항목이 C(스킵) — CLAUDE.md 추가나 트러블슈팅 문서 생성 없이 완료.

---

## 다음 작업

Task #20: `InsertPageCommand` 또는 v0.3 다음 편집 커맨드 (마일스톤 확인 필요).
