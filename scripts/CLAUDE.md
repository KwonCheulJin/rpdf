# scripts/ — 개발 자동화 스크립트

| 스크립트 | 용도 |
|----------|------|
| `fetch-pdfium.sh` | pdfium 동적 라이브러리 다운로드 (macOS + Linux). CI `Set pdfium env` 스텝 전에 실행. |

## fetch-pdfium.sh 사용법

```bash
# 기본 (빌드번호 7763, pdfium/ 디렉터리)
bash scripts/fetch-pdfium.sh

# 빌드번호 지정
PDFIUM_BUILD=7800 bash scripts/fetch-pdfium.sh
```

환경변수 `PDFIUM_DYNAMIC_LIB_PATH`를 출력하며, CI(`$GITHUB_ENV` 존재 시) 자동 기록.

## 빌드번호 ↔ pdfium-render 버전 호환표

| pdfium-render | 빌드번호 | 확인일 |
|---------------|----------|--------|
| 0.9.1 | 7763 | 2026-05-04 |

> 업그레이드 시 `fetch-pdfium.sh` 상단 `PDFIUM_BUILD` 기본값 + CI yml `env.PDFIUM_BUILD` 동시 변경.
