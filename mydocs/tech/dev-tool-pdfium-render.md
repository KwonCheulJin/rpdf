# dev-tool-pdfium-render.md

**도입 Task**: #11  
**버전**: pdfium-render 0.9.1  
**pdfium 빌드**: 7763 (Chromium 148.0.7763.0)

---

## 도구 개요

pdfium-render는 Google Chromium 프로젝트가 사용하는 C++ PDF 라이브러리 PDFium의 Rust 바인딩이다.
PDF 페이지를 비트맵으로 렌더링하는 핵심 기능을 제공한다.

| 항목 | 내용 |
|------|------|
| crates.io | https://crates.io/crates/pdfium-render |
| 라이선스 | MIT OR Apache-2.0 |
| 링킹 방식 | 런타임 동적 링킹 (`libloading` 사용) |
| pdfium 바이너리 | 별도 다운로드 필요 (아래 참조) |

---

## 로컬 개발 환경 설정

### 1. pdfium 동적 라이브러리 설치

```bash
# pdfium 바이너리 다운로드 (프로젝트 루트에서 실행)
bash scripts/fetch-pdfium.sh

# 설치 경로 확인
ls pdfium/lib/
# macOS: libpdfium.dylib
# Linux: libpdfium.so
```

스크립트는 현재 플랫폼을 자동 감지한다 (macOS arm64/x64, Linux x64 지원).

### 2. 환경변수 설정

```bash
# 매 터미널 세션에서 설정 필요
export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib

# Linux 추가 설정
export LD_LIBRARY_PATH=$(pwd)/pdfium/lib:$LD_LIBRARY_PATH
```

`~/.zshrc` 또는 `~/.bashrc`에 추가하거나, `direnv`를 사용해 프로젝트 루트 `.envrc`에 설정한다.

### 3. 검증

```bash
PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib cargo nextest run -p rpdf-render
```

`pdfium_dynamic_links` 테스트가 통과하면 정상.

---

## 빌드번호 관리

| pdfium-render 버전 | 호환 pdfium 빌드번호 |
|-------------------|---------------------|
| 0.9.1 | **7763** (pdfium_latest feature) |
| 0.8.37 | 7543 |

빌드번호는 `scripts/fetch-pdfium.sh`의 `PDFIUM_BUILD` 기본값과 `.github/workflows/ci.yml`의 `PDFIUM_BUILD` env에서 관리한다.
업그레이드 시 두 곳 모두 동일한 값으로 수정한다.

---

## 빌드번호 업데이트 절차

1. pdfium-render 신규 릴리즈 README에서 `pdfium_latest` feature에 대응하는 빌드번호 확인
2. `scripts/fetch-pdfium.sh` `PDFIUM_BUILD` 기본값 수정
3. `.github/workflows/ci.yml` `PDFIUM_BUILD` env 수정
4. 로컬에서 재설치 후 테스트 통과 확인

---

## CI 설정

`.github/workflows/ci.yml`의 rust job에 다음이 포함됨:

```yaml
env:
  PDFIUM_BUILD: "7763"

steps:
  - name: Cache pdfium
    uses: actions/cache@v4
    with:
      path: pdfium/
      key: pdfium-${{ env.PDFIUM_BUILD }}-${{ runner.os }}-${{ runner.arch }}

  - name: Fetch pdfium
    if: steps.cache-pdfium.outputs.cache-hit != 'true'
    run: bash scripts/fetch-pdfium.sh

  - name: Set pdfium env
    run: |
      echo "PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib" >> "$GITHUB_ENV"
      echo "LD_LIBRARY_PATH=$(pwd)/pdfium/lib:${LD_LIBRARY_PATH:-}" >> "$GITHUB_ENV"
```

`Set pdfium env` 스텝은 cache hit 여부와 무관하게 항상 실행된다.
cache hit 시에도 `PDFIUM_DYNAMIC_LIB_PATH`와 `LD_LIBRARY_PATH`가 설정되어야 테스트가 통과한다.

---

## 라이선스

| 구성요소 | 라이선스 |
|---------|---------|
| pdfium-render crate | MIT OR Apache-2.0 |
| PDFium 바이너리 | BSD 3-Clause + Apache 2.0 |

PDFium 바이너리를 애플리케이션과 함께 배포할 경우 `pdfium/LICENSE` 및 `pdfium/licenses/` 를 포함해야 한다.
Task #16 (통합 테스트 + 문서화)에서 `LICENSES/pdfium-notice.txt` 생성 예정.

---

## 도입 근거

- PDFium은 Chromium 프로젝트에서 수년간 실전 검증된 PDF 렌더링 엔진
- pdfium-render는 Rust에서 PDFium을 가장 관용적(idiomatic)으로 사용할 수 있는 바인딩
- 런타임 동적 링킹으로 `cargo build`가 바이너리 없이도 성공 (CI 유연성)
- MIT OR Apache-2.0 라이선스로 상업적 사용 가능
- stars 600+, 최근 6개월 내 유지보수 활성화 확인 (2026-05-04 기준)
