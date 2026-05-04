# Task #16 완료 보고서: v0.2 통합 테스트 + 문서화

**브랜치**: `local/task16`  
**이슈**: #30  
**완료 일시**: 2026-05-04

---

## 구현 결과

### A. `--all-pages` 플래그

| 파일 | 변경 내용 |
|------|----------|
| `crates/rpdf-cli/src/main.rs` | `Commands::Render`에 `--all-pages` 플래그 추가 |
| `crates/rpdf-cli/src/commands/render.rs` | `RenderParams.all_pages`, `run_svg_all_pages()`, `resolve_all_pages_output()` 추가 |

동작 정책:

| 조합 | 동작 |
|------|------|
| `--svg --all-pages` | 전체 페이지 → `{stem}_p0.svg`, `{stem}_p1.svg`, ... |
| `--svg --all-pages -o dir/` | 지정 디렉토리에 `p0.svg`, `p1.svg`, ... |
| `--svg --all-pages -o prefix.svg` | `prefix_p0.svg`, `prefix_p1.svg`, ... |
| `--all-pages` (--svg 없음) | `bail!` → Error + exit 1 |
| `--svg --all-pages -p N` | `--all-pages` 우선, `-p N` 무시 |

### B. LICENSE 파일

`LICENSE` (MIT) 루트에 추가.

### C. SVG 렌더러 tech 문서

`mydocs/tech/svg-renderer.md` 신규 작성 — 설계 결정, 지원 연산자, 예제, 알려진 한계.

---

## 테스트 결과

| 테스트 | 결과 |
|--------|------|
| 기존 9개 (IT-F1~F5, IT-S4~S5, IT-D4~D5) | 전부 통과 |
| IT-A1: fw4-2024.pdf 다중 페이지 → 여러 svg 생성 | 통과 |
| IT-A2: -o `<tempdir>/` → 디렉토리에 p0.svg | 통과 |
| IT-A3: --all-pages without --svg → exit 1 | 통과 |
| IT-A4: 단일 페이지 PDF → svg 1개만 생성 | 통과 |
| IT-A5: -o prefix.svg → prefix_p0.svg | 통과 |
| `cargo clippy -- -D warnings` | 경고 없음 |
| `cargo fmt --check` | 통과 |

**전체**: 14개 render 테스트 전부 통과.

---

## 계획 대비 변경 사항

**IT-A4 PDF 교체**: 계획서에 `pdfjs-basicapi.pdf`(단일 페이지로 가정)를 썼으나 실제로 3페이지임이 확인됨. `pdfjs-annotation-border.pdf`(실제 1페이지)로 교체.

---

## v0.2 성공 기준 최종 점검

| # | 기준 | 상태 |
|---|------|------|
| 1 | examples/ 5개 PDF → PNG 변환 성공 | ✅ Task #12 |
| 2 | samples/ 28개 PNG + 이미지 스냅샷 회귀 CI 통과 | ✅ Task #13 |
| 3 | `rpdf render <pdf> -o output.png` CLI 동작 | ✅ Task #12 |
| 4 | `rpdf render <pdf> --svg --debug-overlay` CLI 동작 | ✅ Task #15 |
| 5 | CI pdfium 자동 설치 + 회귀 통과 | ✅ Task #11, #13 |
| 6 | v0.1 IR → SVG 렌더러로 시각화 | ✅ Task #14 |
| 7 | `--all-pages` 일괄 SVG 출력 | ✅ Task #16 |
| 8 | MIT LICENSE 파일 | ✅ Task #16 |
| 9 | SVG 렌더러 tech 문서 | ✅ Task #16 |

**v0.2 완료.**

---

## 회고 분류표

| 항목 | 분류 | 처리 |
|------|------|------|
| IT-A4: `pdfjs-basicapi.pdf`가 실제로 3페이지 (계획서에 단일 페이지로 잘못 기재) | C | 보고서 메모 — 테스트 작성 전 PDF 페이지 수를 `rpdf info`로 먼저 확인 |
| `process::exit(1)` 대신 `bail!` 사용 — anyhow 에러 전파로 `main()` 통해 일관된 ExitCode::FAILURE 반환 | C | 보고서 메모 — 향후 CLI 에러 처리는 `bail!` 패턴 유지 |
