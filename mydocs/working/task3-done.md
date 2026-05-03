# Task #3 — Xref 테이블 파싱 완료 보고서

**Issue**: M010 / v0.1 Task #3
**브랜치**: `local/task3`
**최종 커밋**: `96925a0`
**완료일**: 2026-05-03

## 완료된 작업

- [x] `rpdf-core/src/types/xref.rs` — `XrefTable`, `XrefEntry` 타입 정의 (`pub(crate)` 캡슐화)
- [x] `rpdf-parser/src/error.rs` — `ParseError` 신규 변형 5개 + `XrefStreamUnsupported` 필드 추가
- [x] `rpdf-parser/src/xref.rs` — `parse_xref` 전체 구현 (단일 섹션 + `/Prev` chain 순회)
- [x] `rpdf-parser/src/lib.rs` — `ParsedXref`, `XrefSectionInfo`, `parse_xref` 공개
- [x] 에러 변형 6개 모두 단위 테스트로 도달 가능성 확인
- [x] chain 단위 테스트 5개 + 회귀 테스트 2개
- [x] IT-1·IT-3·IT-5에 `parse_xref` 검증 추가
- [x] IT-6: `parse_trailer`·`parse_xref` 양쪽 모두 `XrefStreamUnsupported` 반환 확인
- [x] `arbitrary_input_never_panics_parse_xref` proptest 추가
- [x] `cargo clippy -- -D warnings` 경고 없음, `cargo fmt --check` 통과

## 실제 변경 사항

### 새로 추가된 파일
- `crates/rpdf-core/src/types/xref.rs` — `XrefTable`, `XrefEntry`
- `crates/rpdf-parser/src/xref.rs` — 전체 xref 파싱 로직
- `crates/rpdf-parser/tests/parser/xref_tests.rs` — 단위 테스트 20개
- `mydocs/troubleshootings/xref-chain-check-order.md` — chain 검사 순서 버그 트러블슈팅

### 수정된 파일
- `crates/rpdf-core/src/types/mod.rs` — xref 모듈 공개
- `crates/rpdf-parser/src/error.rs` — ParseError 보강
- `crates/rpdf-parser/src/lib.rs` — parse_xref 공개
- `crates/rpdf-parser/src/trailer.rs` — `is_xref_stream` pub(crate) 승격
- `crates/rpdf-parser/tests/parser/mod.rs` — xref_tests 등록
- `crates/rpdf-parser/tests/parser/integration_tests.rs` — IT-1~IT-6 업데이트
- `crates/rpdf-parser/tests/parser/fuzz_tests.rs` — proptest 추가
- `mydocs/plans/task3-xref-parser.md` — `MalformedXref.reason: String` 설계 결정 반영

### 삭제된 파일
- 없음 (기존 inline `#[cfg(test)]` 블록을 별도 파일로 이동)

## 계획 대비 달라진 점

1. **버그 발견 및 수정 (Checkpoint B 리뷰)**: `parse_xref_chain`에서 `depth` 검사와 `visited` 검사 순서 오류
   - 원인: 100개의 고유 오프셋으로 구성된 순환 chain에서 `XrefChainCycle` 대신 `XrefChainTooDeep`이 반환됨
   - 수정: `visited` 검사를 먼저 수행하도록 순서 변경
   - 회귀 테스트 2개 추가 (계획서 원래 목표 23개 → 20개이나 회귀 테스트로 보완)

2. **`consume_newline` 엄격화**: 계획서에는 `\r\n`·`\n` 허용으로만 명시했으나, 구현 중 `\r` 단독 처리를 명시적으로 거부하는 것이 더 안전함을 확인 → Option A 선택

3. **테스트 위치 변경**: 계획서에는 inline `#[cfg(test)]`를 암묵적으로 허용했으나, 사용자 검토에서 별도 파일(`xref_tests.rs`) 분리 요청 → 변경 반영

## 발견된 이슈

- **`TrailerTooLarge { found_bytes }` 보강** (별도 Issue 등록 예정): Task #2 범위였으나 Task #3에서도 미처리. 낮은 우선순위 백로그.

## 배운 점

### 기술적
- **에러 우선순위가 있는 루프**: 검사 순서가 의미론적으로 중요하다. "우선순위가 높은 에러를 먼저 검사"가 원칙. 경계값(`depth == MAX`)에서의 동작을 명시적 테스트로 검증해야 한다.
- **순환 참조 vs 깊이 초과**: visited 집합 검사를 depth 검사보다 먼저 수행해야 "순환 chain은 항상 `XrefChainCycle`"이라는 명세가 보장된다.
- **`build_chain` 반복 수렴**: 섹션 크기가 오프셋(자릿수)에 의존하므로, 테스트용 chain 데이터는 고정점 반복(10회)으로 안정적인 오프셋을 계산해야 한다.

### 프로세스
- Checkpoint B 단계에서 자가 검토를 통해 설계 명세 위반 버그를 발견함 — 구현 직후 검토가 효과적
- 에러 변형 도달 가능성은 경계값에서 특히 취약하다 (CLAUDE.md 원칙 재확인)

## 테스트 결과

- 단위 테스트: 91/91 통과 (xref 관련 20개 포함)
- proptest: `arbitrary_input_never_panics` + `arbitrary_input_never_panics_parse_xref` 통과
- 통합 테스트: IT-1~IT-6 모두 통과
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과

## 다음 관련 작업

- Task #4: `object_parser.rs` 확장 (실수, 스트림, boolean, null) — `XrefTable::get`으로 오프셋 확인 후 객체 파싱
- Task #5: xref 스트림 파싱 (`flate2` 도입) — Task #4 객체 파서 활용

## 회고 분류 결과

| # | 항목 | 카테고리 | 적용 위치 | 비고 |
|---|------|---------|---------|-----|
| 1 | 테스트 파일 배치 | **A** | CLAUDE.md `## 설계 원칙 > ### 테스트 파일 배치` | |
| 2 | xref chain 검사 순서 버그 | **C** | — | 트러블슈팅 문서 작성됨 |
| 3 | `consume_newline` 엄격화 | **C** | — | 코드 수정으로 해결 |
| 4 | 체크포인트 시점 셀프 리뷰 | **A** | CLAUDE.md `## 설계 원칙 > ### 체크포인트 시점 셀프 리뷰` | C → A 재분류 |
| 5 | `build_chain` 반복 수렴 | **C** | — | 일반화 가치 낮음 |

**카테고리 분포**: A 2건, C 3건

**누적 통계**:
- Task #2: A 2건, C 3건
- Task #3: A 2건, C 3건
- 총: A 4건, C 6건

## 참고 자료

- 트러블슈팅: `mydocs/troubleshootings/xref-chain-check-order.md`
- 계획서: `mydocs/plans/task3-xref-parser.md`
