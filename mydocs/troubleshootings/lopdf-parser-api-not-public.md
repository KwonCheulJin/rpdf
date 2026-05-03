# 트러블슈팅 — lopdf 내부 parser API 비공개로 인한 딕셔너리 파싱 위임 불가

**발생일**: 2026-05-03
**해결일**: 2026-05-03
**관련 Issue**: Task #2 (parse_trailer 구현)
**심각도**: 중간 (설계 변경으로 해결, 기능 영향 없음)
**환경**: Rust 1.85 / macOS Darwin 25.4 / lopdf 0.32

## 증상

Task #2 설계 단계에서 "옵션 B — trailer 영역 추출 후 lopdf에 딕셔너리 파싱 위임"을 선택했으나,
실제 구현 시 lopdf의 딕셔너리 파서를 직접 호출할 수 없었다.

구체적으로 시도한 방법들:

- `lopdf::parser::dictionary(...)` → `parser` 모듈이 `pub(crate)` 수준이라 외부 크레이트에서 접근 불가
- `lopdf::Document::load_mem(bytes)` → 전체 PDF 문서 파싱을 시도하므로, 합성 bytes slice 단위 테스트에서 실패

## 재현 방법

```rust
// Cargo.toml에 lopdf = "0.32" 추가 후
use lopdf::parser; // error: module `parser` is private
```

```
error[E0603]: module `parser` is private
 --> src/trailer.rs:2:12
  |
2 | use lopdf::parser;
  |            ^^^^^^ private module
```

## 원인 분석

### 1차 가설
`lopdf::parser` 모듈이 공개 API일 것으로 예상했으나 아니었다.

### 최종 원인
lopdf 0.32의 `parser` 모듈은 `pub(crate)` 가시성으로 선언되어 있어 외부에서 직접 접근 불가.
공개 API는 `Document::load`, `Document::load_mem` 등 전체 문서 단위 파싱만 제공한다.

전체 문서 파싱 API(`load_mem`)는 다음 이유로 부적합:
1. 합성 bytes slice(단위 테스트용 최소 PDF 조각)에서 동작하지 않음
2. 함수 시그니처 `parse_trailer(data: &[u8], search_end: usize)`와 맞지 않음

## 해결책

### 적용한 수정

lopdf 의존을 추가하지 않고, trailer 딕셔너리 파싱에 필요한 최소 파서를 자체 구현.

- `crates/rpdf-parser/src/object_parser.rs` 신규 생성
- 지원 타입: 정수, 간접 참조(`N G R`), 이름(`/Name`), 중첩 딕셔너리(`<< >>`), 배열(`[...]`), 리터럴 문자열(`(...)`), hex 문자열(`<hex>`)
- 약 160줄, 9개 함수로 구성

### 향후 계획

`object_parser.rs`는 Task #4(전체 PDF 객체 파서)에서 확장될 예정:
- 추가 타입: 실수(real number), 스트림(stream), boolean, null
- 공개 API로 승격 여부는 Task #4 설계 시 결정

## 재발 방지

외부 크레이트의 내부 파서 API에 의존하기 전, docs.rs에서 **공개 API 범위를 먼저 확인**한다.
`pub(crate)` 모듈은 문서에 노출되지 않으므로, 문서에서 보이지 않으면 접근 불가로 간주.

## 참고 자료

- lopdf 0.32 소스: `src/parser.rs` 상단 `pub(crate) mod parser` 선언
- ISO 32000-1:2008 §7.3 — PDF 기본 객체 타입 명세
- Task #4 계획서: `mydocs/plans/task4-object-parser.md` (작성 예정)
