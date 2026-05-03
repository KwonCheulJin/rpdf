# serde Serialize derive — PdfDict 필드 직렬화 실패

## 증상

`Page` 구조체에 `#[derive(Serialize)]`를 추가했을 때 컴파일 오류 발생:

```
error[E0277]: the trait bound `PdfObject: Serialize` is not satisfied
 --> crates/rpdf-core/src/types.rs
  |
  |     pub resources: Option<PdfDict>,
  |         ^^^^^^^^^ the trait `Serialize` is not implemented for `PdfObject`
```

## 원인

`PdfDict`는 `IndexMap<Vec<u8>, PdfObject>` 기반이다.
`PdfObject` 열거형은 `serde::Serialize`를 구현하지 않는다.
추가로 `Vec<u8>` 타입 키는 serde의 기본 직렬화(JSON 등)에서 처리 방식 결정이 필요하다.

`Page::resources` 필드가 `Option<PdfDict>` 타입이므로, `Page`에 `#[derive(Serialize)]`를 추가하면 위 문제가 연쇄적으로 발생한다.

## 영향 범위

`rpdf-core`의 `Page` 구조체에서 `resources` 필드만 해당한다.
CLI 덤프(`rpdf dump -p <page>`) 기능에서 resources는 `#[serde(skip)]`으로 인해 출력에서 제외된다.

## 해결

해당 필드에 `#[serde(skip)]` 어트리뷰트를 추가하여 직렬화 대상에서 제외한다.

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub page_number: usize,
    pub media_box: Option<[f64; 4]>,
    pub crop_box: Option<[f64; 4]>,
    pub rotate: i32,
    #[serde(skip)]           // PdfObject가 Serialize 미구현
    pub resources: Option<PdfDict>,
    pub content_stream: Vec<u8>,
}
```

## 재발 방지

1. `#[derive(Serialize)]`를 추가하기 전에 모든 필드 타입의 `Serialize` 구현 여부를 확인한다.
2. `IndexMap<Vec<u8>, ...>` 같은 비표준 키 타입은 직렬화 정책을 사전에 결정한다.
3. `PdfObject`에 `Serialize` 구현을 추가하려면 다음을 결정해야 한다:
   - `Stream` 변형의 바이너리 데이터를 Base64로 인코딩할지 제외할지
   - `Vec<u8>` 이름(key)을 UTF-8 문자열로 변환할지 hex로 표현할지
4. 향후 Task #9 이후 `PdfObject::Serialize` 구현을 검토한다.

## 참고

- [serde — Field Attributes](https://serde.rs/field-attrs.html)
- [serde — skip](https://serde.rs/field-attrs.html#skip)
