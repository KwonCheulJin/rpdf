#!/usr/bin/env bash
# samples/large/ PDF 다운로드 스크립트 (macOS/Linux)
# CI에서는 실행하지 않음. 로컬 개발자가 대용량 PDF 검증 시 사용.
#
# 사용법:
#   bash scripts/fetch-samples.sh
#
# 다운로드 후 다음 명령으로 전체 회귀 테스트 실행:
#   cargo nextest run --all --features samples-large
#
# Windows 사용자: 수동으로 아래 URL에서 다운로드 후 samples/large/에 저장하세요.

set -euo pipefail

LARGE_DIR="$(dirname "$0")/../samples/large"
mkdir -p "$LARGE_DIR"

echo "samples/large/ 대용량 PDF 다운로드 시작..."

# L1: Mozilla pdf.js corpus — PDFJS-9279-reduced.pdf (~3MB)
# 라이선스: Apache 2.0 (https://github.com/mozilla/pdf.js/blob/master/LICENSE)
L1_URL="https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/PDFJS-9279-reduced.pdf"
L1_DEST="$LARGE_DIR/large-pdfjs-9279.pdf"
if [ ! -f "$L1_DEST" ]; then
  echo "  다운로드: large-pdfjs-9279.pdf"
  curl -sSL --fail "$L1_URL" -o "$L1_DEST"
  echo "  완료: $(wc -c < "$L1_DEST") bytes"
else
  echo "  스킵 (이미 있음): large-pdfjs-9279.pdf"
fi

# L2: Mozilla pdf.js corpus — issue12841_reduced.pdf (~5.7MB)
# 라이선스: Apache 2.0
L2_URL="https://raw.githubusercontent.com/mozilla/pdf.js/master/test/pdfs/issue12841_reduced.pdf"
L2_DEST="$LARGE_DIR/large-pdfjs-issue12841.pdf"
if [ ! -f "$L2_DEST" ]; then
  echo "  다운로드: large-pdfjs-issue12841.pdf"
  curl -sSL --fail "$L2_URL" -o "$L2_DEST"
  echo "  완료: $(wc -c < "$L2_DEST") bytes"
else
  echo "  스킵 (이미 있음): large-pdfjs-issue12841.pdf"
fi

echo ""
echo "완료! 대용량 PDF 다운로드 성공."
echo "  $LARGE_DIR/"
ls -lh "$LARGE_DIR/" 2>/dev/null | grep -v '^total'
echo ""
echo "전체 회귀 테스트 실행:"
echo "  cargo nextest run --all --features samples-large"
