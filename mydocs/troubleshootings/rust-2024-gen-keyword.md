# Rust 2024 Edition — `gen` 예약어 충돌

## 증상

`cargo build` 실행 시 다음과 같은 컴파일 오류 발생:

```
error: expected identifier, found keyword `gen`
 --> crates/rpdf-core/src/types.rs:42:12
  |
42 |     pub gen: u16,
   |         ^^^ reserved keyword
```

## 원인

Rust 2024 edition(Edition 2024)에서 `gen`이 예약어(reserved keyword)로 추가됐다.
`Cargo.toml`에 `edition = "2024"`가 설정된 크레이트에서 `gen`을 식별자(변수명, 필드명, 함수명)로 사용하면 컴파일 오류가 발생한다.

## 영향 범위

Task #8에서는 PDF 객체 ID의 generation 번호를 나타내는 필드/변수에 `gen`이라는 이름을 사용하려 했으나, Rust 2024 예약어 충돌로 인해 `generation`으로 변경했다.

영향 크레이트:
- `rpdf-core` — `ObjectId` 등 obj generation 관련 타입
- `rpdf-parser` — obj generation 관련 변수

## 해결

`gen` 식별자를 `generation` 또는 맥락에 맞는 다른 이름으로 변경한다.

```rust
// Before (Rust 2024에서 컴파일 오류)
pub struct ObjectId {
    pub number: u32,
    pub gen: u16,
}

// After
pub struct ObjectId {
    pub number: u32,
    pub generation: u16,
}
```

## 재발 방지

1. 새 식별자 사용 시 Rust 2024 예약어 목록을 확인한다.
2. 현재 확정 예약어: `gen`, `async`, `await`, `dyn`, `try`
3. 미래 예약어(단계적 도입 예정): `become`, `do`, `priv`, `yeet` 등
4. `edition = "2024"` 크레이트에서 위 이름을 식별자로 사용하면 즉시 컴파일 실패한다.

## 참고

- [Rust Reference — Keywords](https://doc.rust-lang.org/reference/keywords.html)
- [Rust 2024 Migration Guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
