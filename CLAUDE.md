# CLAUDE.md

이 문서는 AI 페어 프로그래밍(Claude Code) 사용 시의 **작업 규칙**입니다.
AI는 이 규칙을 따르고, 사람은 이 규칙이 지켜지는지 검토합니다.

## 기본 원칙

**이것은 바이브 코딩이 아닙니다.** AI가 생성한 코드는 반드시 사람이 읽고 이해하고 승인한 뒤에 merge합니다.

| 사람의 역할 | AI의 역할 |
| --- | --- |
| 방향 설정, 우선순위 결정 | 분석, 계획, 구현 |
| 계획 검토 및 승인 | 구현 계획서 초안 작성 |
| 아키텍처 결정 | 정밀한 실행 |
| 코드 리뷰 및 품질 판단 | 코드·문서·테스트 생성 |

## 도구 활용 원칙

손으로 빈 파일을 만드는 것보다 **검증된 CLI 도구의 출력을 활용한 뒤 프로젝트에 맞게 조정**한다.
이는 베스트 프랙티스를 자동으로 반영하고, 공식 도구의 변경에 자동으로 따라가기 위함이다.

### 우선순위

1. **공식 표준 도구가 있으면 그것을 사용** (`cargo new`, `pnpm init`, `create-tauri-app` 등)
2. **검증된 서드파티 CLI가 있으면 그것을 사용** (`gitignore.io`, `wasm-pack` 등)
3. **위 둘이 없을 때만 손으로 작성**

### 단계별 보일러플레이트 매핑

| 단계 | 도구 | 비고 |
| --- | --- | --- |
| Task #1 디렉터리 구조 | `mkdir`, `cargo new --lib` | 표준 cargo 활용 |
| Task #1 .gitignore | `gitignore.io` API | 베스트 프랙티스 자동 |
| Task #1 package.json | `pnpm init` 후 수정 | 표준 도구 활용 |
| Task #1 CI 파일 | 직접 작성 | 단순한 워크플로우, 보일러플레이트 불필요 |
| v0.4 WASM 빌드 | `wasm-pack` | WASM 표준 |
| v0.5 Tauri 앱 | `pnpm create tauri-app` | 필수 보일러플레이트 |

### 구체적 활용 명령

#### Rust 크레이트 생성
```bash
# 라이브러리 크레이트
cargo new --lib crates/rpdf-core --vcs none

# 바이너리 크레이트 (CLI 등)
cargo new --bin crates/rpdf-cli --vcs none
```

`--vcs none` 옵션으로 cargo가 자체 git 초기화를 시도하지 않게 한다 (워크스페이스 루트의 git을 사용).

#### .gitignore 생성
```bash
curl -L https://www.toptal.com/developers/gitignore/api/rust,node,macos,windows,linux,visualstudiocode > .gitignore
```

이렇게 받은 후 프로젝트 특화 항목(예: `pkg/`, `src-tauri/target/`)만 추가한다.

#### Node 프로젝트 초기화
```bash
pnpm init
```

생성된 `package.json`의 `name`, `private: true`, `scripts`, `workspaces` 항목만 수정한다.

#### Tauri 앱 (v0.5에서 사용)
```bash
cd packages
pnpm create tauri-app desktop --template react-ts
```

비대화형으로 React + TypeScript 템플릿을 즉시 생성한다.

#### WASM 빌드 (v0.4에서 사용)
```bash
cargo install wasm-pack
wasm-pack build crates/rpdf-core --target web --out-dir ../../pkg
```

### 손으로 만들어야 하는 것

다음은 표준 도구가 없거나, 우리 프로젝트의 의도가 강하게 들어가야 하므로 직접 작성한다.

- 워크스페이스 루트의 `Cargo.toml` (workspace 정의)
- `pnpm-workspace.yaml`
- `.github/workflows/*.yml` (CI 파이프라인)
- `.tool-versions`
- 우리 프로젝트의 `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`
- `mydocs/` 안의 모든 문서

---

## Rust 개발 도구 표준 세트

다음 도구들을 1회 설치해두면 모든 타스크에서 활용된다. Task #1 진행 중 함께 설치하고
`mydocs/manual/dev-tools.md`에 설치 명령과 용도를 정리한다.

### 필수 (Task #1에서 설치)

```bash
# 의존성 관리 자동화
cargo install cargo-edit

# 빠른 테스트 러너 (표준 cargo test보다 2~3배 빠름)
cargo install cargo-nextest --locked

# 파일 변경 감지 자동 빌드/테스트
cargo install cargo-watch
```

### 활용 예시

```bash
# 의존성 추가/업데이트 (Cargo.toml 직접 수정 대신)
cargo add lopdf
cargo add tracing --features=log
cargo upgrade

# 코드 변경 시 자동 테스트
cargo watch -x test
cargo watch -x 'clippy -- -D warnings'

# 빠른 테스트 실행
cargo nextest run
cargo nextest run parser::          # 모듈 한정
```

### 권장 (필요 시점에 설치)

```bash
# 워크스페이스 다중 크레이트 버전/배포 관리 (v0.4에서 도입)
cargo install cargo-workspaces

# 라이선스 및 보안 감사 (오픈소스 공개 시점에 도입)
cargo install cargo-deny

# WASM 빌드 (v0.4에서 도입)
cargo install wasm-pack

# Tauri CLI (v0.5에서 도입, 단 프로젝트 내 devDependencies로 설치 권장)
cargo install tauri-cli --version "^2.0"
```

### 도구 선택 기준

새로운 도구 도입 시 다음 기준을 충족해야 한다.

1. **활발한 유지보수**: 최근 6개월 내 커밋 또는 릴리즈
2. **널리 쓰임**: GitHub stars 1,000+ 또는 cargo install 통계가 충분
3. **명확한 가치**: 손으로 하는 것 대비 시간/품질 이득이 명확
4. **호환 라이선스**: MIT, Apache 2.0, BSD 계열

새 도구를 도입하기 전에 `mydocs/tech/dev-tool-{도구명}.md`에 도입 근거를 짧게 기록한다.

---

## 도구 활용 시 Claude Code의 행동

Claude Code가 다음과 같은 작업을 할 때는 이 원칙을 따른다.

### 새 프로젝트/크레이트 생성 시
- 손으로 디렉터리·파일 만들기 금지
- 반드시 `cargo new`, `pnpm init`, `create-tauri-app` 등 공식 도구 사용
- 도구 출력 후 우리 프로젝트 컨벤션에 맞게 조정

### 의존성 추가 시
- `Cargo.toml` 직접 편집 대신 `cargo add` 사용
- 그래야 최신 안정 버전이 자동 반영됨
- feature가 필요하면 `cargo add <crate> --features=<feat>`

### 설정 파일 생성 시
- `.gitignore` → `gitignore.io` API
- `.editorconfig` → editorconfig.org 표준 사용
- 라이선스 파일 → SPDX 표준 텍스트 사용

### 예외: 손으로 만들어야 할 때

다음 경우는 표준 도구가 있어도 손으로 작성한다.

- 우리 프로젝트의 아키텍처 의도가 들어가야 할 때 (workspace `Cargo.toml`)
- 파일 내용이 외부 표준이 아닌 우리 컨벤션을 따라야 할 때 (mydocs 문서)
- 도구 출력이 우리에게 불필요한 항목을 너무 많이 포함할 때

---

## 작업 시작 전 체크리스트

새 타스크 시작 시 Claude Code는 다음을 확인한다.

- [ ] 이 작업에 표준 CLI 도구가 있는가?
- [ ] 있다면 어떤 옵션을 줘야 우리 프로젝트에 맞는가?
- [ ] 도구 출력 후 무엇을 추가/수정해야 하는가?
- [ ] 손으로 만들어야 한다면 그 이유가 명확한가?

이 체크리스트를 통과한 뒤에야 실제 명령 실행을 시작한다.

---

## 작업 프로토콜

모든 타스크는 다음 순서를 따릅니다.

1. **Issue 생성** — `gh issue create`로 GitHub Issue 등록, 마일스톤 지정
2. **브랜치 생성** — `local/task{Issue번호}` 브랜치
3. **계획서 작성** — `mydocs/plans/task{N}-{slug}.md`
   - 데이터 모델 변경 사항
   - 새/변경되는 API
   - 엣지 케이스
   - 테스트 전략
4. **계획 승인** — 사람이 읽고 승인
5. **구현** — 계획서대로, 계획 외 변경 시 계획서부터 수정
6. **테스트** — `cargo test`, `cargo clippy`, `pnpm test` 통과 필수
7. **완료 보고서** — `mydocs/working/task{N}-done.md`
8. **PR 및 merge** — `devel` 브랜치로 PR, `closes #{N}`

## 금지 사항

다음은 AI가 절대 하지 말아야 합니다.

- 계획서 없이 구현을 시작하는 것
- 테스트 없이 새 기능을 추가하는 것
- 아키텍처 결정을 독자적으로 내리는 것
- `unsafe` 블록을 사람 확인 없이 추가하는 것
- 외부 크레이트를 사람 승인 없이 추가하는 것
- 마일스톤 범위 밖의 기능을 "겸사겸사" 구현하는 것

## 커밋 메시지 규칙

```
Task #{번호}: 한 줄 요약

상세 설명 (선택)
- 변경된 파일의 의미
- 왜 이 방식을 택했는지

closes #{번호}
```

예시:
```
Task #12: 페이지 추출 커맨드 구현

- document_core/commands/extract_pages.rs 추가
- CLI `rpdf extract --pages 1-3` 지원
- Undo를 위한 역연산 구현

closes #12
```

## 품질 관문

merge 전 모두 통과해야 합니다.

- `cargo test` — 전체 테스트 통과
- `cargo clippy -- -D warnings` — 경고 없음
- `cargo fmt --check` — 포맷 정리됨
- 웹/데스크톱: `pnpm test`, `pnpm lint`, `pnpm typecheck`
- E2E 회귀: 주요 워크플로우 Puppeteer 통과

## 문서화 규칙

- 모든 공개 API는 `///` Rust 문서 주석 작성
- 복잡한 로직은 이유를 주석으로 남김 (코드가 "무엇"을 하는지는 코드에서, "왜" 하는지는 주석에서)
- 새 기능은 `mydocs/tech/` 또는 `mydocs/manual/`에 기술 노트 추가
- 버그 수정은 `mydocs/troubleshootings/`에 원인 분석 기록

## 디버깅 프로토콜

PDF 관련 버그는 다음 순서로 진단합니다.

1. `rpdf info <file>` — 파일 메타데이터 확인
2. `rpdf dump <file> -p <page>` — 해당 페이지 IR 덤프
3. `rpdf export-svg <file> --debug-overlay` — 시각적 디버그
4. 재현 케이스를 `tests/regression/`에 추가
5. 수정 후 동일 케이스가 통과하는지 확인

## 파일 명명 규칙

- Rust 파일: `snake_case.rs`
- TypeScript 파일: `kebab-case.ts` 또는 `PascalCase.tsx` (컴포넌트)
- 문서: `kebab-case.md`, 단 `orders/`는 `yyyymmdd.md`
- 브랜치: `local/task{N}` 또는 `feature/{slug}`

## 참고

- 개발 방법론: `mydocs/manual/hyper-waterfall.md`
- 아키텍처: `mydocs/manual/architecture.md`
- 온보딩: `mydocs/manual/onboarding.md`
