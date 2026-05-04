# Task #11: pdfium 환경 구축 계획서

**Issue**: #20  
**브랜치**: local/task11  
**마일스톤**: v0.2  
**날짜**: 2026-05-04

---

## 목표

`crates/rpdf-render` crate를 신규 생성하고 pdfium-render 0.9.2를 의존성으로 추가한다.
`Pdfium::bind_to_library()` 런타임 검증 테스트 통과 + CI (ubuntu-latest) 자동화까지 완료한다.

**PNG 출력 코드는 Task #12. 이번 Task에서 구현하지 않는다.**

---

## 완료 기준 (7가지)

1. `crates/rpdf-render/` 신규 crate 생성, 워크스페이스 등록
2. `scripts/fetch-pdfium.sh` 작성 (macOS arm64/x64 + Linux x64)
3. `cargo build --workspace` 성공
4. `rpdf-render`에 `Pdfium::bind_to_library()` 최소 테스트 작성 + 통과
5. CI (`.github/workflows/ci.yml`) pdfium 자동 설치 + `cargo nextest run -p rpdf-render` 통과
6. `LD_LIBRARY_PATH` + `PDFIUM_DYNAMIC_LIB_PATH` CI에 설정됨
7. `mydocs/tech/dev-tool-pdfium-render.md` 작성
8. `.gitignore`에 `pdfium/` 추가

---

## 구현 단계

### 1단계: crate 생성 및 워크스페이스 등록

```bash
cargo new --lib crates/rpdf-render --vcs none
```

`Cargo.toml` 워크스페이스 members에 `"crates/rpdf-render"` 추가.

`crates/rpdf-render/Cargo.toml` 의존성:
```toml
[dependencies]
pdfium-render = "0.9.2"
image = { version = "0.25", default-features = false, features = ["png"] }
rpdf-core = { path = "../rpdf-core" }
thiserror.workspace = true
```

`image`는 Task #12에서 사용. 지금은 의존성만 추가.

### 2단계: fetch-pdfium.sh 작성

`scripts/fetch-pdfium.sh`:
- PDFIUM_BUILD 변수로 빌드번호 고정 (기본값 6721)
- Darwin-arm64 / Darwin-x86_64 / Linux-x86_64 지원
- macOS Gatekeeper 격리 자동 해제
- CI 환경에서 `$GITHUB_ENV` 자동 기록

### 3단계: lib.rs 최소 스텁 + 테스트

```rust
// lib.rs 최소 공개 API
pub use pdfium_render::prelude::Pdfium;

#[cfg(test)]
mod tests {
    use pdfium_render::prelude::*;

    #[test]
    fn pdfium_dynamic_links() {
        let lib_path = std::env::var("PDFIUM_DYNAMIC_LIB_PATH")
            .expect("PDFIUM_DYNAMIC_LIB_PATH not set");
        Pdfium::bind_to_library(
            Pdfium::pdfium_platform_library_name_at_path(&lib_path)
        )
        .expect("pdfium dynamic link failed");
    }
}
```

### 4단계: CI yaml 수정

기존 `ci.yml`의 `rust` job에 추가:
- `PDFIUM_BUILD` env 선언
- pdfium cache step (actions/cache@v4)
- fetch-pdfium.sh 실행 step
- Set pdfium env step (cache hit 여부 무관하게 항상 실행)
- `cargo nextest run -p rpdf-render` step

### 5단계: .gitignore + 문서화

- `.gitignore`에 `pdfium/` 추가
- `mydocs/tech/dev-tool-pdfium-render.md` 작성

---

## 위험 요소

| 위험 | 확률 | 대응 |
|------|------|------|
| 빌드번호 6721 ↔ pdfium-render 0.9.2 미스매치 | 중 | 실제 다운로드 후 테스트 실행으로 즉시 확인 |
| macOS Gatekeeper 격리 | 높 | xattr 자동 처리 |
| CI LD_LIBRARY_PATH 미설정 | 중 | fetch-pdfium.sh + yaml 양쪽에 설정 |
| cache hit 시 env 미설정 | 중 | Set pdfium env를 cache 조건 없이 항상 실행 |

---

## 트러블슈팅 예상 항목

1. pdfium-render 빌드번호 호환성 문제 → `releases` 페이지에서 최신 호환 번호 확인
2. macOS: `cannot be opened` 에러 → `xattr -d com.apple.quarantine`
3. Linux: `libpdfium.so not found` → `LD_LIBRARY_PATH` 확인
4. `PDFIUM_DYNAMIC_LIB_PATH` 경로 형식 (디렉터리 vs 파일) → pdfium-render는 디렉터리 경로를 받음

---

## 버전 정보

- pdfium-render: 0.9.2
- image: 0.25.x (Task #12에서 실제 사용)
- pdfium 빌드번호: 6721 (실제 확인 후 조정 가능)

---

## 아키텍처 원칙

- `rpdf-render` → `rpdf-core` 의존 (단방향)
- `rpdf-core`는 `rpdf-render`를 참조하면 안 됨
- `rpdf-render`는 렌더링 레이어만 담당
- `rpdf-core`에 feature flag로 넣지 않음
