<!-- /autoplan restore point: /Users/kwoncheuljin/.gstack/projects/KwonCheulJin-rpdf/local-task18-autoplan-restore-20260504-160020.md -->
# Task #18: RotatePageCommand 구현

**이슈**: #34  
**브랜치**: `local/task18`  
**마일스톤**: v0.3 — 편집 커맨드  
**선행 조건**: Task #17 완료 (`Command` 트레이트, `CommandStack`)

---

## 목표

`rpdf-edit` 크레이트에 첫 번째 실제 `Command` 구현체인 `RotatePageCommand`를 추가한다.  
PDF 페이지의 `/Rotate` 값을 변경하고 Undo 대칭성을 보장한다.

---

## 설계 결정

### 위치

`crates/rpdf-edit/src/commands/rotate.rs` 신규 파일.  
`mod.rs`에서 `pub mod rotate; pub use rotate::RotatePageCommand;` 추가.

### RotatePageCommand 구조체

```rust
pub struct RotatePageCommand {
    page_index: usize,
    degrees: i32,
    prev_rotation: Mutex<Option<i32>>,
}
```

- `page_index`: 0-based 페이지 인덱스
- `degrees`: 상대 회전값. 90의 배수 (양수: 시계방향, 음수: 반시계방향). 0은 no-op.
- `prev_rotation`: execute 시 현재 rotation을 캡처 → undo 시 복원. `Mutex<Option<i32>>`로 `Command: Sync` 경계 충족. `None`이면 execute 없이 undo 호출 → `UndoFailed` 반환 (CLAUDE.md Mutex 패턴).

**생성자:**
```rust
pub fn new(page_index: usize, degrees: i32) -> Self {
    Self { page_index, degrees, prev_rotation: Mutex::new(None) }
}
```

### execute 로직

```rust
fn execute(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 1. 범위 검증: page_index >= doc.pages.len() → ExecutionFailed("page index out of bounds: {page_index} (document has {len} pages)")
    // 2. degrees % 90 != 0 → ExecutionFailed("degrees must be a multiple of 90, got {degrees}, valid: 90, 180, 270, -90 ...")
    // 3. prev_rotation에 현재 rotation 저장: *self.prev_rotation.lock().unwrap() = Some(current)
    // 4. 새 rotation = (current + degrees).rem_euclid(360)
    // 5. doc.pages[page_index].rotation = new_rotation
}
```

### undo 로직

```rust
fn undo(&self, doc: &mut Document) -> Result<(), CommandError> {
    // 1. 범위 검증: page_index >= doc.pages.len() → UndoFailed("page index out of bounds during undo: {page_index}")
    // 2. prev = self.prev_rotation.lock().unwrap().ok_or_else(|| UndoFailed("undo called before execute"))?
    // 3. doc.pages[page_index].rotation = prev
}
```

### 각도 정규화

PDF 스펙(ISO 32000) `/Rotate`는 0, 90, 180, 270만 유효하다.  
`rem_euclid(360)`으로 음수 각도도 올바르게 정규화:
- `-90 + 90 = 0` (정상)
- `270 + 90 = 360 → rem_euclid(360) = 0` (정상)
- `-90 → rem_euclid(360) = 270` (undo 시 음수 입력 방지)

### 에러 처리

| 조건 | 에러 변형 |
|------|---------|
| `page_index >= doc.pages.len()` | `CommandError::ExecutionFailed("page index out of bounds: {}")` |
| `degrees % 90 != 0` | `CommandError::ExecutionFailed("degrees must be a multiple of 90, got {}")` |
| undo 시 page_index out of bounds | `CommandError::UndoFailed("page index out of bounds during undo: {}")` |
| execute 없이 undo 호출 | `CommandError::UndoFailed("undo called before execute")` |

> undo 시 범위 오류는 이론상 불가능하지만 (execute가 먼저 성공했으므로), 방어적으로 처리한다.
> execute 없이 undo를 직접 호출하는 경우는 CommandStack 밖의 비정상 사용이지만, `None`으로 감지해 명시적 에러를 반환한다.

---

## 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (rotate 모듈 추가)
├── error.rs    (변경 없음)
├── rotate.rs   ← 신규
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

`mod.rs` 변경:
```rust
mod error;
mod rotate;   // 신규
mod stack;
mod traits;

pub use error::CommandError;
pub use rotate::RotatePageCommand;  // 신규
pub use stack::CommandStack;
pub use traits::{Command, Query};
```

---

## 테스트 전략

위치: `rotate.rs` 하단 `#[cfg(test)] mod tests {}`

**테스트 헬퍼**: `make_doc(pages: usize, rotations: &[i32]) -> Document`

| # | 테스트명 | 검증 내용 |
|---|---------|---------|
| 1 | `rotate_90_forward` | 0° → +90° = 90° |
| 2 | `rotate_180` | 90° → +180° = 270° |
| 3 | `rotate_wraps_at_360` | 270° → +90° = 0° (mod 360) |
| 4 | `rotate_negative_degrees` | 0° → -90° = 270° (rem_euclid) |
| 5 | `undo_restores_original` | execute → undo → rotation 복원 |
| 6 | `execute_undo_redo_via_stack` | CommandStack 통해 execute → undo → redo 라운드트립 |
| 7 | `invalid_degrees_not_multiple_of_90` | 45° → `ExecutionFailed` |
| 8 | `page_index_out_of_bounds` | 존재하지 않는 페이지 → `ExecutionFailed` |
| 9 | `zero_degrees_is_noop` | 0° 회전 → rotation 변경 없음, undo 후에도 동일 |
| 10 | `rotate_720_is_noop` | 270° → +720° = 270° (rem_euclid: 720%360=0) |
| 11 | `undo_before_execute_fails` | execute 없이 undo → `UndoFailed("undo called before execute")` |

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| `degrees = 0` | execute 성공, rotation 변경 없음. undo도 no-op |
| `degrees = 360` | `360 % 90 == 0` 통과, `rem_euclid(360) = 0` → 원래 값 유지 |
| `degrees = -360` | 동일하게 no-op |
| 빈 문서 (pages 0개) | `page_index = 0` → `ExecutionFailed` |
| 다중 undo (execute 없이) | CommandStack이 `NothingToUndo` 반환 (Stack 책임) |

---

## 의존성

- `rpdf-core::types::document::Document` — 기존 (변경 없음)
- `rpdf-edit::commands::traits::Command` — 기존
- `rpdf-edit::commands::error::CommandError` — 기존
- `std::sync::Mutex` — 표준 라이브러리

신규 외부 의존성 없음.

---

## 체크포인트

| 체크포인트 | 내용 | 완료 조건 |
|-----------|------|-----------|
| CP-1 | `rotate.rs` 파일 생성 + `mod.rs` 등록 | `cargo build -p rpdf-edit` 통과 |
| CP-2 | `execute` + `undo` 구현 | `cargo build -p rpdf-edit` 통과 |
| CP-3 | 11개 단위 테스트 | `cargo test -p rpdf-edit` 통과 |
| CP-4 | 전체 품질 게이트 | `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` |

---

## 완료 기준

1. `RotatePageCommand` 구현체가 `rpdf-edit::commands` 공개 API로 노출됨
2. execute/undo 대칭성 보장 (`CommandStack` 통해 라운드트립 테스트)
3. 각도 정규화 (`rem_euclid(360)`) 및 유효성 검증 (90의 배수)
4. 10개 단위 테스트 통과 (degrees=720 케이스 추가)
5. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 통과
6. 공개 API `///` 문서 주석 완비 (상대값 명시 + Example 섹션 포함)
7. `pub fn new(page_index: usize, degrees: i32) -> Self` 생성자 노출
8. 에러 메시지에 현재 상태 포함 (`has {len} pages`, valid values hint)

---

<!-- AUTONOMOUS DECISION LOG -->
## Decision Audit Trail

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|-------|----------|----------------|-----------|-----------|----------|
| 1 | Pre-review | HOLD SCOPE | Mechanical | P3 | Feature enhancement on existing infrastructure | EXPANSION |
| 2 | CEO-0C | Approach A (AtomicI32) | Mechanical | P3,P5 | Correct for single-writer pattern | Approach B (Mutex) |
| 3 | CEO-Gemini | AtomicI32 anti-pattern → TASTE | Taste | P4 | Task #17 design locked; Gemini overreaches | Accepted Gemini |
| 4 | CEO-Gemini | page_index instability → DEFER | Mechanical | P3 | YAGNI; DeletePages task handles this | Added now |
| 5 | CEO-both | degrees relative vs absolute → TASTE | Taste | P5 | Relative is more natural UX | Absolute API |
| 6 | CEO-Claude | PageRotation enum → DEFER | Mechanical | P3 | rpdf-core scope change; out of blast radius | Added now |
| 7 | CEO | Premise gate auto-pass | Mechanical | P6 | Premise reasonable and clear | — |
| 8 | Eng-both | name() → ADD to plan | Mechanical | P1 | Missing required trait method | Omit |
| 9 | Eng-both | Mutex<Option<i32>> → USER CHALLENGE | UserChallenge | — | Both models flag; CommandStack guarantees or not? | — |
| 10 | Eng-Gemini | double-execute false positive | Mechanical | P3 | Stack undo restores state before redo | Flag as gap |
| 11 | Eng-Claude | constructor validation → TASTE | Taste | P5 | execute-time validation is idiomatic in Command pattern | Constructor |
| 12 | Eng-Claude | degrees=720 test → ADD | Mechanical | P1 | Edge case of rem_euclid behavior | Omit |
| 13 | DX | DX POLISH mode | Mechanical | P3 | HOLD SCOPE equivalent | EXPANSION |
| 14 | DX | pub fn new() constructor → ADD | Mechanical | P1 | Struct fields must not be exposed directly | Omit |
| 15 | DX | degrees doc comment → ADD | Mechanical | P5 | Relative vs absolute ambiguity | Omit |
| 16 | DX | error message improvement → ADD | Mechanical | P1 | Error = problem + cause + fix | Current msgs |
| 17 | DX | Examples in doc → ADD | Mechanical | P1 | docs.rs discoverability | Omit |
