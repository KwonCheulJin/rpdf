#!/usr/bin/env bash
# fetch-pdfium.sh — pdfium 동적 라이브러리 다운로드
#
# 사용법:
#   bash scripts/fetch-pdfium.sh
#
# 환경 변수:
#   PDFIUM_BUILD  — 다운로드할 Chromium/pdfium 빌드번호 (기본값: 7763)
#                   pdfium-render 0.9.1 호환 빌드번호: 7763
#   PDFIUM_DIR    — 설치 대상 디렉터리 (기본값: pdfium)
#
# 라이선스:
#   PDFium 바이너리: BSD 3-Clause + Apache 2.0
#   참조: https://pdfium.googlesource.com/pdfium/+/main/LICENSE

set -euo pipefail

PDFIUM_BUILD="${PDFIUM_BUILD:-7763}"
PDFIUM_DIR="${PDFIUM_DIR:-pdfium}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  PLATFORM="mac-arm64" ;;
  Darwin-x86_64) PLATFORM="mac-x64" ;;
  Linux-x86_64)  PLATFORM="linux-x64" ;;
  *)
    echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${PDFIUM_BUILD}/pdfium-${PLATFORM}.tgz"

echo "Fetching pdfium build ${PDFIUM_BUILD} for ${PLATFORM}..."
mkdir -p "${PDFIUM_DIR}"
curl -fsSL "${URL}" | tar xz -C "${PDFIUM_DIR}"

# macOS Gatekeeper 격리 해제 (다운로드 파일은 자동으로 quarantine 속성이 붙음)
if [[ "$(uname -s)" == "Darwin" ]]; then
  find "${PDFIUM_DIR}" -name "*.dylib" -exec xattr -d com.apple.quarantine {} + 2>/dev/null || true
fi

LIB_PATH="$(pwd)/${PDFIUM_DIR}/lib"
echo "pdfium installed to: ${LIB_PATH}"
echo "PDFIUM_DYNAMIC_LIB_PATH=${LIB_PATH}"

# CI 환경(GitHub Actions)에서는 GITHUB_ENV에 직접 기록
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "PDFIUM_DYNAMIC_LIB_PATH=${LIB_PATH}" >> "${GITHUB_ENV}"
  echo "LD_LIBRARY_PATH=${LIB_PATH}:${LD_LIBRARY_PATH:-}" >> "${GITHUB_ENV}"
fi
