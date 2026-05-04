# Task #14 계획서: SVG 렌더러 (v0.1 IR 활용)

**Issue**: #26  
**브랜치**: local/task14  
**날짜**: 2026-05-04  
**선행 조건**: Task #13 완료 (이미지 회귀 인프라, PR #25 머지)

---

## 목표

v0.1 파서가 생성한 `ContentStreamOperation` IR을 SVG로 시각화한다.  
기본 경로(path)·텍스트 placeholder를 SVG로 출력하고,  
`rpdf render <pdf> --svg [-o output.svg]` CLI 명령을 동작시킨다.

> SVG는 디버그 전용 출력. 시각적 완전성보다 IR → SVG 파이프라인 구축이 목표.

---

## 완료 기준

| # | 기준 |
|---|------|
| 1 | 신규 크레이트 `rpdf-svg` 추가. 공개 API: `render_page_svg(page: &Page) -> String` |
| 2 | `examples/` 5개 PDF 각 첫 페이지 → SVG 변환 성공 (유효한 `<svg>` 루트 포함) |
| 3 | `rpdf render <pdf> --svg [-o output.svg]` CLI 명령 동작 |
| 4 | 지원 연산자: `MoveTo`, `LineTo`, `CurveTo`, `CurveToV`, `CurveToY`, `ClosePath`, `Rect`, `Stroke`, `Fill`, `FillStroke`, `SetFillRGB`, `SetStrokeRGB`, `ShowText`, `SetTextMatrix`, `ConcatMatrix`, `SaveState`, `RestoreState` |
| 5 | `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과 |
| 6 | CI 통과 |

---

## 크레이트 설계

### 신규 크레이트: `rpdf-svg`

pdfium 의존성 없이 `rpdf-core` IR만으로 SVG 생성.  
WASM 환경(v0.4)에서도 사용 가능하도록 네이티브 의존성 배제.

```
의존성: rpdf-core, rpdf-parser (테스트 전용)
```

#### 공개 API

```rust
// crates/rpdf-svg/src/lib.rs

/// Page IR을 SVG 문자열로 렌더링한다.
///
/// - media_box가 없으면 A4 크기(595 × 842 pt)를 기본값으로 사용.
/// - 지원하지 않는 연산자는 SVG 주석으로 기록하고 계속 진행한다.
pub fn render_page_svg(page: &rpdf_core::types::Page) -> String;
```

---

## SVG 좌표 변환

PDF는 좌하단(0,0) 기준, SVG는 좌상단(0,0) 기준.  
뷰포트 루트 `<g>`에 Y축 반전 transform을 적용한다.

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <g transform="matrix(1 0 0 -1 0 {h})">
    <!-- 모든 PDF 콘텐츠 -->
  </g>
</svg>
```

media_box `[x0, y0, x1, y1]`에서 `w = x1 - x0`, `h = y1 - y0`.

---

## 그래픽 상태 머신

SVG 생성 중 다음 상태를 스택으로 관리한다.

```rust
struct GraphicsState {
    fill_color: Color,    // 기본: 검정 (0,0,0)
    stroke_color: Color,  // 기본: 검정 (0,0,0)
    line_width: f64,      // 기본: 1.0
}

// Color 표현: RGB (0~255). SetFillRGB/SetStrokeRGB 피연산자(0.0~1.0) * 255 반올림.
// SVG 출력: fill="rgb(r,g,b)" 형식.
struct Color { r: u8, g: u8, b: u8 }  // 기본값: (0, 0, 0)
```

### SVG 그룹 기반 CTM 관리 (plan-eng-review D2 결정)

CTM은 SVG `<g transform="matrix(a b c d e f)">` 그룹으로 표현한다.  
Rust에서 행렬 곱셈 불필요 — SVG가 transform 합성을 처리한다.

```
SaveState (q)       → SVG 출력에 <g> 여는 태그 추가; Rust 스택에 색상/선폭 상태 push
ConcatMatrix (cm)   → SVG 출력에 <g transform="matrix(a b c d e f)"> 여는 태그 추가
RestoreState (Q)    → 스택 pop 횟수만큼 </g> 닫는 태그 추가; Rust 스택 pop
```

> `cm`이 `q`/`Q` 없이 단독으로 등장하는 경우에도 `<g transform>` 열고 콘텐츠 끝에 닫는다.

---

## 경로 연산 → SVG `<path>`

경로 구성 연산자 시퀀스를 SVG `d` 속성으로 변환한다.

| ContentStreamOperator | SVG path command |
|-----------------------|-----------------|
| `MoveTo(x, y)` | `M {x} {y}` |
| `LineTo(x, y)` | `L {x} {y}` |
| `CurveTo(x1,y1,x2,y2,x3,y3)` | `C {x1} {y1} {x2} {y2} {x3} {y3}` |
| `CurveToV(x2,y2,x3,y3)` | `C {cur_x} {cur_y} {x2} {y2} {x3} {y3}` (첫 제어점 = 현재 점) |
| `CurveToY(x1,y1,x3,y3)` | `C {x1} {y1} {x3} {y3} {x3} {y3}` (두 번째 제어점 = 끝 점) |
| `ClosePath` | `Z` |
| `Rect(x,y,w,h)` | `M {x} {y} h {w} v {h} h -{w} Z` |

경로 그리기 연산자에서 `<path>` 요소 생성:

| ContentStreamOperator | SVG 속성 |
|-----------------------|---------|
| `Stroke` | `fill="none" stroke="{stroke_color}" stroke-width="{line_width}"` |
| `Fill` / `FillObsolete` | `fill="{fill_color}" stroke="none"` |
| `FillStroke` | `fill="{fill_color}" stroke="{stroke_color}" stroke-width="{line_width}"` |
| `EndPath` | 경로 버리기 (요소 미생성) |
| 기타 Fill/Stroke 변형 | Stroke/Fill 규칙 동일하게 처리 |

---

## 텍스트 → SVG `<text>`

텍스트는 시각화 placeholder 수준. 실제 폰트 렌더링은 범위 밖.

| ContentStreamOperator | 처리 |
|-----------------------|------|
| `BeginText` | 텍스트 상태 초기화 |
| `SetTextMatrix(a,b,c,d,e,f)` | 텍스트 위치 = `(e, f)` |
| `ShowText(string)` | `<text x="{e}" y="{f}" fill="{fill_color}" transform="scale(1,-1) translate(0,-{f})">{string}</text>` |
| `ShowTextAdjusted(array)` | 문자열 항목만 추출해 동일하게 처리 |
| `EndText` | 텍스트 상태 리셋 |
| `MoveText(tx, ty)` | 현재 텍스트 위치에 (tx, ty) 더하기 |

---

## 연산자 처리 정책

지원하지 않는 연산자는 SVG 주석으로 기록하고 계속 진행한다.

```svg
<!-- unsupported: SetGraphicsState -->
```

> 파싱 에러 아님 — `Unknown(_)` 포함 미구현 연산자도 동일 처리.

---

## 모듈 설계

```
crates/rpdf-svg/
├── Cargo.toml
└── src/
    ├── lib.rs          — render_page_svg() 진입점
    ├── state.rs        — GraphicsState, Color 타입
    ├── path.rs         — 경로 구성 → SVG d 문자열 빌더
    └── text.rs         — 텍스트 상태 → <text> 요소 빌더
```

---

## CLI 변경

`rpdf-cli`의 `render` 서브커맨드에 `--svg` 플래그 추가.

```
rpdf render <PDF> [OPTIONS]

OPTIONS:
  -o, --output <PATH>    출력 파일 경로 (기본: <pdf_stem>_p<page>.png 또는 .svg)
  -p, --page <N>         0-based 페이지 인덱스 (기본: 0)
      --scale <FLOAT>    해상도 배율 (PNG 전용, 기본: 2.0)
      --svg              SVG 출력 모드 (pdfium 불필요)
```

`--svg` 지정 시:
- `PDFIUM_DYNAMIC_LIB_PATH` 불필요
- `rpdf_parser::load_document()` → `rpdf_svg::render_page_svg()` → 파일 저장
- 기본 출력 경로: `{pdf_stem}_p{page}.svg`

---

## 에지 케이스

| 케이스 | 처리 |
|--------|------|
| `media_box` 없는 페이지 | A4 기본값 (595 × 842) 사용 |
| `content` 빈 페이지 | 빈 `<svg>` 반환 (에러 아님) |
| `SaveState` 초과 pop | 스택 언더플로 무시 (로그 없이 계속) |
| operands 수 부족 | 해당 연산자 skip + SVG 주석 |
| 음수 크기 `Rect` | 절댓값 사용 |

---

## 파일 목록 (예상)

| 파일 | 변경 |
|------|------|
| `crates/rpdf-svg/Cargo.toml` | 신규 |
| `crates/rpdf-svg/src/lib.rs` | 신규 |
| `crates/rpdf-svg/src/state.rs` | 신규 |
| `crates/rpdf-svg/src/path.rs` | 신규 |
| `crates/rpdf-svg/src/text.rs` | 신규 |
| `crates/rpdf-svg/tests/svg_render_tests.rs` | 신규 |
| `Cargo.toml` (workspace) | `rpdf-svg` 멤버 추가 |
| `crates/rpdf-cli/Cargo.toml` | `rpdf-svg` 의존성 추가 |
| `crates/rpdf-cli/src/commands/render.rs` | `--svg` 플래그 처리 추가 |
| `crates/rpdf-cli/src/commands/mod.rs` | 변경 없음 |

---

## 테스트 전략

### 단위 테스트 (src/path.rs, src/state.rs, src/text.rs 인라인)

- `GraphicsState::new()` 기본값 검증
- `MoveTo` + `LineTo` + `Stroke` → `<path d="M ... L ..." .../>` 생성 검증
- `CurveToV(x2,y2,x3,y3)` → `d` 속성에 `C {cur_x} {cur_y} {x2} {y2} {x3} {y3}` 포함 (D1 수정 검증)
- `CurveToY(x1,y1,x3,y3)` → `d` 속성에 `C {x1} {y1} {x3} {y3} {x3} {y3}` 포함 (D1 수정 검증)
- `Rect(x,y,w,h)` + `Fill` → `<path d="M {x} {y} ..." fill="{color}" stroke="none"/>`
- `SetFillRGB(r,g,b)` + `Fill` → `fill="rgb(...)"`로 색상 반영 검증
- `SaveState` / `RestoreState` 스택 동작 검증 (색상·선폭 복원)
- `SaveState` / `RestoreState` 스택 동작 검증

### 통합 테스트 (`tests/svg_render_tests.rs`)

- IT-S1: `examples/pdfjs-basicapi.pdf` 첫 페이지 → `render_page_svg()` 결과에 `<svg` 포함
- IT-S2: `examples/pdfjs-basicapi.pdf` → 결과에 `</svg>` 포함 (태그 닫힘)
- IT-S3: media_box 없는 빈 Page → 에러 없이 유효한 `<svg>` 반환
- IT-S4: `rpdf render pdfjs-basicapi.pdf --svg` CLI 실행 → .svg 파일 생성 확인

---

## 미포함 (이후 Task)

- `--debug-overlay` (경계 박스·좌표 표기) — Task #15
- 여러 페이지 SVG — Task #16
- 클리핑 경로, 셰이딩, XObject 인라인 — 마일스톤 범위 밖

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR (PLAN) | 4 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0
- **VERDICT:** ENG CLEARED — ready to implement
