# Task #2 — PDF 헤더·trailer 파서 완료 보고서

**Issue**: v0.1 Task #2
**브랜치**: `dev`
**완료일**: 2026-05-03
**소요 시간**: 2세션

## 완료된 작업

- [x] `parse_header` — `%PDF-` 시그니처 탐색, 버전 파싱, 바이너리 마커 감지
- [x] `parse_startxref` — `%%EOF` 앞 `startxref` 키워드 + 오프셋 값 파싱
- [x] `find_eof` — 파일 끝 `%%EOF` 위치 탐색
- [x] `parse_trailer` — trailer 키워드 역방향 탐색, 딕셔너리 파싱 (`/Size`, `/Root`, `/Info`, `/Prev`)
- [x] `PdfTrailer`, `ParsedTrailer` 타입 정의
- [x] 단위 테스트 63개 (eof 14 + header 18 + startxref 12 + trailer 19)
- [x] 통합 테스트 6개 (실제 PDF 파일 사용)
- [x] proptest 1개 (`arbitrary_input_never_panics`)
- [x] `object_parser.rs` 모듈 분리 (Task #4 기반)

## 실제 변경 사항

### 새로 추가된 파일

- `crates/rpdf-parser/src/object_parser.rs` (160줄) — 미니 객체 파서 (Task #4에서 확장 예정)
- `crates/rpdf-parser/src/trailer.rs` — `parse_trailer` 구현
- `crates/rpdf-parser/tests/parser/trailer_tests.rs` (310줄) — 19개 단위 테스트
- `crates/rpdf-parser/tests/parser/integration_tests.rs` (130줄) — 6개 통합 테스트
- `crates/rpdf-parser/tests/parser/fuzz_tests.rs` (16줄) — proptest
- `mydocs/troubleshootings/lopdf-parser-api-not-public.md`

### 수정된 파일

- `crates/rpdf-parser/src/lib.rs` — `object_parser` 모듈 추가, `parse_trailer` 공개
- `crates/rpdf-parser/src/error.rs` — `MalformedTrailer`, `TrailerTooLarge` 에러 변형 추가
- `crates/rpdf-parser/tests/parser/mod.rs` — 새 테스트 모듈 등록

## 계획 대비 달라진 점

1. **lopdf 딕셔너리 파서 위임 → 자체 미니 파서**
   - 이유: `lopdf::parser` 모듈이 `pub(crate)` 가시성으로 외부 접근 불가
   - 해결: `object_parser.rs`에 160줄 미니 파서 자체 구현
   - 영향: `Cargo.toml`에 lopdf 의존성 없음, Task #4에서 확장 예정
   - 기록: `mydocs/troubleshootings/lopdf-parser-api-not-public.md`

2. **SEARCH_WINDOW = 8192 (계획: 4096)**
   - 이유: `SEARCH_WINDOW == DICT_MAX_BYTES`이면 `TrailerTooLarge` 에러가 도달 불가능 (dead code)
   - 해결: SEARCH_WINDOW를 DICT_MAX_BYTES의 2배로 설정

3. **IT-1 사용 파일 변경: `fw4-2024.pdf` → `pdfjs-tracemonkey.pdf`**
   - 이유: `fw4-2024.pdf`는 xref stream 방식(PDF 1.7)이라 IT-6 전용으로 배정.
     IT-1(전체 연동 통합 테스트)에는 trailer 파싱이 가능한 `pdfjs-tracemonkey.pdf`(PDF 1.4) 사용

4. **실제 테스트 수: 계획 30개 이상 → 실제 70개**
   - 이유: 단위 테스트가 19개(trailer) + 18개(header) + 14개(eof) + 12개(startxref) = 63개로 계획 23개보다 많아짐
   - 통합 테스트 6개 + proptest 1개로 총합 70개

5. **`InvalidVersion` 에러 변형 필드 추가: `{ found }` → `{ offset, found }`**
   - 이유: 헤더 오프셋이 0이 아닌 파일에서 잘못된 버전 위치를 즉시 파악하기 위해 `offset` 필드 추가
   - 계획서에는 `found` 필드만 정의되어 있었음

## 발견된 이슈

- xref stream 방식 PDF (PDF 1.5+, `fw4-2024.pdf` 등)는 현재 `XrefStreamUnsupported` 반환
  → Task #3에서 처리 예정

## 배운 점

### 기술적

- 외부 라이브러리의 내부 API에 의존하기 전 docs.rs에서 공개 범위 먼저 확인 필수
- 에러 변형이 도달 가능한지 코드 경로를 항상 추적해야 함 (dead error variant 문제)
- `rposition`으로 마지막 `trailer` 키워드를 찾아야 점진적 업데이트 PDF가 올바르게 처리됨

### 프로세스

- TDD: 테스트 19개를 먼저 작성하고 구현 진입 → 설계 명확화에 큰 도움
- 통합 테스트용 실제 PDF를 미리 `examples/`에 준비해두면 빠른 검증 가능

## 테스트 결과

- 단위 테스트: 70/70 통과
  - eof: 14, header: 18, startxref: 12, trailer: 19, integration: 6, fuzz: 1
- proptest: 실행 성공, panic 없음
- `cargo clippy -- -D warnings`: 경고 0
- `cargo fmt --check`: 통과

## 다음 관련 작업

- Task #3: xref 테이블 파서 (`parse_xref`)
- Task #4: 전체 PDF 객체 파서 (`object_parser.rs` 확장)

## 참고 자료

- 트러블슈팅: `mydocs/troubleshootings/lopdf-parser-api-not-public.md`
- ISO 32000-1:2008 §7.5.5 (File Trailer), §7.5.8 (Cross-Reference Streams)
