# Task #25 계획서 — wasm_api 모듈 작성

**Issue**: #48  
**브랜치**: `local/task25`  
**마일스톤**: M040 (v0.4 WASM 바인딩)  
**선행 조건**: v0.3 완료 ✅ (Task #21~#24)

---

## 목표

Rust 코어를 WebAssembly로 컴파일하기 위한 `rpdf-wasm` crate를 신규 생성한다.  
`PdfDocument` wasm_bindgen 구조체를 통해 JS/TypeScript에서 PDF 파싱·편집·저장 API를 제공한다.  
렌더링은 JS 측 pdf.js에 위임한다 (`pdfium-render`는 WASM 미지원).

---

## 버전 정보 (실제 설치 기준)

| 도구 | 버전 |
|------|------|
| Rust | 1.87.0 |
| wasm-pack | 미설치 → 이번 Task에서 설치 |
| wasm-bindgen | 0.2 (최신 호환) |
| wasm32-unknown-unknown target | rust target 추가 필요 확인 |

---

## 공개 API 확인 완료

### wasm-bindgen 0.2
- `#[wasm_bindgen]` — 구조체·함수 노출 매크로
- `JsValue` — JS 오류 전달 타입
- `#[wasm_bindgen(js_name = "...")]` — JS 측 이름 커스텀
- `wasm_bindgen::throw_str(...)` — JS에 에러 전달
- `js_sys::Uint8Array` — JS Uint8Array ↔ Rust Vec<u8>

### 현재 Rust 코어 공개 API
- `rpdf_parser::load_document_tracked(data: &[u8]) → Result<(Document, Vec<PageSource>), ParseError>`
  - 불가. `load_document_tracked`는 `rpdf-serializer`에 있음
- `rpdf_serializer::load_document_tracked(data: &[u8]) → Result<(Document, Vec<PageSource>), ParseError>`
- `rpdf_serializer::serialize_document(doc: &Document, sources: &[PageSource]) → Result<Vec<u8>, SerializeError>`
- `rpdf_edit::commands::RotatePageCommand`, `DeletePagesCommand`, `MergeCommand`, `SplitCommand`, `ExtractPagesCommand`
- `rpdf_edit::commands::CommandStack::new(max_depth: usize) → CommandStack`
- `CommandStack::execute(cmd, doc)`, `::undo(doc)`, `::redo(doc)`

---

## 데이터 모델

### `PdfDocument` (wasm_bindgen 구조체)

```rust
#[wasm_bindgen]
pub struct PdfDocument {
    doc: Document,
    sources: Vec<PageSource>,
    stack: CommandStack,
    // undo/redo 시 sources 동기화를 위한 스냅샷 스택
    sources_undo: Vec<Vec<PageSource>>,
    sources_redo: Vec<Vec<PageSource>>,
}
```

**설계 근거**:
- WASM은 단일 스레드 → `Send + Sync` 불필요
- `CommandStack`의 `Box<dyn Command: Send + Sync>`는 WASM에서 trivially 충족
- `sources`는 `rpdf-serializer`의 `PageSource` — `serialize_document` 호출 시 필요
- **sources 스냅샷 필수**: `delete_pages` 후 `undo()` 시 `doc.pages`는 복원되지만 `sources`는 자동 복원 안 됨. `sources_undo`/`sources_redo` 스택으로 동기화.
  - `PageSource`는 `Arc<Vec<u8>>`를 포함하므로 clone 비용이 낮음 (bytes 공유, 포인터만 복제)

### `execute_cmd` 내부 헬퍼

`rotate_page`, `delete_pages` 등 커맨드 실행 공통 흐름을 헬퍼로 추출:

```rust
fn execute_cmd(&mut self, cmd: Box<dyn Command>, new_sources: Option<Vec<PageSource>>) -> Result<(), JsValue> {
    // ⚠️ sources_undo push는 반드시 execute 성공 후에 수행
    // execute 실패 시 스냅샷을 push하면 스택 탈동기화 발생 (Gemini 지적)
    self.stack.execute(cmd, &mut self.doc).map_err(|e| JsValue::from_str(&e.to_string()))?;
    // execute 성공 → 현재 sources를 undo 스택에 push, redo 스택 클리어
    // rotate_page 는 new_sources=None (sources 변경 없음, 스냅샷도 push 불필요)
    if let Some(s) = new_sources {
        self.sources_undo.push(self.sources.clone());
        self.sources_redo.clear();
        self.sources = s;
    }
    Ok(())
}
```

**중요**: `rotate_page`는 sources를 변경하지 않으므로 `new_sources = None` 전달. `sources_undo` push 생략으로 불필요한 메모리 낭비 방지.

### WasmError

wasm_bindgen 구조체의 `Result<T, JsValue>` 패턴을 사용한다.  
에러 변환 시 `js_sys::Error::new(&e.to_string()).into()`를 사용한다.

> **근거**: `JsValue::from_str()`는 JS 문자열 원시값만 반환하여 스택 트레이스가 없고 `instanceof Error` 체크도 실패함. `js_sys::Error::new()`는 JS `Error` 객체를 생성해 표준 에러 처리 패턴을 지원 (Gemini 지적).

### 내부 로직 분리 전략

네이티브 `cargo test`에서 `JsValue`/`js_sys` 타입 없이 로직을 테스트하기 위해:

```rust
// 내부 헬퍼 (JsValue 없는 플레인 Rust — 네이티브 단위 테스트 가능)
fn validate_page_index(idx: usize, count: usize) -> Result<(), String> { ... }
fn compute_new_sources(sources: &[PageSource], deleted: &[usize]) -> Vec<PageSource> { ... }
fn validate_degrees(degrees: i32) -> Result<(), String> { ... }

// wasm 바인딩 레이어 (보일러플레이트만, 로직 없음)
#[wasm_bindgen]
pub fn rotate_page(&mut self, index: usize, degrees: i32) -> Result<(), JsValue> {
    validate_page_index(index, self.doc.page_count())
        .map_err(|e| js_sys::Error::new(&e).into())?;
    validate_degrees(degrees).map_err(|e| js_sys::Error::new(&e).into())?;
    ...
}
```

`#[cfg(test)]` 블록은 내부 헬퍼만 테스트. wasm 바인딩 레이어는 `wasm-pack test --node`로 별도 검증.

---

## 공개 API 명세

### `PdfDocument::new(data: &[u8]) → Result<PdfDocument, JsValue>`

```rust
#[wasm_bindgen(constructor)]
pub fn new(data: &[u8]) -> Result<PdfDocument, JsValue>
```

- `load_document_tracked(data)` 호출
- 성공 시 `CommandStack::new(50)` 생성 (undo 50단계)
- 실패 시 `js_sys::Error::new(&e.to_string()).into()`

### `page_count() → usize`

```rust
pub fn page_count(&self) -> usize
```

- `self.doc.page_count()` 반환

### `page_info(index: usize) → JsValue`

```rust
pub fn page_info(&self, index: usize) -> Result<JsValue, JsValue>
```

- `index < page_count()` 검증
- JSON 직렬화: `{ index, rotation, media_box, crop_box }`
- `serde-wasm-bindgen`으로 Rust struct → JsValue 변환
- 실패 시 JsValue 에러

### `rotate_page(index: usize, degrees: i32) → Result<(), JsValue>`

```rust
pub fn rotate_page(&mut self, index: usize, degrees: i32) -> Result<(), JsValue>
```

- `index` 범위 검증 (`index >= page_count()` → 에러)
- `degrees` 유효값 검증 (`0, 90, 180, 270` 외 → 에러)
- `RotatePageCommand::new(index, degrees)` — **0-based index** (CLI 핸들러 확인)
- `self.stack.execute(Box::new(cmd), &mut self.doc)`

### `delete_pages(indices: Vec<u32>) → Result<(), JsValue>`

```rust
pub fn delete_pages(&mut self, indices: Vec<u32>) -> Result<(), JsValue>
```

- `indices` 비어있으면 에러
- `indices`의 각 항목이 `0-based` 유효 범위인지 검증 (내부 헬퍼 `validate_page_index` 사용)
- `usize` 변환 후 `DeletePagesCommand::new(indices_usize)` — **0-based indices** (CLI 핸들러 확인)
- `execute_cmd(cmd, Some(new_sources))` 호출
- **new_sources 계산 (execute_cmd 호출 전)**:
  ```rust
  let idx_set: HashSet<usize> = indices_usize.iter().copied().collect();
  let new_sources: Vec<PageSource> = self.sources.iter()
      .enumerate()
      .filter(|(i, _)| !idx_set.contains(i))
      .map(|(_, s)| s.clone())
      .collect();
  ```

> **타입 근거**: `Vec<u32>` 사용 시 JS `Uint32Array`와 명확히 매핑됨. `Vec<usize>`는 플랫폼에 따라 32/64비트 불일치 가능 (Gemini 지적).

### `save() → Result<Vec<u8>, JsValue>`

```rust
pub fn save(&self) -> Result<Vec<u8>, JsValue>
```

- `serialize_document(&self.doc, &self.sources)` 호출
- 성공 시 `Vec<u8>` 반환
- 실패 시 JsValue 에러

### `undo() → Result<(), JsValue>`

```rust
pub fn undo(&mut self) -> Result<(), JsValue>
```

- `sources_undo` 비어있으면 에러
- `self.stack.undo(&mut self.doc)` 호출
- 성공 시: `sources_redo.push(self.sources.clone())`, `self.sources = sources_undo.pop()`
- `CommandError::NothingToUndo` → JsValue 에러 ("되돌릴 커맨드 없음")

### `redo() → Result<(), JsValue>`

```rust
pub fn redo(&mut self) -> Result<(), JsValue>
```

- `sources_redo` 비어있으면 에러
- `self.stack.redo(&mut self.doc)` 호출
- 성공 시: `sources_undo.push(self.sources.clone())`, `self.sources = sources_redo.pop()`
- `CommandError::NothingToRedo` → JsValue 에러 ("다시 실행할 커맨드 없음")

### `undo_len() → usize` / `redo_len() → usize`

```rust
pub fn undo_len(&self) -> usize
pub fn redo_len(&self) -> usize
```

---

## 에러 처리 표

| 상황 | 에러 | 발생 위치 |
|------|------|-----------|
| PDF 파싱 실패 | `ParseError::...` → JsValue | `new()` |
| `rotate_page` index 범위 초과 | `"페이지 인덱스 범위 초과: {index}"` → JsValue | `rotate_page()` |
| `rotate_page` 유효하지 않은 degrees | `"유효하지 않은 회전각: {degrees} (0/90/180/270만 허용)"` → JsValue | `rotate_page()` |
| `delete_pages` 빈 목록 | `"삭제할 페이지 목록이 비어있습니다"` → JsValue | `delete_pages()` |
| `delete_pages` index 범위 초과 | `"페이지 인덱스 범위 초과: {i}"` → JsValue | `delete_pages()` |
| serialize 실패 | `SerializeError::...` → JsValue | `save()` |
| undo 없음 | `"되돌릴 커맨드 없음"` → JsValue | `undo()` |
| redo 없음 | `"다시 실행할 커맨드 없음"` → JsValue | `redo()` |
| `page_info` index 범위 초과 | `"페이지 인덱스 범위 초과: {index}"` → JsValue | `page_info()` |

---

## 하위 커맨드 인덱스 정책 (CLI 핸들러 확인 완료)

- `RotatePageCommand::new(page_index, degrees)` — **0-based** index
  - CLI는 1-based 입력을 `page - 1`로 변환 후 호출
- `DeletePagesCommand::new(indices)` — **0-based** indices
  - CLI의 `parse_page_list`가 0-based 변환 처리
- wasm API는 **0-based index를 JS 측에서 받아 그대로 Command에 전달**
- `delete_pages` 실행 후 `self.sources`를 수동으로 동기화해야 함 (CLI 핸들러 패턴 동일)

---

## 파일 구조

```
crates/rpdf-wasm/
├── Cargo.toml
└── src/
    └── lib.rs          — PdfDocument wasm_bindgen 구현
```

---

## Cargo.toml 설계

```toml
[package]
name = "rpdf-wasm"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rpdf-core.workspace = true
rpdf-parser.workspace = true
rpdf-edit.workspace = true
rpdf-serializer.workspace = true
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
serde.workspace = true
js-sys = "0.3"
```

> **주의**: `rpdf-render`는 pdfium 네이티브 의존이므로 WASM crate에서 제외.

### workspace Cargo.toml 추가

```toml
# [workspace.dependencies] 추가
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
js-sys = "0.3"

# [workspace] members 추가
"crates/rpdf-wasm",
```

---

## 체크포인트

### CP-A: 환경 준비 + crate 생성 + 빌드 통과

**작업 범위**:
1. wasm-pack 설치 (`cargo install wasm-pack`)
2. wasm32 target 추가 (`rustup target add wasm32-unknown-unknown`)
3. `cargo new --lib crates/rpdf-wasm --vcs none`
4. workspace Cargo.toml — members + dependencies 추가
5. `rpdf-wasm/Cargo.toml` 작성 (cdylib + rlib, wasm-bindgen 의존)
6. `src/lib.rs` — 빈 `PdfDocument` 구조체 + `#[wasm_bindgen]` 스캐폴딩
7. `wasm-pack build --target web --dev` 빌드 성공

**통과 기준**: `wasm-pack build` 성공, `pkg/` 디렉터리 생성

---

### CP-B: 핵심 API 구현 (new / page_count / save / page_info)

**작업 범위**:
1. `PdfDocument::new(data: &[u8])` — `load_document_tracked` 호출
2. `page_count()` — `self.doc.page_count()` 반환
3. `page_info(index)` — JSON 직렬화 반환
4. `save()` — `serialize_document` 호출

**테스트 (wasm-pack test용 아닌 단위 테스트, `rlib` 대상)**:
- `rpdf-wasm` crate의 `src/lib.rs` 내 `#[cfg(test)]` 블록
- 단, WASM 컨텍스트가 아닌 네이티브에서 단위 테스트 가능한 로직 분리

**통과 기준**: `cargo test -p rpdf-wasm` 성공

---

### CP-C: 편집 API 구현 (rotate_page / delete_pages / undo / redo)

**작업 범위**:
1. `rotate_page(index, degrees)` — 범위·각도 검증 + RotatePageCommand
2. `delete_pages(indices)` — 빈 목록·범위 검증 + DeletePagesCommand
3. `undo()` / `redo()` — CommandStack 위임
4. `undo_len()` / `redo_len()`

**에러 처리**: 모든 에러 → `JsValue::from_str()`

**통과 기준**: `cargo test -p rpdf-wasm` 성공, clippy 경고 없음

---

### CP-D: wasm-pack 빌드 검증 + 번들 크기

**작업 범위**:
1. `wasm-pack build --target web` (release 빌드)
2. `wasm-opt` 적용 확인 (wasm-pack이 자동 적용)
3. `pkg/rpdf_wasm_bg.wasm` gzip 크기 측정 → 2MB 이하 확인
4. `cargo clippy -p rpdf-wasm --target wasm32-unknown-unknown -- -D warnings`

**통과 기준**: 번들 크기 2MB 이하 (gzip), clippy 경고 없음

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `new()` — 빈 슬라이스 | ParseError 반환 |
| `rotate_page(0, 90)` — 유효한 인덱스 | 정상 처리 |
| `rotate_page(999, 90)` — 범위 초과 | 에러 반환 |
| `rotate_page(0, 45)` — 유효하지 않은 각도 | 에러 반환 |
| `delete_pages([])` — 빈 목록 | 에러 반환 |
| `delete_pages` 전체 페이지 삭제 | DeletePagesCommand 에러 전파 |
| `undo()` — 스택 비어있음 | 에러 반환 |
| `save()` — delete 후 빈 문서 | SerializeError::EmptyDocument 전파 |

---

## 테스트 전략

WASM 환경 테스트는 `wasm-pack test --headless --chrome`이 필요하지만,
`wasm-bindgen`을 사용하지 않는 내부 로직은 네이티브 단위 테스트로 검증한다.

### 네이티브 단위 테스트 (`cargo test -p rpdf-wasm`)

```
UT-01: new() — 유효한 PDF bytes → page_count > 0
UT-02: new() — 빈 bytes → Err
UT-03: rotate_page — 유효 인덱스·각도 → undo_len 1 증가
UT-04: rotate_page — 범위 초과 → Err
UT-05: rotate_page — 유효하지 않은 각도 → Err
UT-06: delete_pages — 유효 인덱스 → page_count 감소
UT-07: delete_pages — 빈 목록 → Err
UT-08: undo → undo_len 감소, redo_len 증가, sources.len() == page_count()
UT-09: redo → redo_len 감소, undo_len 증가, sources.len() == page_count()
UT-10: save — 유효 문서 → bytes 비어있지 않음
UT-11: delete_pages → undo → save → 성공 (sources.len() 복원 검증)
UT-12: page_info — 유효 인덱스 → rotation/media_box 필드 포함
```

**테스트용 PDF fixture**: `crates/rpdf-parser/tests/fixtures/` 기존 파일 재사용

---

## 위험 요소

| 위험 | 대응 |
|------|------|
| `RotatePageCommand` 시그니처가 1-based인지 0-based인지 불명확 | 구현 전 CLI 핸들러 코드 확인 필수 |
| `sources`와 `doc.pages` 동기화 (편집 후) | `RotatePageCommand`·`DeletePagesCommand`의 sources 처리 방식 확인 |
| WASM target에서 `Box<dyn Command: Send + Sync>` 컴파일 | WASM은 단일 스레드이므로 trivially 충족 — 문제 없음 예상 |
| wasm-bindgen 버전 ↔ wasm-pack 버전 불일치 | wasm-pack 설치 후 호환 버전 확인 |

---

## 완료 기준

1. `rpdf-wasm` crate 신규 생성, `wasm-pack build --target web` 성공
2. `PdfDocument` 공개 API — `new`, `page_count`, `page_info`, `rotate_page`, `delete_pages`, `save`, `undo`, `redo`, `undo_len`, `redo_len`
3. 네이티브 단위 테스트 UT-01~UT-12 모두 통과 (내부 헬퍼 분리 기반)
4. `cargo clippy -p rpdf-wasm -- -D warnings` 경고 없음
5. WASM 번들 크기 2MB 이하 (gzip)

---

## 범위 외

- npm 패키지 구성 (Task #28)
- rpdf-studio 웹 에디터 (Task #29)
- GitHub Pages 배포 (Task #30)
- PDFium WASM 대체 전략 상세 구현 (Task #26)

---

closes #48

---

## Implementation Tasks

Synthesized from this review's findings. Each task derives from a specific finding above.

- [ ] **T1 (P1, human: ~30min / CC: ~5min)** — rpdf-wasm/src/lib.rs — execute_cmd: sources_undo push를 execute() 성공 후로 이동
  - Surfaced by: Architecture Review — execute 실패 시 스냅샷 탈동기화
  - Files: `crates/rpdf-wasm/src/lib.rs`
  - Verify: UT-11 (delete→undo→save) 통과

- [ ] **T2 (P1, human: ~1h / CC: ~10min)** — rpdf-wasm/src/lib.rs — 내부 로직을 JsValue-free 헬퍼로 분리해 native cargo test 가능하게 함
  - Surfaced by: Test Review — JsValue 타입으로 네이티브 컴파일 실패 (Gemini 지적)
  - Files: `crates/rpdf-wasm/src/lib.rs`
  - Verify: `cargo test -p rpdf-wasm` 성공

- [ ] **T3 (P2, human: ~15min / CC: ~3min)** — rpdf-wasm/src/lib.rs — JsValue::from_str → js_sys::Error::new().into() 교체
  - Surfaced by: Code Quality — JS Error 타입 표준 준수 (Gemini 지적)
  - Files: `crates/rpdf-wasm/src/lib.rs`
  - Verify: 모든 에러 반환이 `js_sys::Error` 객체

- [ ] **T4 (P2, human: ~30min / CC: ~5min)** — rpdf-wasm/src/lib.rs — UT-11 (delete→undo→save) / UT-12 (page_info) 추가
  - Surfaced by: Test Review — sources 복원 경로 미검증
  - Files: `crates/rpdf-wasm/src/lib.rs`
  - Verify: `cargo test -p rpdf-wasm` 12개 테스트 통과

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 4 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0
- **VERDICT:** ENG CLEARED — ready to implement
