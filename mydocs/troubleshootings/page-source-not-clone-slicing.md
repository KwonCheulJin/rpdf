# 트러블슈팅 — PageSource: !Clone 로 인한 슬라이싱 불가

**발생일**: 2026-05-19  
**해결일**: 2026-05-19  
**관련 Issue**: #46  
**심각도**: 중간  
**환경**: rpdf-serializer PageSource 타입

## 증상

계획서에서 extract/split 핸들러에 `sources[(start-1)..=(end-1)].to_vec()` 패턴을 명시했으나,
실제 구현 시 컴파일 에러 발생.

```
error[E0277]: the trait bound `PageSource: Clone` is not satisfied
  --> crates/rpdf-cli/src/commands/edit/extract.rs:25:59
   |
   | let sub_sources = sources[(start-1)..=(end-1)].to_vec();
   |                                                ^^^^^^^^
   | the trait `Clone` is not implemented for `PageSource`
```

## 재현 방법

```rust
use rpdf_serializer::{load_document_tracked, PageSource};

let (doc, sources) = load_document_tracked(&data)?;
// 이 패턴은 PageSource: Clone이 없어 컴파일 실패
let sub: Vec<PageSource> = sources[0..3].to_vec();
```

## 원인 분석

`PageSource`는 `bytes: Arc<Vec<u8>>`와 `page_index: usize` 두 필드만 가지며,
`#[derive(Clone)]`이 없다. `Arc<T>: Clone`이지만 구조체 자체가 `Clone`을 구현하지 않는다.

**관련 코드 위치**: `crates/rpdf-serializer/src/types.rs:14`

**왜 이 문제가 발생했는가**:
계획서 작성 시 `PageSource`가 Clone을 구현한다고 가정하고 `.to_vec()` 패턴을 명시했으나,
실제 타입 정의를 확인하지 않았음.

## 해결책

### 적용한 수정

`PageSource { bytes, page_index }`를 수동으로 재구성하는 `into_iter().enumerate().filter()` 패턴 사용.

**extract.rs**:
```rust
// 변경 전 (컴파일 실패)
let sub_sources = sources[(start - 1)..=(end - 1)].to_vec();

// 변경 후
let sub_sources: Vec<PageSource> = sources
    .into_iter()
    .enumerate()
    .filter(|(i, _)| *i >= start - 1 && *i <= end - 1)
    .map(|(_, s)| s)
    .collect();
```

**split.rs** (PageSource 수동 재구성):
```rust
let sub_sources: Vec<PageSource> = sources
    .iter()
    .enumerate()
    .filter(|(i, _)| *i >= range_start && *i <= range_end)
    .map(|(_, s)| PageSource {
        bytes: Arc::clone(&s.bytes),
        page_index: s.page_index,
    })
    .collect();
```

split은 여러 sub_docs를 순회해야 하므로 `sources`를 소비하지 않고 `iter()`를 사용하고
`Arc::clone`으로 bytes를 공유한다.

## 재발 방지

- 계획서에서 외부 크레이트 타입의 `Clone` 구현 여부를 가정하지 않는다.
- 계획서 작성 시 `.to_vec()` 패턴을 명시하기 전에 해당 타입이 `Clone`을 구현하는지 확인한다.
- `PageSource`에 `Clone`이 필요하면 `rpdf-serializer` 타입 정의에 `#[derive(Clone)]` 추가를 검토한다
  (Arc 공유 의미론 유지됨).

## 배운 점

- `Arc<T>: Clone`이어도 구조체에 `#[derive(Clone)]`이 없으면 구조체 자체는 `!Clone`이다.
- 슬라이스를 여러 번 참조해야 하는 경우(split)와 한 번만 소비하는 경우(extract)에
  `iter()` vs `into_iter()` 선택이 달라진다.

## 참고 자료

- `crates/rpdf-serializer/src/types.rs` — PageSource 타입 정의
- `crates/rpdf-cli/src/commands/edit/extract.rs` — 해결된 구현
- `crates/rpdf-cli/src/commands/edit/split.rs` — Arc::clone 재구성 패턴
