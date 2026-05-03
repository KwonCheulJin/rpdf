# 크레이트 선택 의사결정 기록

## 개요

rpdf 구현에 사용할 주요 Rust 크레이트들을 비교 검토하고, 선택 근거를 기록합니다. 이 문서는 v0.1 시작 전에 작성되었으며, 이후 선택이 바뀌면 섹션 말미에 업데이트 이력을 남깁니다.

## 1. PDF 파싱 및 저장

### 후보

| 크레이트 | 장점 | 단점 |
| --- | --- | --- |
| `lopdf` | 순수 Rust, 파싱+저장 모두 지원, 활발한 유지보수 | 성능은 보통, 렌더링 없음 |
| `pdf` | 견고한 타입 시스템, 좋은 에러 메시지 | 저장 지원 약함, 활동 적음 |
| `pdfium-render` | 렌더링 품질 최상급 | 저장 및 편집 기능 없음, PDFium 바이너리 필요 |
| `mupdf-rs` | 렌더링+편집 일부 가능 | AGPL 라이선스 문제 |

### 선택: `lopdf` + `pdfium-render`

**근거**
- `lopdf`로 파싱/편집/저장
- `pdfium-render`로 렌더링 전담
- 역할 분리로 각 크레이트의 장점만 활용
- MIT/BSD 라이선스만 조합 (상용화 가능성 열어둠)

**리스크**
- 두 크레이트의 PDF 객체 표현 차이로 변환 레이어가 필요할 수 있음
- `pdfium-render`는 PDFium 다이나믹 라이브러리가 필요하여 배포가 복잡함

**대응**
- 코어 IR을 중간에 두고 양쪽을 흡수
- Tauri 빌드에 PDFium 바이너리를 번들

## 2. 이미지 처리

### 후보

| 크레이트 | 장점 | 단점 |
| --- | --- | --- |
| `image` | Rust 표준에 가까운 위치, 포맷 지원 광범위 | 성능은 중간 |
| `imageproc` | `image` 위의 고수준 작업 | 기본 변환은 `image`에 의존 |
| `libvips` (via FFI) | 최고 성능 | FFI 복잡, 배포 어려움 |
| Node `sharp` | 빠름 | Rust에서 불가, Electron 전용 |

### 선택: `image`

**근거**
- 기존 pdf-studio의 `sharp` 의존을 제거하는 게 Tauri 전환의 목적 중 하나
- 이미지 처리는 TIFF → PDF 변환 정도의 제한적 범위
- `image` 크레이트만으로 충분

**향후 재검토 조건**
- 대용량 이미지 일괄 처리가 주요 유스케이스가 되면 `libvips` 재검토

## 3. WASM 바인딩

### 후보

| 크레이트 | 장점 | 단점 |
| --- | --- | --- |
| `wasm-bindgen` | 사실상 표준, 생태계 성숙 | WASM 크기 증가 |
| `uniffi` | 다중 언어 바인딩 | WASM 전용이 아님, Mozilla 주도 |
| 직접 FFI | 번들 작음 | 작성 부담 큼 |

### 선택: `wasm-bindgen` + `wasm-pack`

**근거**
- rhwp도 동일 선택, 검증됨
- TypeScript 타입 자동 생성 가능
- npm 배포 파이프라인 표준화됨

## 4. 데스크톱 프레임워크

### 후보

| 프레임워크 | 번들 크기 | 메모리 | 학습 비용 | 비고 |
| --- | --- | --- | --- | --- |
| Electron | 100-150MB | 300-500MB | 낮음 | 기존 pdf-studio 사용 |
| Tauri 2 | 5-15MB | 80-150MB | 중간 | Rust 백엔드, WebView 사용 |
| Native (Egui/Slint) | 20-50MB | 50-100MB | 높음 | 웹 UI 재사용 불가 |
| Flutter | 40-80MB | 150-250MB | 높음 | Dart 필요 |

### 선택: Tauri 2

**근거**
- "저사양에서도 안정적" 목표에 최적
- Rust 코어 직접 호출로 WASM 오버헤드 없음
- 웹 UI 재사용 가능 (`rpdf-studio`와 공유)
- rhwp 프로세스와 유사한 다중 배포 아키텍처 가능

**리스크**
- 각 OS 네이티브 WebView 차이 (pdf.js 렌더링 불일치 가능)
- Linux WebKitGTK 이슈 보고 많음

**대응**
- 렌더링은 pdf.js 대신 `pdfium-render`의 출력을 Canvas에 그리는 방식 우선
- WebKitGTK는 일단 지원 목표 낮게 설정 (여자친구 환경 = Windows/macOS)

## 5. 프론트엔드 프레임워크

### 후보

| 프레임워크 | 장점 | 단점 |
| --- | --- | --- |
| React 19 | 생태계 광대, 기존 pdf-studio 경험 | 번들 크기 |
| Svelte 5 | 번들 작고 빠름 | 학습 비용 |
| SolidJS | React와 유사하지만 빠름 | 생태계 작음 |
| Vanilla TS | 번들 최소 | 규모 커지면 관리 어려움 |

### 선택: React 19

**근거**
- 기존 pdf-studio 코드베이스와 연속성
- 학습 비용 최소화 (본 프로젝트 목적이 학습이더라도 중복 학습은 피함)
- Tauri에서의 React 레퍼런스 풍부

## 6. 상태 관리

### 후보

| 도구 | 장점 | 단점 |
| --- | --- | --- |
| Zustand | 가볍고 단순 | DevTools 제한적 |
| Jotai | 원자적 상태, 세밀한 리렌더 | 학습 곡선 |
| Redux Toolkit | 표준, DevTools 강력 | boilerplate |
| React Context | 기본 제공 | 대규모에서 성능 문제 |

### 선택: Zustand

**근거**
- 기존 pdf-studio 선택과 동일
- PDF 편집기의 상태는 "현재 문서, 선택된 페이지, Undo 스택" 정도로 단순
- 필요 시 세밀한 스토어 분할 가능

## 7. CLI 파서

### 후보

| 크레이트 | 장점 | 단점 |
| --- | --- | --- |
| `clap` | 표준, derive 매크로 | 약간 무거움 |
| `structopt` | `clap` v2 시절 인기 | 이제 `clap` v3+에 통합됨 |
| `argh` | 가벼움 | 기능 제한적 |

### 선택: `clap` (derive feature)

**근거**
- 서브커맨드가 많아질 예정
- derive 매크로로 타입 안전
- `--help` 자동 생성 품질 좋음

## 8. 테스트

### 도구 조합

- **단위 테스트**: `cargo test` + 표준 `#[test]`
- **속성 기반 테스트**: `proptest` (파서 견고성 검증)
- **스냅샷 테스트**: `insta` (렌더링 출력 회귀 방지)
- **E2E**: Puppeteer + Chrome CDP (웹 에디터) / `tauri-driver` (데스크톱)
- **벤치마크**: `criterion`

### 회귀 테스트 샘플

`samples/` 디렉터리에 다음 카테고리의 실제 PDF를 수집:
- 단순 텍스트 PDF
- 한글 포함 PDF
- 스캔 이미지 PDF
- 복잡한 레이아웃 (이단, 표)
- 폼 포함 PDF
- 암호화된 PDF
- 큰 파일 (100MB+)
- 깨진 파일 (복구 테스트)

## 9. 에러 처리

### 선택: `thiserror` (라이브러리) + `anyhow` (바이너리)

- `thiserror`: 코어 크레이트의 상세 에러 타입 정의
- `anyhow`: CLI 내부의 에러 chaining

## 10. 로깅

### 선택: `tracing` + `tracing-subscriber`

**근거**
- 비동기 코드와 호환
- WASM 환경에서도 동작 (콘솔 출력)
- 구조화된 로그 필드 지원

## 종합 의존성 목록 (초안)

```toml
[dependencies]
# PDF 처리
lopdf = "0.32"
pdfium-render = "0.8"

# 이미지
image = "0.25"

# CLI
clap = { version = "4", features = ["derive"] }

# 에러
thiserror = "1"
anyhow = "1"

# 로깅
tracing = "0.1"
tracing-subscriber = "0.3"

# 직렬화 (WASM 바인딩용)
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# WASM
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }

[dev-dependencies]
proptest = "1"
insta = "1"
criterion = "0.5"

[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
```

## 업데이트 이력

- 2026-04-24: 초안 작성 (v0.1 이전)
