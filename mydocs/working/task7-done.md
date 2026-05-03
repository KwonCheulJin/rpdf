# Task #7 — Content Stream 파서 완료 보고서

**Issue**: M010 / v0.1 Task #7 (#12)
**브랜치**: `local/task7`
**완료일**: 2026-05-03
**소요 시간**: 계획 1세션 / 실제 1세션

## 완료된 작업

계획서 완료 기준 대비 결과:

- [x] `ContentStreamOperator` enum — 73개 시맨틱 이름 변형 + `Unknown(Vec<u8>)`
- [x] `ContentStreamOperation { operator, operands, inline_data }` 타입
- [x] `parse_content_stream(data: &[u8]) -> Result<Vec<ContentStreamOperation>, ParseError>`
- [x] 2단계 토큰화 (`Token::Keyword` 저장 → `keyword_to_operator` 변환)
- [x] 인라인 이미지 (`BI...ID...EI`) → `InlineImage` 연산자 + `inline_data`
- [x] q/Q 그래픽 상태 스택 검증 (Option B: 즉각 에러 + EOF 에러)
- [x] `Unknown(Vec<u8>)` 보존 (에러 아님)
- [x] 에러 변형 3개: `MalformedContentStream`, `MalformedInlineImage`, `UnbalancedGraphicsState`
- [x] IT-11: 합성 content stream 통합 테스트 (21개 연산자, 모든 그룹 포함)
- [x] IT-12: 실제 PDF (fw4-2024.pdf) — 228개 연산자, 3개 오프셋 검증
- [x] proptest — `arbitrary_input_never_panics_parse_content_stream` 1종
- [x] `cargo clippy -- -D warnings` 경고 없음, `cargo fmt --check` 통과

## 실제 변경 사항

### 새로 추가된 파일

- `crates/rpdf-core/src/types/content_stream.rs` (~160줄) — 도메인 타입
- `crates/rpdf-parser/src/content_stream.rs` (~600줄) — 파서 + 29개 단위 테스트
- `crates/rpdf-parser/tests/parser/content_stream_tests.rs` — 공개 API 테스트 1개
- `crates/rpdf-parser/examples/scan_content_stream.rs` — D-2 사전 확인 진단 바이너리

### 수정된 파일

- `crates/rpdf-core/src/types/mod.rs` — `ContentStreamOperation`, `ContentStreamOperator` pub 노출
- `crates/rpdf-parser/src/error.rs` — 에러 변형 3개 추가
- `crates/rpdf-parser/src/lib.rs` — `parse_content_stream` pub 노출
- `crates/rpdf-parser/tests/parser/mod.rs` — `mod content_stream_tests` 등록
- `crates/rpdf-parser/tests/parser/integration_tests.rs` — IT-11, IT-12 신규
- `crates/rpdf-parser/tests/parser/fuzz_tests.rs` — proptest 1개 추가

### 삭제된 파일

- 없음

## 계획 대비 달라진 점

1. **체크포인트 분할 (D-2 독립 추출)**
   - 원인: 실제 PDF 스캔 결과(연산자 수 228, 첫 연산자 `BeginMarkedContentProp`)를 확인해야 IT-12 기대값을 설정 가능
   - D-2를 별도 진단 체크포인트로 분리 → 계획서보다 세분화됨

2. **`examples/scan_content_stream.rs` 추가**
   - 원인: fw4-2024.pdf 실제 content stream 구조를 `parse_content_stream`으로 사전 확인하기 위해 진단 바이너리 필요
   - 효과: D-2 스캔 후 IT-12 기대값 근거 확보

3. **IT-11 인라인 이미지 operands 수 수정 (12 → 8)**
   - 원인: `/W 4 /H 2 /CS /G /BPC 8` = 4쌍 = 8개 operand. 초기 주석 오산
   - 수정: 셀프 리뷰에서 즉시 발견·수정

## 발견된 이슈 및 해결

### CRITICAL: `parse_object` 반환값 오용

- **현상**: `next_token`이 `parse_object(data, pos)` 반환값을 절대 위치로 반환 → 토크나이저가 역방향 이동
- **근본 원인**: `parse_object`의 두 번째 반환값은 `consumed` (상대 바이트 수), 절대 위치 아님
  - 정확한 계약: `Ok((obj, bytes_consumed_from_offset))`, 다음 절대 위치 = `pos + consumed`
- **수정**: `Ok(Some((Token::Operand(obj), consumed)))` → `Ok(Some((Token::Operand(obj), pos + consumed)))`
- **감지**: `graphics_state_operators`, `move_show_text_operators` 단위 테스트에서 `UnbalancedGraphicsState` 오류로 즉시 노출

### q/Q offset 계산 오류

- **현상**: `keyword_start` 위치 계산에 `saturating_sub` 사용 → 정밀도 부족
- **수정**: `keyword_start = next_pos - keyword.len()` (next_pos 확보 후 역산)

### 인라인 이미지 operand 수 오산 (단위 + 통합 테스트 2건)

- 단위 테스트 `inline_image_basic`: 12 → 8
- IT-11: 10 → 8

## 배운 점

### 기술적

- **`parse_object` API 계약 명확화**: 반환값 두 번째 원소는 절대 위치가 아닌 소비된 바이트 수(상대값). `next_token`처럼 래퍼에서 호출할 때 반드시 `pos + consumed`로 절대 위치 복원 필요. 이 계약은 `parse_object_with_depth` 구현(`ws = pos - offset; Ok((obj, ws + consumed))`)에서 확인.
- **인라인 이미지 EI 탐지**: PDF spec §8.9.7의 whitespace-prefix 규칙 — 이미지 데이터 내 `EI` 오탐 방지를 위해 `<whitespace>EI<non-keyword>` 패턴으로 종료 감지.
- **D-2 사전 확인이 IT-12 품질을 높인다**: 실제 PDF에서 연산자 수·오프셋을 미리 확인하면 기대값을 근거 있게 설정 가능.

### 프로세스

- **두 번째 반환값 계약 주석**: 공개 API 반환 계약이 직관과 다를 경우 호출 시점마다 혼란 발생. 계약을 함수 doc comment에 명시하면 미래 호출자 실수 방지.
- **operand 수 직접 계산 습관**: 테스트에서 operand 수를 `n쌍 × 2`로 암산하면 오류 발생. 명시적으로 열거해 검증 필요.

## 테스트 결과

- 단위 테스트 (content_stream.rs 내부): **29개 신규**
- 외부 공개 API 테스트: **1개 신규** (content_stream_tests.rs)
- 통합 테스트: IT-11 (합성 21연산자), IT-12 (실제 fw4-2024.pdf 228연산자) = **2개 신규**
- proptest: `arbitrary_input_never_panics_parse_content_stream` — panic 0 = **1개 신규**
- **신규 합계: 33개**
- **누적 전체: 259/259 통과** (1 core + 82 parser unit + 176 parser external)
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과
- 변경 라인: 약 +1,000

## 다음 관련 작업

- Task #8: Document IR — Catalog → Pages → Page 트리 탐색, `parse_content_stream` 활용

## 회고 분류 결과

| # | 항목 | 카테고리 | 적용 위치 | 판단 근거 |
|---|------|---------|---------|---------|
| 1 | `parse_object` 반환값 계약 — 상대 바이트 수 | **A** | CLAUDE.md `### 외부 입력 검증` 보강 또는 별도 API 주의사항 항목 | 반복 발생 가능. 동일 API를 Task #8에서도 사용 |
| 2 | D-2 사전 확인 → IT-12 근거 있는 기대값 | **B** | `mydocs/tech/test-synthetic-data.md` | Task #6에서도 동일 전략 검증됨 — 표준 절차 확정 |
| 3 | 인라인 이미지 EI whitespace-prefix 규칙 | **B** | `mydocs/tech/content-stream-parser.md` (신규) | PDF spec §8.9.7 구현 세부사항. 향후 유지보수자 참고 |
| 4 | operand 수 직접 열거 검증 | **C** | — | 범용 교훈, 코드 관례 아님 |
| 5 | proptest panic 0건 | **C** | — | 범용 교훈 아님 |

**카테고리 분포**: A 1건, B 2건, C 2건

**누적 통계**:

| 태스크 | A | B | C |
|--------|---|---|---|
| #2 | 2 | 0 | 3 |
| #3 | 2 | 0 | 3 |
| #4 | 2 | 0 | 3 |
| #5 | 2 | 1 | 3 |
| #6 | 1 | 2 | 2 |
| #7 | 1 | 2 | 2 |
| **총** | **10** | **5** | **16** |

## 참고 자료

- 계획서: `mydocs/plans/task7-content-stream-parser.md`
- 진단 바이너리: `crates/rpdf-parser/examples/scan_content_stream.rs`
