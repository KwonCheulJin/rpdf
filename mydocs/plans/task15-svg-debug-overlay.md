# Task #15 계획서: SVG 디버그 오버레이

**Issue**: #28  
**브랜치**: local/task15  
**날짜**: 2026-05-04  
**선행 조건**: Task #14 완료 (SVG 렌더러, PR #27 머지)

---

## 목표

`rpdf render <pdf> --svg --debug-overlay` 명령 시  
페이지 경계·좌표 그리드·원점 표기를 SVG 위에 시각화한다.

> 디버그 오버레이는 파서 출력 좌표계 검증 전용. 인쇄용 품질 불필요.

---

## 완료 기준

| # | 기준 |
|---|------|
| 1 | `rpdf-svg` 공개 API 추가: `render_page_svg_with_options(page: &Page, opts: &RenderOptions) -> String` |
| 2 | 기존 `render_page_svg()` 함수 동작 불변 (후방 호환) |
| 3 | `rpdf render <pdf> --svg --debug-overlay` CLI 동작 |
| 4 | 오버레이 요소: 페이지 경계 사각형, 100pt 간격 좌표 그리드, 원점 (0,0) 마커 |
| 5 | `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과 |

---

## 공개 API 설계

### `rpdf-svg` 변경

```rust
// crates/rpdf-svg/src/lib.rs

/// 렌더링 옵션.
pub struct RenderOptions {
    /// true면 페이지 경계·좌표 그리드·원점 마커를 SVG에 추가한다.
    pub debug_overlay: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { debug_overlay: false }
    }
}

/// 기존 함수 — 동작 불변. RenderOptions::default()로 위임.
pub fn render_page_svg(page: &Page) -> String {
    render_page_svg_with_options(page, &RenderOptions::default())
}

/// 옵션을 지정해 Page IR을 SVG 문자열로 렌더링한다.
pub fn render_page_svg_with_options(page: &Page, opts: &RenderOptions) -> String;
```

---

## SVG 구조

오버레이 요소는 PDF 좌표 변환 그룹 **바깥**에 배치한다.  
PDF 콘텐츠는 y-flip 그룹 내부, 오버레이는 SVG 좌표계(좌상단 기준) 그룹에 독립 배치.

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <!-- PDF 콘텐츠 (y-flip 적용) -->
  <g transform="matrix(1 0 0 -1 0 {h})">
    {pdf_content}
  </g>
  <!-- 디버그 오버레이 (SVG 좌표계, y-flip 없음) -->
  <g id="debug-overlay">
    {overlay_elements}
  </g>
</svg>
```

---

## 오버레이 요소 명세

### 1. 페이지 경계 사각형

페이지 전체를 둘러싸는 파란 점선 사각형.

```svg
<rect x="0.5" y="0.5" width="{w-1}" height="{h-1}"
      fill="none" stroke="rgba(0,0,255,0.6)"
      stroke-width="1.5" stroke-dasharray="6 3"/>
```

### 2. 좌표 그리드 (100pt 간격)

PDF 좌표(좌하단 기준) 100pt 간격으로 수평·수직 가이드선 + 레이블.  
SVG y 좌표 = `h - pdf_y`.

```svg
<!-- 수평선 (PDF y=100 → SVG y=h-100) -->
<line x1="0" y1="{h-y}" x2="{w}" y2="{h-y}"
      stroke="rgba(128,128,128,0.3)" stroke-width="0.5"/>
<!-- 레이블 -->
<text x="4" y="{h-y-3}" font-size="9" fill="rgba(0,0,200,0.7)"
      font-family="monospace">{y}</text>

<!-- 수직선 (PDF x=100 → SVG x=100) -->
<line x1="{x}" y1="0" x2="{x}" y2="{h}"
      stroke="rgba(128,128,128,0.3)" stroke-width="0.5"/>
<!-- 레이블 -->
<text x="{x+3}" y="{h-4}" font-size="9" fill="rgba(0,0,200,0.7)"
      font-family="monospace">{x}</text>
```

그리드 생성 범위:
- x: 100, 200, ... (w 미만)
- y: 100, 200, ... (h 미만)

### 3. 원점 마커

PDF 좌표 (0,0) = SVG 좌표 (0, h) 위치에 원점 표시.

```svg
<!-- 원점 원 -->
<circle cx="0" cy="{h}" r="5"
        fill="rgba(255,0,0,0.7)" stroke="none"/>
<!-- 원점 레이블 -->
<text x="7" y="{h-4}" font-size="10" fill="rgba(200,0,0,0.9)"
      font-family="monospace">(0,0)</text>
```

---

## 모듈 설계

```
crates/rpdf-svg/
└── src/
    ├── lib.rs      — render_page_svg_with_options() + RenderOptions 추가
    ├── overlay.rs  — 신규: build_overlay(w, h) -> String
    ├── state.rs    — 변경 없음
    ├── path.rs     — 변경 없음
    └── text.rs     — 변경 없음
```

### overlay.rs 공개 함수

```rust
/// 디버그 오버레이 SVG 문자열을 생성한다.
/// 반환값은 `<g id="debug-overlay">...</g>` 요소.
pub(crate) fn build_overlay(w: f64, h: f64) -> String;
```

---

## CLI 변경

`crates/rpdf-cli/src/commands/render.rs`:

```rust
pub struct RenderParams {
    pub file: PathBuf,
    pub output: Option<PathBuf>,
    pub page: u16,
    pub scale: f32,
    pub svg: bool,
    pub debug_overlay: bool,  // 신규
}
```

`crates/rpdf-cli/src/main.rs` — `Render` 서브커맨드에 플래그 추가:

```
--debug-overlay    SVG 출력 시 좌표 그리드·페이지 경계·원점 마커 추가 (--svg 전용)
```

`run_svg()` 내부에서 `RenderOptions { debug_overlay: params.debug_overlay }` 전달.

---

## 파일 목록

| 파일 | 변경 |
|------|------|
| `crates/rpdf-svg/src/lib.rs` | `RenderOptions` 타입 + `render_page_svg_with_options()` 추가, 기존 함수 위임 |
| `crates/rpdf-svg/src/overlay.rs` | 신규: `build_overlay()` |
| `crates/rpdf-cli/src/commands/render.rs` | `RenderParams.debug_overlay` 추가, `run_svg()` 에 옵션 전달 |
| `crates/rpdf-cli/src/main.rs` | `--debug-overlay` 플래그 파싱 추가 |

---

## 테스트 전략

### 단위 테스트 (`overlay.rs` 인라인)

- `build_overlay(595.0, 842.0)` 결과에 `<rect` 포함
- `build_overlay(595.0, 842.0)` 결과에 `(0,0)` 포함 (원점 레이블)
- `build_overlay(595.0, 842.0)` 결과에 `100` 포함 (그리드 레이블)
- `build_overlay(595.0, 842.0)` 결과에 `id="debug-overlay"` 포함
- `build_overlay(50.0, 80.0)` 결과에 `<line` 미포함 (소형 페이지 그리드 없음) (D2 결정)
- `build_overlay(50.0, 80.0)` 결과에 `<rect` 포함 (경계는 표시) (D2 결정)

### 통합 테스트 (`crates/rpdf-svg/tests/svg_render_tests.rs` 추가)

- IT-D1: `render_page_svg_with_options(page, &RenderOptions { debug_overlay: true })` → `id="debug-overlay"` 포함
- IT-D2: `render_page_svg_with_options(page, &RenderOptions::default())` → `id="debug-overlay"` 미포함 (후방 호환 검증)
- IT-D3: `render_page_svg(page)` 와 `render_page_svg_with_options(page, &RenderOptions::default())` 결과 동일

### CLI 테스트 (`crates/rpdf-cli/tests/render_tests.rs` 추가)

- IT-D4: `rpdf render examples/pdfjs-basicapi.pdf --svg --debug-overlay` → 생성된 파일에 `id="debug-overlay"` 포함
- IT-D5: `rpdf render examples/pdfjs-basicapi.pdf --debug-overlay` (--svg 없음) → stderr에 `Warning:` 포함 (D2 결정)

---

## 에지 케이스

| 케이스 | 처리 |
|--------|------|
| `--debug-overlay` + `--svg` 없음 (PNG 모드) | `eprintln!("Warning: --debug-overlay has no effect without --svg")` 출력 후 PNG 생성 진행 (D1 결정) |
| media_box 없는 페이지 (A4 기본값 사용) | A4 (595 × 842) 기준 그리드 생성 |
| 페이지 크기가 100 미만 | 그리드 선 없음 (100 미만 → 0개), 경계·원점만 표시 |

---

## 미포함 (이후 Task)

- 연산자별 인덱스 번호 (각 `<path>` 옆에 순번 표시) — 범위 확대 시 추가 가능
- 커스텀 그리드 간격 옵션 — Task #16 이후

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR (PLAN) | 2 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0
- **VERDICT:** ENG CLEARED — ready to implement
