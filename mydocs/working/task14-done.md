# Task #14 — SVG 렌더러 완료 보고서

**Issue**: #26  
**브랜치**: `local/task14`  
**완료일**: 2026-05-04  
**소요 시간**: 계획 ~4시간 / 실제 ~3시간

## 완료된 작업

계획서에 정의된 완료 기준 대비 결과:

- [x] 기준 1: 신규 크레이트 `rpdf-svg` 추가. 공개 API `render_page_svg(page: &Page) -> String` 구현
- [x] 기준 2: `examples/` 5개 PDF 각 첫 페이지 → SVG 변환 성공 (유효한 `<svg>` 루트 포함)
- [x] 기준 3: `rpdf render <pdf> --svg [-o output.svg]` CLI 명령 동작
- [x] 기준 4: 17개 지원 연산자 전부 구현 (MoveTo, LineTo, CurveTo, CurveToV, CurveToY, ClosePath, Rect, Stroke, Fill, FillStroke, SetFillRGB, SetStrokeRGB, ShowText, SetTextMatrix, ConcatMatrix, SaveState, RestoreState)
- [x] 기준 5: `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과

## 실제 변경 사항

### 새로 추가된 파일
- `crates/rpdf-svg/Cargo.toml` — rpdf-core 의존성
- `crates/rpdf-svg/src/lib.rs` (356줄) — render_page_svg() 진입점, content stream 디스패치
- `crates/rpdf-svg/src/state.rs` — Color, GraphicsState, StateStack + 단위 테스트 7개
- `crates/rpdf-svg/src/path.rs` — PathBuilder (경로 연산자 → SVG d 문자열) + 단위 테스트 8개
- `crates/rpdf-svg/src/text.rs` — TextState (텍스트 연산자 → SVG text 요소) + 단위 테스트 5개
- `crates/rpdf-svg/tests/svg_render_tests.rs` — 통합 테스트 5개 (IT-S1~S5)
- `mydocs/plans/task14-svg-renderer.md` — 계획서 (plan-eng-review D1~D4 결정 반영)

### 수정된 파일
- `Cargo.toml` (workspace) — `rpdf-svg` 멤버 및 workspace dependency 추가
- `crates/rpdf-cli/Cargo.toml` — `rpdf-svg` 의존성 추가
- `crates/rpdf-cli/src/main.rs` — `--svg` 플래그 추가
- `crates/rpdf-cli/src/commands/render.rs` — run_svg() / run_png() 분기 구현
- `crates/rpdf-cli/tests/render_tests.rs` — SVG CLI 통합 테스트 2개 추가

## 계획 대비 달라진 점

1. **evaluator가 loose_cm_depth 버그 발견 → 즉시 수정**
   - `SaveState (q)` 처리 시 기존 open cm `<g>` 태그를 닫지 않고 `loose_cm_depth = 0`으로 리셋하는 버그
   - 패턴: `cm → q → content → Q` 에서 cm의 `<g transform>` 태그가 영구 미닫음
   - 수정: SaveState 처리 전에 열린 cm `<g>` 모두 닫기
   - 실제 5개 예제 PDF에서는 해당 패턴 미등장으로 현재 동작에 영향 없었으나 예방적 수정

2. **테스트 수 더 많음 (계획서 8개 → 실제 27개)**
   - state.rs 7개 + path.rs 8개 + text.rs 5개 + 통합 5개 + CLI 2개 = 총 27개

## 발견된 이슈

- **loose_cm_depth 리셋 버그** (`lib.rs:54`): SaveState 처리 시 열린 cm `<g>` 닫지 않는 버그. `cm → q` 패턴 PDF에서 SVG 구조 파손 가능. evaluator 검증 과정에서 발견해 즉시 수정.

## 배운 점

### 기술적
- SVG CTM 관리를 Rust 행렬 곱셈 없이 `<g transform>` 중첩으로 처리 가능 — SVG가 transform 합성을 처리함
- PDF Y축 반전 (`matrix(1 0 0 -1 0 h)`)과 텍스트 역보정 (`scale(1,-1) translate(0,-y)`)의 분리 필요
- loose_cm_depth처럼 상태를 추적할 때, 상태 변경(SaveState)이 기존 상태를 삼켜버리는 버그 패턴 주의

### 프로세스
- evaluator 검증이 generator 구현에서 발견하지 못한 edge case 버그를 찾아냄 — generator+evaluator 2단계 검증의 효과

## 테스트 결과

- rpdf-svg 단위 테스트: 20/20 통과
- rpdf-svg 통합 테스트: 5/5 통과
- rpdf-cli render 통합 테스트: 7/7 통과
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과

## 회귀 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | loose_cm_depth 리셋 버그 (`SaveState` 시 open cm <g> 미닫음) | **A: CLAUDE.md** | 상태 추적 변수가 있을 때 Save/Restore 처리에서 기존 상태 닫기 패턴 — 재발 방지 규칙으로 가치 있음 |
| 2 | evaluator가 edge case 버그 발견 → generator+evaluator 2단계 효과 확인 | **C: 스킵** | 이번 task 전용 관찰, CLAUDE.md에 이미 generator+evaluator 패턴 정의됨 |

## 다음 관련 작업

- Task #15: SVG debug overlay (`--debug-overlay` 플래그)
- Task #16: 여러 페이지 SVG 출력

## 참고 자료

- 계획서: `mydocs/plans/task14-svg-renderer.md`
