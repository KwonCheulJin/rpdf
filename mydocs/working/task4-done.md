# Task #4 — PDF 객체 파서 완료 보고서

**Issue**: M010 / v0.1 Task #4
**브랜치**: `local/task4`
**완료일**: 2026-05-03
**소요 시간**: 계획 미기재 / 실제 3세션

## 완료된 작업

- [x] `rpdf-core/src/types/object.rs` — `PdfObject`, `PdfDict` 타입 정의 (11개 variant)
- [x] `rpdf-parser/src/objects.rs` — `parse_object`, `parse_indirect_object` 공개 API
- [x] 깊이 제한 `MAX_OBJECT_DEPTH = 50` 도입, 초과 시 `ParseError::MaxDepthExceeded`
- [x] `PdfObject::as_dict`, `as_string_bytes`, `as_u64` 등 헬퍼 메서드
- [x] E-1 미니 파서 통합 (옵션 C): `object_parser.rs` 폐기 + `extract_trailer_fields` 단일화
- [x] E-2 통합 테스트 확장: IT-1·IT-3·IT-5에 `parse_indirect_object` 검증 추가
- [x] E-3 IT-5 vacuous pass 수정 + proptest 3개 추가 (panic 0건)
- [x] `cargo clippy -- -D warnings` 경고 없음, `cargo fmt --check` 통과

## 실제 변경 사항

### 새로 추가된 파일
- `crates/rpdf-core/src/types/object.rs` — `PdfObject` (11 variant), `PdfDict`
- `crates/rpdf-parser/src/objects.rs` — 전체 객체 파서 (parse_object·parse_indirect_object·유틸리티)
- `crates/rpdf-parser/tests/parser/objects_tests.rs` — 단위 테스트 71개
- `mydocs/plans/task4-object-parser.md` — 계획서 (517줄)

### 수정된 파일
- `crates/rpdf-core/src/types/mod.rs` — object 모듈 공개
- `crates/rpdf-parser/src/error.rs` — `MaxDepthExceeded` 변형 추가
- `crates/rpdf-parser/src/lib.rs` — `parse_object`, `parse_indirect_object` 공개; `mod object_parser` 제거
- `crates/rpdf-parser/src/trailer.rs` — `parse_dictionary` 활용, `extract_trailer_fields` 추출
- `crates/rpdf-parser/src/xref.rs` — `parse_trailer_dict_fields` 삭제, `extract_trailer_fields` 재사용
- `crates/rpdf-parser/tests/parser/mod.rs` — objects_tests 등록
- `crates/rpdf-parser/tests/parser/integration_tests.rs` — IT-1·IT-3·IT-5 Catalog·Info 검증 추가
- `crates/rpdf-parser/tests/parser/fuzz_tests.rs` — proptest 3개 추가

### 삭제된 파일
- `crates/rpdf-parser/src/object_parser.rs` (209줄) — E-1에서 완전 폐기

## 계획 대비 달라진 점

1. **`extract_trailer_fields` 공통 추출 (계획에 없던 보너스)**
   - 원인: `trailer.rs`와 `xref.rs`가 각각 독립 구현한 동일 로직 발견
   - 조치: `pub(crate) extract_trailer_fields`를 `trailer.rs`로 통합, `xref.rs`에서 import
   - 효과: DRY 달성, 단일 진실 원천 확보

2. **IT-5 검증 필드 변경 (`/Title`·`/Author` → `/Producer`)**
   - 원인: 셀프 리뷰 중 `if let Some` 가드가 한 번도 실행되지 않는 vacuous pass 발견
   - 조치: debug 테스트로 `/Info` dict 키 목록 확인 → `/Producer` 선택
   - 효과: 실제 검증이 이루어지는 의미 있는 어서션

3. **깊이 제한 경계값 명시 검증**
   - 원인: 계획서에는 "50 거부"만 명시, 경계값 49/50/51 동작을 별도 검증
   - 조치: depth=49 통과·depth=50 거부 단위 테스트 추가
   - 효과: off-by-one 위험 제거

## 발견된 이슈

- **u32 초과 객체 번호 처리 정책 분리**
  - `try_parse_reference` (내부 파서): `u32`로 파싱 실패 시 폴백(Reference 불인식)
  - `parse_indirect_object` (공개 API): 명시적 `ParseError::InvalidObjectNumber` 반환
  - 정책이 두 곳에서 다르게 동작하지만 의도적 설계. 문서화함.

- **Stream `Length`가 indirect reference인 경우**
  - `<< /Length 10 0 R >> stream ...` 형식은 현재 거부
  - Task #7 Document IR 해소 영역 — xref로 참조 resolve 후 처리 필요
  - 현재 `ParseError::StreamLengthNotInteger`로 명시적 거부

## 배운 점

### 기술적
- **미니 파서 폐기로 단일 진실 원천 확보**: 두 모듈이 동일 로직을 독립 유지하면 하나가 업데이트될 때 다른 하나가 조용히 달라진다. 공통 추출은 타이밍이 맞을 때 즉시 해야 한다.
- **vacuous pass는 테스트 신뢰도를 파괴한다**: 가드가 한 번도 실행되지 않으면 어서션이 통과해도 아무것도 검증하지 않는다. `if let Some` 패턴은 그 분기가 실제로 실행되는지 확인해야 한다.
- **proptest 0 panic**: 임의 바이트에서 panic이 없다는 것은 C·D 단계의 파서 견고성 확인. 향후 발견 시 즉시 단위 테스트로 고정.

### 프로세스
- **셀프 리뷰가 vacuous pass 발견에 결정적**: IT-5 이슈는 코드를 다시 읽으면서 "이 if let이 실제로 실행되는가?" 질문에서 발견됨. 자동화 테스트는 이 종류의 문제를 잡지 못한다.
- **E-1 범위 확장**: 계획서에는 `trailer.rs`만 마이그레이션으로 명시했으나 `xref.rs`도 6개 import를 가졌음. 실제 소비자를 모두 파악한 뒤 계획을 확정해야 한다.

## 테스트 결과

- 단위 테스트: 27 (B) + 28 (C) + 16 (D) + E 확장 = 71개 신규
- proptest: 3개 신규 (총 5개 누적)
- 통합 테스트: IT-1~IT-6 전체 통과 (IT-1·3·5 Catalog·Info 검증 추가)
- 누적 전체: **165/165 통과**
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과
- 변경 라인: 약 +2,464 (A~D) / E-1~E-3 +208 -407 (net -199) / 총 순증 ~2,265줄

## 다음 관련 작업

- Task #5: xref 스트림 파싱 (`flate2` 도입) — `parse_object`, `parse_dictionary` 활용
  - `parse_xref_stream(data, xref_offset)` 신규 함수
  - W1·W2·W3 배열 파싱에 `parse_object` array 파서 활용 가능
- Task #7: Stream `Length` indirect reference 해소 — Document IR에서 xref lookup 후 스트림 재파싱

## 회고 분류 결과

| # | 항목 | 카테고리 | 적용 위치 | 판단 근거 |
|---|------|---------|---------|---------|
| 1 | E-1 범위 확장: 실제 소비자 사전 파악 | **A** | CLAUDE.md `## 작업 시작 전 체크리스트` | 계획 단계에서 반복 가능한 실수 |
| 2 | vacuous pass 발견 (셀프 리뷰) | **A** | CLAUDE.md `### 체크포인트 시점 셀프 리뷰` 강화 항목 | 프로세스 교훈, 다음 타스크에서 재발 방지 필요 |
| 3 | extract_trailer_fields 공통화 | **C** | — | 코드 수정으로 이미 해결됨 |
| 4 | u32 초과 처리 정책 분리 | **C** | — | 의도적 설계, 문서화 충분 |
| 5 | proptest 0 panic의 의미 | **C** | — | C·D 단계 견고성 확인, 범용 교훈 아님 |

**카테고리 분포**: A 2건, C 3건

**누적 통계**:
- Task #2: A 2건, C 3건
- Task #3: A 2건, C 3건
- Task #4: A 2건, C 3건
- 총: A 6건, C 9건

## 참고 자료

- 계획서: `mydocs/plans/task4-object-parser.md`
