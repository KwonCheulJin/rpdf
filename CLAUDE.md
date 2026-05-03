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

## 설계 원칙

코드를 작성할 때 다음 원칙을 따른다.

### KISS — Keep It Simple, Stupid

가장 단순한 해결책을 먼저 택한다. 복잡한 추상화나 일반화는 단순한 방법이 실제로 부족하다고 증명된 뒤에 도입한다.

- 함수는 한 가지 일만 한다
- 네이밍은 의도를 그대로 드러낸다
- 중첩 깊이는 3 이하를 목표로 한다

### DRY — Don't Repeat Yourself

지식은 시스템 내에서 단 한 곳에만 존재해야 한다. 코드 중복보다 **지식 중복**을 경계한다.

- 같은 로직이 세 번 등장하면 추출을 검토한다
- 단, 우연히 비슷한 코드는 억지로 합치지 않는다 (WET: Write Everything Twice 원칙 — 두 번까지는 허용)

### YAGNI — You Aren't Gonna Need It

지금 필요하지 않은 기능은 지금 만들지 않는다. 마일스톤 계획서에 없는 기능을 "나중을 대비해" 추가하는 것은 금지한다.

- 확장 포인트는 실제 확장이 필요할 때 추가한다
- 현재 Issue 범위 밖의 코드는 작성하지 않는다

### SOLID

객체지향 설계 5원칙. Rust에서는 trait과 모듈 경계로 표현한다.

| 원칙 | 의미 | Rust 적용 |
| --- | --- | --- |
| **S**RP (단일 책임) | 하나의 모듈은 하나의 이유로만 변경됨 | 모듈·파일 분리 기준 |
| **O**CP (개방-폐쇄) | 확장에는 열려있고, 수정에는 닫혀있음 | trait으로 확장 포인트 정의 |
| **L**SP (리스코프 치환) | 하위 타입은 상위 타입을 대체할 수 있음 | trait impl이 계약을 깨지 않음 |
| **I**SP (인터페이스 분리) | 사용하지 않는 메서드를 강요하지 않음 | trait을 작게 쪼갬 |
| **D**IP (의존성 역전) | 구체 타입이 아닌 추상에 의존 | 함수·구조체가 trait을 받음 |

### TDD — Test First

테스트를 **구현보다 먼저** 작성한다. 이것이 TDD의 핵심이다.

#### Red → Green → Refactor

1. **Red** — 아직 존재하지 않는 동작을 검증하는 테스트를 먼저 작성한다. 컴파일이 안 되거나 실패해야 정상이다.
2. **Green** — 테스트를 통과하는 **최소한의** 코드만 작성한다. 예쁘지 않아도 된다.
3. **Refactor** — 테스트가 통과된 상태에서 중복 제거·이름 개선·설계 개선을 한다. 테스트가 여전히 통과해야 한다.

이 사이클을 한 함수, 한 엣지 케이스 단위로 반복한다.

#### F.I.R.S.T 원칙

| 속성 | 의미 | 적용 |
| --- | --- | --- |
| **F**ast | 빠르게 실행됨 | 단위 테스트는 I/O 없이 |
| **I**solated | 다른 테스트에 독립적 | 공유 상태·순서 의존 금지 |
| **R**epeatable | 어느 환경에서나 동일 결과 | 외부 서비스·시간 의존 금지 |
| **S**elf-validating | 통과/실패가 명확 | `assert!` / `assert_eq!` 사용 |
| **T**imely | **구현 직전**에 작성 | 구현 후 추가 작성은 원칙 위반 |

테스트 없이 new feature를 merge하지 않는다.

### DDD — Domain-Driven Design

도메인 지식을 코드에 직접 반영한다. PDF 스펙(`ISO 32000`)의 용어가 코드 곳곳에 그대로 등장해야 한다.

#### 핵심 개념

| 개념 | 의미 | 이 프로젝트에서 |
| --- | --- | --- |
| **유비쿼터스 언어** | 도메인 전문가와 개발자가 공유하는 단일 용어집 | PDF 스펙 용어를 코드에 그대로 사용 |
| **바운디드 컨텍스트** | 모델이 일관성을 유지하는 경계 | 크레이트 = 바운디드 컨텍스트 (`rpdf-core`, `rpdf-parser`, …) |
| **값 객체 (Value Object)** | 식별자 없이 값으로만 동등성 판단 | `ObjectId`, `PdfVersion` — `#[derive(Copy, Clone, PartialEq)]` |
| **엔티티 (Entity)** | 생명주기와 식별자를 가진 객체 | `PdfDocument`, 페이지 객체 (향후) |
| **도메인 서비스** | 특정 엔티티에 속하지 않는 도메인 로직 | `parse_header()`, `parse_trailer()` |

#### 적용 규칙

- **용어 일관성**: PDF 스펙 용어를 그대로 쓴다. 임의로 바꾸지 않는다.
  - `xref` → `XrefTable` (❌ `CrossReferenceTable`)
  - `trailer` → `PdfTrailer` (❌ `PdfFooter`)
  - `startxref` → 키워드 그대로 변수명·함수명에 반영
- **크레이트 경계 = 도메인 경계**: `rpdf-core`는 도메인 타입만 보유하고 파싱 로직을 갖지 않는다.
- **값 객체는 불변**: `Copy + Clone + PartialEq + Eq`를 기본으로 derive한다.
- **원시 타입 포장**: `u32` 대신 `ObjectId`, `(u8, u8)` 대신 `PdfVersion`처럼 의미 있는 도메인 타입을 만든다.

### 에러 변형 도달 가능성

에러 enum 변형을 추가할 때, 해당 에러가 실제로 발생하는 코드 경로가 있는지 확인한다.
도달 불가능한 에러 변형(dead error variant)은 테스트에서 감지되지 않는다.

- 예: `SEARCH_WINDOW == DICT_MAX_BYTES`이면 `TrailerTooLarge`는 도달 불가 → SEARCH_WINDOW를 더 크게 잡아야 함
- 에러 변형을 추가한 뒤, 해당 에러를 발생시키는 단위 테스트도 함께 작성한다.
- 테스트로 발생시킬 수 없는 에러 변형은 도달 불가능한 dead variant이거나 설계가 잘못된 것이다. 둘 중 하나를 결정해야 한다 — 변형을 제거하거나 코드 경로를 수정하거나.

### 테스트 파일 배치

파서 또는 라이브러리 크레이트의 단위 테스트는 다음 원칙을 따른다.
프로덕션 코드와 테스트 코드를 분리해 파일 크기를 관리하고, 계획서와 실제 구현 일치성을 확인하기 위함이다.

**기본**: 별도 테스트 파일에 작성
- 위치: `tests/parser/<module>_tests.rs` 또는 동등한 외부 위치
- 새 파서 모듈을 추가하면 `tests/parser/mod.rs`에도 등록한다

**예외**: 다음 경우에만 인라인 `#[cfg(test)] mod internal_tests {}` 사용
- private 함수를 직접 테스트해야 할 때
- 인라인 모듈 이름은 `internal_tests`로 명명하여 의도 명시
- 공개 API 테스트는 그래도 별도 파일에 둠

**판단 기준**: "이 테스트가 공개 API만으로 검증 가능한가"
- 가능 → 별도 파일
- 불가(private 동작 검증 필요) → 인라인 `internal_tests`

### 체크포인트 시점 셀프 리뷰

각 체크포인트 도달 시(빌드·테스트 통과 확인 직후), 다음 체크포인트로
넘어가기 전에 짧은 셀프 리뷰를 수행한다. 단순한 "통과 여부 확인"이
아니라 **명세 위반 시나리오를 능동적으로 탐색**하는 활동이다.
이 리뷰는 단위 테스트로 잡히지 않는 종류의 버그(경계값 조건, 에러 우선순위
역전 등)를 구현 직후에 발견하는 데 특히 효과적이다.

**검토 항목**:
- 방금 작성한 코드가 계획서의 명세를 모두 만족하는가
- 명세를 위반할 시나리오를 의식적으로 탐색한다 (예: 경계값, 명세상 "항상" 또는 "절대"라고 명시된 조건)
- 계획서와 실제 구현이 일치하는가 (파일 위치, 함수명, 시그니처)
- 정책 일관성이 깨지지 않았는가 (예: 같은 종류의 처리가 두 곳에서 다르게 동작하지 않는가)

**발견 사항 처리**:
- 즉시 수정 가능: 다음 체크포인트 진행 전에 처리
- 별도 작업 필요: 체크포인트 보고에 명시 후 사람의 결정 받음
- 회고 가치 있음: `troubleshootings/` 또는 회고 분류 대상

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

#### GitHub Actions CI에서 pnpm 설정
```yaml
- uses: pnpm/action-setup@v4   # version: 키 없음 — packageManager 필드 자동 인식
- uses: actions/setup-node@v4
  with:
    node-version: 22
    cache: pnpm
```

`pnpm/action-setup@v4`는 `package.json`의 `packageManager` 필드를 자동으로 읽는다.
`version:` 키를 함께 지정하면 **ERR_PNPM_BAD_PM_VERSION** 충돌로 CI가 실패한다.
→ `packageManager` 필드를 단일 소스로 유지하고, CI에서 `version:` 키를 제거한다.
→ 참고: `mydocs/troubleshootings/pnpm-action-setup-version-conflict.md`

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

#### 외부 크레이트 의존 계획 시 사전 확인

외부 크레이트의 내부 파서나 유틸리티 함수에 의존하는 계획을 세울 때,
docs.rs에서 해당 기능이 공개 API인지 먼저 확인한다.

- docs.rs에 보이지 않으면 1차로 접근 불가로 간주한다
- feature flag, 별도 export 경로 등 우회 수단이 있는지 추가 확인한다
- 우회 수단이 있어도 그것이 안정 보장되지 않는 internal API라면 의존하지 않는다
- 승인 전 계획서에 "공개 API 확인 완료" 항목을 명시한다
- 확인 결과 비공개라면 자체 구현 또는 다른 크레이트로 대안을 제시한다

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
- [ ] 외부 크레이트를 새로 도입한다면 docs.rs에서 공개 API 확인 완료?
- [ ] 새 에러 변형을 추가한다면 그 에러를 발생시키는 테스트가 있는가?

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
   - ⚠️ 버전 숫자는 "최소 요구사항"이 아닌 **실제 설치된 버전** 기준으로 기재한다.
     `cargo new`는 현재 Rust의 기본 edition을 사용하므로, 계획서가 이전 버전 기준이면 매번 수정이 발생한다.
4. **계획 승인** — 사람이 읽고 승인
5. **구현** — 계획서대로, 계획 외 변경 시 계획서부터 수정
6. **테스트** — `cargo test`, `cargo clippy`, `pnpm test` 통과 필수
7. **완료 보고서** — `mydocs/working/task{N}-done.md`
8. **회고** — `/task-retro` 실행: 교훈을 CLAUDE.md·트러블슈팅 문서에 반영, 커밋
9. **PR 및 merge** — `devel` 브랜치로 PR, `closes #{N}`

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
