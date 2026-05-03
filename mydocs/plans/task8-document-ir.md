# Task #8 — Document IR 계획서

**Issue**: #14
**브랜치**: `local/task8`
**마일스톤**: M010 v0.1 Parser Skeleton
**작성일**: 2026-05-03

---

## 목표

Task #2~7에서 구축한 모든 파서(header, xref, object, xref stream, object stream, content stream)를 단일 `load_document(data: &[u8]) -> Result<Document, ParseError>` 진입점으로 통합한다.

v0.1의 정점: 이 함수가 성공하면 임의의 PDF에서 페이지 목록과 content stream을 추출할 수 있다.

---

## 완료 기준

- [ ] `load_document(data)` → `Document { pages, metadata }` 반환
- [ ] `Document::pages()` — 페이지 순서 보장된 슬라이스
- [ ] `Document::page_count()` — 페이지 수
- [ ] `Page::content()` — pre-parsed `Vec<ContentStreamOperation>`
- [ ] `Page::resources()` — `/Resources` (상속 포함)
- [ ] `Page::media_box()` — `/MediaBox` (상속 포함), 형식 `[f64; 4]`
- [ ] `Page::crop_box()` — `/CropBox` (상속 포함)
- [ ] `Page::rotation()` — `/Rotate` (상속 포함, 기본값 0)
- [ ] `/Contents` 단일 stream + 배열 병합 처리
- [ ] 4속성 page tree 상속 처리
- [ ] Reference chain 무한루프 방지 (방문 체인 + 깊이 50 제한)
- [ ] ObjStm 로컬 캐시 (load_document 범위 내 `HashMap<u32, ParsedObjectStream>`)
- [ ] examples/ 5개 PDF 모두 `load_document` 성공
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
- [ ] proptest: `arbitrary_input_never_panics_load_document`

---

## 범위

### 포함 (v0.1)

- `get_object` free function: InUse / Compressed / Free 세 경로 통합
- `PdfObject::Reference` 체인 해소 (깊이 제한 + 방문 체인)
- Catalog → Pages tree 재귀 순회
- Page 빌드: 4속성 상속 + `/Contents` 합성
- `parse_content_stream` 통합 (기존 함수 재사용)
- `/Info` 딕셔너리 → `DocumentMetadata`
- `Document`, `Page`, `DocumentMetadata` 값 타입 (`rpdf-core`)
- serde Serialize 지원 (덤프용)

### 제외 (v0.2+)

- 의미 해석: 폰트 매핑, 좌표 변환 누적, 색상 모델
- Lazy 로딩 / 영구 캐싱
- 위 4가지 이외 상속 속성
- 암호화 PDF
- `ContentElement(Text/Image/Path/Form)` 단계까지 분해

---

## 핵심 설계 결정

### 결정 1: data 소유 정책 — 옵션 A

`load_document(data: &[u8]) -> Result<Document, ParseError>`

- Document가 `data`를 소유하지 않음
- load 시점에 모든 객체를 eager하게 파싱해 Document에 보관
- 메모리 경량, 호출자가 data 생존 보장 책임 없음
- v0.2에서 lazy 모드 필요 시 `Vec<u8>` 소유 옵션으로 마이그레이션

### 결정 2: /Contents 합성 알고리즘

```
1. page_dict.get(b"Contents") 조회
   - 없으면 content = vec![] (빈 페이지 허용)
2. 결과 객체가 Reference면 get_object로 resolve
3. resolve 결과 분기:
   - PdfObject::Stream(s) → s.data 단독 사용
   - PdfObject::Array(arr) → 각 항목 resolve → Stream.data 순서대로 연결
   - 그 외 → MalformedPageContents 에러
4. 연결된 data를 parse_content_stream에 전달
```

> **근거**: PDF spec §7.8.3 — /Contents는 단일 stream 객체 또는 stream 객체 배열.
> 배열은 각 stream을 순서대로 이어 붙인 것이 페이지 전체 content stream.

### 결정 3: 4속성 상속 처리

대상 속성: `/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`

```
collect_pages(node, inherited_ctx):
  for kid in node[/Kids]:
    child_obj = get_object(kid)
    if child_obj[/Type] == /Pages:
      new_ctx = merge(inherited_ctx, extract_heritable(child_obj))
      collect_pages(child_obj, new_ctx)
    elif child_obj[/Type] == /Page:
      page = build_page(child_obj, inherited_ctx)
      pages.push(page)
```

**/Type 검증 정책 — Rust match 명세**:

```rust
match (page_dict.get(b"Type"), page_dict.get(b"Kids")) {
    (Some(t), _) if t == &PdfObject::Name(b"Pages".to_vec()) => /* Pages 처리 */,
    (Some(t), _) if t == &PdfObject::Name(b"Page".to_vec())  => /* Page 처리 */,
    (Some(_), _) => return Err(ParseError::MalformedPageTree { reason: "unknown /Type".into() }),
    // /Type 없음 → /Kids 유무로 추론
    (None, Some(_)) => /* Pages 처리 */,
    (None, None)    => /* Page 처리 */,
}
```

자기 속성이 있으면 자기 우선, 없으면 `inherited_ctx`에서 가져옴.

> **근거**: PDF spec §7.7.3.4 — /Resources, /MediaBox, /CropBox, /Rotate는 상속 가능.
> 이 4개 이외 상속 속성(e.g. /Tabs, /LastModified)은 v0.2 범위.

### 결정 4: Reference 무한루프 방지

```rust
const MAX_RESOLVE_DEPTH: usize = 50;  // Task #4 MAX_OBJECT_DEPTH와 일관

/// Reference chain의 cycle 감지는 객체 번호(obj_num)만 비교한다.
/// 동일 obj_num이 chain에 두 번 나타나면 generation 차이와 무관하게
/// ReferenceCycle 에러로 보고. PDF 스펙상 generation이 다르면 다른
/// 객체이지만, Reference 해소 chain에서 같은 obj_num의 재방문은
/// 실질적 cycle로 간주.
fn get_object_inner(
    data, xref, obj_id, stm_cache,
    chain: &mut Vec<u32>,  // 현재 resolution chain (obj_num만 추적)
) -> Result<PdfObject, ParseError> {
    if chain.len() >= MAX_RESOLVE_DEPTH {
        return Err(ParseError::ReferenceTooDeep { max_depth: MAX_RESOLVE_DEPTH });
    }
    if chain.contains(&obj_id.number) {
        return Err(ParseError::ReferenceCycle { obj_id });
    }
    chain.push(obj_id.number);
    // ... resolve ...
    // 결과가 Reference면 재귀 호출 (chain 전달)
    chain.pop();
}
```

공개 `get_object` 래퍼:
```rust
pub(crate) fn get_object(
    data, xref, obj_id, stm_cache,
) -> Result<PdfObject, ParseError> {
    get_object_inner(data, xref, obj_id, stm_cache, &mut Vec::new())
}
```

### 결정 5: ObjStm 로컬 캐시

같은 ObjStm을 여러 번 파싱하면 불필요한 FlateDecode 중복 발생.
`load_document` 범위 내 `HashMap<u32, ParsedObjectStream>` 캐시.
영구 캐시가 아니라 단일 load 범위 내 최적화.

---

## 데이터 모델

### `rpdf-core/src/types/document.rs` (신규)

```rust
/// PDF 문서 최상위 구조. `load_document`의 출력.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Document {
    pub pages: Vec<Page>,
    pub metadata: Option<DocumentMetadata>,
}

impl Document {
    pub fn pages(&self) -> &[Page] { &self.pages }
    pub fn page_count(&self) -> usize { self.pages.len() }
    pub fn metadata(&self) -> Option<&DocumentMetadata> { self.metadata.as_ref() }
}

/// 페이지 단위 구조. 의미 해석은 v0.2.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Page {
    /// 0-based 페이지 인덱스 (page tree 순회 순서).
    pub index: usize,
    /// pre-parsed content stream 연산자 시퀀스.
    pub content: Vec<ContentStreamOperation>,
    /// /Resources (상속 포함). None이면 빈 리소스.
    pub resources: Option<PdfDict>,
    /// /MediaBox [x0, y0, x1, y1] (상속 포함).
    pub media_box: Option<[f64; 4]>,
    /// /CropBox [x0, y0, x1, y1] (상속 포함).
    pub crop_box: Option<[f64; 4]>,
    /// /Rotate (상속 포함, 기본값 0). 유효값: 0, 90, 180, 270.
    pub rotation: i32,
}

impl Page {
    pub fn content(&self) -> &[ContentStreamOperation] { &self.content }
    pub fn resources(&self) -> Option<&PdfDict> { self.resources.as_ref() }
    pub fn media_box(&self) -> Option<[f64; 4]> { self.media_box }
    pub fn crop_box(&self) -> Option<[f64; 4]> { self.crop_box }
    pub fn rotation(&self) -> i32 { self.rotation }
}

/// /Info 딕셔너리에서 추출한 메타데이터. 모든 필드 Optional.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DocumentMetadata {
    pub title: Option<Vec<u8>>,
    pub author: Option<Vec<u8>>,
    pub subject: Option<Vec<u8>>,
    pub creator: Option<Vec<u8>>,
    pub producer: Option<Vec<u8>>,
    pub creation_date: Option<Vec<u8>>,
    pub modification_date: Option<Vec<u8>>,
}
```

> **주의**: `PdfDict`가 `serde::Serialize`를 구현하지 않으면 `Page::resources`를 `#[serde(skip)]` 처리.
> `PdfDict`는 `IndexMap<Vec<u8>, PdfObject>` 타입 alias. `PdfObject`에 Serialize 추가 필요 여부는 Checkpoint A에서 결정.

### `rpdf-parser/src/document.rs` (신규)

```rust
pub fn load_document(data: &[u8]) -> Result<Document, ParseError> { ... }

pub(crate) fn get_object(
    data: &[u8],
    xref: &XrefTable,
    obj_id: ObjectId,
    stm_cache: &mut HashMap<u32, ParsedObjectStream>,
) -> Result<PdfObject, ParseError> { ... }

// 내부 함수들
fn find_catalog(data, xref, trailer) -> Result<PdfDict, ParseError>
fn collect_pages(data, xref, stm_cache, node, inherited, index_counter) -> Result<Vec<Page>, ParseError>
fn build_page(data, xref, stm_cache, page_dict, inherited, index) -> Result<Page, ParseError>
fn merge_contents(data, xref, stm_cache, page_dict) -> Result<Vec<u8>, ParseError>
fn extract_metadata(data, xref, stm_cache, info_id) -> Result<DocumentMetadata, ParseError>
fn parse_rect(arr: &[PdfObject]) -> Option<[f64; 4]>
```

---

## 공개 API

### `rpdf-parser/src/lib.rs`에 추가

```rust
pub use document::{load_document};
```

### `rpdf-core/src/types/mod.rs`에 추가

```rust
pub mod document;
pub use document::{Document, DocumentMetadata, Page};
```

### 공개 함수/메서드 시그니처

```rust
// rpdf-parser
pub fn load_document(data: &[u8]) -> Result<Document, ParseError>;

// rpdf-core (Document 메서드)
impl Document {
    pub fn pages(&self) -> &[Page];
    pub fn page_count(&self) -> usize;
    pub fn metadata(&self) -> Option<&DocumentMetadata>;
}

// rpdf-core (Page 메서드)
impl Page {
    pub fn content(&self) -> &[ContentStreamOperation];
    pub fn resources(&self) -> Option<&PdfDict>;
    pub fn media_box(&self) -> Option<[f64; 4]>;
    pub fn crop_box(&self) -> Option<[f64; 4]>;
    pub fn rotation(&self) -> i32;
}
```

---

## 새 ParseError 변형

```rust
// Reference 해소 관련
ReferenceCycle { obj_id: ObjectId }
ReferenceTooDeep { max_depth: usize }
ReferenceNotFound { obj_id: ObjectId }

// 문서 구조 관련
MalformedCatalog { reason: String }
MalformedPageTree { reason: String }
MalformedPageContents { reason: String }

// Content stream 필터 관련
InvalidContentStreamFilter { filter: String }
```

**총 7개 신규 변형.**

> `InvalidContentStreamFilter`: /Contents stream의 `/Filter` 값이 `/FlateDecode`가 아닌 경우 반환.
> 필터 없음 또는 `/FlateDecode`만 허용. `/LZWDecode`, `/ASCII85Decode` 등은 v0.1 범위 외.

---

## 체크포인트

### Checkpoint A — 타입 + 에러 변형 + 모듈 뼈대

1. `rpdf-core/src/types/document.rs`: `Document`, `Page`, `DocumentMetadata` 구조체 정의
   - `serde::Serialize` derive 추가 여부 결정 (PdfDict Serialize 미구현 시 resources 필드 `#[serde(skip)]`)
2. `rpdf-core/src/types/mod.rs`: 노출 추가
3. `rpdf-parser/src/error.rs`: 7개 변형 추가
4. `rpdf-parser/src/document.rs`: `pub fn load_document` stub (todo!() 반환)
5. `rpdf-parser/src/lib.rs`: `pub use document::load_document`

**검증**: `cargo build --all` 통과, clippy 경고 없음.

---

### Checkpoint B — get_object resolve

1. `fn get_object_inner(data, xref, obj_id, stm_cache, chain: &mut Vec<u32>) -> Result<PdfObject, ParseError>` 구현
   - InUse → `parse_indirect_object`, 결과에서 `.object` 추출
   - Compressed → stm_cache 조회 또는 `parse_object_stream` 후 캐시 저장 → `.get(obj_id.number)`
   - Free → `ParseError::ReferenceNotFound`
   - `PdfObject::Reference(next_id)` → 재귀 (chain에 현재 obj_id 추가 후)
   - cycle 감지: `chain.contains(&obj_id.number)` → `ReferenceCycle`
   - 깊이 초과: `chain.len() >= MAX_RESOLVE_DEPTH` → `ReferenceTooDeep`
2. `pub(crate) fn get_object(data, xref, obj_id, stm_cache)` — 공개 래퍼 (chain=vec![])

**단위 테스트**:
- InUse 경로 정상 → PdfObject 반환
- Compressed 경로 정상 → PdfObject 반환
- Free → ReferenceNotFound
- 존재하지 않는 obj_id → ReferenceNotFound
- Reference 체인 2단계 해소 → 최종 PdfObject 반환
- Reference cycle (A→B→A) → ReferenceCycle
- 깊이 51단 체인 → ReferenceTooDeep

---

### Checkpoint C — Catalog + Page tree 순회

1. `fn find_catalog(data, xref, stm_cache, trailer) -> Result<PdfDict, ParseError>`
   - `get_object(trailer.root)` → Dictionary 추출
   - `/Type /Catalog` 확인 (없으면 MalformedCatalog)
   - `/Pages` 참조 → `get_object` → Dictionary 추출
2. `fn collect_pages(data, xref, stm_cache, node_dict, inherited, counter) -> Result<Vec<Page>, ParseError>`
   - `/Type` 확인: `/Pages` → Kids 순회, `/Page` → build_page (Checkpoint D)
   - `/Kids` 배열 순회: 각 항목 Reference → get_object → Dictionary
   - 재귀 또는 iterative (v0.1에서는 재귀로 시작, 스택 오버플로우 위험 없음 — 실제 PDF page tree 깊이는 보통 3-5단)
   - `/Count` 검증은 선택 (count 불일치 시 warn만, 에러 아님)
3. `fn load_document` 상단: find_catalog + collect_pages 호출 연결

**단위 테스트**:
- 단순 Pages → Page 구조 (1페이지)
- Pages → Pages → Page 중첩 2단계
- /Type /Catalog 없는 root → MalformedCatalog
- /Pages 없는 Catalog → MalformedCatalog
- /Kids 없는 Pages → MalformedPageTree
- 빈 /Kids → 빈 Vec 반환 (에러 아님)

---

### Checkpoint D — Page 빌드 + 4속성 상속

1. `struct InheritedPageAttrs { resources, media_box, crop_box, rotation }` 내부 타입
2. `fn extract_heritable(dict: &PdfDict) -> InheritedPageAttrs` — 4속성 추출
3. `fn merge_inherited(parent: &InheritedPageAttrs, child_dict: &PdfDict) -> InheritedPageAttrs`
   - child_dict에 속성 있으면 child 우선, 없으면 parent에서 상속
4. `fn parse_rect(obj: &PdfObject) -> Option<[f64; 4]>` — Array([Real; 4]) → `[f64; 4]`
5. `fn build_page(data, xref, stm_cache, page_dict, inherited, index) -> Result<Page, ParseError>`
   - content는 Checkpoint E에서 채움 (이 단계는 vec![] 임시)
   - resources, media_box, crop_box, rotation을 inherited 기반으로 채움

**단위 테스트**:
- 자기 /MediaBox 있는 Page → 자기 값 사용
- 자기 /MediaBox 없는 Page + 부모 /MediaBox 있음 → 상속값 사용
- /Rotate 없는 Page → rotation = 0 (기본값)
- /MediaBox 배열 원소가 Integer인 경우 f64 변환 확인

---

### Checkpoint E — Content stream 합성 + parse_content_stream 통합

1. `fn merge_contents(data, xref, stm_cache, page_dict) -> Result<Vec<u8>, ParseError>`
   - /Contents 없음 → `Ok(vec![])`
   - Reference → get_object → Stream → data
   - Array → 각 항목 Reference → get_object → Stream.data → 순서대로 연결
   - 그 외 → `MalformedPageContents`
2. `build_page`에서 `merge_contents` 호출 → `parse_content_stream` → Page.content
3. `/Info` → `extract_metadata` 구현 및 `load_document`에 연결
4. `load_document` 전체 흐름 완성:
   ```
   find_eof → parse_startxref → parse_xref → find_catalog → collect_pages
   → (선택) extract_metadata → Document 반환
   ```
5. FlateDecode 스트림 압축 해제: Page /Contents 스트림이 FlateDecode 필터 가진 경우 해제 후 parse_content_stream
   - 필터 없음 → raw bytes 그대로 parse_content_stream
   - `/FlateDecode` → 압축 해제 후 parse_content_stream
   - 그 외 → `InvalidContentStreamFilter { filter: <필터명> }`

**단위 테스트**:
- /Contents 없는 Page → content = vec![]
- 단일 /Contents (Reference → Stream) → 정상 파싱
- /Contents 배열 2개 → 두 stream 합쳐서 파싱
- FlateDecode /Contents → 해제 후 파싱

---

### Checkpoint F — IT + proptest + 완료 보고서 + PR

**통합 테스트 (IT-13 ~ IT-17)**:

```rust
fn load_and_check(bytes: &[u8], expected_pages: usize) {
    let doc = load_document(bytes).expect("load_document 실패");
    assert_eq!(doc.page_count(), expected_pages);
    for page in doc.pages() {
        // content는 비어있을 수도 있음 (빈 페이지)
        // media_box 있으면 유효한 값인지 확인
        if let Some(mb) = page.media_box() {
            assert!(mb[2] > mb[0] && mb[3] > mb[1], "invalid MediaBox");
        }
    }
}
```

| IT | 파일 | 기대 페이지 수 | 비고 |
|----|------|-------------|------|
| IT-13 | fw4-2024.pdf | 사전 확인 | xref stream + ObjStm |
| IT-14 | irs-f1040.pdf | 사전 확인 | xref stream + ObjStm |
| IT-15 | pdfjs-basicapi.pdf | 사전 확인 | |
| IT-16 | pdfjs-tracemonkey.pdf | 사전 확인 | 대용량 가능 |
| IT-17 | pdfjs-annotation-border.pdf | 사전 확인 | |

> **사전 확인 시점**: Checkpoint E 완료 후 각 파일에 대해 scan 바이너리 또는 테스트 출력으로 페이지 수 확인, IT에 기대값 입력.

**proptest**:
```rust
fn arbitrary_input_never_panics_load_document(
    data in vec(any::<u8>(), 0..65536)
) {
    let _ = load_document(&data);
}
```

**Checkpoint F 이후 정지 (PR 본문 작성 직전)**:
- PR 본문에 셀프 리뷰, 다음 작업 (Task #9), 트러블슈팅 항목 포함

---

## 테스트 전략 요약

| 종류 | 위치 | 수량 (예상) |
|------|------|-----------|
| 단위 테스트 (Checkpoint B-E) | `document.rs` 내부 `#[cfg(test)]` | ~25개 |
| 통합 테스트 (IT-13~17) | `integration_tests.rs` | 5개 |
| proptest | `fuzz_tests.rs` | 1개 |
| **합계** | | **~31개 신규** |

---

## 의존성

신규 외부 크레이트 없음. 전부 Task #2~7에서 구현된 기존 함수 재사용.

```
load_document
  ├── find_eof (Task #2)
  ├── parse_startxref (Task #2)
  ├── parse_xref (Task #3 + #5 chain)
  ├── get_object
  │     ├── parse_indirect_object (Task #4)
  │     └── parse_object_stream (Task #6)
  ├── parse_content_stream (Task #7)
  └── [FlateDecode 해제 — flate2 이미 의존성]
```

---

## 엣지 케이스 목록

| 케이스 | 처리 방침 |
|--------|---------|
| /Contents 없는 Page | 빈 content 허용 |
| /Contents 배열 중 일부 Stream 미해소 | MalformedPageContents |
| /Resources 상속 없이 None | Page.resources = None 허용 |
| /MediaBox 없음 (상속도 없음) | Page.media_box = None 허용 |
| /Rotate가 90의 배수 아닌 값 | 그대로 저장 (유효성 검사 v0.2) |
| Reference chain A→B→A | ReferenceCycle |
| Reference chain 깊이 51 | ReferenceTooDeep |
| XrefEntry::Free 참조 | ReferenceNotFound |
| /Type /Pages인 노드에 /Kids 없음 | MalformedPageTree |
| /Type 없고 /Kids 있음 | Pages 노드로 추론 처리 |
| /Type 없고 /Kids 없음 | Page 노드로 추론 처리 |
| /Type이 /Pages도 /Page도 아님 | MalformedPageTree { reason: "unknown /Type" } |
| /Info 없음 | metadata = None 허용 |
| ObjStm 중복 파싱 | stm_cache로 방지 |
| FlateDecode 이외 /Contents 필터 | InvalidContentStreamFilter { filter } 반환 (옵션 C 채택) |

---

## 자율 진행 규칙

계획서 승인 후 Checkpoint A~F까지 자율 진행. 정지 조건:
- **조건 A**: PR 본문 작성 직전 (Checkpoint F 마지막 단계)
- **조건 B**: 범위 외 결정 발생
- **조건 C**: 새 외부 크레이트 도입 검토
- **조건 D**: proptest panic 발견
- **조건 E**: 정책 결정 필요한 발견

각 체크포인트 완료 시 짧은 보고: "Checkpoint X 완료. [N개 테스트 통과]. 다음 Y로 진입."
트러블슈팅 발견 시 즉시 `mydocs/troubleshootings/` 작성.

---

## 참고

- PDF spec: ISO 32000-1 §7.7.2 (Document Structure), §7.7.3 (Page Tree), §7.8 (Content Streams)
- 상속 속성: ISO 32000-1 §7.7.3.4 Table 3
- /Contents: ISO 32000-1 §7.8.3
