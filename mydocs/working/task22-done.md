# Task #22 완료 보고서: ExtractPagesCommand 구현

**이슈**: #42  
**브랜치**: `local/task22`  
**완료일**: 2026-05-19

---

## 요약

`rpdf-edit` 크레이트에 `ExtractPagesCommand`를 추가했다. `Query` 트레이트를 구현하며, 1-based 시작/끝 페이지 번호를 받아 원본 Document에서 해당 범위의 페이지를 추출한 새 `Document`를 반환한다. 원본 Document는 변경하지 않는다.

추가로 4개 파일에 중복되어 있던 `make_doc` 테스트 헬퍼를 `mod.rs`의 공유 `test_utils` 모듈로 통합하는 DRY 리팩터링도 함께 진행했다.

---

## 구현 내용

### 신규/수정 파일

| 파일 | 변경 |
|------|------|
| `crates/rpdf-edit/src/commands/extract.rs` | 신규 생성 |
| `crates/rpdf-edit/src/commands/mod.rs` | `extract` 모듈 추가, `test_utils` 공유 모듈 추가 |
| `crates/rpdf-edit/src/commands/rotate.rs` | 로컬 `make_doc` 제거, `test_utils` 임포트 |
| `crates/rpdf-edit/src/commands/delete.rs` | 동일 |
| `crates/rpdf-edit/src/commands/merge.rs` | 동일 |
| `crates/rpdf-edit/src/commands/split.rs` | 동일 |

### 핵심 설계 결정

- **`Query` 트레이트 구현**: 원본 불변이므로 Command 아님. `type Output = Document`.
- **정수 인터페이스**: `new(start_page, end_page)` — 문자열 파싱 없이 명시적. SplitCommand와 인터페이스는 다르지만 1-based 정책은 일관.
- **DRY 리팩터링 번들**: `make_doc`이 4개 파일에 이미 중복된 상태. 5번째 추가 전에 통합 결정.

### 에러 처리

| 조건 | 에러 메시지 | 발생 위치 |
|------|------------|---------|
| `start_page == 0` 또는 `end_page == 0` | "page numbers are 1-based, got 0" | `new()` |
| `start_page > end_page` | "invalid range N-M: start > end" | `new()` |
| 0-page Document | "document has no pages" | `execute()` |
| 범위 초과 | "page index out of bounds: N" | `execute()` |

---

## 테스트 결과

78개 단위 테스트 + 5개 doctest = 전체 통과.

| # | 테스트명 | 결과 |
|---|---------|------|
| 1 | `extract_basic_range` | PASS |
| 2 | `extract_single_page` | PASS |
| 3 | `extract_entire_document` | PASS |
| 4 | `extract_preserves_page_content` | PASS |
| 5 | `extract_page_indices_reindexed` | PASS |
| 6 | `extract_metadata_copied` | PASS |
| 7 | `extract_out_of_bounds_end` | PASS |
| 8 | `extract_start_out_of_bounds` | PASS |
| 9 | `extract_zero_start_page` | PASS |
| 10 | `extract_zero_end_page` | PASS |
| 11 | `extract_start_greater_than_end` | PASS |
| 12 | `extract_on_empty_document` | PASS |
| 13 | `extract_original_doc_unchanged` | PASS |
| 14 | `extract_last_page_boundary` | PASS |

---

## 품질 게이트

| 게이트 | 결과 |
|--------|------|
| `cargo test --workspace --exclude rpdf-render` | PASS |
| `cargo clippy --workspace --exclude rpdf-render -- -D warnings` | PASS (경고 0) |
| `cargo fmt --all --check` | PASS |

---

## plan-eng-review 결과

Outside Voice (Gemini) 6건 발견:

| # | 발견 내용 | 판정 | 처리 |
|---|---------|------|------|
| 1 | Architectural Schizophrenia (string vs int API) | SCOPE-OUT | SplitCommand 이미 merge됨. 범위 외 |
| 2 | Semantic Error Abuse (new()에서 ExecutionFailed 사용) | VALID/TODO | SplitCommand와 일관성 유지. 미래 리팩터링 후보 |
| 3 | Redundant is_empty() check | FALSE POSITIVE | 명확한 에러 메시지 UX 목적 |
| 4 | Memory/Performance (Page.clone) | VALID/TODO | 기존 SplitCommand와 동일 패턴 |
| 5 | PDF Spec Violation (metadata ID) | FALSE POSITIVE | DocumentMetadata에 trailer ID 없음 |
| 6 | Broken Internal References | VALID | NOT in scope에 명시 완료 |

eng-review에서 추가 발견:
- Code Quality: `make_doc` DRY 통합 → 이번 PR에서 처리
- Test Gap: `extract_last_page_boundary` 누락 → 테스트 #14 추가

---

## 회고 분류

| 항목 | 분류 | 내용 |
|------|------|------|
| make_doc 중복이 4개 파일에 이미 존재했지만 이전 task에서 방치됨 | B | `pub(crate) mod test_utils` 패턴: Rust에서 `#[cfg(test)]` 공유 헬퍼는 mod.rs에 test_utils 모듈로 두면 각 하위 모듈에서 `use super::test_utils::...`로 접근 가능 |
| Outside Voice가 DocumentMetadata에 trailer ID 없음을 놓침 (false positive) | C | 도메인 모델 지식 없는 외부 AI는 구조체 내용을 모르는 상태에서 PDF 스펙 이슈를 추측할 수 있음 |
| last-page boundary 테스트 plan-eng-review에서 발견 | A (긍정) | 리뷰 단계가 경계값 누락을 잡아냄 (13 → 14 테스트) |
