# Task #17 완료 보고서: Command/Query 트레이트 정의 및 CommandStack 구현

**이슈**: #32  
**브랜치**: `local/task17`  
**마일스톤**: v0.3 — 편집 커맨드  
**완료일**: 2026-05-04

---

## 완료 체크리스트

- [x] `crates/rpdf-edit` 신규 크레이트 workspace 등록
- [x] `Command`, `Query` 트레이트 정의 (아키텍처 문서 시그니처)
- [x] `CommandError` 4가지 변형 정의
- [x] `CommandStack` execute/undo/redo 구현 + undo 실패 원자성
- [x] 9개 단위 테스트 통과
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 전체 통과

---

## 구현 내용

### 파일 구조

```
crates/rpdf-edit/
├── Cargo.toml              (rpdf-core, thiserror 의존)
└── src/
    ├── lib.rs              (pub mod commands;)
    └── commands/
        ├── mod.rs          (pub use 재내보내기)
        ├── traits.rs       (Command, Query 트레이트)
        ├── error.rs        (CommandError 4변형)
        └── stack.rs        (CommandStack + 9개 인라인 테스트)
```

### 주요 설계 결정

1. **rpdf-core가 아닌 rpdf-edit 신규 크레이트**: `CommandStack(Vec<Box<dyn Command>>)`은 Copy 불가 → rpdf-core 불변 규칙 위반. 별도 크레이트로 분리.

2. **CommandEffect 없음**: 아키텍처 문서 시그니처 준수. `undo(&self, doc)` 직접 호출 방식. 타입 소거·역직렬화 오류 경로 없음.

3. **undo 실패 원자성**: `cmd.undo(doc)` 실패 시 pop한 cmd를 다시 `undo_stack.push(cmd)` 복원 → 스택 상태 불변 보장.

4. **ToggleTitleCommand 더미**: `Cell` 대신 `Mutex` 사용. `Command: Sync` 경계 충족 요건. `Cell<T>: !Sync`이므로 `Box<dyn Command>` 트레이트 객체로 사용 불가.

---

## 품질 게이트

| 명령 | 결과 |
|------|------|
| `cargo test -p rpdf-edit` | 9/9 통과 |
| `cargo clippy -p rpdf-edit -- -D warnings` | 경고 없음 |
| `cargo fmt --check` | 통과 |
| `cargo build` (전체 워크스페이스) | 성공 |

---

## 회고 분류

### A — 즉시 CLAUDE.md 반영

- **`Cell<T>: !Sync` — `Box<dyn Trait: Sync>` 더미 구현 시 `Mutex` 사용**: `Cell`은 `Sync`를 구현하지 않으므로 `Send + Sync` 경계가 있는 트레이트 객체(`Box<dyn Command>`)의 테스트 더미로 사용 불가. `Mutex<T>`로 대체해야 컴파일 통과.

### B — 트러블슈팅 문서

- 없음

### C — 완료 보고서 메모

- `max_depth=0` 강제 정책: silent coerce(1로 강제) vs `NonZeroUsize` 타입 논쟁. YAGNI 원칙으로 현 단계에선 silent coerce 유지. 추후 공개 API로 노출 시 재검토.

---

## 다음 작업

Task #18: 실제 커맨드 구현 (RotatePageCommand, DeletePagesCommand 등) — 이 Task에서 정의한 `Command` 트레이트 구현 시작.
