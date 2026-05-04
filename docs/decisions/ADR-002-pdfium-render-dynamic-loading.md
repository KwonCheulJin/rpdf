# ADR-002: PDF 렌더링 — pdfium-render + 동적 라이브러리 전략

**날짜**: 2026-05-04  
**상태**: 승인됨  
**결정자**: KwonCheulJin

## 맥락

v0.2 렌더링 구현에서 PDF→PNG 변환 백엔드 선택이 필요하다.  
후보: `pdfium-render` (Chromium PDFium 래퍼) vs `pdf-rs` (순수 Rust) vs `mupdf-rs`.

## 결정

`pdfium-render 0.9.1` + PDFium 바이너리 `빌드번호 7763`을 사용한다.  
라이브러리는 정적 링크하지 않고 `Pdfium::bind_to_library()` 런타임 동적 로딩 방식을 채택한다.

## 근거

- `pdf-rs`는 복잡한 PDF 렌더링(폰트, 이미지 합성)이 미흡 — 시각적 정확도 부족
- `mupdf-rs` 라이선스 AGPL → 상업 배포 제약
- PDFium은 Chromium 품질 렌더링 + 적극 유지보수
- 동적 로딩: `cargo build`는 pdfium 바이너리 없이도 성공 → CI 캐시 전략 분리 가능

## 결과

- `scripts/fetch-pdfium.sh`로 플랫폼별 바이너리 자동 다운로드
- CI: `actions/cache@v4`로 pdfium 캐시, cache miss 시만 재다운로드
- 개발 환경: `PDFIUM_DYNAMIC_LIB_PATH` 환경변수 설정 필요
- macOS: Gatekeeper quarantine 해제 필수 (`xattr -d com.apple.quarantine`)

## 주의

- v0.4 WASM: 브라우저 렌더링은 pdf.js에 위임 (pdfium은 WASM 미지원)
- pdfium-render 버전 ↔ 빌드번호 미스매치 시 런타임 심볼 오류 → `scripts/CLAUDE.md` 호환표 참조

## 기각된 대안

- **정적 링크**: 바이너리 크기 +50MB, 라이선스 표시 복잡
- **pdf.js 네이티브 바인딩**: Node.js 의존, CLI 도구에 부적합
