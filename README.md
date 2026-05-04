# rpdf

**가벼운 PDF 편집기 — Rust + WASM 기반**

저사양 환경에서도 안정적으로 돌아가는, 꼭 필요한 기능만 담은 PDF 편집기입니다.
실제 사용자의 요구에서 출발해, 아키텍처 성숙도 기준으로 버전을 올립니다.

## 철학

- **작게, 빠르게, 확실하게.** 기능을 늘리기 전에 품질을 먼저 확보합니다.
- **실사용자 피드백 기반.** 상상이 아닌 실제 사용 패턴이 기능 우선순위를 결정합니다.
- **뼈대 먼저, 살은 나중에.** 아키텍처가 흔들리지 않을 만큼 견고해진 뒤 기능을 추가합니다.

## 로드맵

```
0.1 ─── 0.2 ─── 0.3 ─── 0.4 ─── 0.5 ─── 0.6+
파서     렌더     편집     WASM     Tauri    실사용 피드백
뼈대     뼈대     커맨드   바인딩   데스크톱
```

| 버전 | 목표 | 산출물 |
| --- | --- | --- |
| **v0.1** | 파서 뼈대 | CLI `rpdf info/dump`, PDF IR 정의 |
| **v0.2** | 렌더링 뼈대 | `rpdf export-svg`, 회귀 테스트 인프라 |
| **v0.3** | 편집 커맨드 | CQRS 레이어, merge/split/rotate/delete |
| **v0.4** | WASM 바인딩 | `@rpdf/core` npm, 웹 데모 |
| **v0.5** | Tauri 데스크톱 | 실사용자 배포, 기존 pdf-studio 기능 이식 |
| **v0.6+** | 실사용 피드백 | 주석·서명·하이라이트·페이지 추출 등 |

## 기술 스택

- **코어**: Rust (Edition 2021)
- **PDF 처리**: `lopdf`, `pdfium-render`
- **WASM**: `wasm-bindgen`, `wasm-pack`
- **프론트엔드**: TypeScript, React 19, Vite, Tailwind CSS 4
- **상태 관리**: Zustand
- **데스크톱**: Tauri 2
- **테스트**: `cargo test`, Vitest, Puppeteer (E2E)

## 프로젝트 구조

```
rpdf/
├── src/                    # Rust 코어 (파서/모델/렌더러)
├── rpdf-studio/            # 웹 에디터
├── rpdf-desktop/           # Tauri 데스크톱 앱
├── npm/                    # npm 배포 패키지
├── examples/               # 샘플 PDF
├── tests/                  # 통합 테스트
├── mydocs/                 # 개발 과정 문서
└── scripts/                # 빌드/품질 도구
```

## 시작하기

### 사전 요구

- Rust 1.75+
- Node.js 18+
- pnpm
- Docker (WASM 빌드 시)

### 개발 빌드

```bash
# Rust 코어
cargo build
cargo test

# 웹 에디터
cd rpdf-studio
pnpm install
pnpm dev

# 데스크톱 앱
cd rpdf-desktop
pnpm tauri dev
```

## 문서

`mydocs/` 디렉터리에 전체 개발 과정이 기록되어 있습니다.

- `mydocs/orders/` — 일일 할일 (yyyymmdd.md)
- `mydocs/plans/` — 구현 계획서
- `mydocs/working/` — 완료 보고서
- `mydocs/tech/` — 기술 연구 문서
- `mydocs/manual/` — 온보딩·아키텍처 가이드

## 주의사항 (Gotcha)

- **pdfium 런타임 필요**: `rpdf export-png` 실행 전 `scripts/fetch-pdfium.sh` 실행 필수
- **PDFIUM_DYNAMIC_LIB_PATH는 디렉토리**: 파일 경로가 아닌 `pdfium/lib/` 같은 디렉토리를 지정
- **rpdf-core에 파싱 로직 금지**: 파싱은 `rpdf-parser`만 — 위반 시 WASM 빌드 오류
- **insta 스냅샷 첫 실행**: `*.snap.new` 생성됨 → `cargo insta accept` 후 커밋 필요

## 기여

이 프로젝트는 개인 학습 + 실사용 도구 제작이 목적입니다. 이슈 제보는 환영하지만, 기능 추가는 실사용 피드백을 우선합니다.

[기여 가이드 전체 보기](CONTRIBUTING.md)

## 라이선스

MIT
