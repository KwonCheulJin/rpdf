# ADR-001: Rust 워크스페이스 크레이트 분리 전략

**날짜**: 2026-04-01  
**상태**: 승인됨  
**결정자**: KwonCheulJin

## 맥락

PDF 처리 라이브러리를 단일 크레이트로 만들면 CLI / WASM / Tauri 배포 시 불필요한 의존성이 모두 포함된다.

## 결정

도메인 역할별로 4개 크레이트로 분리한다:

- `rpdf-core` — 도메인 타입만 (파싱 로직 없음)
- `rpdf-parser` — PDF 파싱
- `rpdf-render` — 렌더링 (pdfium 동적 로딩)
- `rpdf-cli` — 바이너리 진입점

## 근거

- WASM 타겟은 `rpdf-core` + `rpdf-parser`만 컴파일 → 바이너리 크기 최소화
- `rpdf-render`는 pdfium 네이티브 의존 → WASM 제외 가능
- `rpdf-core`가 순수 값 객체만 보유 → 어느 타겟에도 컴파일 가능

## 결과

- v0.4 WASM 빌드 시 pdfium 링크 불필요
- 크레이트 경계가 아키텍처 레이어를 강제 (컴파일 타임 보장)

## 기각된 대안

- **단일 크레이트 + feature flag**: feature 조합이 늘어날수록 유지보수 복잡도 증가
