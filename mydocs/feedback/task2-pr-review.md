# Task #2 PR 셀프 리뷰

PR #3 머지 직전 작성한 셀프 리뷰 결과. 발견된 사항은 cherry-pick 또는 Task #3 백로그로 처리됨.

---

## 점검 항목별 결과

### 1. 파일별 핵심 변경 요약

| 파일 | 핵심 변경 | 설명 가능 여부 |
|------|-----------|--------------|
| `trailer.rs` (198줄) | `rposition`으로 마지막 trailer 탐색, xref stream 감지 분기 | ✓ |
| `object_parser.rs` (209줄) | 7개 `pub(crate)` 헬퍼, 중첩 딕셔너리·배열·문자열 처리 | ✓ |
| `integration_tests.rs` | IT-1(전체 연동)~IT-6(xref stream 에러), 실 PDF 5종 사용 | ✓ |

### 2. 테스트 이름 일관성

- **패턴**: `parse_<시나리오>` / `reject_<시나리오>` / `it{N}_<시나리오>` 혼합 — 내부적으로 일관됨
- **발견 1**: `reject_empty_input`이 `eof_tests.rs:93`과 `header_tests.rs:89`에 중복 존재 → 후순위 정리
- **발견 2**: `parse_xref_offset_zero` — 이름이 성공/실패 여부 불명확 → **cherry-pick으로 즉시 처리** (`parse_xref_offset_of_zero_is_returned_as_is`)

### 3. ParseError 변형 위치/값 정보

| 변형 | 정보 포함 여부 | 비고 |
|------|--------------|------|
| `HeaderNotFound` | `searched_bytes` ✓ | |
| `InvalidVersion` | `offset + found` ✓ | |
| `InvalidStartXref` / `InvalidObjectRef` | `found` ✓ | |
| `MalformedTrailer` | `reason` ✓ | |
| `MissingRequiredKey` | `key` ✓ | |
| `TrailerTooLarge` | `limit_kb` △ | `found_bytes` 없음 — Task #3 백로그 |
| `XrefStreamUnsupported` | 없음 △ | `xref_offset` 추가 시 Task #3 디버깅 유용 — **우선순위 높음** |
| `MissingEof` / `MissingStartXref` / `MissingTrailer` | 없음 △ | `searched_range` 추가 가능 — 후순위 |

### 4. 문서 일관성

| 점검 항목 | 결과 |
|---------|------|
| 미니 파서 결정 (3개 문서) | ✓ 모두 반영 |
| `parse_trailer` 시그니처 변경 | ✓ 모두 반영 |
| IT-1 파일 (fw4 → tracemonkey) | △ 계획서 미반영 → **cherry-pick으로 처리** |
| 테스트 수 (30개 → 70개) | △ 계획서 미반영 → **cherry-pick으로 처리** |
| `InvalidVersion` 필드 추가 | △ 계획서 미반영 → **cherry-pick으로 처리** |
| 중복 계획서 (`task2-pdf-header-trailer.md`) | **cherry-pick으로 삭제** |

---

## 처리 결과

### cherry-pick으로 즉시 처리 (3개 커밋, devel 반영 완료)

1. `docs: Task #2 중복 계획서 정리` — `task2-pdf-header-trailer.md` 삭제
2. `docs: Task #2 셀프 리뷰 반영` — 테스트 이름 개선 + working 보고서 3건 보강
3. `docs: Task #3 시작 시 보강할 ParseError 목록 메모` — v0.1-parser-skeleton.md 에 기록

### Task #3 백로그 (v0.1-parser-skeleton.md 에 기록)

- `XrefStreamUnsupported { xref_offset: u64 }` [우선순위 높음]
- `TrailerTooLarge { found_bytes: usize, limit_kb: u32 }` [디버깅 보조]
- `reject_empty_input` 이름 중복 정리 [낮음]
- `MissingEof` / `MissingStartXref` / `MissingTrailer` `searched_range` [후순위]

---

## 분류 통계

- 필수 처리: 1건 (중복 계획서 — 머지 후 굳어짐 방지)
- 권장 처리: 2건 (테스트 이름, working 보고서 누락 3항목)
- 후순위 (메모): 4건 (에러 변형 보강, 테스트 이름 중복)
- **합계**: 7건
