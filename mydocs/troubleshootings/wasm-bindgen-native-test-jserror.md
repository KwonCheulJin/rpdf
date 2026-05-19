# 트러블슈팅 — wasm-bindgen JsError를 포함한 메서드의 네이티브 cargo test 실패

**발생일**: 2026-05-19  
**해결일**: 2026-05-19  
**관련 Issue**: #48 (Task #25)  
**심각도**: 중간  
**환경**: Rust 1.87.0 / wasm-bindgen 0.2 / wasm32-unknown-unknown

---

## 증상

`#[wasm_bindgen]` impl 블록 안에서 `js_sys::Error::new(...)` 또는 `JsError::new(...)`를 에러 변환에 사용하는 메서드를, 네이티브 타겟(`x86_64-apple-darwin` 등)에서 `cargo test -p rpdf-wasm`로 호출하면 컴파일은 통과하지만 런타임에 패닉 발생.

```
# cargo test -p rpdf-wasm
test ut_07_delete_pages_empty ... FAILED

thread 'ut_07_delete_pages_empty' panicked at 'not implemented',
wasm-bindgen/src/lib.rs:XXX:XX
```

또는 메서드 자체를 인라인으로 네이티브 테스트에서 호출할 수 없어 컴파일 에러 발생:
```
error[E0433]: failed to resolve: use of undeclared crate or module `js_sys`
```

---

## 재현 방법

```rust
#[wasm_bindgen]
pub fn delete_pages(&mut self, indices: Vec<u32>) -> Result<(), JsValue> {
    if indices.is_empty() {
        return Err(JsValue::from(JsError::new("삭제할 페이지 목록이 비어있습니다")));
    }
    // ...
}

// 테스트에서 직접 호출 시도
#[cfg(test)]
mod tests {
    #[test]
    fn test_delete_empty() {
        let mut pdf = make_pdf_document();
        let result = pdf.delete_pages(vec![]);  // 네이티브에서 호출 → 런타임 패닉
        assert!(result.is_err());
    }
}
```

```bash
cargo test -p rpdf-wasm
# → 패닉 또는 컴파일 에러
```

---

## 원인 분석

### 최종 원인

`js_sys::Error`·`JsError` 등 wasm-bindgen JS 런타임 타입은 **wasm32 타겟에서만 유효**한 타입이다. 네이티브 타겟에서 컴파일은 조건부로 가능하지만, 런타임에 실제 JS 엔진이 없으므로 패닉을 일으킨다.

`wasm-bindgen`의 많은 JS 타입이 네이티브에서 `unimplemented!()` 매크로로 구현되어 있어 `cargo test` 시 컴파일은 되지만 호출 즉시 패닉.

**관련 코드 위치**: `crates/rpdf-wasm/src/lib.rs` — `#[wasm_bindgen]` impl 블록 내 에러 반환 경로

---

## 해결책

### 원칙: 검증 로직을 JsValue-free 내부 헬퍼로 분리

`#[wasm_bindgen]` 메서드 안에 비즈니스 로직을 직접 작성하지 않는다. 검증·계산 로직은 `JsValue`를 반환하지 않는 내부 헬퍼 함수로 추출하고, wasm 레이어는 변환만 담당한다.

```rust
// 내부 헬퍼 (JsValue 없음 → 네이티브 cargo test 가능)
fn validate_delete_indices(indices: &[usize], page_count: usize) -> Result<(), String> {
    if indices.is_empty() {
        return Err("삭제할 페이지 목록이 비어있습니다".to_string());
    }
    for &i in indices {
        if i >= page_count {
            return Err(format!("페이지 인덱스 범위 초과: {}", i));
        }
    }
    Ok(())
}

fn validate_page_index(idx: usize, count: usize) -> Result<(), String> {
    if idx >= count {
        Err(format!("페이지 인덱스 범위 초과: {}", idx))
    } else {
        Ok(())
    }
}

fn validate_degrees(degrees: i32) -> Result<(), String> {
    match degrees {
        0 | 90 | 180 | 270 => Ok(()),
        _ => Err(format!("유효하지 않은 회전각: {} (0/90/180/270만 허용)", degrees)),
    }
}

// wasm 레이어 (보일러플레이트만 — 내부 헬퍼 호출 + JsValue 변환)
#[wasm_bindgen]
impl PdfDocument {
    pub fn delete_pages(&mut self, indices: Vec<u32>) -> Result<(), JsValue> {
        let idx_usize: Vec<usize> = indices.iter().map(|&i| i as usize).collect();
        validate_delete_indices(&idx_usize, self.doc.page_count())
            .map_err(|e| JsValue::from(JsError::new(&e)))?;
        // ...
    }
}
```

### 테스트 패턴

네이티브 단위 테스트에서는 **내부 헬퍼만 직접 테스트**한다.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_validate_delete_empty() {
        let result = validate_delete_indices(&[], 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("비어있습니다"));
    }

    #[test]
    fn ut_validate_page_index_out_of_bounds() {
        assert!(validate_page_index(10, 5).is_err());
        assert!(validate_page_index(4, 5).is_ok());
    }
}
```

---

## 재발 방지

- wasm 크레이트의 `#[wasm_bindgen]` impl 블록에 로직을 직접 작성하지 않는다.
- `cargo test -p <wasm-crate>`가 CI에 포함되어 있으면 이 패턴이 강제된다.
- 계획서 작성 시 "내부 헬퍼 분리 전략" 섹션을 명시해야 한다 (Task #25 계획서 참조).

---

## 배운 점

- wasm-bindgen의 JS 타입은 네이티브 단위 테스트에서 "컴파일은 되지만 런타임 패닉"이므로 조심. 링커 에러가 아니라 런타임 패닉이어서 발견이 늦을 수 있다.
- `#[wasm_bindgen]` 레이어를 얇게 유지하면 WASM 환경 없이도 핵심 로직을 완전히 테스트할 수 있다.
- `JsValue-free` 헬퍼 패턴은 향후 CLI·WASM 공통 로직에도 재사용 가능하다.

---

## 참고 자료

- wasm-bindgen 공식 문서: https://rustwasm.github.io/docs/wasm-bindgen/
- 관련 PR: local/task25
- 계획서: `mydocs/plans/task25-wasm-api.md`
