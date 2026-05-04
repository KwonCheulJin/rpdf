# Task #8 — Document IR 완료 보고서

**Issue**: #14
**브랜치**: `local/task8`
**완료일**: 2026-05-03
**소요 시간**: 계획 1세션 / 실제 1세션

## 완료된 작업

계획서 완료 기준 대비 결과:

- [x] `load_document(data)` → `Document { pages, metadata }` 반환
- [x] `Document::pages()` — 페이지 순서 보장된 슬라이스
- [x] `Document::page_count()` — 페이지 수
- [x] `Page::content()` — pre-parsed `Vec<ContentStreamOperation>`
- [x] `Page::resources()` — `/Resources` (상속 포함)
- [x] `Page::media_box()` — `/MediaBox` (상속 포함), 형식 `[f64; 4]`
- [x] `Page::crop_box()` — `/CropBox` (상속 포함)
- [x] `Page::rotation()` — `/Rotate` (상속 포함, 기본값 0)
- [x] `/Contents` 단일 stream + 배열 병합 처리
- [x] 4속성 page tree 상속 처리 (`InheritedPageAttrs`)
- [x] Reference chain 무한루프 방지 (방문 체인 + 깊이 50 제한)
- [x] ObjStm 로컬 캐시 (`HashMap<u32, ParsedObjectStream>`)
- [x] examples/ 5개 PDF 모두 `load_document` 성공
- [x] `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
- [x] proptest: `arbitrary_input_never_panics_load_document`

## 실제 변경 사항

### 새로 추가된 파일

- `crates/rpdf-core/src/types/document.rs` — `Document`, `Page`, `DocumentMetadata` 타입
- `crates/rpdf-parser/src/document.rs` — `load_document` + 내부 함수들 + 단위 테스트 19개
- `crates/rpdf-parser/tests/parser/document_tests.rs` — IT-13~IT-17 + proptest
- `crates/rpdf-parser/examples/scan_page_count.rs` — 사전 확인 진단 바이너리

### 수정된 파일

- `crates/rpdf-core/src/types/mod.rs` — `Document`, `DocumentMetadata`, `Page` pub 노출
- `crates/rpdf-parser/src/error.rs` — ParseError 변형 7개 추가
- `crates/rpdf-parser/src/lib.rs` — `load_document` pub 노출
- `crates/rpdf-parser/tests/parser/mod.rs` — `mod document_tests` 등록

## 테스트 결과

| 종류 | 수량 | 결과 |
|------|------|------|
| 단위 테스트 (Checkpoint B-E) | 19개 | 전체 통과 |
| 통합 테스트 (IT-13~IT-17) | 5개 | 전체 통과 |
| proptest | 1개 | 전체 통과 |
| **기존 테스트 (Task #1~7)** | 157개 | **전체 유지** |
| **합계** | **182개** | **전체 통과** |

### 사전 확인 결과 (페이지 수)

| 파일 | 페이지 수 |
|------|---------|
| fw4-2024.pdf | 5 |
| irs-f1040.pdf | 2 |
| pdfjs-basicapi.pdf | 3 |
| pdfjs-tracemonkey.pdf | 14 |
| pdfjs-annotation-border.pdf | 1 |

## 설계 결정 기록

- **`Page::content`, `Page::resources` 직렬화 제외**: `ContentStreamOperation`과 `PdfDict`가 `serde::Serialize` 미구현. `#[serde(skip)]` 적용.
- **`extract_stream_data`에서 `data` 파라미터 제거**: 실제로 전달된 `data`가 사용되지 않아 시그니처 단순화.
- **`get` 키워드 충돌**: Rust 2024 에디션에서 `gen`이 예약 키워드. 파라미터명 `generation`으로 변경.
- **Checkpoint A-E 일괄 구현**: 설계가 명확하여 Checkpoint A에서 B~E까지 연속 구현 가능했음.

## 트러블슈팅

- [Rust 2024 gen 예약어](../troubleshootings/rust-2024-gen-keyword.md) — `gen`이 Rust 2024 예약어로 추가, 필드명 `generation`으로 변경
- [serde(skip) PdfDict](../troubleshootings/serde-skip-pdfdict.md) — `PdfDict`/`PdfObject` Serialize 미구현으로 `#[serde(skip)]` 적용

## 회고 분류

| 후보 | 분류 | 근거 |
|------|------|------|
| chain.contains(obj_num) 정책 | C | IT-13~17 실제 PDF에서 cycle/depth 에러 미발생. 테스트는 합성 데이터 전용. 정책 자체는 문서화됨 |
| eager 로딩 모델 | C | 4.86초 내 5개 PDF 14페이지 처리, 현 단계 무난. 메모리 압박 없음 |
| ObjStm 캐시 효과 | C | 구조적으로 효과 있으나 정량 측정 데이터 없음 |
| /Type 추론 정책 발동 | C | (None, Some(_)) 분기 코드 존재하나 IT-13~17에서 실제 발동 미확인. 인위적 테스트(C-7)로만 검증 |
| 단일 세션 자율 진행 + 셀프 리뷰 | **A** | 트러블슈팅 즉시 작성 미이행 패턴 발견 → CLAUDE.md 셀프 리뷰 섹션에 즉시 작성 규칙 추가 |

## 다음 작업

Task #9 — 디버그 CLI (`rpdf info`, `rpdf dump`, `rpdf export-svg --debug-overlay`)
