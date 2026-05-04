# Task #15 — SVG 디버그 오버레이 완료 보고서

**Issue**: #28  
**브랜치**: `local/task15`  
**완료일**: 2026-05-04  
**소요 시간**: 계획 ~3시간 / 실제 ~2시간

## 완료된 작업

계획서에 정의된 완료 기준 대비 결과:

- [x] 기준 1: `rpdf-svg` 공개 API 추가: `render_page_svg_with_options(page: &Page, opts: &RenderOptions) -> String`
- [x] 기준 2: 기존 `render_page_svg()` 함수 동작 불변 (IT-D3으로 동등성 검증)
- [x] 기준 3: `rpdf render <pdf> --svg --debug-overlay` CLI 동작
- [x] 기준 4: 오버레이 요소 3종 — 페이지 경계 사각형, 100pt 간격 좌표 그리드, 원점 (0,0) 마커
- [x] 기준 5: `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과

## 실제 변경 사항

### 새로 추가된 파일
- `crates/rpdf-svg/src/overlay.rs` — `build_overlay(w, h) -> String` 함수 + 인라인 단위 테스트 6개
- `mydocs/plans/task15-svg-debug-overlay.md` — 계획서

### 수정된 파일
- `crates/rpdf-svg/src/lib.rs` — `RenderOptions` 타입 + `render_page_svg_with_options()` 추가, `render_page_svg()` 위임으로 변경
- `crates/rpdf-svg/tests/svg_render_tests.rs` — IT-D1~D3 통합 테스트 3개 추가
- `crates/rpdf-cli/src/main.rs` — `--debug-overlay` 플래그 파싱 추가
- `crates/rpdf-cli/src/commands/render.rs` — `RenderParams.debug_overlay` 추가, 경고 처리, `run_svg()` 옵션 전달
- `crates/rpdf-cli/tests/render_tests.rs` — IT-D4~D5 CLI 통합 테스트 2개 추가

## 계획 대비 달라진 점

없음. 계획서 명세와 100% 일치. D1·D2 결정 사항 모두 반영.

## 발견된 이슈

없음. evaluator 검증에서 Critical/Important 항목 없음.

## 배운 점

### 기술적
- SVG 오버레이를 y-flip 그룹 **바깥**에 배치하면 PDF 좌표 변환 없이 SVG 좌표 직접 사용 가능 — 오버레이 요소 좌표 계산이 단순해짐
- `while x < w` 패턴은 소형 페이지에서 자연스럽게 그리드 없음 처리 — 별도 조건 분기 불필요

### 프로세스
- D1/D2 결정을 계획서에 명시한 덕분에 구현 시 edge case 누락 없음 — plan-eng-review가 구현 품질에 직접 기여

## 테스트 결과

- rpdf-svg 단위 테스트: 26/26 통과 (기존 20 + 신규 6)
- rpdf-svg 통합 테스트: 8/8 통과 (기존 5 + 신규 3)
- rpdf-cli 통합 테스트: 9/9 통과 (기존 7 + 신규 2)
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과

## 회귀 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | 오버레이를 y-flip 그룹 바깥에 배치하는 SVG 구조 패턴 | **C: 스킵** | 이번 구현 전용 설계 결정, 일반화 규칙으로 가치 없음 |
| 2 | while x < w 패턴으로 소형 페이지 자동 처리 | **C: 스킵** | 당연한 Rust 이터레이션 패턴, 별도 규칙 불필요 |

## 다음 관련 작업

- Task #16: 여러 페이지 SVG 출력

## 참고 자료

- 계획서: `mydocs/plans/task15-svg-debug-overlay.md`
