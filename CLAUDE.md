# CLAUDE.md

AI 페어 프로그래밍 작업 규칙. AI는 이 규칙을 따르고, 사람은 검토한다.
**이것은 바이브 코딩이 아닙니다.** AI 코드는 반드시 사람이 읽고 승인한 뒤 merge한다.

## 설계 원칙

**KISS**: 가장 단순한 해결책 먼저. 함수는 한 가지 일. 중첩 깊이 3 이하.
**DRY**: 지식 중복 경계. 세 번 등장하면 추출 검토. (두 번까지는 WET 허용)
**YAGNI**: 지금 필요 없는 기능은 만들지 않는다. 마일스톤 범위 밖 코드 금지.
**SOLID**: SRP·OCP·LSP·ISP·DIP — Rust에서는 trait과 모듈 경계로 표현.

### TDD — Test First

테스트를 구현보다 먼저 작성한다. Red → Green → Refactor 사이클을 한 함수 단위로 반복.
- **F.I.R.S.T**: Fast(I/O 없이)·Isolated(공유 상태 금지)·Repeatable·Self-validating·**Timely(구현 직전)**
- 테스트 없이 new feature merge 금지.

### DDD — Domain-Driven Design

PDF 스펙(ISO 32000) 용어를 코드에 그대로 반영한다.
- `xref` → `XrefTable` (❌ `CrossReferenceTable`), `trailer` → `PdfTrailer` (❌ `PdfFooter`)
- `rpdf-core`는 도메인 타입만. 파싱 로직 없음. 값 객체는 `Copy + Clone + PartialEq + Eq`.

### 에러 변형 도달 가능성

에러 변형 추가 시 실제 발생 코드 경로가 있는지 확인한다.
테스트로 발생시킬 수 없는 변형은 dead variant이거나 설계 오류 — 변형을 제거하거나 코드 경로를 수정한다.

### 외부 입력 검증 (silent failure 방지)

외부 입력(파일, 네트워크 등)을 정수로 변환할 때 다음을 의식한다.

- **silent wrap-around·truncation은 보안 취약점**이 될 수 있다.
- "스펙상 정상 범위"라는 이유만으로 검증을 생략하지 않는다 (악성 입력은 비정상 범위를 시도한다).
- 명시적 범위 검증 + 명확한 에러 변형 반환.

> **사례**: PDF xref 스트림 `/W` 배열은 스펙상 각 필드가 8바이트 이내.
> 악성 입력 `[1, 100, 2]`가 들어오면 u64 읽기 시 silent truncation으로 잘못된 엔트리 디코딩.
> → `W[i] > 8` 조건을 명시적으로 거부하고 `XrefStreamInvalidW` 반환.

### 테스트 파일 배치

공개 API 테스트 → `tests/parser/<module>_tests.rs` (별도 파일). 새 모듈 추가 시 `mod.rs`에 등록.
private/`pub(crate)` 함수 테스트 → 인라인 `#[cfg(test)] mod internal_tests {}` 사용.

> **이유**: `tests/` 폴더는 크레이트 외부에서 컴파일되므로 `pub(crate)` 함수에 접근할 수 없다.
> 공개 API만으로 동등한 검증이 가능하면 별도 파일을 우선한다.

### 체크포인트 시점 셀프 리뷰

각 체크포인트(빌드·테스트 통과 직후) 다음으로 넘어가기 전, 명세 위반 시나리오를 능동적으로 탐색한다.

- 계획서 명세를 모두 만족하는가. 경계값·"항상"/"절대" 조건을 의식적으로 검증한다.
- 계획서와 구현의 파일 위치·함수명·시그니처가 일치하는가.
- 정책 일관성이 깨지지 않았는가 (같은 종류 처리가 두 곳에서 다르게 동작하지 않는가).
- **조건 분기(`if let`, `if`, `match guard`)가 실제로 실행되는지 확인 (vacuous pass 방지)**.
- 발견 즉시 수정하거나 체크포인트 보고에 명시한다.

## 도구 활용 원칙

1. 공식 표준 도구 우선 (`cargo new`, `pnpm init`, `create-tauri-app`)
2. 검증된 서드파티 CLI (`gitignore.io`, `wasm-pack`)
3. 위 둘 없을 때만 손으로 작성

**cargo**: `cargo new --lib crates/<name> --vcs none` (`--vcs none` 필수). 의존성 추가는 `cargo add`.
**gitignore**: `curl -L https://www.toptal.com/developers/gitignore/api/rust,node,macos,linux > .gitignore`
**pnpm CI**: `pnpm/action-setup@v4`에 `version:` 키 없이 사용 — `packageManager` 필드 자동 인식.
  `version:` 추가 시 **ERR_PNPM_BAD_PM_VERSION** 충돌 → `mydocs/troubleshootings/pnpm-action-setup-version-conflict.md`
**외부 크레이트**: docs.rs에서 공개 API 확인 완료 후 사용. 계획서에 "공개 API 확인 완료" 명시.

손으로 만들 것: 워크스페이스 `Cargo.toml`, `pnpm-workspace.yaml`, CI yml, `CLAUDE.md`, `mydocs/`

## Rust 개발 도구

필수: `cargo-edit`(cargo add/upgrade), `cargo-nextest`(빠른 테스트), `cargo-watch`(파일 감지)
권장: `cargo-workspaces`(v0.4), `cargo-deny`(보안 감사), `wasm-pack`(v0.4), `tauri-cli`(v0.5)
새 도구 기준: 최근 6개월 유지보수·stars 1k+·명확한 가치·호환 라이선스.
도입 근거 기록: `mydocs/tech/dev-tool-{도구명}.md`

## 작업 시작 전 체크리스트

- [ ] 이 작업에 표준 CLI 도구가 있는가?
- [ ] 손으로 만들어야 한다면 그 이유가 명확한가?
- [ ] 외부 크레이트를 새로 도입한다면 docs.rs에서 공개 API 확인 완료?
- [ ] 새 에러 변형을 추가한다면 그 에러를 발생시키는 테스트가 있는가?
- [ ] **마이그레이션·리팩토링 범위 확정 전, 대상 모듈을 import하는 모든 파일을 grep으로 파악했는가?**

## 작업 프로토콜

1. **Issue 생성** — `gh issue create`, 마일스톤 지정
2. **브랜치 생성** — `local/task{N}`
3. **계획서 작성** — `mydocs/plans/task{N}-{slug}.md` (데이터 모델·API·엣지 케이스·테스트 전략)
   - ⚠️ 버전은 **실제 설치된 버전** 기재 (최소 요구사항 아님)
4. **계획 승인** — 사람이 읽고 승인
5. **구현** — 계획서대로. 계획 외 변경 시 계획서 먼저 수정.
6. **테스트** — `cargo test`, `cargo clippy`, `pnpm test` 통과 필수
7. **완료 보고서** — `mydocs/working/task{N}-done.md`
8. **회고** — `/task-retro` 실행
9. **PR** — `devel` 브랜치로, `closes #{N}`

## 금지 사항

- 계획서 없이 구현 시작
- 테스트 없이 new feature 추가
- 아키텍처 결정 독자 결정
- `unsafe` 블록 사람 확인 없이 추가
- 외부 크레이트 사람 승인 없이 추가
- 마일스톤 범위 밖 기능 "겸사겸사" 구현

## 커밋 메시지 규칙

```
Task #{N}: 한 줄 요약

상세 설명 (선택): 변경 파일의 의미, 왜 이 방식인지

closes #{N}
```

## 품질 관문

- `cargo test` — 전체 통과
- `cargo clippy -- -D warnings` — 경고 없음
- `cargo fmt --check` — 포맷 정리됨
- 웹/데스크톱: `pnpm test`, `pnpm lint`, `pnpm typecheck`

## 문서화 규칙

- 공개 API: `///` 문서 주석 필수
- 복잡한 로직: WHY를 주석으로 (WHAT은 코드 자체가 설명)
- 새 기능: `mydocs/tech/` / 버그 수정: `mydocs/troubleshootings/`

## 디버깅 프로토콜

1. `rpdf info <file>` — 메타데이터 확인
2. `rpdf dump <file> -p <page>` — 페이지 IR 덤프
3. `rpdf export-svg <file> --debug-overlay` — 시각적 디버그
4. 재현 케이스 → `tests/regression/`에 추가
5. 수정 후 동일 케이스 통과 확인

## 파일 명명 규칙

- Rust: `snake_case.rs` / TypeScript: `kebab-case.ts`, `PascalCase.tsx` (컴포넌트)
- 문서: `kebab-case.md` / 브랜치: `local/task{N}` 또는 `feature/{slug}`

## 참고

- 개발 방법론: `mydocs/manual/hyper-waterfall.md`
- 아키텍처: `mydocs/manual/architecture.md`
- 온보딩: `mydocs/manual/onboarding.md`
