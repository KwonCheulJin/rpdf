# Task #16: v0.2 통합 테스트 + 문서화

**이슈**: #30  
**브랜치**: `local/task16`  
**마일스톤**: v0.2 렌더링 뼈대 (마지막 구현 태스크)  
**선행 조건**: Task #15 완료 (PR #29 → devel 머지)  
**상태**: 계획 작성 중

---

## 목표

v0.2 마무리 태스크. 세 가지 결과물을 목표로 한다:

1. **여러 페이지 SVG 배치 출력** — `--all-pages` 플래그로 전체 페이지를 SVG 파일들로 저장
2. **프로젝트 라이선스 파일** — `LICENSE` (MIT) 루트에 추가
3. **SVG 렌더러 tech 문서** — `mydocs/tech/svg-renderer.md` 신규 작성

---

## 현재 상태 (devel HEAD)

| 항목 | 상태 |
|------|------|
| `rpdf render <pdf> --svg -p N` | 단일 페이지 SVG ✅ |
| `rpdf render <pdf> --svg --debug-overlay` | 디버그 오버레이 ✅ |
| `rpdf render <pdf> -o out.png` | PNG (pdfium) ✅ |
| `--all-pages` 플래그 | ❌ 없음 |
| `LICENSE` 파일 | ❌ 없음 |
| SVG 렌더러 tech 문서 | ❌ 없음 |

---

## v0.2 성공 기준 최종 점검

| # | 기준 | 달성 태스크 | 상태 |
|---|------|------------|------|
| 1 | examples/ 5개 PDF → PNG 변환 성공 | Task #12 | ✅ |
| 2 | samples/ 28개 PNG + 이미지 스냅샷 회귀 CI 통과 | Task #13 | ✅ |
| 3 | `rpdf render <pdf> -o output.png` CLI 동작 | Task #12 | ✅ |
| 4 | `rpdf render <pdf> --svg --debug-overlay` CLI 동작 | Task #15 | ✅ |
| 5 | CI pdfium 자동 설치 + 회귀 통과 | Task #11, #13 | ✅ |
| 6 | v0.1 IR → SVG 렌더러로 시각화 | Task #14 | ✅ |

성공 기준 6가지 모두 달성. Task #16은 v0.2 polish 태스크.

---

## 구현 범위

### A. `--all-pages` 플래그 (rpdf-cli)

#### CLI 변경

`Commands::Render`에 `all_pages: bool` 필드 추가:

```
/// 전체 페이지를 SVG로 일괄 출력한다 (--svg 전용).
#[arg(long = "all-pages")]
all_pages: bool,
```

동작 정책:

| 조합 | 동작 |
|------|------|
| `--svg --all-pages` | 전체 페이지 → `<stem>_p0.svg`, `<stem>_p1.svg`, ... |
| `--svg --all-pages -o dir/` | 지정 디렉토리(사전 존재 필요)에 `p0.svg`, `p1.svg`, ... |
| `--svg --all-pages -o prefix.svg` | `prefix_p0.svg`, `prefix_p1.svg`, ... (suffix 분리) |
| `--all-pages` (--svg 없음) | stderr 경고: `--all-pages requires --svg`, exit 1 |
| `--svg --all-pages -p N` | `--all-pages` 우선, `-p N` 무시 (page는 clap default_value = "0"이므로 별도 경고 불필요) |
| `--svg -p N` (기존) | 단일 페이지 (변경 없음) |

#### 출력 경로 결정 로직

```
fn resolve_all_pages_output(output: Option<&Path>, stem: &str, page: usize) -> PathBuf {
    match output {
        None => PathBuf::from(format!("{stem}_p{page}.svg")),
        Some(p) if p.is_dir() => p.join(format!("p{page}.svg")),
        Some(p) => {
            let parent = p.parent().unwrap_or(Path::new("."));
            let file_stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(stem);
            parent.join(format!("{file_stem}_p{page}.svg"))
        }
    }
}
```

#### 공개 API 변경 없음

`rpdf-svg` 크레이트 변경 없음. `render_page_svg_with_options`를 반복 호출하는 것으로 충분.

### B. `LICENSE` 파일

루트에 MIT 라이선스 텍스트 추가.  
`Cargo.toml`에 이미 `license = "MIT"` 선언 — 파일만 없는 상태.

### C. SVG 렌더러 tech 문서

`mydocs/tech/svg-renderer.md` 신규 작성:
- 설계 결정 (y-flip 그룹, loose_cm_depth 패턴, 오버레이 레이어 분리)
- 지원/미지원 연산자 목록
- 사용 예제 (`render_page_svg`, `render_page_svg_with_options`)
- 알려진 한계

---

## 데이터 모델 변경

### `RenderParams` 구조체

```rust
pub struct RenderParams {
    pub file: PathBuf,
    pub output: Option<PathBuf>,
    pub page: u16,
    pub scale: f32,
    pub svg: bool,
    pub debug_overlay: bool,
    pub all_pages: bool,   // ← 신규
}
```

---

## 테스트 전략

### 신규 통합 테스트 (rpdf-cli/tests/render_tests.rs)

| ID | 설명 | 방법 |
|----|------|------|
| IT-A1 | `--svg --all-pages` fw4-2024.pdf (다중 페이지) → 여러 .svg 생성 | assert 파일 개수 == 페이지 수 |
| IT-A2 | `--svg --all-pages -o <tempdir>/` → 디렉토리에 p0.svg, p1.svg, ... | `tempfile::tempdir()` 사전 생성 후 경로 전달, 파일 목록 검증 |
| IT-A3 | `--all-pages` (--svg 없음) → exit 1, stderr 포함 | assert failure |
| IT-A4 | 단일 페이지 PDF(`pdfjs-basicapi.pdf`)에 `--svg --all-pages` → 파일 1개만 생성 | assert 파일 개수 == 1 |
| IT-A5 | `--svg --all-pages -o /tmp/out.svg` → `/tmp/out_p0.svg`, `/tmp/out_p1.svg`, ... 생성 | assert 각 파일 존재 + `<svg` 포함 |

### 기존 테스트 유지

IT-F1~IT-F5, IT-S4~IT-S5, IT-D4~IT-D5 (9개) 모두 회귀 없이 통과해야 함.

### 주의: IT-A2 사전 조건

`-o <dir>/` 형식은 디렉토리가 **사전에 존재해야** 동작한다 (`p.is_dir()` 검사 의존).
테스트에서는 `tempfile::tempdir()`으로 미리 생성한 경로를 전달한다.
실제 사용자가 존재하지 않는 디렉토리를 넘기면 `is_dir()`이 false → file-path 분기로 처리됨 (의도와 다른 동작). 이는 알려진 한계로 tech 문서에 명시한다.

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| 0페이지 PDF | 경고 메시지 출력, exit 0 (빈 성공) |
| `-o` 경로의 부모 디렉토리 없음 | `anyhow::Context`로 명확한 에러 메시지 |
| `--svg --all-pages --debug-overlay` | 각 페이지에 오버레이 적용 |

---

## 체크포인트

### CP-1: `--all-pages` 구현 + 테스트 통과

- `RenderParams.all_pages` 추가
- `run_svg_all_pages()` 함수 구현 (`--all-pages`가 true이면 `page` 필드 무시)
- IT-A1~IT-A5 통과
- 기존 9개 테스트 회귀 없음

### CP-2: LICENSE + tech 문서

- 루트 `LICENSE` 파일 추가 (MIT)
- `mydocs/tech/svg-renderer.md` 작성
- `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과

---

## 파일 위치 요약

| 파일 | 변경 유형 |
|------|----------|
| `crates/rpdf-cli/src/main.rs` | `--all-pages` 플래그 추가 |
| `crates/rpdf-cli/src/commands/render.rs` | `RenderParams.all_pages`, `run_svg_all_pages()` |
| `crates/rpdf-cli/tests/render_tests.rs` | IT-A1~IT-A4 추가 |
| `LICENSE` | 신규 (MIT) |
| `mydocs/tech/svg-renderer.md` | 신규 |

---

## 공개 API 확인 완료

- `rpdf_svg::render_page_svg_with_options` — 기존 API, 변경 없음
- `rpdf_parser::load_document` — 기존 API, 변경 없음
- `doc.pages()`, `doc.page_count()` — 기존 API, 변경 없음

---

## 외부 의존성 변경

없음. 기존 크레이트만 사용.

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAN | 3 issues found, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **VERDICT:** ENG CLEARED — 3개 이슈 리뷰 중 해결됨. 구현 시작 가능.

---

## NOT in scope

| 항목 | 이유 |
|------|------|
| `--all-pages` PNG 배치 출력 | PNG는 pdfium 의존 + YAGNI. SVG에만 적용. |
| 존재하지 않는 `-o dir/` 자동 생성 | YAGNI. 알려진 한계로 문서화로 충분. |
| 0-page PDF 테스트 | 합성 0-page PDF 생성 인프라 없음. 로직만 구현하고 엣지케이스로 명시. |
| cargo-deny 의존성 감사 | v0.2 범위 밖. v0.3 시작 전 별도 태스크. |
| 멀티 스레드 병렬 렌더링 | debug 도구 용도 → 순차 처리로 충분. |

## What already exists

| 기존 코드 | 재사용 여부 |
|-----------|------------|
| `run_svg()` — 단일 페이지 SVG 렌더링 | 재사용: `run_svg_all_pages()`가 동일 API 호출 |
| `render_page_svg_with_options()` | 그대로 사용 |
| `IT-S4`, `IT-S5` 테스트 패턴 | IT-A1~IT-A5 작성 시 참고 |
| `tempfile::tempdir()` — IT-F1, IT-F2에서 이미 사용 중 | IT-A2에서 동일 패턴 적용 |
