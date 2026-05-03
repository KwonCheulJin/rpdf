# Rust 개발 도구 설치 가이드

rpdf 개발에 사용하는 Rust 도구 목록입니다.
새 개발 환경을 세팅할 때 이 문서를 기준으로 설치합니다.

## 필수 도구 (초기 세팅 시 1회 설치)

### cargo-edit

의존성 추가·업데이트·삭제를 CLI로 처리합니다. `Cargo.toml`을 직접 편집하는 대신 사용합니다.

```bash
cargo install cargo-edit
```

**주요 명령**

```bash
# 최신 안정 버전 추가
cargo add lopdf

# feature 포함 추가
cargo add tracing --features=log

# workspace.dependencies에 추가
cargo add serde --workspace

# 버전 지정 추가
cargo add thiserror@2

# 의존성 제거
cargo rm lopdf

# 설치된 의존성 최신 버전으로 업그레이드
cargo upgrade
```

**왜 사용하는가**: `cargo add`는 crates.io에서 최신 버전을 자동으로 조회해 `Cargo.toml`에 기록합니다.
버전을 수동으로 찾아 입력하는 과정을 제거하고, workspace.dependencies 상속도 올바르게 처리합니다.

---

### cargo-nextest

표준 `cargo test`보다 빠른 테스트 러너입니다. 테스트를 프로세스 단위로 격리해 실행합니다.

```bash
cargo install cargo-nextest --locked
```

**주요 명령**

```bash
# 전체 테스트 실행
cargo nextest run

# 특정 모듈만 실행
cargo nextest run parser::

# 특정 테스트명 패턴
cargo nextest run version_is_set

# 실패 시 즉시 중단
cargo nextest run --fail-fast

# 상세 출력
cargo nextest run --no-capture
```

**왜 사용하는가**: 테스트 수가 늘어날수록 표준 `cargo test` 대비 2–3배 빠릅니다.
실패한 테스트만 재실행하는 `--retries` 기능과 CI 친화적인 JUnit XML 출력도 지원합니다.

> CI에서는 표준 `cargo test`를 유지합니다 (`cargo-nextest`가 설치되지 않은 환경 대비).
> 로컬 개발 시 `cargo nextest run`을 권장합니다.

---

### cargo-watch

파일 변경을 감지해 지정한 cargo 명령을 자동으로 재실행합니다.

```bash
cargo install cargo-watch
```

**주요 명령**

```bash
# 변경 시 자동 테스트
cargo watch -x test

# 변경 시 자동 clippy
cargo watch -x 'clippy -- -D warnings'

# 변경 시 빌드 후 테스트 순차 실행
cargo watch -x build -x test

# 특정 디렉터리만 감시
cargo watch -w src -x test

# 처음 한 번 즉시 실행 후 감시
cargo watch -x test --why
```

**왜 사용하는가**: 코드 수정 후 터미널로 전환해 `cargo test`를 수동으로 치는 반복을 제거합니다.
빠른 피드백 루프로 TDD 사이클을 지원합니다.

---

## 권장 도구 (필요 시점에 설치)

아래 도구는 해당 마일스톤 착수 시점에 설치합니다. 미리 설치하면 충돌 위험이 있으므로
시점이 되면 이 문서를 업데이트합니다.

| 도구 | 설치 시점 | 명령 |
| --- | --- | --- |
| `cargo-workspaces` | v0.4 (WASM) | `cargo install cargo-workspaces` |
| `cargo-deny` | 오픈소스 공개 시 | `cargo install cargo-deny` |
| `wasm-pack` | v0.4 (WASM) | `cargo install wasm-pack` |
| `tauri-cli` | v0.5 (Desktop) | `cargo install tauri-cli --version "^2.0"` |

---

## 전체 필수 도구 한 번에 설치

새 환경 세팅 시:

```bash
cargo install cargo-edit cargo-watch
cargo install cargo-nextest --locked
```

---

## 버전 확인

```bash
cargo add --version
cargo nextest --version
cargo watch --version
```

---

## 참고

- cargo-edit: <https://github.com/killercup/cargo-edit>
- cargo-nextest: <https://nexte.st>
- cargo-watch: <https://github.com/watchexec/cargo-watch>
