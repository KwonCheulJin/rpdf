<!-- /autoplan restore point: /Users/kwoncheuljin/.gstack/projects/KwonCheulJin-rpdf/local-task17-autoplan-restore-20260504-153154.md -->
# Task #17: Command/Query 트레이트 정의 및 CommandStack 구현

**이슈**: #32  
**브랜치**: `local/task17`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: v0.2 완료 (Task #16 머지 확인)

---

## 목표

v0.3 편집 레이어의 기반이 되는 CQRS 패턴 타입을 **신규 `rpdf-edit` 크레이트**에 추가한다.  
이 Task에서는 **편집 실행보다 인터페이스 정의**가 핵심이다.  
실제 커맨드(Rotate, Delete 등)는 Task #18부터 이 트레이트를 구현한다.

> **변경**: `/autoplan` 리뷰 결과 `rpdf-core` 불변 규칙(외부 의존성 금지, Copy+Clone+Eq 전용) 위반을 피하기 위해 신규 `rpdf-edit` 크레이트로 분리.

---

## 설계 결정

### 크레이트 위치

**`crates/rpdf-edit` 신규 크레이트 생성** (`rpdf-core`가 아님).

이유:
- `rpdf-core` 불변 규칙: 외부 의존성 추가 금지, 모든 타입 `Copy+Clone+PartialEq+Eq` 필수
- `CommandStack(Vec<Box<dyn Command>>)`은 Copy 불가 → `rpdf-core` 배치 불가
- 아키텍처 문서 의존성 방향: `document_core → model(rpdf-core)`

```
rpdf-edit → rpdf-core (Document, Page 타입)
rpdf-edit 의존: thiserror (workspace)
```

### Command 트레이트

아키텍처 문서(`mydocs/manual/architecture.md`) 시그니처 그대로 따름:

```rust
pub trait Command: Send + Sync {
    fn execute(&self, doc: &mut Document) -> Result<(), CommandError>;
    fn undo(&self, doc: &mut Document) -> Result<(), CommandError>;
    fn name(&self) -> &'static str;
}
```

설계 의도:
- 각 Command 구현체가 undo에 필요한 이전 상태를 **내부 필드**에 직접 저장
- `execute` 호출 시 현재 상태를 캡처해 구조체 필드에 저장 → `undo` 시 복원
- `CommandEffect` 없음 → 타입 소거·역직렬화 오류 경로 없음

### Query 트레이트

```rust
pub trait Query {
    type Output;
    fn execute(&self, doc: &Document) -> Result<Self::Output, CommandError>;
}
```

설계 의도:
- `&Document` (불변) 참조 → 절대 `doc`을 변경하지 않음을 타입으로 보장
- 연관 타입 `Output`으로 반환값 타입 다형성

### CommandStack

```rust
pub struct CommandStack {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
}
```

동작 정책:
| 상황 | 동작 |
|------|------|
| `execute` 호출 시 | 커맨드 실행 → `redo_stack` 비움 → `undo_stack` push |
| `execute` 후 `undo_stack.len() > max_depth` | 가장 오래된 항목 제거 (FIFO) |
| `undo` 호출 시 | `undo_stack` pop → `cmd.undo(doc)` → `redo_stack` push |
| `redo` 호출 시 | `redo_stack` pop → `cmd.execute(doc)` 재실행 → `undo_stack` push |
| `undo_stack`이 비었을 때 `undo` 호출 | `CommandError::NothingToUndo` |
| `redo_stack`이 비었을 때 `redo` 호출 | `CommandError::NothingToRedo` |
| `undo` 실패 시 | cmd를 스택에 되돌림 (실패 원자성 보장) |

### CommandError

```rust
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("실행 중 오류: {0}")]
    ExecutionFailed(String),
    #[error("Undo 중 오류: {0}")]
    UndoFailed(String),
    #[error("Undo할 커맨드 없음")]
    NothingToUndo,
    #[error("Redo할 커맨드 없음")]
    NothingToRedo,
}
```

> `UndoDataDeserializeFailed` 제거: `CommandEffect`/`serde_json` 사용하지 않으므로 dead variant.

---

## 파일 구조

```
crates/rpdf-edit/
├── Cargo.toml          (rpdf-core, thiserror 의존)
└── src/
    ├── lib.rs          (pub mod commands;)
    └── commands/
        ├── mod.rs      (pub use, 모듈 선언)
        ├── traits.rs   (Command, Query 트레이트)
        ├── error.rs    (CommandError)
        └── stack.rs    (CommandStack)
```

루트 `Cargo.toml` workspace members에 `crates/rpdf-edit` 추가.

---

## 테스트 전략

### 단위 테스트 (인라인 `#[cfg(test)]`)

위치: 각 파일 하단 `mod tests {}`

**traits.rs 테스트**:
- 더미 커맨드로 트레이트 객체 동적 디스패치 동작 확인
- `Box<dyn Command>`로 `CommandStack`에 저장 가능한지 컴파일 확인

**stack.rs 테스트**:
- `execute → undo → redo` 라운드트립 (상태 일치 확인)
- `undo` 후 `redo_stack`에 항목 생김
- `execute` 후 기존 `redo_stack` 비워짐
- 빈 스택에서 `undo()` → `NothingToUndo` 에러
- 빈 스택에서 `redo()` → `NothingToRedo` 에러
- `max_depth` 초과 시 오래된 항목 드롭
- **[추가] redo 후 재undo**: `execute → undo → redo → undo` 4단계 라운드트립
- **[추가] undo 실패 원자성**: undo 실패 후 `undo_stack.len()`이 변경 전과 동일
- **[추가] max_depth = 1**: execute 2번 → undo_stack.len() == 1 (첫 항목 드롭)

**테스트용 더미 커맨드** (ToggleMetadataTitleCommand):
```rust
#[cfg(test)]
struct ToggleTitleCommand {
    prev_title: std::cell::Cell<Option<Vec<u8>>>,
}

impl Command for ToggleTitleCommand {
    fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
        let current = doc.metadata.as_ref().and_then(|m| m.title.clone());
        self.prev_title.set(current);
        if let Some(ref mut meta) = doc.metadata {
            meta.title = Some(b"test".to_vec());
        }
        Ok(())
    }
    fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
        if let Some(ref mut meta) = doc.metadata {
            meta.title = self.prev_title.take();
        }
        Ok(())
    }
    fn name(&self) -> &'static str { "toggle_title" }
}
```

> `Document.metadata`는 `Option<DocumentMetadata>` — 실제 구조와 일치하는 더미.  
> `std::cell::Cell<Option<Vec<u8>>>`로 `&self`에서 내부 가변성 허용.

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `max_depth = 0` | 생성 시 `max_depth = 1`로 강제 (최소 1) |
| 연속 `undo` 후 `execute` | `redo_stack` 비워진 상태 확인 |
| `undo` 실패 시 | pop한 cmd를 `undo_stack`에 되돌림 → 실패 원자성 |

---

## 의존성

- `thiserror` — `CommandError` (기존 workspace 의존성, `rpdf-edit/Cargo.toml`에 추가)
- `rpdf-core` — `Document`, `Page` 타입 접근

**`serde_json` 불필요**: `CommandEffect` 제거로 이 Task에서 사용 안 함.

**공개 API 확인**: `thiserror::Error` — docs.rs 확인 완료. 기존 크레이트에서 사용 중.

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `rpdf-edit` 크레이트 생성 + workspace 등록 + CommandError | `cargo build` 통과 |
| CP-2 | Command/Query 트레이트 정의 + 더미 구현 컴파일 확인 | `cargo build` 통과 |
| CP-3 | CommandStack 구현 + 단위 테스트 전체 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `crates/rpdf-edit` 신규 크레이트가 workspace에 등록됨
2. `Command`, `Query` 트레이트가 `rpdf-edit`에 정의됨 (아키텍처 문서 시그니처)
3. `CommandError` 타입 정의됨 (4가지 변형)
4. `CommandStack`의 `execute/undo/redo`가 단위 테스트로 검증됨 (9개 케이스)
5. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 모두 통과
6. 공개 API에 `///` 문서 주석 완비

---

## /autoplan 리뷰 결과

리뷰 일시: 2026-05-04 | 브랜치: local/task17

### 발견 사항 요약

| # | 심각도 | 문제 | 결정 |
|---|--------|------|------|
| 1 | CRITICAL | `rpdf-core`에 CQRS 배치 — 불변 규칙 위반 | **확정: `rpdf-edit` 신규 크레이트** |
| 2 | HIGH | `CommandEffect` + `serde_json` — 아키텍처 문서 불일치 | **확정: 아키텍처 문서 시그니처 따름** |
| 3 | HIGH | `IncrementCommand` 더미 — `Page` 생성자 없어 구현 불가 | 자동 결정: `ToggleTitleCommand` 교체 |
| 4 | MEDIUM | redo 후 재undo, undo 실패 원자성 테스트 누락 | 자동 결정: 계획에 추가 (9개 케이스) |
| 5 | LOW | `max_depth=0` silent coerce → `NonZeroUsize` 권장 | 유지: YAGNI, silent coerce 단순함 우선 |

### 자동 결정 로그

<!-- AUTONOMOUS DECISION LOG -->
## Decision Audit Trail

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|-------|----------|---------------|-----------|----------|---------|
| 1 | CEO | `rpdf-edit` vs `rpdf-commands` 크레이트명 | Mechanical | P5 (명시적) | `rpdf-edit`이 역할 명확 | `rpdf-commands`, `rpdf-document-core` |
| 2 | Eng | 더미 커맨드 교체: `IncrementCommand` → `ToggleTitleCommand` | Mechanical | P5 | Page 생성자 없음으로 구현 불가 | `IncrementCommand` |
| 3 | Eng | 누락 테스트 3개 추가 (redo→undo, 실패 원자성, max_depth=1) | Mechanical | P1 (완전성) | 발견된 갭 전부 커버 | 선택적 추가 |
| 4 | D1 | rpdf-edit 신규 크레이트 생성 | User Decision | — | 사용자 확정 | rpdf-core 배치 |
| 5 | D2 | Command::undo 아키텍처 문서 시그니처 사용 | User Decision | — | 사용자 확정 | CommandEffect 방식 |

### NOT in scope (이 Task에서 제외)

- `Document`의 `dirty` 플래그 및 `original_bytes` 추가 (v0.3-editing-commands.md 계획됨, Task 별도)
- 실제 PDF serialize/roundtrip 검증
- 외부 크레이트(`rpdf-edit` → `rpdf-core` 연결) 구성 설정

### What already exists

| 서브 문제 | 기존 코드 |
|-----------|-----------|
| `Document`, `Page` 타입 | `crates/rpdf-core/src/types/document.rs` |
| `thiserror` workspace dep | `Cargo.toml` |
| `serde_json` workspace dep | `Cargo.toml` (신규 크레이트에서 사용 가능) |

### Architecture ASCII Diagram

```
       BEFORE (계획서 원안)              AFTER (권장)
       
  ┌─────────────────────┐          ┌───────────────────────┐
  │      rpdf-core      │          │      rpdf-edit        │  ← NEW
  │  types/ + commands/ │  →→→→→   │  commands/ (CQRS)     │
  │  (불변 규칙 위반)    │          │  - Command trait      │
  └─────────────────────┘          │  - Query trait        │
                                   │  - CommandEffect      │
                                   │  - CommandError       │
                                   │  - CommandStack       │
                                   └──────────┬────────────┘
                                              │ depends on
                                   ┌──────────▼────────────┐
                                   │      rpdf-core        │
                                   │  types/ (value objs)  │
                                   │  Copy+Clone+Eq 전용   │
                                   └───────────────────────┘
```

### DX Review (Phase 3.5)

이 Task는 내부 라이브러리 크레이트 추가. 외부 사용자(개발자)에게 직접 노출되는 API는 없음.
향후 `rpdf-edit`이 public crate로 공개될 경우 DX 검토 필요. 현 단계에서는 N/A.

TTHW: N/A (내부 추상화)

### 테스트 계획 아티팩트

`~/.gstack/projects/KwonCheulJin-rpdf/local-task17-test-plan-20260504-153544.md`
