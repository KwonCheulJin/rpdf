# 아키텍처

## 개요

rpdf의 아키텍처는 rhwp(edwardkim/rhwp)의 설계 원칙을 차용하여, PDF 도메인에 맞게 변형한 것입니다. 핵심 아이디어는 다음과 같습니다.

- **코어는 순수 Rust, UI 비의존**
- **CQRS로 Command와 Query 분리**
- **파이프라인: Parser → Model → Document Core → Renderer → Serializer**
- **한 번 작성한 코어를 CLI, WASM, Tauri에서 공유**

## 전체 구조

```
┌──────────────────────────────────────────────────────────────┐
│                      배포 타겟                                │
│  ┌──────────┐    ┌─────────────┐    ┌────────────────────┐   │
│  │ CLI      │    │ Web         │    │ Desktop (Tauri)    │   │
│  │ (rpdf)   │    │ (Studio)    │    │                    │   │
│  └─────┬────┘    └──────┬──────┘    └─────────┬──────────┘   │
│        │                │                     │              │
│        │            WASM│bridge          Tauri│invoke         │
│        │                │                     │              │
└────────┼────────────────┼─────────────────────┼──────────────┘
         │                │                     │
         ▼                ▼                     ▼
┌──────────────────────────────────────────────────────────────┐
│                      바인딩 레이어                            │
│  ┌──────────┐    ┌─────────────┐    ┌────────────────────┐   │
│  │ main.rs  │    │ wasm_api.rs │    │ tauri_commands.rs  │   │
│  └─────┬────┘    └──────┬──────┘    └─────────┬──────────┘   │
└────────┼────────────────┼─────────────────────┼──────────────┘
         │                │                     │
         └────────────────┴─────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│                       코어 레이어 (Rust)                      │
│                                                              │
│    Parser ──→ Model ──→ Document Core ──→ Renderer ──→ SVG   │
│                             │                   ↓            │
│                             │              Canvas/PNG        │
│                             ↓                                │
│                         Serializer ──→ PDF                   │
└──────────────────────────────────────────────────────────────┘
```

## 레이어별 역할

### Parser

**책임**: PDF 바이너리를 읽어 `Document` IR로 변환

- 입력: `&[u8]` (PDF 파일 바이트)
- 출력: `Result<Document>`
- 사용 크레이트: `lopdf` (기본 파싱) + 필요 시 직접 작성한 보조 파서

**핵심 서브모듈**
- `xref.rs`: 교차 참조 테이블 파싱
- `objects.rs`: PDF 기본 객체 (Dictionary, Array, Stream, Name 등)
- `content_stream.rs`: 페이지의 content stream 파싱 (텍스트·그래픽 명령)
- `encryption.rs`: 암호화된 PDF 복호화

**원칙**
- 파싱 실패 시 상세한 에러 반환 (`어디서 어떤 바이트가 문제인지`)
- 미지원 기능을 만나도 최대한 복구 시도
- 성능보다 정확성 우선

### Model (IR: Intermediate Representation)

**책임**: PDF 도큐먼트를 편집·렌더링 가능한 중립 형태로 표현

핵심 타입:

```rust
pub struct Document {
    pub pages: Vec<Page>,
    pub metadata: Metadata,
    pub resources: ResourcePool,
    pub outline: Option<Outline>,
}

pub struct Page {
    pub index: usize,
    pub size: PageSize,        // A4, Letter 등
    pub rotation: i32,         // 0, 90, 180, 270
    pub content: Vec<ContentElement>,
    pub annotations: Vec<Annotation>,
}

pub enum ContentElement {
    Text(TextBlock),
    Image(ImageRef),
    Path(PathElement),
    Form(FormReference),
}
```

**원칙**
- IR은 파일 포맷과 독립적. HWPX가 생기면 parser만 바꾸면 됨
- 모든 좌표는 **PDF 사용자 공간 단위(1/72 인치 = 1 포인트)**로 통일
- 편집을 위해 필요한 정보는 모두 IR에 포함, 아니면 원본 바이트 레퍼런스만 보관

### Document Core (CQRS)

**책임**: 문서에 대한 모든 연산을 Command 또는 Query로 표현

#### Command 트레이트

```rust
pub trait Command {
    fn execute(&self, doc: &mut Document) -> Result<()>;
    fn undo(&self, doc: &mut Document) -> Result<()>;
    fn name(&self) -> &'static str;
}
```

**구현된 Command 예시**
- `MergeCommand` — 다른 Document 병합
- `SplitCommand` — 페이지 범위로 분할
- `RotatePageCommand` — 페이지 회전
- `DeletePagesCommand` — 페이지 삭제
- `AddAnnotationCommand` — 주석 추가
- `InsertImageCommand` — 이미지 삽입

#### Query 트레이트

```rust
pub trait Query {
    type Output;
    fn execute(&self, doc: &Document) -> Result<Self::Output>;
}
```

**구현된 Query 예시**
- `PageInfoQuery` — 페이지 메타데이터
- `ThumbnailQuery` — 썸네일 PNG 생성
- `RenderPageQuery` — SVG 렌더링
- `TextExtractQuery` — 텍스트 추출

#### Undo/Redo 스택

```rust
pub struct CommandStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}
```

### Renderer

**책임**: `Document` IR을 시각적 출력으로 변환

- SVG 출력: CLI 디버깅, 품질 검증용
- Canvas/PNG 출력: WASM 환경 및 썸네일
- 직접 렌더링하지 않고 `pdfium-render`에 위임하는 옵션도 고려

**파이프라인**
```
Document
   ↓
Layout (페이지별 레이아웃 확정)
   ↓
Rasterizer or SVG Writer
   ↓
Output
```

### Serializer

**책임**: `Document` IR을 다시 PDF 바이트로 저장

- `lopdf`의 writer 재활용
- 원본을 최대한 보존 (객체 재작성 최소화)
- 새로 생긴 객체만 추가, xref 업데이트

## 배포 타겟별 바인딩

### CLI (`src/main.rs`)

`clap`을 사용해 서브커맨드 구성:

```
rpdf info <file>
rpdf dump <file> [-p PAGE]
rpdf dump-pages <file> [-p PAGE]
rpdf export-svg <file> [-o DIR] [--debug-overlay]
rpdf merge <files...> -o <output>
rpdf split <file> --pages <range>
rpdf rotate <file> --page <N> --degrees <90|180|270>
rpdf diff <a> <b>
```

### WASM API (`src/wasm_api.rs`)

`wasm-bindgen`으로 JavaScript 바인딩:

```rust
#[wasm_bindgen]
pub struct PdfDocument {
    inner: Document,
}

#[wasm_bindgen]
impl PdfDocument {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<PdfDocument, JsValue> { ... }

    pub fn page_count(&self) -> usize { ... }
    pub fn render_page_svg(&self, index: usize) -> String { ... }
    pub fn rotate_page(&mut self, index: usize, degrees: i32) -> Result<(), JsValue> { ... }
    pub fn save(&self) -> Vec<u8> { ... }
}
```

### Tauri Commands

```rust
#[tauri::command]
async fn open_pdf(path: String) -> Result<PdfHandle, String> { ... }

#[tauri::command]
async fn merge_pdfs(handles: Vec<PdfHandle>, output: String) -> Result<(), String> { ... }

#[tauri::command]
async fn get_thumbnail(handle: PdfHandle, page: usize) -> Result<Vec<u8>, String> { ... }
```

핵심: Tauri는 WASM을 쓰지 않고, **Rust 코어를 직접 링크**합니다. 네이티브 성능을 얻기 위함입니다.

## 의존성 방향

의존성은 한 방향으로만 흐릅니다.

```
UI → 바인딩 → document_core → model → parser/serializer/renderer
```

**금지 사항**
- `parser`가 `document_core`를 참조하는 것
- `model`이 UI 타입을 참조하는 것
- `renderer`가 `Command`를 실행하는 것

## 에러 처리

- 코어는 `thiserror` 기반 자체 에러 타입 (`RpdfError`)
- WASM 바인딩에서는 `JsValue`로 변환
- Tauri 커맨드에서는 `String`으로 변환 (Tauri 요구사항)
- UI는 사용자 친화적 메시지로 번역

## 성능 원칙

- 파일 전체를 메모리에 올리지 않음 (스트리밍 가능한 경우)
- 썸네일은 lazy 생성 및 캐시
- 큰 페이지는 viewport 기반 부분 렌더링
- 백그라운드 작업은 항상 async, UI 블록 금지

## 보안 원칙

- `unsafe` 사용 금지 (외부 크레이트 내부는 예외)
- 파일 경로는 Tauri capability 시스템으로 제어
- 악성 PDF 대비: 파싱 시 재귀 깊이 제한, 객체 수 상한

## 확장 포인트

향후 확장을 위해 열어둔 지점:

- **새 파일 포맷**: parser만 추가, IR은 공유
- **새 Command**: `document_core/commands/`에 파일 하나 추가
- **협업 기능**: Command를 직렬화해서 네트워크 전파
- **플러그인**: Command/Query 트레이트를 외부에서 구현 가능
