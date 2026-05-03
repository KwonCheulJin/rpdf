# 온보딩 가이드

이 문서는 프로젝트에 처음 참여하는 사람(또는 오랜만에 돌아온 본인)이 빠르게 전체 구조와 워크플로우를 파악하도록 돕습니다.

## 10분 안에 이해해야 할 것

### 1. 이 프로젝트가 무엇을 하는가

**rpdf**는 Rust + WASM 기반의 가벼운 PDF 편집기입니다.

- 저사양 환경에서 안정적으로 동작하는 것이 최우선 목표
- 기능이 많기보다 **실제 사용자가 자주 쓰는 기능이 확실하게 되는 것**을 추구
- Rust 코어 하나로 웹, 데스크톱, CLI 세 가지 배포 타겟 지원

### 2. 3층 구조 이해하기

```
┌──────────────────────────────────────┐
│  3. UI 레이어                         │
│  rpdf-studio (웹) / rpdf-desktop     │
│  (Tauri) / CLI                        │
└──────────────────────────────────────┘
              ↑ invoke / WASM bridge
┌──────────────────────────────────────┐
│  2. 바인딩 레이어                     │
│  wasm_api / Tauri commands / CLI     │
└──────────────────────────────────────┘
              ↑ 함수 호출
┌──────────────────────────────────────┐
│  1. 코어 레이어 (Rust)                │
│  parser / model / document_core      │
│  / renderer / serializer              │
└──────────────────────────────────────┘
```

**핵심 원칙**: 코어 레이어는 어떤 UI에도 의존하지 않습니다. 코어만 단위 테스트 가능해야 합니다.

### 3. CQRS 패턴

`document_core/`는 Command Query Responsibility Segregation 패턴을 따릅니다.

- **Commands** (`commands/`): 문서를 변경하는 연산. 예: `RotatePageCommand`, `MergeCommand`
- **Queries** (`queries/`): 문서를 읽기만 하는 연산. 예: `ThumbnailQuery`, `PageInfoQuery`

**왜 이렇게 나누는가**
- Undo/Redo가 자연스러워짐 (Command 스택만 유지하면 됨)
- 테스트가 쉬워짐 (Command/Query 단위로 독립 테스트)
- 나중에 협업 기능 추가 시 Command를 네트워크로 전파하면 됨

### 4. 개발 워크플로우

```
GitHub Issue 생성
      ↓
local/task{N} 브랜치 생성
      ↓
mydocs/plans/ 계획서 작성
      ↓
사람 승인
      ↓
구현 + 테스트
      ↓
mydocs/working/ 완료 보고서
      ↓
devel merge (PR)
      ↓
릴리즈 시 main 태깅
```

## 로컬 환경 세팅

### 사전 요구

- Rust 1.75+ (`rustup update`)
- Node.js 18+ + pnpm
- Docker (WASM 빌드 시)
- VS Code 또는 RustRover 권장

### 첫 빌드

```bash
git clone <repo>
cd rpdf

# Rust 코어
cargo build
cargo test                       # 통과하는지 확인

# CLI 동작 확인
cargo run -- info examples/sample.pdf

# 웹 에디터
cd rpdf-studio
pnpm install
pnpm dev                         # http://localhost:5173

# 데스크톱 앱
cd ../rpdf-desktop
pnpm install
pnpm tauri dev
```

## 자주 쓰는 커맨드

| 목적 | 커맨드 |
| --- | --- |
| 테스트 실행 | `cargo test` |
| 특정 테스트만 | `cargo test parser::` |
| 린트 | `cargo clippy -- -D warnings` |
| 포맷 | `cargo fmt` |
| 릴리즈 빌드 | `cargo build --release` |
| WASM 빌드 | `docker compose run --rm wasm` |
| 웹 개발 서버 | `cd rpdf-studio && pnpm dev` |
| Tauri 개발 | `cd rpdf-desktop && pnpm tauri dev` |
| Tauri 빌드 | `cd rpdf-desktop && pnpm tauri build` |

## 디버깅 도구

문제가 생겼을 때 **코드를 고치기 전에** 먼저 이 도구들로 현상을 파악하세요.

```bash
# 파일 기본 정보
rpdf info sample.pdf

# IR 전체 덤프
rpdf dump sample.pdf

# 특정 페이지만
rpdf dump sample.pdf -p 3

# 페이지 레이아웃
rpdf dump-pages sample.pdf -p 3

# 시각적 디버그 (페이지 경계, 객체 위치 표시)
rpdf export-svg sample.pdf --debug-overlay

# 두 PDF의 구조 비교
rpdf diff a.pdf b.pdf
```

## 자주 가는 디렉터리

| 경로 | 언제 가는가 |
| --- | --- |
| `src/parser/` | PDF 파싱 버그, 새 PDF 버전 지원 |
| `src/model/` | 데이터 구조 이해, IR 수정 |
| `src/document_core/commands/` | 새 편집 기능 추가 |
| `src/document_core/queries/` | 렌더링·조회 로직 |
| `src/renderer/` | 렌더링 품질 이슈 |
| `rpdf-studio/src/ui/` | 웹 UI 변경 |
| `rpdf-desktop/src-tauri/` | 데스크톱 백엔드 |
| `mydocs/plans/` | 작업 전 계획서 작성 |
| `mydocs/tech/` | 기술적 맥락 이해 |
| `tests/regression/` | 회귀 테스트 추가 |

## 첫 기여로 추천하는 타스크

처음이라면 다음 순서로 익히세요.

1. `cargo run -- info <sample.pdf>` 실행해보고 출력을 이해
2. `src/parser/mod.rs` 읽어보기
3. `tests/` 안의 테스트 케이스 하나 읽어보기
4. 간단한 문서 오타 수정 PR
5. 작은 엣지 케이스 테스트 추가 PR

## 막혔을 때

1. `mydocs/troubleshootings/` 검색
2. 기존 Issue 검색
3. 새 Issue 생성 (재현 케이스 포함)

## 참고 문서

- 아키텍처 상세: `mydocs/manual/architecture.md`
- 개발 방법론: `mydocs/manual/hyper-waterfall.md`
- PDF 스펙 정리: `mydocs/tech/pdf-spec-summary.md`
