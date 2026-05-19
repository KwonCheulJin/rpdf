# Task #23: Serializer 구현 (Document → PDF 저장)

**이슈**: #44  
**브랜치**: `local/task23`  
**마일스톤**: v0.3

---

## 목적 및 배경

Task #17~#22에서 5개 편집 커맨드(Rotate/Delete/Merge/Split/Extract)를 구현했으나,
편집 결과를 PDF 파일로 저장하는 수단이 없다. 이 작업은 `Document` IR을 유효한 PDF
바이트로 직렬화하는 `rpdf-serializer` 크레이트를 구현한다.

### 핵심 설계 결정: lopdf 백엔드 + source tracking 분리 (plan-eng-review 반영)

현재 `Document` IR은 논리적 페이지 구조만 보유하며, 페이지가 참조하는 embedded 객체
(폰트, 이미지, ICC 프로파일 등)를 포함하지 않는다. 이 객체들을 재구성하는 것은
v0.3 범위를 벗어나므로, **lopdf를 serialization 백엔드**로 사용한다.

**결정 사항 (plan-eng-review):**
- `rpdf-core` 변경 없음 — 직렬화 관심사를 도메인 타입에 오염하지 않는다.
- `rpdf-parser` 변경 없음 — 파싱 API는 기존 그대로.
- `rpdf-serializer`가 `PageSource` 추적 + lopdf 조작을 모두 담당한다.
- 호출자(CLI)가 `Vec<PageSource>`를 `doc.pages`와 동기화할 책임을 진다.

공개 API 확인: lopdf 0.40.0 (`Document::load_mem`, `save_to`, `get_pages`,
`delete_pages`, `get_object_mut`, `renumber_objects_with`) — docs.rs 확인 완료.

---

## API 설계

### rpdf-serializer (신규 크레이트)

**Cargo.toml**:
```toml
[package]
name = "rpdf-serializer"
version = "0.1.0"

[dependencies]
rpdf-core.workspace = true
rpdf-parser.workspace = true
lopdf = "0.40.0"
thiserror.workspace = true
```

**타입 및 공개 API**:
```rust
use std::sync::Arc;

/// 단일 페이지의 원본 PDF 출처를 기록하는 직렬화 힌트.
///
/// load_document_tracked()가 반환하는 Vec<PageSource>는
/// Document.pages와 1:1 대응한다: sources[i] → pages[i].
pub struct PageSource {
    /// 원본 PDF 바이트.
    pub bytes: Arc<Vec<u8>>,
    /// 원본 PDF에서의 0-based 페이지 인덱스.
    pub page_index: usize,
}

/// PDF 바이트에서 Document와 PageSource 목록을 함께 반환한다.
///
/// Document.pages[i]에 대응하는 원본 출처가 sources[i]에 담겨있다.
/// 커맨드 실행 후에는 호출자가 sources를 pages와 동기화해야 한다.
pub fn load_document_tracked(
    data: &[u8],
) -> Result<(Document, Vec<PageSource>), ParseError>;

/// Document IR을 PDF 바이트로 직렬화한다.
///
/// `sources[i]`는 `doc.pages[i]`의 원본 출처여야 한다.
/// `sources.len() != doc.pages.len()` 이면 에러를 반환한다.
///
/// # Errors
///
/// - [`SerializeError::EmptyDocument`] — pages가 비어있음
/// - [`SerializeError::SourceLengthMismatch`] — sources와 pages 개수 불일치
/// - [`SerializeError::LoadSource`] — lopdf가 source_bytes 로드 실패
/// - [`SerializeError::PageOutOfBounds`] — source.page_index가 원본 페이지 수 초과
/// - [`SerializeError::Save`] — lopdf save_to 실패
pub fn serialize_document(
    doc: &Document,
    sources: &[PageSource],
) -> Result<Vec<u8>, SerializeError>;

/// 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum SerializeError {
    #[error("document has no pages")]
    EmptyDocument,
    #[error("sources length {sources} != pages length {pages}")]
    SourceLengthMismatch { sources: usize, pages: usize },
    #[error("failed to load source PDF (lopdf incompatible): {0}")]
    LoadSource(lopdf::Error),
    #[error("source_page_index {idx} out of bounds (source has {count} pages)")]
    PageOutOfBounds { idx: usize, count: usize },
    #[error("lopdf save failed: {0}")]
    Save(#[from] std::io::Error),
}
```

### 호출자의 sources 동기화 책임

커맨드 실행 후 호출자는 `sources`를 `doc.pages`와 동기화해야 한다.

| 커맨드 | sources 처리 |
|--------|-------------|
| RotatePageCommand | 변경 없음 (page 순서 동일) |
| DeletePagesCommand (0-based idx: [1, 3]) | `[s0, s2, s4]` — 삭제 인덱스 제거 |
| ExtractPagesCommand (1-5페이지 extract) | `sources[0..=4]` — 범위 슬라이스 |
| SplitCommand (결과 docs 여러 개) | 각 결과 doc의 page.index로 sources 슬라이스 |
| MergeCommand (target + source들) | a_sources + b_sources + ... 순서로 연결 |

---

## 알고리즘 설계

### load_document_tracked

```
fn load_document_tracked(data: &[u8]) -> Result<(Document, Vec<PageSource>), ParseError>
    let doc = load_document(data)?          // rpdf-parser
    let bytes_arc = Arc::new(data.to_vec()) // 한 번만 clone
    let sources = doc.pages.iter()
        .map(|p| PageSource { bytes: Arc::clone(&bytes_arc), page_index: p.index })
        .collect()
    Ok((doc, sources))
```

### serialize_document — 단일 소스 경로

모든 `sources[i].bytes`가 동일한 Arc 포인터를 가리키는 경우.

```
fn serialize_document(doc, sources) -> Result<Vec<u8>, SerializeError>

1. doc.pages 비어있으면 → EmptyDocument
2. sources.len() != doc.pages.len() → SourceLengthMismatch
3. 소스 그룹핑: Arc 포인터로 동일 소스 식별
   → groups: Vec<(Arc<Vec<u8>>, Vec<(out_pos, src_page_idx)>)>

4. [단일 소스]
   a. lopdf::Document::load_mem(&bytes) → lopdf_doc
      실패 시 → LoadSource (메시지: "lopdf incompatible")
   b. page_oid_map 구성 (삭제 전):
      lopdf_doc.get_pages() → BTreeMap<u32, ObjectId>
      → HashMap<usize(src_page_idx), ObjectId>
   c. 유지할 원본 페이지 번호 = { source.page_index + 1 | source ∈ sources }
   d. 삭제할 번호 = 전체 페이지 번호 집합 - 유지 집합
   e. lopdf_doc.delete_pages(&delete_list)
   f. [rotation 항상 적용 — D2 수정]
      for (page, source) in doc.pages.iter().zip(sources.iter()):
          let oid = page_oid_map[source.page_index]
          let page_obj = lopdf_doc.get_object_mut(oid)?
          if let Dictionary(ref mut dict) = page_obj:
              dict.set("Rotate", Object::Integer(page.rotation as i64))
              // rotation == 0이어도 반드시 쓴다
   g. out = Vec::new()
      lopdf_doc.save_to(&mut out) → Save 에러
   h. return Ok(out)
```

### serialize_document — 다중 소스 경로 (D3 추가)

```
4. [다중 소스 — MergeCommand 결과]
   a. 각 소스 그룹별 lopdf::Document::load_mem() 로드
      → lopdf_docs: Vec<(Arc<Vec<u8>>, lopdf::Document)>

   b. max_id 추적 변수 (처음엔 첫 번째 소스의 max object id)
      lopdf_docs[0].get_object_id_count() 또는
      *lopdf_docs[0].objects.keys().max().unwrap() 사용

   c. 두 번째 소스부터:
      lopdf_doc.renumber_objects_with(max_id + 1)
      max_id = 새 max object id

   d. 첫 번째 소스를 base로:
      base = lopdf_docs[0]
      for doc in lopdf_docs[1..]:
          for (oid, obj) in doc.objects.iter():
              base.objects.insert(*oid, obj.clone())

   e. base의 페이지 트리 재구성:
      final_kids: Vec<ObjectId> = doc.pages 순서대로
          sources[i].page_index → 해당 소스 lopdf_doc의 page ObjectId
          (renumber 후 ObjectId 매핑 유지 필요 → step b에서 매핑 테이블 구성)
      base의 /Pages 딕셔너리 /Kids 배열 = final_kids
      base의 /Pages /Count = final_kids.len()

   f. rotation 항상 적용 (단일 소스와 동일)

   g. base.save_to(&mut out)
```

---

## 에러 표 ↔ 로직 교차 검증

| 에러 변형 | 로직 발생 지점 |
|-----------|--------------|
| `EmptyDocument` | 1. (pages 비어있음 즉시) |
| `SourceLengthMismatch` | 2. (len 체크) |
| `LoadSource` | 4a, 다중 소스 a (lopdf load_mem 실패) |
| `PageOutOfBounds` | 4b, 다중 소스 e (source.page_index 유효성 검사) |
| `Save` | 4h, 다중 소스 g (save_to 실패) |

모든 에러 변형에 대응하는 로직 경로 존재 — 교차 검증 완료.

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| 0-page document | `EmptyDocument` 에러 |
| sources.len() != pages.len() | `SourceLengthMismatch` 에러 |
| source.page_index >= 원본 페이지 수 | `PageOutOfBounds` 에러 |
| rotation == 0 (원본이 /Rotate 90이었다가 0으로 복원) | 항상 0으로 덮어씀 — D2 수정 |
| 단일 페이지 PDF에서 extract | 삭제 없이 그대로 저장 |
| 역순 페이지 정렬 | v0.3 scope-out 참조 |

---

## v0.3 알려진 한계 (D5, D6 반영)

| 한계 | 설명 |
|------|------|
| 이중 파서 비호환 리스크 | rpdf-parser가 허용한 PDF를 lopdf가 거부하면 `LoadSource` 에러 발생. `LoadSource` 에러 메시지: "failed to load source PDF (lopdf incompatible)". 사용자는 원본 PDF 문제임을 명확히 인지 가능. |
| Merge 시 PDF 구조 유실 | Outlines(북마크), AcroForms(폼 데이터), Named Destinations 등 전역 구조가 유실될 수 있다. v0.4에서 보완. |
| O(N²) Split 성능 | 대량 페이지 split 시 각 Document 직렬화마다 lopdf::load_mem 호출. 동일 소스 Document를 일괄 직렬화하거나 외부 캐싱으로 완화 가능. v0.3 CLI 사용 범위에서는 허용 수준. |
| 역순 페이지 재정렬 | doc.pages 순서가 원본 오름차순과 다른 경우 미지원. v0.4 이후 Pages Kids 재구성으로 해결 예정. |
| synthesized Page 직렬화 | source_bytes 없이 동적으로 생성된 Page 직렬화 불가. |
| 암호화 PDF 저장 | 미지원. |
| incremental update 모드 | 미지원 (full rewrite만). |

---

## 테스트 전략

### 단위 테스트 (`crates/rpdf-serializer/tests/`)

기존 `samples/` 및 `examples/` 픽스처 활용 — 신규 fixture 불필요.
**테스트 전 `rpdf info <file>`로 페이지 수 확인 필수.**

| # | 테스트명 | 검증 | Fixture |
|---|---------|------|---------|
| 1 | `serialize_basic_roundtrip` | parse → serialize → re-parse → page count 동일 | examples/pdfjs-basicapi.pdf (3p) |
| 2 | `serialize_after_rotate` | rotation 0→90 후 re-parse → rotation == 90 | examples/pdfjs-basicapi.pdf |
| 3 | `serialize_after_rotate_to_zero` | rotation 90→0 후 re-parse → rotation == 0 (D4 추가) | examples/irs-f1040.pdf (rotation 있는 fixture) or 수동 구성 |
| 4 | `serialize_after_delete` | DeletePagesCommand 후 re-parse → page count 감소 | examples/pdfjs-basicapi.pdf (3p→2p) |
| 5 | `serialize_after_extract` | ExtractPagesCommand 후 re-parse → 추출 페이지 수 | examples/fw4-2024.pdf (5p) |
| 6 | `serialize_after_split` | SplitCommand 후 각 Document serialize → 각 페이지 수 | examples/fw4-2024.pdf (5p) |
| 7 | `serialize_after_merge` | MergeCommand 후 serialize → re-parse → 합산 페이지 수 | examples/irs-f1040.pdf (2p) + pdfjs-basicapi.pdf (3p) → 5p |
| 8 | `serialize_empty_doc_error` | 빈 Document → EmptyDocument 에러 | — |
| 9 | `serialize_source_length_mismatch_error` | sources.len() != pages.len() → SourceLengthMismatch 에러 | — |
| 10 | `serialize_page_out_of_bounds_error` | source.page_index 초과 → PageOutOfBounds 에러 | — |

---

## 테스트 커버리지 다이어그램

```
CODE PATHS: serialize_document()
  ├── [★★★ TEST #8] EmptyDocument error
  ├── [★★★ TEST #9] SourceLengthMismatch error
  ├── [SINGLE SOURCE]
  │   ├── [★★★ TEST #1] basic roundtrip
  │   ├── [★★★ TEST #10] PageOutOfBounds error
  │   ├── [★★★ TEST #2] rotate 0→90
  │   ├── [★★★ TEST #3] rotate 90→0 (D4 추가)
  │   ├── [★★★ TEST #4] delete pages
  │   ├── [★★★ TEST #5] extract pages
  │   └── [★★★ TEST #6] split pages
  └── [MULTI SOURCE]
      └── [★★★ TEST #7] merge two sources

COVERAGE: 10/10 paths (100%)
QUALITY: ★★★:10
GAPS: 0
```

---

## 파일 변경 목록

| 파일 | 변경 종류 |
|------|---------|
| ~~crates/rpdf-core/src/types/document.rs~~ | 변경 없음 (D1=B 결정) |
| ~~crates/rpdf-parser/src/document.rs~~ | 변경 없음 (D1=B 결정) |
| ~~crates/rpdf-edit/src/commands/mod.rs~~ | 변경 없음 |
| `crates/rpdf-serializer/` | 신규 크레이트 (lib.rs, serialize.rs, types.rs, error.rs, tests/) |
| `Cargo.toml` (workspace) | rpdf-serializer 멤버 추가, lopdf workspace dep 추가 |
| `crates/rpdf-serializer/Cargo.toml` | 신규 |

**참고**: rpdf-core·rpdf-parser·rpdf-edit 변경 없음.

---

## 체크포인트

1. **CP-A**: rpdf-serializer 크레이트 스캐폴딩 — `cargo build` 통과
2. **CP-B**: EmptyDocument/SourceLengthMismatch/PageOutOfBounds 에러 테스트 통과
3. **CP-C**: 단일 소스 roundtrip (test #1, #8, #9, #10) 통과
4. **CP-D**: rotate/delete/extract/split 테스트 (test #2, #3, #4, #5, #6) 통과
5. **CP-E**: merge (다중 소스) 테스트 (test #7) 통과
6. **CP-F**: 전체 `cargo test`, `cargo clippy`, `cargo fmt --check` 통과

---

## NOT in scope (이번 PR에서 다루지 않는 것)

- 역순 페이지 재정렬 serialization
- synthesized Page (source 없이 생성된 Page) 직렬화
- incremental update 모드
- 암호화 PDF 저장
- Outlines/AcroForms 보존 merge (v0.4)
- O(N²) split 성능 최적화 (v0.4)
- rpdf-core 변경 (도메인 순수성 유지)

---

## What already exists

- `samples/` 및 `examples/`에 28개 이상 PDF fixture — 신규 생성 불필요
- `rpdf-parser::load_document` — 내부적으로 `load_document_tracked`가 래핑
- `rpdf-edit::commands` 5개 — 변경 없이 그대로 사용
- `rpdf-core::types::document::Document, Page` — 변경 없이 그대로 사용

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 6 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0 (모든 6개 결정 사항 해결됨)
- **VERDICT:** ENG CLEARED — 구현 시작 가능.
