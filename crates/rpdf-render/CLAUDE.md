# rpdf-render — 렌더링 크레이트

## 역할

PDFium 동적 라이브러리를 런타임 로딩해 PDF → 이미지 렌더링.  
현재: Task #11 (환경 구축) 완료. Task #12에서 PNG 출력 구현 예정.

## 환경 설정 (필수)

```bash
# 1. pdfium 바이너리 다운로드
bash scripts/fetch-pdfium.sh

# 2. 환경변수 설정 (디렉토리, 파일 경로 아님)
export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib
```

## 테스트 실행

```bash
PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib cargo nextest run -p rpdf-render
```

## 주의

- `PDFIUM_DYNAMIC_LIB_PATH`는 **디렉토리** 경로 (`.dylib` 파일 경로 아님)
- macOS: `scripts/fetch-pdfium.sh`가 Gatekeeper quarantine 자동 해제
- WASM 타겟에서 이 크레이트 미포함 — pdfium은 브라우저 미지원 (v0.4에서 pdf.js 대체)

## 빌드번호 ↔ pdfium-render 버전 호환표

| pdfium-render | PDFium 빌드번호 |
|---------------|---------------|
| 0.9.1 | 7763 |
