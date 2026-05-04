# Task #20 완료 보고서: MergeCommand

**이슈**: #38  
**브랜치**: `local/task20`  
**마일스톤**: v0.3 — 편집 커맨드  
**완료일**: 2026-05-04

---

## 완료 내용

`rpdf-edit` 크레이트에 `MergeCommand`를 추가했다.  
하나 이상의 소스 Document 페이지를 대상 Document 뒤에 순서대로 append하고,  
Undo 시 truncate로 원래 상태를 복원한다.

### 변경 파일

| 파일 | 변경 내용 |
|------|---------|
| `crates/rpdf-edit/src/commands/merge.rs` | 신규 생성 — MergeCommand + 13개 단위 테스트 |
| `crates/rpdf-edit/src/commands/mod.rs` | merge 모듈 등록, MergeCommand re-export, reindex_pages 헬퍼 추가 |
| `crates/rpdf-edit/src/commands/delete.rs` | 인라인 reindex 루프 2개 → reindex_pages() 호출로 교체 |

---

## 구현 결정 사항

### MergeCommand 구조체

```rust
pub struct MergeCommand {
    sources: Vec<Document>,
    snapshot: Mutex<Option<usize>>,
}
```

- `snapshot: Mutex<Option<usize>>` — execute 전 `doc.pages.len()`만 저장. Page clone 없음.
- `Mutex<T>`: `Command: Send + Sync` 경계 충족.

### reindex_pages DRY 추출

`pub(crate) fn reindex_pages(pages: &mut [Page])`를 `mod.rs`에 추가.  
이전 delete.rs의 인라인 루프 2개 + merge.rs 2개 → 4회 등장 → CLAUDE.md "3회 등장 → 추출" 기준 충족.

### undo 방식

`doc.pages.truncate(original_len)` — append 기반이므로 truncate만으로 완전 복원 가능.  
DeletePagesCommand처럼 페이지별 스냅샷 불필요.

---

## 계획 대비 달라진 점

없음. 계획서를 그대로 구현했다.

단, evaluator 지적으로 `merge_multiple_sources` 테스트에 소스 간 순서 검증(rotation 값 비교)을 추가함.  
계획서에는 "소스 간 순서 보존"이 완료 기준에 있었으나 테스트에 명시적 assertion이 빠져있었음.

---

## 품질 게이트

| 항목 | 결과 |
|------|------|
| `cargo test -p rpdf-edit` | 48 passed, 0 failed |
| doctest | 3 passed (rotate, merge, delete) |
| `cargo clippy -p rpdf-edit -- -D warnings` | 경고 0개 |
| `cargo fmt --check` | 통과 |

---

## 배운 점

- `truncate` 기반 undo는 append-only 패턴에서 매우 단순하고 효율적이다.  
  DeletePagesCommand의 페이지별 스냅샷과 대조되는 설계.
- "소스 페이지 순서 보존" 같은 정책은 구현이 자연스럽게 보장해도, 테스트가 명시적으로 검증해야 완료 기준을 충족한다.

---

## 회고 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | 소스 간 순서 보존 → 테스트에 명시적 assertion 필요 | **C: 스킵** | CLAUDE.md §체크포인트 셀프 리뷰에 "조건 분기 실제 실행 확인" 이미 있음. 새 규칙 불필요 |
| 2 | append-only undo는 truncate로 충분 (snapshot 최소화 패턴) | **C: 스킵** | 구현 패턴은 코드로 남음. 별도 규칙 가치 없음 |
