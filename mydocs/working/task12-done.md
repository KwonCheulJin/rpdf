# Task #12 완료 보고서: 첫 페이지 PNG 출력 + render CLI

**날짜**: 2026-05-04  
**Issue**: #22  
**브랜치**: local/task12

---

## 완료 기준 달성 여부

| # | 완료 기준 | 달성 |
|---|-----------|------|
| 1 | `rpdf-render` 공개 함수 `render_page` 구현 완료 | ✅ |
| 2 | `examples/` 5개 PDF 첫 페이지 렌더링 성공 통합 테스트 통과 | ✅ (IT-R1~R5) |
| 3 | `rpdf-cli` `render` 서브커맨드 추가 (`-o`, `-p`, `--scale`) | ✅ |
| 4 | CLI 통합 테스트 통과 | ✅ (IT-F1~F5) |
| 5 | `cargo nextest run --workspace` 전체 통과 | ✅ (353 passed, 2 skipped) |
| 6 | `cargo clippy -- -D warnings`, `cargo fmt --check` 통과 | ✅ |

---

## 생성/수정된 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/rpdf-render/src/lib.rs` | `RenderError` + `render_page()` + `load_pdfium()` 추가 |
| `crates/rpdf-render/tests/render_tests.rs` | 신규 — IT-R1~R8 (8개 테스트) |
| `Cargo.toml` (workspace) | `rpdf-render` workspace dependency 추가 |
| `crates/rpdf-cli/Cargo.toml` | `rpdf-render.workspace = true` 의존성 추가 |
| `crates/rpdf-cli/src/main.rs` | `Render` 서브커맨드 + `run()` 분기 추가 |
| `crates/rpdf-cli/src/commands/render.rs` | 신규 — `RenderParams` + `run()` 구현 |
| `crates/rpdf-cli/src/commands/mod.rs` | `pub mod render` 추가 |
| `crates/rpdf-cli/tests/render_tests.rs` | 신규 — IT-F1~F5 (5개 테스트) |

---

## 구현 세부 사항

### render_page() 함수
```rust
pub fn render_page(lib_path: &Path, pdf_path: &Path, page_index: u16, scale: f32) -> Result<DynamicImage, RenderError>
```
- `scale <= 0.0` 조기 반환 (`RenderError::InvalidScale`)
- `load_pdfium()` 내부 헬퍼로 pdfium 초기화 (`AlreadyInitialized` 재사용 처리)
- `page.width().value * scale` → `i32` 캐스팅으로 픽셀 크기 계산

### load_pdfium() 헬퍼
pdfium-render의 전역 `OnceCell` 동작으로 인해 동일 프로세스에서 `bind_to_library`를 두 번 호출하면 `PdfiumLibraryBindingsAlreadyInitialized` 에러가 발생한다. 이를 처리하기 위해 `Pdfium::default()`로 기존 바인딩을 재사용한다. (통합 테스트 8개가 한 프로세스에서 실행되기 때문에 필수)

### CLI 기본 출력 경로
`-o` 미지정 시 `{pdf_stem}_p{page}.png` (현재 디렉터리).

---

## 검증 결과

```
# cargo nextest run --workspace (PDFIUM_DYNAMIC_LIB_PATH 설정 후)
Summary: 353 tests run: 353 passed, 2 skipped

# cargo clippy --all-targets -- -D warnings
(경고 없음)

# cargo fmt --all -- --check
(출력 없음 = 포맷 정상)
```

---

## 트러블슈팅 발생/해결 내역

### T1: pdfium OnceCell 재초기화 에러 (`AlreadyInitialized`)

- **현상**: 통합 테스트 여러 개가 한 프로세스에서 실행될 때 두 번째 테스트부터 `bind_to_library` 실패
- **원인**: pdfium-render가 내부적으로 전역 `OnceCell`을 사용 — 이미 초기화된 경우 `PdfiumLibraryBindingsAlreadyInitialized` 반환
- **해결**: `load_pdfium()` 헬퍼에서 해당 에러를 `Pdfium::default()`로 재사용 처리
- **문서**: 완료 보고서 메모 (C 분류)

---

## 회고 분류표

| 분류 | 항목 | 처리 |
|------|------|------|
| C | pdfium-render OnceCell: 동일 프로세스에서 `bind_to_library` 2회 호출 시 `AlreadyInitialized` — `Pdfium::default()` 재사용으로 해결 | 완료 보고서 메모 |
| C | `Pixels` = `i32`, `PdfPageIndex` = `c_int` = `i32` — 계획서 `u16` 가정과 달라 캐스팅 필요 | 완료 보고서 메모 |
| C | `bitmap.as_image()` 반환이 `DynamicImage`가 아닌 `Result<DynamicImage, PdfiumError>` — 추가 `.map_err` 처리 필요 | 완료 보고서 메모 |

### A 항목 (CLAUDE.md 즉시 반영)
없음 — 기존 규칙으로 커버됨.

### B 항목 (트러블슈팅 문서)
없음 — T1은 pdfium-render 내부 동작이며 `load_pdfium()` 헬퍼 주석으로 충분히 설명됨.

---

## 다음 작업

Task #13: samples/ 28개 PNG 회귀 검증 (이미지 스냅샷 인프라, OS별 허용 오차 정책)
