# Task #21 완료 보고서: SplitCommand 구현

**이슈**: #40  
**브랜치**: `local/task21`  
**완료일**: 2026-05-19

---

## 요약

`rpdf-edit` 크레이트에 `SplitCommand`를 추가했다. `Query` 트레이트를 구현하며, 1-based 페이지 범위 명세를 파싱해 원본 Document에서 각 범위에 해당하는 페이지를 추출한 새 `Vec<Document>`를 반환한다. 원본 Document는 변경하지 않는다.

---

## 구현 내용

### 신규/수정 파일

| 파일 | 변경 |
|------|------|
| `crates/rpdf-edit/src/commands/split.rs` | 신규 생성 |
| `crates/rpdf-edit/src/commands/mod.rs` | `mod split; pub use split::SplitCommand;` 추가 |

### 핵심 설계 결정

- **`Query` 트레이트 구현**: `Command`는 `&mut Document`를 변경하는 연산 전용. `SplitCommand`는 원본을 읽기만 하므로 `Query`가 적합. `CommandStack`에 push 불필요.
- **파싱 시점**: `new()` 생성자에서 범위 명세를 완전히 파싱·검증 (parse-then-validate). `execute()` 시점에는 페이지 수 초과만 검사.
- **1-based → 0-based 변환**: 사용자 입력은 1-based, 내부 `PageRange.{start, end}`는 0-based.
- **원자성**: `execute()` 내 선행 루프에서 모든 범위의 out-of-bounds를 검사한 뒤 실제 분리 수행. 첫 실패 즉시 에러 반환.
- **공백 처리**: `spec.split(',').map(str::trim)` — `"1-3, 5"` 허용.

### 에러 처리

| 조건 | 에러 메시지 | 발생 위치 |
|------|------------|---------|
| 빈 spec | "range spec must not be empty" | `new()` |
| 숫자 파싱 실패 | "invalid range spec: {token}" | `new()` |
| 0 포함 (1-based 위반) | "page numbers are 1-based, got 0" | `new()` |
| start > end | "invalid range {N}-{M}: start > end" | `new()` |
| 0-page Document | "document has no pages" | `execute()` |
| 범위 초과 | "page index out of bounds: {index}" | `execute()` |

---

## 테스트 결과

16개 단위 테스트 + 1개 doctest = 17개 신규, 전체 통과.

| # | 테스트명 | 결과 |
|---|---------|------|
| 1 | `split_single_range` | PASS |
| 2 | `split_multiple_ranges` | PASS |
| 3 | `split_single_page_spec` | PASS |
| 4 | `split_entire_document` | PASS |
| 5 | `split_preserves_page_content` | PASS |
| 6 | `split_page_indices_reindexed` | PASS |
| 7 | `split_metadata_copied` | PASS |
| 8 | `split_out_of_bounds` | PASS |
| 9 | `split_invalid_spec_letters` | PASS |
| 10 | `split_invalid_spec_empty` | PASS |
| 11 | `split_range_start_greater_than_end` | PASS |
| 12 | `split_on_empty_document` | PASS |
| 13 | `split_range_order_preserved` | PASS |
| 14 | `split_original_doc_unchanged` | PASS |
| 15 | `split_zero_page_number` | PASS |
| 16 | `split_spec_with_spaces` | PASS |

---

## 품질 게이트

| 게이트 | 결과 |
|--------|------|
| `cargo test --workspace --exclude rpdf-render` | PASS |
| `cargo clippy --workspace --exclude rpdf-render -- -D warnings` | PASS (경고 0) |
| `cargo fmt --all --check` | PASS |

*참고: `rpdf-render`는 `PDFIUM_DYNAMIC_LIB_PATH` 환경변수 미설정으로 항상 제외 (기존 패턴 유지).*

---

## plan-eng-review 결과

Outside voice (Claude subagent) 5건 발견:

| # | 발견 내용 | 판정 | 처리 |
|---|---------|------|------|
| 1 | `reindex_pages` 접근 우려 | FALSE POSITIVE | 동일 크레이트, `super::reindex_pages` 정상 |
| 2 | 중복/겹치는 범위 정책 미정의 | VALID | 허용 유지 (YAGNI, 엣지 케이스 표에 이미 명시) |
| 3 | `execute()` 0-page 검사 누락 | VALID | execute() 첫 줄에 `doc.pages.is_empty()` 검사 추가 |
| 4 | 공백 처리 미정의 | VALID | `str::trim` 적용, test #16 `split_spec_with_spaces` 추가 |
| 5 | 테스트 5번·13번 중복 | VALID | 테스트명/주석으로 의도 구분 (기능적 문제 없음) |

---

## 회고 분류

| 항목 | 분류 | 내용 |
|------|------|------|
| outside voice가 `execute()` 0-page 검사 누락 발견 | B | execute() 로직 설명이 에러 표와 불일치하는 패턴 — 계획서 작성 시 pseudo-code 로직 설명과 에러 표를 교차 검증해야 함 |
| trim 허용 정책이 계획서에 없었음 | C | 사용자 친화적 입력 처리 정책은 계획서에 명시하는 게 좋음 |
| plan-eng-review에서 발견 → 구현 전 모두 반영 | A (긍정) | 리뷰 단계가 구현 품질을 높임 (15개 → 16개 테스트) |
