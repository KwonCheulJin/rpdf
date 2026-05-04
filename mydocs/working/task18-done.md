# Task #18 완료 보고서: RotatePageCommand 구현

**이슈**: #34  
**브랜치**: `local/task18`  
**마일스톤**: v0.3 — 편집 커맨드  
**완료일**: 2026-05-04

---

## 완료 체크리스트

- [x] `crates/rpdf-edit/src/commands/rotate.rs` 신규 파일 생성
- [x] `mod.rs`에 `mod rotate` + `pub use rotate::RotatePageCommand` 추가
- [x] `RotatePageCommand::new(page_index, degrees)` 공개 생성자 + `///` 문서 주석 + `# Examples` doctest
- [x] `execute` 구현: 경계 검증, `prev_rotation` 저장, `rem_euclid(360)` 정규화
- [x] `undo` 구현: `None` 감지 → `UndoFailed("undo called before execute")`
- [x] 11개 단위 테스트 + 1개 doctest = 총 12개 통과
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` 전체 통과

---

## 구현 내용

### 파일 구조 변경

```
crates/rpdf-edit/src/commands/
├── mod.rs      (rotate 모듈 등록 + pub use 추가)
├── error.rs    (변경 없음)
├── rotate.rs   ← 신규
├── stack.rs    (변경 없음)
└── traits.rs   (변경 없음)
```

### 주요 설계 결정

1. **`Mutex<Option<i32>>`**: `Command: Send + Sync` 경계 충족. `None` 센티넬로 execute 없이 undo 호출 시 `UndoFailed("undo called before execute")` 반환.

2. **`rem_euclid(360)`**: Rust의 음수 나머지를 올바르게 처리. `-90.rem_euclid(360) = 270`. 부호에 무관하게 0–359 범위로 정규화.

3. **execute-time 유효성 검증**: 생성자 시점이 아닌 execute 시점에 `degrees % 90 != 0` 및 page bounds 검증 — Command 패턴의 관용적 방식.

4. **에러 메시지에 현재 상태 포함**: `"page index out of bounds: 5 (document has 1 pages)"`, `"degrees must be a multiple of 90, got 45; valid: 90, 180, 270, -90, ..."`

---

## 품질 게이트

| 명령 | 결과 |
|------|------|
| `cargo test -p rpdf-edit` | 20/20 + doctest 1 통과 |
| `cargo clippy -p rpdf-edit -- -D warnings` | 경고 없음 |
| `cargo fmt --check` | 통과 |

---

## 테스트 커버리지

| # | 테스트명 | 검증 |
|---|---------|------|
| 1 | `rotate_90_forward` | 0° + 90° = 90° |
| 2 | `rotate_180` | 90° + 180° = 270° |
| 3 | `rotate_wraps_at_360` | 270° + 90° = 0° (mod 360) |
| 4 | `rotate_negative_degrees` | 0° + (-90°) = 270° (rem_euclid) |
| 5 | `undo_restores_original` | execute → undo → 원래 값 복원 |
| 6 | `execute_undo_redo_via_stack` | CommandStack 라운드트립 |
| 7 | `invalid_degrees_not_multiple_of_90` | 45° → ExecutionFailed |
| 8 | `page_index_out_of_bounds` | 존재하지 않는 페이지 → ExecutionFailed |
| 9 | `zero_degrees_is_noop` | 0° → 변경 없음 |
| 10 | `rotate_720_is_noop` | 270° + 720° = 270° |
| 11 | `undo_before_execute_fails` | execute 없이 undo → UndoFailed |
| doc | `RotatePageCommand::new` doctest | 컴파일·실행 성공 |

---

## 회고 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | `Mutex<Option<T>>`로 None 센티넬 패턴 — execute 선행 여부 추적 | C: 완료 보고서 메모 | CLAUDE.md `## Task #17` 완료 보고에서 Mutex 패턴 이미 기록됨 |
| 2 | `rem_euclid` vs `%` — 음수 처리 차이 | C: 스킵 | Rust 표준 라이브러리 관용구, 별도 규칙 불필요 |
| 3 | Page 구조체 직접 조회 필수 — 가정 금지 | C: 스킵 | CLAUDE.md 작업 시작 전 체크리스트에 fixture 확인 항목 포함 |

모든 항목이 C(스킵) — CLAUDE.md 추가나 트러블슈팅 문서 생성 없이 완료.

---

## 다음 작업

Task #19: `DeletePagesCommand` 구현 — RotatePageCommand와 동일한 `Command` 트레이트 패턴.
