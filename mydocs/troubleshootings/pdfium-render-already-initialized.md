# 트러블슈팅 — pdfium-render `PdfiumLibraryBindingsAlreadyInitialized`

**발생일**: 2026-05-04  
**해결일**: 2026-05-04  
**관련 Issue**: #22  
**심각도**: 중간  
**환경**: pdfium-render 0.9.1 / Rust / cargo nextest

## 증상

통합 테스트 여러 개가 한 프로세스에서 실행될 때 두 번째 테스트부터 `bind_to_library`가 실패한다.

- 에러: `PdfiumLibraryBindingsAlreadyInitialized`
- 첫 번째 테스트는 통과, 이후 테스트 전부 실패

## 재현 방법

```rust
// 테스트 2개가 같은 프로세스에서 실행될 때 재현됨
#[test]
fn test_a() {
    let lib = std::env::var("PDFIUM_DYNAMIC_LIB_PATH").unwrap();
    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib)).unwrap();
    // 통과
}

#[test]
fn test_b() {
    let lib = std::env::var("PDFIUM_DYNAMIC_LIB_PATH").unwrap();
    Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&lib)).unwrap();
    // PdfiumLibraryBindingsAlreadyInitialized 에러 발생
}
```

## 원인 분석

### 최종 원인

pdfium-render는 내부적으로 전역 `OnceCell`을 사용해 라이브러리 바인딩을 저장한다.  
`bind_to_library`를 동일 프로세스에서 두 번 이상 호출하면 `OnceCell`이 이미 초기화 상태이므로 `PdfiumLibraryBindingsAlreadyInitialized`를 반환한다.

cargo nextest는 기본적으로 테스트를 같은 프로세스 내 별도 스레드에서 실행하므로, 통합 테스트 파일 내 여러 테스트가 모두 `bind_to_library`를 호출하면 첫 번째 이후 전부 실패한다.

## 해결책

### 적용한 수정

`bind_to_library`를 직접 호출하는 대신 `AlreadyInitialized` 케이스를 처리하는 헬퍼 함수를 사용한다.

```rust
fn load_pdfium(lib_path: &Path) -> Result<Pdfium, RenderError> {
    match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(lib_path)) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => {
            // 이미 초기화된 경우 기존 전역 바인딩을 재사용한다.
            Ok(Pdfium::default())
        }
        Err(e) => Err(RenderError::LibraryLoad(e.to_string())),
    }
}
```

`Pdfium::default()`는 내부적으로 이미 초기화된 `OnceCell`을 사용해 안전하게 `Pdfium` 인스턴스를 반환한다.

## 재발 방지

- pdfium-render를 사용하는 모든 함수에서 `bind_to_library` 직접 호출 금지 — `load_pdfium()` 헬퍼를 통해서만 초기화한다.
- 새 테스트 추가 시 `load_pdfium()` 헬퍼를 사용하면 자동으로 안전하게 처리된다.

## 참고 자료

- pdfium-render 소스: `src/pdfium.rs` — `OnceCell` 전역 바인딩 구현 확인
- 관련 PR: #23
