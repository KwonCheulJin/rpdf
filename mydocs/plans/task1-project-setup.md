# Task #1 — 프로젝트 초기 세팅

**Issue**: M010 #1
**브랜치**: `local/task1`
**작성일**: 2026-04-24
**상태**: 계획

## 목적

rpdf 프로젝트의 뼈대를 만든다. 이후 모든 타스크는 이 구조 위에 올라가므로, 디렉터리·툴·CI를 이 단계에서 확정한다.

## 배경

- rhwp의 디렉터리 구조를 참고
- pnpm workspace + Cargo workspace 결합
- 향후 `packages/ui`, `packages/studio`, `packages/desktop`, `crates/rpdf-core`로 확장

## 완료 기준

1. `cargo build`, `cargo test`, `pnpm install`이 모두 성공
2. `cargo clippy -- -D warnings` 통과
3. GitHub Actions CI가 PR에서 자동 실행
4. `mydocs/` 디렉터리와 매뉴얼 문서 배치 완료
5. 최소한의 README로 프로젝트 의도가 전달됨

## 작업 항목

### 1. 디렉터리 구조 생성

```
rpdf/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── crates/
│   └── rpdf-core/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── packages/
│   ├── ui/           # (v0.4에서 채움)
│   ├── studio/       # (v0.4에서 채움)
│   └── desktop/      # (v0.5에서 채움)
├── npm/              # (v0.4에서 채움)
├── examples/         # 샘플 PDF
├── tests/            # 통합 테스트
├── samples/          # 회귀 테스트 샘플
├── scripts/
├── mydocs/
│   ├── orders/
│   ├── plans/
│   ├── working/
│   ├── report/
│   ├── feedback/
│   ├── tech/
│   ├── manual/
│   └── troubleshootings/
├── Cargo.toml
├── package.json
├── pnpm-workspace.yaml
├── .gitignore
├── .tool-versions
├── README.md
├── CLAUDE.md
└── CONTRIBUTING.md
```

### 2. Cargo Workspace 설정

**루트 `Cargo.toml`**:
```toml
[workspace]
resolver = "2"
members = [
    "crates/rpdf-core",
]

[workspace.package]
edition = "2021"
rust-version = "1.75"
authors = ["KwonCheulJin"]
license = "MIT"
repository = "https://github.com/KwonCheulJin/rpdf"

[workspace.dependencies]
thiserror = "1"
anyhow = "1"
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
```

**`crates/rpdf-core/Cargo.toml`**:
```toml
[package]
name = "rpdf-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
thiserror.workspace = true
tracing.workspace = true
serde.workspace = true
```

**`crates/rpdf-core/src/lib.rs`** (placeholder):
```rust
//! rpdf-core: Rust PDF 편집기 코어 라이브러리

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!crate::version().is_empty());
    }
}
```

### 3. pnpm Workspace 설정

**루트 `package.json`**:
```json
{
  "name": "rpdf",
  "private": true,
  "scripts": {
    "lint": "pnpm -r lint",
    "test": "pnpm -r test",
    "typecheck": "pnpm -r typecheck"
  },
  "devDependencies": {
    "@types/node": "^20",
    "typescript": "^5.4",
    "prettier": "^3"
  }
}
```

**`pnpm-workspace.yaml`**:
```yaml
packages:
  - "packages/*"
  - "npm/*"
```

### 4. CI 파이프라인

**`.github/workflows/ci.yml`**:
```yaml
name: CI
on:
  push:
    branches: [main, devel]
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all

  node:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      # 패키지가 생기면 추가
```

### 5. 툴 버전 고정

**`.tool-versions`**:
```
rust 1.75.0
nodejs 20.11.0
pnpm 9.0.0
```

### 6. Git 설정

**`.gitignore`**:
```
# Rust
target/
Cargo.lock   # 바이너리 크레이트이므로 커밋
**/*.rs.bk

# Node
node_modules/
dist/
*.log

# OS
.DS_Store
Thumbs.db

# IDE
.idea/
.vscode/
*.iml

# 환경
.env
.env.local

# 빌드 산출물
*.pdb
*.dll
*.dylib
*.so

# WASM
pkg/

# Tauri
src-tauri/target/
```

### 7. 브랜치 전략 설정

- `main`: 릴리즈 태그 전용, 직접 푸시 금지
- `devel`: 개발 통합, 모든 task 브랜치가 merge되는 곳
- `local/task{N}`: 개별 타스크 작업 브랜치

GitHub 브랜치 보호 규칙:
- `main`: PR 필수, CI 통과 필수, 직접 푸시 금지
- `devel`: CI 통과 필수

### 8. 매뉴얼 문서 배치

이 문서 작성 시점에 이미 만들어둔 다음 문서들을 `mydocs/manual/`에 배치:

- `onboarding.md`
- `architecture.md`
- `hyper-waterfall.md`

기술 노트는 `mydocs/tech/`에:
- `pdf-spec-summary.md`
- `crate-decisions.md`

## 엣지 케이스

### 이미 Cargo.lock이 생성된 상태라면
기본값으로 `.gitignore`에 `Cargo.lock`을 넣었지만, **바이너리 프로젝트라면 커밋하는 것이 표준**. 주석 처리만 하고 커밋. 라이브러리 크레이트였다면 제외.

### pnpm workspace가 루트에 packages/*가 없다고 에러를 내면
`pnpm-workspace.yaml`의 패턴이 매칭되는 디렉터리가 있어야 함. 빈 디렉터리라도 placeholder 파일을 둔다 (`packages/.gitkeep`).

## 테스트 계획

이 타스크는 인프라 세팅이므로 전통적 의미의 테스트보다 **체크리스트**로 검증:

- [ ] `cargo build` 성공
- [ ] `cargo test` 성공 (1개 placeholder 테스트 통과)
- [ ] `cargo clippy` 경고 없음
- [ ] `cargo fmt --check` 통과
- [ ] `pnpm install` 성공
- [ ] GitHub push 후 CI 녹색
- [ ] README 렌더링 확인

## 예상 소요 시간

총 4시간

- 디렉터리/파일 생성: 1시간
- CI 파이프라인: 1시간
- 문서 배치 및 README: 1시간
- 실제 테스트 및 수정: 1시간

## 완료 후 산출물

- `mydocs/working/task1-done.md` 완료 보고서
- GitHub 리포지토리 공개 (private 가능)
- 다음 타스크 (#2) 계획서 착수 가능 상태
