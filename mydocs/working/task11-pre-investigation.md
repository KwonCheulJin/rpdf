# Task #11 사전 조사: pdfium 환경 구축

**날짜**: 2026-05-04  
**Task**: #11 — pdfium 환경 구축 (의존성 + 동적 라이브러리 + CI)  
**목표**: `Pdfium::bind_to_library()` 최소 테스트 통과 + CI(ubuntu-latest) 통과까지만

> ⚠️ plan-eng-review 지적: `cargo build`는 PDFium 바이너리 없이도 성공 (런타임 동적 로딩).
> 완료 기준을 런타임 검증 테스트로 강화.

---

## 1. pdfium-render crate 확인 완료

| 항목 | 확인 결과 |
|------|-----------|
| 버전 | 0.9.2 |
| 라이선스 | MIT OR Apache-2.0 |
| 주요 API | `Pdfium::bind_to_library(path)`, `pdfium.load_pdf_from_file(path, pw)?` → `PdfDocument` |
| PNG 경로 | `page.render_with_config(&cfg)?.as_image()` → `DynamicImage` → `image` 크레이트로 저장 |
| 동기/비동기 | 완전 동기. 비동기 필요 시 `spawn_blocking` 래핑 |
| WASM | 부분 지원 (메모리 제한). v0.4에서 pdf.js 대체 결정 유지 |

---

## 2. 플랫폼별 동적 라이브러리 설치 절차

### 공통: 바이너리 배포처
`https://github.com/bblanchon/pdfium-binaries/releases`
- 빌드 번호 = Chromium 빌드 번호 (예: `6721`)
- 아키텍처별 파일명 패턴: `pdfium-{platform}.tgz`

### macOS (arm64, 개발 환경 = 현재 환경)
```bash
curl -L https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F{BUILD}/pdfium-mac-arm64.tgz \
  | tar xz -C pdfium/

# Gatekeeper 해제 필수 (다운로드 파일 자동 격리됨)
xattr -d com.apple.quarantine pdfium/lib/libpdfium.dylib

export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib
```

### macOS (x64, Intel)
```bash
# 파일명만 다름: pdfium-mac-x64.tgz
```

### Linux (CI: ubuntu-latest, x64)
```bash
curl -L https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F{BUILD}/pdfium-linux-x64.tgz \
  | tar xz -C pdfium/

export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib
# Linux: LD_LIBRARY_PATH 추가 필요할 수 있음
export LD_LIBRARY_PATH=$(pwd)/pdfium/lib:$LD_LIBRARY_PATH
```

---

## 3. scripts/fetch-pdfium.sh 설계

```bash
#!/usr/bin/env bash
set -euo pipefail

PDFIUM_BUILD="${PDFIUM_BUILD:-6721}"   # 핀: Cargo.lock처럼 명시적 버전 고정
PDFIUM_DIR="${PDFIUM_DIR:-pdfium}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) PLATFORM="mac-arm64" ;;
  Darwin-x86_64) PLATFORM="mac-x64" ;;
  Linux-x86_64) PLATFORM="linux-x64" ;;
  *) echo "Unsupported platform"; exit 1 ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${PDFIUM_BUILD}/pdfium-${PLATFORM}.tgz"

mkdir -p "$PDFIUM_DIR"
curl -fsSL "$URL" | tar xz -C "$PDFIUM_DIR"

# macOS quarantine 해제
if [[ "$(uname -s)" == "Darwin" ]]; then
  find "$PDFIUM_DIR" -name "*.dylib" -exec xattr -d com.apple.quarantine {} + 2>/dev/null || true
fi

LIB_PATH="$(pwd)/$PDFIUM_DIR/lib"
echo "PDFIUM_DYNAMIC_LIB_PATH=$LIB_PATH"
# CI 환경에서는 GITHUB_ENV에 직접 기록
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "PDFIUM_DYNAMIC_LIB_PATH=$LIB_PATH" >> "$GITHUB_ENV"
  echo "LD_LIBRARY_PATH=$LIB_PATH:${LD_LIBRARY_PATH:-}" >> "$GITHUB_ENV"
fi
```

---

## 4. CI 자동화 설계 (GitHub Actions)

`taiki-e/install-action`은 pdfium 미지원 → 커스텀 스텝 필요.

```yaml
env:
  PDFIUM_BUILD: "6721"   # 상단에 고정, 업그레이드 시 한 곳만 변경

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache pdfium
        id: cache-pdfium
        uses: actions/cache@v4
        with:
          path: pdfium/
          key: pdfium-${{ env.PDFIUM_BUILD }}-${{ runner.os }}-${{ runner.arch }}

      - name: Fetch pdfium
        if: steps.cache-pdfium.outputs.cache-hit != 'true'
        run: bash scripts/fetch-pdfium.sh

            - name: Set pdfium env
        run: |
          echo "PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib" >> $GITHUB_ENV
          echo "LD_LIBRARY_PATH=$(pwd)/pdfium/lib:$LD_LIBRARY_PATH" >> $GITHUB_ENV

      - name: cargo build
        run: cargo build --workspace

      - name: cargo nextest (rpdf-render 링킹 검증)
        run: cargo nextest run -p rpdf-render
        env:
          INSTA_UPDATE: "no"
```

---

## 5. Cargo.toml 설계

### rpdf-render crate (신규)
```toml
[package]
name = "rpdf-render"
version.workspace = true
edition.workspace = true

[dependencies]
pdfium-render = "0.9.2"
image = { version = "0.25.10", default-features = false, features = ["png"] }
rpdf-core = { path = "../rpdf-core" }
thiserror.workspace = true
```

`image`는 `default-features = false, features = ["png"]`으로 PNG만 활성화.  
불필요한 포맷(gif, jpeg, bmp 등) 컴파일 제외 → 빌드 시간 단축.

---

## 6. 라이선스 표시

| 구성요소 | 라이선스 | 표시 필요 |
|----------|----------|-----------|
| `pdfium-render` crate | MIT OR Apache-2.0 | Cargo.lock 자동 기록, 별도 표시 불필요 |
| PDFium 바이너리 (Google/Chromium) | BSD 3-Clause + Apache 2.0 포함 | 배포 시 `LICENSES/pdfium-notice.txt` 필요 |
| `image` crate | MIT OR Apache-2.0 | 자동 기록 |

> Task #16 (통합 테스트 + 문서화)에서 `LICENSES/` 디렉터리 생성.  
> Task #11에서는 `scripts/fetch-pdfium.sh`에 라이선스 URL 주석으로만 기록.

---

## 7. 예상 트러블슈팅 항목

| # | 함정 | 증상 | 해결 |
|---|------|------|------|
| 1 | macOS Gatekeeper 격리 | `cannot be opened because the developer cannot be verified` | `xattr -d com.apple.quarantine libpdfium.dylib` |
| 2 | pdfium-render 버전 ↔ 바이너리 빌드번호 미스매치 | 런타임 심볼 찾기 실패 | README의 호환 빌드번호 표 참조 후 핀 조정 |
| 3 | CI `LD_LIBRARY_PATH` 미설정 | Linux 런타임 `libpdfium.so not found` | `LD_LIBRARY_PATH` env 추가 |
| 4 | GitHub Actions cache miss 폭주 | 매 run 마다 재다운로드 | cache key에 PDFIUM_BUILD 포함 (이미 설계에 반영) |
| 5 | `PDFIUM_DYNAMIC_LIB_PATH`가 디렉터리가 아닌 파일 경로 | 링크 에러 | 환경변수가 `.dylib` 경로가 아닌 **디렉터리** 경로인지 확인 |

---

## 8. Task #11 완료 기준 (강화 — plan-eng-review 반영)

- `cargo build --workspace` 성공
- `rpdf-render`에 `Pdfium::bind_to_library()` 최소 테스트 작성 + 통과
- CI (ubuntu-latest): pdfium 자동 설치 + `cargo nextest run -p rpdf-render` 통과
- `LD_LIBRARY_PATH` + `PDFIUM_DYNAMIC_LIB_PATH` 양쪽 CI에 설정됨
- `scripts/fetch-pdfium.sh` macOS/Linux 동작 (macOS는 로컬 수동 검증)
- 개발 환경 설정 방법 `mydocs/tech/dev-tool-pdfium-render.md` 작성
- pdfium 빌드번호 6721 ↔ pdfium-render 0.9.2 호환성 실제 확인 완료

**PNG 출력 코드는 Task #12에서 작성.**

---

## 9. 결정 완료 (plan-eng-review 자동 채택)

1. **rpdf-render**: 별도 crate (`crates/rpdf-render/`) — `rpdf-core` 원칙("도메인 타입만, UI 비의존") 유지
2. **빌드번호 핀**: 스크립트 상단 변수 + CI env 동일 값. 별도 파일 불필요.
3. **`.gitignore`**: `pdfium/` 추가 필요.
4. **scripts/fetch-pdfium.sh**: `$GITHUB_ENV` 자동 쓰기 추가 (CI yaml 단순화).
5. **완료 기준**: `cargo build` → `cargo nextest run -p rpdf-render` (런타임 링킹 검증).
