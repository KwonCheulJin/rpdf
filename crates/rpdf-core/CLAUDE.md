# rpdf-core — 도메인 타입 크레이트

## 역할

PDF 도메인 타입만 정의한다. 파싱·렌더링·I/O 로직 없음.

## 모듈 구조

| 모듈 | 내용 |
|------|------|
| `types/document.rs` | `Document`, `Page` — 최상위 IR |
| `types/object.rs` | `PdfObject` — PDF 객체 열거형 |
| `types/object_id.rs` | `ObjRef(u64, u16)` — 간접 참조 (num, gen) |
| `types/xref.rs` | `XrefTable`, `XrefEntry` |
| `types/content_stream.rs` | `ContentStreamOperation` — 페이지 콘텐츠 IR |
| `types/pdf_version.rs` | `PdfVersion` |

## 불변 규칙

- 모든 값 객체: `#[derive(Copy, Clone, PartialEq, Eq)]` — 이를 만족 못하면 `rpdf-core`에 둘 수 없음
- 파싱 로직 추가 금지 — `rpdf-parser`로 이동할 것
- 외부 의존성 추가 금지 — 어떤 타겟(CLI/WASM/Tauri)에도 컴파일 가능해야 함

## 변경 시 영향 범위

`rpdf-core` 타입 변경 → `rpdf-parser`, `rpdf-render`, `rpdf-cli` 전체 영향.  
타입 변경 전 반드시 모든 크레이트 `cargo check` 확인.
