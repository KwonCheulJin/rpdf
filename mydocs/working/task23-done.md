# Task #23 완료 보고서: rpdf-serializer 구현

**이슈**: #44  
**브랜치**: `local/task23`  
**완료일**: 2026-05-19

---

## 요약

`rpdf-serializer` 크레이트를 신규 생성했다. lopdf 0.40.0을 직렬화 백엔드로 사용해 `Document` IR + `Vec<PageSource>`를 PDF 바이트(`Vec<u8>`)로 변환한다. 단일 소스(같은 원본 PDF에서 온 페이지들)와 다중 소스(여러 PDF를 합친 Document) 두 경로를 각각 구현했다.

rpdf-core / rpdf-parser / rpdf-edit은 변경 없음. source tracking을 rpdf-serializer 내부 타입(`PageSource`)으로 분리해 도메인 레이어 오염을 방지했다.

---

## 구현 내용

### 신규 파일

| 파일 | 역할 |
|------|------|
| `crates/rpdf-serializer/Cargo.toml` | 신규 크레이트 설정 |
| `crates/rpdf-serializer/src/lib.rs` | pub 재수출 |
| `crates/rpdf-serializer/src/types.rs` | `PageSource { bytes: Arc<Vec<u8>>, page_index: usize }` |
| `crates/rpdf-serializer/src/error.rs` | `SerializeError` 5개 변형 |
| `crates/rpdf-serializer/src/serialize.rs` | `load_document_tracked` + `serialize_document` |
| `crates/rpdf-serializer/tests/serialize_tests.rs` | 10개 통합 테스트 |

### 수정 파일

| 파일 | 변경 |
|------|------|
| `Cargo.toml` (루트) | `crates/rpdf-serializer` 멤버 추가, `lopdf = "0.40.0"` workspace dep 추가 |

### 핵심 설계 결정

- **source tracking 분리** (D1=B): `rpdf-core::Page`에 `source_bytes` 추가 불가. 도메인 레이어에 직렬화 관심사 오염. `PageSource`를 rpdf-serializer 내부 타입으로 분리.
- **rotation 항상 쓰기** (D2=A): `rotation == 0`이어도 `/Rotate 0` 명시적 설정. 원본 /Rotate를 무조건 덮어씀. 조건 분기 없음. (90→0 복원 버그 방지)
- **multi-source pseudo-code** (D3=A): renumber_objects_with → objects 병합 → Catalog/Pages 재구성 → /Parent 갱신 → rotation → save_to 전체 흐름 계획서에 명세.
- **rotate-to-zero 테스트** (D4=A): 90→0 복원 경로 테스트 `serialize_after_rotate_to_zero` 추가.
- **이중 파서 리스크** (D5=A): rpdf-parser 허용 PDF를 lopdf가 거부 가능 → `LoadSource` 에러 메시지에 "lopdf incompatible" 명시, 알려진 한계 문서화.
- **Merge Outlines/AcroForms** (D6=A): renumber + Kids 재구성만으로는 북마크/폼 소실 → 알려진 한계 문서화, v0.4 이후 보완.

### 에러 처리

| 변형 | 조건 | 발생 위치 |
|------|------|---------|
| `EmptyDocument` | `doc.pages.is_empty()` | serialize.rs:56 |
| `SourceLengthMismatch` | `sources.len() != doc.pages.len()` | serialize.rs:61 |
| `LoadSource` | lopdf::load_mem 실패 | serialize.rs:88 (single), 164 (multi) |
| `PageOutOfBounds` | `source.page_index >= lopdf_pages.len()` | serialize.rs:104 (single), 189 (multi) |
| `Save` | save_to IO 에러 (`#[from]`) | serialize.rs:358 |

### 알려진 한계

- **이중 파서 불일치**: rpdf-parser가 허용한 PDF를 lopdf가 거부할 수 있음. `LoadSource` 에러로 표면화됨.
- **Merge 시 Outlines/AcroForms 유실**: renumber + Kids 재구성만으로는 북마크·폼 데이터 소실. v0.4에서 보완 예정.
- **O(N²) Split 성능**: 대량 split 시 load_mem 반복 호출. v0.3 범위에서 허용.

---

## 테스트 결과

10개 통합 테스트 전체 통과.

| # | 테스트명 | 결과 |
|---|---------|------|
| 1 | `serialize_basic_roundtrip` | PASS |
| 2 | `serialize_after_rotate` (0→90) | PASS |
| 3 | `serialize_after_rotate_to_zero` (90→0) | PASS |
| 4 | `serialize_after_delete` | PASS |
| 5 | `serialize_after_extract` | PASS |
| 6 | `serialize_after_split` | PASS |
| 7 | `serialize_after_merge` (다중 소스) | PASS |
| 8 | `serialize_empty_doc_error` | PASS |
| 9 | `serialize_source_length_mismatch_error` | PASS |
| 10 | `serialize_page_out_of_bounds_error` | PASS |

Fixtures: `examples/pdfjs-basicapi.pdf` (3p), `examples/irs-f1040.pdf` (2p), `examples/fw4-2024.pdf` (5p)

---

## 품질 게이트

| 게이트 | 결과 |
|--------|------|
| `cargo test --workspace --exclude rpdf-render` | PASS |
| `cargo clippy --workspace --exclude rpdf-render -- -D warnings` | PASS (경고 0) |
| `cargo fmt --all --check` | PASS |

---

## evaluator 검증 결과

4개 generator 요청 항목 모두 PASS.

| # | 항목 | 결과 |
|---|------|------|
| 1 | serialize_multi_source Catalog/Pages 재구성 + /Parent 갱신 | PASS — lopdf 공식 merge 예제와 일치 |
| 2 | renumber_objects_with 후 get_pages() 재호출 | PASS — renumber 후 BTreeMap 키 교체되므로 재계산 필수, 올바름 |
| 3 | delete_pages 후 ObjectId 불변 가정 | PASS — lopdf가 objects.remove만 수행, 다른 OID 재배치 없음 |
| 4 | SerializeError 5개 변형 dead variant 없음 | PASS — 전부 발생 경로 확인 |

Enhancement (선택적): 다중 소스 경로에서 원본 소스 내 유지되지 않는 페이지들이 orphan 오브젝트로 남아 파일 크기 소폭 증가 가능. `prune_objects()` 호출로 제거 가능하나 v0.3 범위에서 허용 가능한 수준.

---

## plan-eng-review 결과

outside voice (Gemini) 3건 발견:

| # | 발견 내용 | 판정 | 처리 |
|---|---------|------|------|
| 1 | 이중 파서 불일치 리스크 | VALID | 알려진 한계 + LoadSource 에러메시지 개선 |
| 2 | Merge 시 Outlines/AcroForms 유실 | VALID | 알려진 한계로 문서화, v0.4 이후 보완 |
| 3 | O(N²) Split 성능 | VALID/TODO | v0.3 범위에서 허용, 알려진 한계 문서화 |

---

## 회고 분류

| 항목 | 분류 | 내용 |
|------|------|------|
| rotation ≠ 0 조건 버그가 계획 초안에 있었음 | A (즉시 반영) | rotation 관련 로직은 "항상 쓰기" 패턴이 안전. 조건 분기는 버그를 숨김. CLAUDE.md "rotation 항상 쓰기" 교훈으로 추가 검토 필요 |
| rpdf-core에 직렬화 관심사 오염 시도 | B | PageSource를 직렬화 크레이트 내부에 두는 패턴: 도메인 레이어(rpdf-core)에는 파싱·직렬화 관심사 없음. 새 크레이트 추가 시 rpdf-core 변경 유혹 주의 |
| evaluator가 orphan 오브젝트 issue 발견 | C | 기능 동작에 영향 없는 파일 크기 소폭 증가. v0.3 허용 수준. prune_objects() 패턴 기억 |
| gemini -C 플래그 미지원 오류 → approval-mode plan으로 수정 | B | gemini CLI 플래그: `--approval-mode plan -p "$PROMPT"` 패턴 사용. `-C` 디렉토리 플래그 미지원 |
