# Task #5 — Xref 스트림 파싱 완료 보고서

**Issue**: M010 / v0.1 Task #5 (#8)
**브랜치**: `local/task5`
**PR**: TBD (E-4에서 생성)
**완료일**: 2026-05-03
**소요 시간**: 계획 2–3세션 / 실제 3세션

## 완료된 작업

계획서 완료 기준 대비 결과:

- [x] `parse_xref_stream(data, xref_offset)` 구현 — `FlateDecode` + PNG 예측 필터 10–15 지원
- [x] `parse_xref_chain` 알고리즘 업데이트 — 전통 xref/xref 스트림 Hybrid Chain 분기
- [x] `parse_xref_section` 시그니처에 `XrefSectionInfo` 추가 반환 (A 단계)
- [x] 에러 변형 8개 추가 (+ W 배열 너비 검증 1개 보강 = 실질 9개 경우)
- [x] `XrefEntry::Compressed` 실제 생성 (타입 2 엔트리)
- [x] IT-6 강화 + IT-7/IT-8 신규 통합 테스트
- [x] 내부 proptest 4종 추가 (`parse_xref_stream`, `decompress_flate`, `decode_entries`, `unpredict_png`) — 1,000회 패닉 0
- [x] `cargo clippy -- -D warnings` 경고 없음, `cargo fmt --check` 통과

## 실제 변경 사항

### 새로 추가된 파일

- `crates/rpdf-parser/src/xref_stream.rs` — 전체 xref 스트림 파서 (~700줄)
- `mydocs/troubleshootings/xref-entry-format-spaces.md` — xref 엔트리 포맷 헬퍼 버그

### 수정된 파일

- `Cargo.toml` (workspace) — `flate2 = "1.1"` 추가
- `crates/rpdf-parser/Cargo.toml` — `flate2.workspace = true` 추가
- `crates/rpdf-parser/src/error.rs` — 에러 변형 8개 추가
- `crates/rpdf-parser/src/lib.rs` — `mod xref_stream` 추가 (비공개)
- `crates/rpdf-parser/src/xref.rs` — `parse_xref_chain` Hybrid Chain 분기, `parse_xref_section` 시그니처 통일
- `crates/rpdf-parser/tests/parser/integration_tests.rs` — IT-6 강화, IT-7·IT-8 신규
- `crates/rpdf-parser/tests/parser/xref_tests.rs` — hybrid chain 테스트 2개 + 회귀 테스트 1개
- `examples/README.md` — xref 포맷 분류 설명 추가

### 삭제된 파일

- 없음

## 계획 대비 달라진 점

1. **W 필드 너비 명시적 검증 추가 (E 진입 전 보강)**
   - 원인: `W` 배열 각 원소가 8 이하여야 `u64` 읽기 시 silent wrap-around가 발생하지 않음
   - 조치: `decode_entries` 진입 시 각 W 값 > 8이면 `XrefStreamInvalidW` 반환
   - 효과: 악의적 W 값으로 인한 데이터 오염 방지 (보안 강화)

2. **내부 proptest 위치 변경 (fuzz_tests.rs → xref_stream.rs 인라인)**
   - 계획: `fuzz_tests.rs`에 `arbitrary_input_never_panics_parse_xref_stream` 1개
   - 구현: `xref_stream.rs` 내 `internal_tests` 모듈에 4개 (`parse_xref_stream`, `decompress_flate`, `decode_entries`, `unpredict_png`) — pub(crate) 함수 직접 호출 필요
   - 효과: 더 세밀한 견고성 검증 (4개 > 1개)

3. **IT-8: 합성 hybrid PDF 사용**
   - 원인: fw4-2024.pdf의 실제 `/Prev` chain이 없어 합성 데이터 필요
   - 조치: `make_hybrid_pdf_for_it8()` 헬퍼로 전통 xref + xref 스트림 혼재 PDF 생성
   - 효과: 실제 파일에서 발견 불가한 엣지 케이스 직접 검증 가능

4. **XrefStreamUnsupported 도달 가능성 재확인**
   - `parse_trailer` 역방향 탐색에서 여전히 반환됨 → 유지 (dead variant 아님)
   - `parse_xref_chain`에서는 더 이상 반환하지 않음 (IT-6 이후)

## 발견된 이슈

- **xref 엔트리 포맷 헬퍼 버그** (구현 범위 밖)
  - `parse_xref_entry` 헬퍼 테스트 생성 함수가 `\r\n` 대신 ` \r\n` 출력
  - 생산 코드 경로에는 영향 없음 (헬퍼 함수만 해당)
  - 회귀 테스트 `reject_malformed_entry_space_before_cr_lf` 추가, troubleshooting 문서 작성
  - 참고: `mydocs/troubleshootings/xref-entry-format-spaces.md`

## 배운 점

### 기술적

- **silent wrap-around는 YAGNI가 아닌 보안 문제**: `W` 필드 너비가 8 초과이면 u64 읽기 시 바이트가 조용히 잘린다. 미래 필요성이 아닌 현재의 정확성 보장이므로 반드시 검증해야 한다.
- **합성 테스트 데이터가 실제 데이터보다 엣지 케이스 발견에 효과적**: 실제 PDF는 항상 "정상" 구조를 가지지만 합성 데이터는 의도적 경계값을 만들 수 있다. 실제 파일이 없는 시나리오에서 합성 데이터 헬퍼 작성은 테스트 품질을 높인다.
- **flate2의 순수 Rust 구현**: `miniz_oxide` 기반, 시스템 라이브러리 의존 없음 → Tauri 배포 환경에서 깔끔히 동작.

### 프로세스

- **pub(crate) 함수 테스트 위치**: external 테스트 파일은 crate 공개 API만 접근 가능하므로, pub(crate) 함수 직접 테스트는 해당 모듈의 `internal_tests` 내에서 진행한다.
- **proptest를 단위 함수 단위로 추가**: xref_stream.rs처럼 새 파싱 모듈 추가 시, fuzz_tests.rs에 1개 추가하는 것보다 각 핵심 함수별로 internal proptest를 추가하는 것이 더 세밀한 검증을 제공한다.

## 테스트 결과

- 내부 단위 테스트 (xref_stream.rs): 27개 신규 + proptest 4개 신규 = **31개 신규**
- 외부 테스트 (+5):
  - xref_tests.rs: +3 (hybrid chain 2개 + 회귀 1개)
  - integration_tests.rs: +2 (IT-7 신규, IT-8 신규) + IT-6 강화
- 누적 외부 테스트: **165 → 170**
- 전체 cargo test 합계: **202/202 통과** (1 + 31 + 170)
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과
- 변경 라인: 약 +1,800

## 다음 관련 작업

- Task #6: Content Stream 파서 — `parse_object`, `parse_dictionary` 재활용 가능
- Task #7: Document IR — `XrefEntry::Compressed` 가 가리키는 `/Type /ObjStm` 실제 추출
- Task #7: `StreamLengthNotInteger` — indirect reference Length 해소 (백로그)

## 회고 분류 결과

| # | 항목 | 카테고리 | 적용 위치 | 판단 근거 |
|---|------|---------|---------|---------|
| 1 | W 필드 silent wrap-around: YAGNI 거부 | **A** | CLAUDE.md `### 에러 변형 도달 가능성` 항목 보강 | 정수 크기 초과로 인한 silent 동작 변화는 현재의 정확성 문제, 반복 발생 가능 |
| 2 | parse_xref_section 시그니처 통일 결정 | **C** | — | 코드로 이미 해결됨, 특별한 재발 가능성 없음 |
| 3 | 합성 IT 데이터가 실제 데이터보다 엣지 케이스 발견 효과적 | **B** | `mydocs/tech/test-synthetic-data.md` (신규) | 다음 Task에서 반복 활용 가능한 전략, 코드로는 표현 불가 |
| 4 | xref 엔트리 포맷 헬퍼 버그 | **C** | — | 이미 troubleshooting 문서 + 회귀 테스트로 처리됨 |
| 5 | proptest panic 0건의 의미 | **C** | — | 범용 교훈 아님 (Task #4와 동일 판단) |
| 6 | test 위치 결정 (pub(crate) → internal_tests) | **A** | CLAUDE.md `### 테스트 파일 배치` 항목 보강 | 공개 API / private / pub(crate) 3가지 케이스 명확화 필요, 반복 발생 가능 |

**카테고리 분포**: A 2건, B 1건, C 3건

**누적 통계**:

| 태스크 | A | B | C |
|--------|---|---|---|
| #2 | 2 | 0 | 3 |
| #3 | 2 | 0 | 3 |
| #4 | 2 | 0 | 3 |
| #5 | 2 | 1 | 3 |
| **총** | **8** | **1** | **12** |

## 참고 자료

- 계획서: `mydocs/plans/task5-xref-stream-parser.md`
- 트러블슈팅: `mydocs/troubleshootings/xref-entry-format-spaces.md`
