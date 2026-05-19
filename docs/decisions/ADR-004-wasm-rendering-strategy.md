# ADR-004: WASM 환경 렌더링 전략 — pdf.js 위임

**날짜**: 2026-05-19
**상태**: 승인됨
**결정자**: KwonCheulJin
**관련**: ADR-001 (크레이트 분리), ADR-002 (pdfium 동적 로딩)

## 맥락

rpdf-wasm (v0.4) crate가 WebAssembly 타겟으로 빌드될 때, 렌더링 기능이 필요하다.
pdfium-render (rpdf-render의 핵심 의존)는 네이티브 PDFium 동적 라이브러리를 런타임에 로딩하므로
wasm32-unknown-unknown 타겟에서 사용 불가하다.

세 가지 전략을 검토했다:
1. PDFium을 WASM으로 직접 컴파일 (google/pdfium-wasm 프로젝트)
2. 렌더링을 브라우저 JS 측 pdf.js에 위임, Rust 코어는 파싱·편집·저장만
3. rpdf-svg 자체 렌더러 품질 향상

## 결정

**전략 2 채택**: 웹 환경 렌더링은 pdf.js에 위임한다.
WASM 번들은 `--target web` 빌드로 브라우저 직접 로딩을 목표로 한다.

**`--target web` 선택 근거**: pdf.js는 브라우저 환경에서 동작하므로, WASM도 브라우저 JS 모듈로
직접 로딩하는 `--target web`이 가장 자연스러운 통합 방식이다. `--target bundler`는 webpack/vite
설정이 추가로 필요하고, `--target nodejs`는 서버 환경으로 이 프로젝트의 목적에 부합하지 않는다.

| 작업 | 담당 |
|------|------|
| 페이지 렌더링 (화면 표시) | pdf.js |
| 페이지 썸네일 | pdf.js |
| 문서 구조 파싱 | Rust (`rpdf-parser`) |
| 편집 커맨드 (회전·삭제 등) | Rust (`rpdf-edit`) |
| 저장 (직렬화) | Rust (`rpdf-serializer`) |

## 근거

- **번들 크기**: PDFium WASM 컴파일은 수 MB 규모로 2MB 제한 초과 위험. pdf.js 위임 시 rpdf-wasm은 336KB (gzip)
- **구현 복잡도**: PDFium WASM 빌드는 별도 빌드 인프라 필요. pdf.js는 이미 성숙한 라이브러리
- **역할 분리**: ADR-001의 크레이트 분리 원칙과 일관 — rpdf-render는 네이티브 전용, 웹은 pdf.js
- **유지보수**: PDF 렌더링 품질은 pdf.js 커뮤니티에 위임, Rust 코어는 편집 로직에 집중

## 결과

- `rpdf-wasm`은 `rpdf-render`를 의존하지 않는다
- WASM 번들에 pdfium 라이브러리가 포함되지 않는다
- rpdf-studio (Task #29)에서 pdf.js로 렌더링, rpdf-wasm으로 편집·저장을 분담한다

## 알려진 단점

- **이중 파싱**: 같은 PDF를 pdf.js와 Rust 파서가 각각 파싱 (v0.6 이후 개선 검토)
- **파싱 결과 불일치**: pdf.js와 rpdf-parser의 파싱 결과가 다를 수 있음 (rpdf-studio에서 동기화 레이어 필요)

## 기각된 대안

- **PDFium → WASM 컴파일**: 번들 크기 초과 위험 + 빌드 복잡도
- **SVG 렌더러 개선**: 시간 소모 과다, v0.6 이후 재검토
