# Task #12 계획서: 첫 페이지 PNG 출력 + render CLI

**Issue**: #22  
**브랜치**: local/task12  
**날짜**: 2026-05-04  
**선행 조건**: Task #11 완료 (pdfium 환경 구축, PR #21 머지)

---

## 목표

`pdfium-render` 0.9.1 API로 PDF 페이지를 PNG 파일로 저장하고,  
`rpdf render <pdf> -o output.png` CLI 명령을 동작시킨다.

---

## 완료 기준

| # | 기준 |
|---|------|
| 1 | `rpdf-render` 크레이트에 `render_page` 공개 함수 구현 완료 |
| 2 | `examples/` 5개 PDF 첫 페이지를 PNG로 렌더링 성공하는 통합 테스트 통과 |
| 3 | `rpdf-cli`에 `render` 서브커맨드 추가 (`-o`, `-p`, `--scale`) |
| 4 | `rpdf render examples/pdfjs-basicapi.pdf -o /tmp/out.png` 실제 실행 성공 |
| 5 | `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과 |
| 6 | CI 통과 |

---

## 데이터 모델 / API 설계

### 1. `rpdf-render` 공개 API

```rust
// crates/rpdf-render/src/lib.rs

/// PDF 단일 페이지를 PNG DynamicImage로 렌더링한다.
///
/// - `lib_path`: pdfium 동적 라이브러리가 있는 디렉터리 경로 (`PDFIUM_DYNAMIC_LIB_PATH`)
/// - `pdf_path`: 렌더링할 PDF 파일 경로
/// - `page_index`: 0-based 페이지 인덱스
/// - `scale`: 해상도 배율 (1.0 = 72 DPI 기준, 2.0 = 144 DPI)
pub fn render_page(
    lib_path: &Path,
    pdf_path: &Path,
    page_index: u16,
    scale: f32,
) -> Result<DynamicImage, RenderError>;
```

### 2. `RenderError` 에러 타입

```rust
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("pdfium 라이브러리 로딩 실패: {0}")]
    LibraryLoad(String),
    #[error("PDF 파일 열기 실패: {0}")]
    FileOpen(String),
    #[error("페이지 {0} 접근 실패")]
    PageAccess(u16),
    #[error("렌더링 실패: {0}")]
    Render(String),
}
```

### 3. `rpdf render` CLI 인터페이스

```
rpdf render <PDF> [OPTIONS]

OPTIONS:
  -o, --output <PATH>    출력 PNG 경로 (기본: <pdf_stem>_p<page>.png)
  -p, --page <N>         0-based 페이지 인덱스 (기본: 0)
      --scale <FLOAT>    해상도 배율 (기본: 2.0 = ~144 DPI)
```

---

## 구현 계획

### 체크포인트 1: `rpdf-render` 렌더링 함수 구현

**파일**: `crates/rpdf-render/src/lib.rs`

구현 순서:
1. `RenderError` 에러 타입 정의
2. `render_page()` 함수 구현
   - `Pdfium::bind_to_library()` 호출
   - `pdfium.load_pdf_from_file()` 호출
   - `document.pages().get(page_index)` 호출
   - `PdfRenderConfig::new().set_target_width(width).set_target_height(height)` 설정
   - `page.render_with_config(&config)?.as_image()` 호출
3. 기존 `pdfium_dynamic_links` 테스트 유지

**pdfium-render 0.9.1 핵심 API** (docs.rs 확인 완료):
```rust
let pdfium = Pdfium::bind_to_library(
    Pdfium::pdfium_platform_library_name_at_path(lib_path)
)?;
let doc = pdfium.load_pdf_from_file(pdf_path, None)?;
let page = doc.pages().get(page_index)?;
let config = PdfRenderConfig::new()
    .set_target_width((page.width().value * scale) as i32)
    .set_target_height((page.height().value * scale) as i32);
let image: DynamicImage = page.render_with_config(&config)?.as_image();
```

### 체크포인트 2: `rpdf-render` 통합 테스트

**파일**: `crates/rpdf-render/tests/render_tests.rs`

```rust
// examples/ PDF 5개 각각 첫 페이지 렌더링 성공 검증
// 출력 이미지 크기 0 이상 확인 (픽셀 내용 검증은 Task #13)
```

`PDFIUM_DYNAMIC_LIB_PATH` 미설정 시 `skip` 처리 (CI 환경은 항상 설정됨).

### 체크포인트 3: `rpdf-cli` render 명령 추가

**수정 파일**:
- `crates/rpdf-cli/Cargo.toml` — `rpdf-render` 의존성 추가
- `crates/rpdf-cli/src/main.rs` — `Render` 서브커맨드 추가
- `crates/rpdf-cli/src/commands/render.rs` — 신규 생성
- `crates/rpdf-cli/src/commands/mod.rs` — `pub mod render` 추가

**기본 출력 경로 규칙**: `-o` 미지정 시 `{pdf_stem}_p{page}.png` (현재 디렉터리).

### 체크포인트 4: CLI 통합 테스트

**파일**: `crates/rpdf-cli/tests/render_tests.rs`

```rust
// assert_cmd로 rpdf render 실행, 출력 파일 존재 확인
// 잘못된 파일 경로 → 비정상 종료(exit code ≠ 0) 확인
```

---

## 에지 케이스 및 처리 방침

| 케이스 | 처리 |
|--------|------|
| `PDFIUM_DYNAMIC_LIB_PATH` 미설정 | `RenderError::LibraryLoad` 반환, CLI는 사람이 읽을 에러 메시지 출력 |
| 존재하지 않는 PDF 파일 | `RenderError::FileOpen` 반환 |
| 페이지 인덱스 범위 초과 | `RenderError::PageAccess(n)` 반환 |
| 출력 경로 쓰기 권한 없음 | `io::Error` → `anyhow::Error` 전파, CLI 에러 메시지 출력 |
| scale ≤ 0.0 | `render_page` 진입 전 `assert` or 에러 반환 (0 이하는 의미 없음) |

---

## 외부 의존성 변경

| 변경 | 내용 |
|------|------|
| `rpdf-cli` Cargo.toml | `rpdf-render = { path = "../rpdf-render" }` 추가 |

신규 크레이트 없음. Task #11에서 승인된 `pdfium-render`, `image` 그대로 사용.

---

## 테스트 전략

- **단위 테스트**: `render_page`의 에러 경로 (`PageAccess`, `LibraryLoad`) 를 `#[cfg(test)] mod internal_tests`로 검증
- **통합 테스트** (`tests/render_tests.rs`): `PDFIUM_DYNAMIC_LIB_PATH` 있을 때 실제 렌더링 성공 확인
- **CLI 통합 테스트** (`tests/render_tests.rs`): `assert_cmd`로 종료 코드 + 파일 생성 확인

---

## 파일 목록 (예상)

| 파일 | 변경 |
|------|------|
| `crates/rpdf-render/src/lib.rs` | 수정 (RenderError + render_page 추가) |
| `crates/rpdf-render/tests/render_tests.rs` | 신규 |
| `crates/rpdf-cli/Cargo.toml` | 수정 (rpdf-render 의존성) |
| `crates/rpdf-cli/src/main.rs` | 수정 (Render 서브커맨드) |
| `crates/rpdf-cli/src/commands/render.rs` | 신규 |
| `crates/rpdf-cli/src/commands/mod.rs` | 수정 (pub mod render) |
| `crates/rpdf-cli/tests/render_tests.rs` | 신규 |

---

## 미포함 (Task #13 이후)

- 이미지 스냅샷 비교 (픽셀 diff) — Task #13
- `--svg --debug-overlay` 옵션 — Task #14·15
- 여러 페이지 일괄 렌더링 (`--all-pages`) — 현재 마일스톤 범위 밖
