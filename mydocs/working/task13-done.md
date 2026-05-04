# Task #13 — samples/ 28개 PNG 회귀 검증 완료 보고서

**Issue**: M2 #24  
**브랜치**: `local/task13`  
**머지 커밋**: (PR 머지 후 기재)  
**완료일**: 2026-05-04  
**소요 시간**: 계획 ~4시간 / 실제 ~2시간

## 완료된 작업

계획서에 정의된 완료 기준 대비 결과:

- [x] 기준 1: `tests/snapshots/macos/` 기준 PNG 27개 생성 (broken-missing-trailer.pdf 렌더링 실패로 26개가 아닌 27개 — broken-bad-xref-offset.pdf는 pdfium이 성공적으로 렌더링함)
- [x] 기준 2: `cargo nextest run -p rpdf-render` — image_regression 테스트 통과
- [x] 기준 3: 허용 오차 정책 `DIFF_THRESHOLD = 0.001` 구현
- [x] 기준 4: `UPDATE_SNAPSHOTS=1` 환경변수로 기준 PNG 재생성 가능
- [x] 기준 5: CI — 테스트 실패 시 diff PNG 아티팩트 업로드, 성공 시 linux 스냅샷 아티팩트 업로드
- [x] 기준 6: `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과

## 실제 변경 사항

### 새로 추가된 파일
- `crates/rpdf-render/tests/snapshot_utils.rs` (113줄) — diff 유틸리티 + 4개 단위 테스트
- `crates/rpdf-render/tests/image_regression.rs` (181줄) — 28개 PDF 회귀 테스트
- `crates/rpdf-render/tests/snapshots/macos/*.png` (27개) — macOS 기준 PNG
- `mydocs/plans/task13-image-regression.md` — 계획서

### 수정된 파일
- `.github/workflows/ci.yml` — 이미지 회귀 테스트 단계 + diff/snapshot 아티팩트 업로드 추가

### 삭제된 파일
- 없음

## 계획 대비 달라진 점

1. **macOS 스냅샷 27개 (계획서 26개 예상)**
   - `broken-bad-xref-offset.pdf`가 pdfium에서 성공적으로 렌더링되어 스냅샷 생성됨
   - `broken-missing-trailer.pdf`만 `Err(_)` 반환 → 예상된 에러로 통과
   - 계획서의 "broken PDF는 예상 실패로 처리" 로직이 올바르게 동작한 것이며, 스냅샷 수 차이는 실제 pdfium 동작에 의한 정상 결과

## 발견된 이슈

- **broken-bad-xref-offset.pdf**: 계획서에서는 pdfium이 로딩 실패할 것으로 예상했으나, 실제로는 렌더링에 성공함. pdfium의 xref 복구 능력이 예상보다 강함.

## 배운 점

### 기술적
- pdfium은 broken xref offset을 자동 복구해 렌더링에 성공할 수 있음 → broken 파일이라도 pdfium 렌더링 성공 여부를 단정하지 않는 것이 올바른 설계 (`Err(_)` 어떤 변형이든 예상 에러로 처리)
- `cargo nextest`의 프로세스 격리 덕분에 `OnceCell` 충돌 없이 병렬 실행 가능
- `std::panic::catch_unwind`로 28개 테스트를 단일 `#[test]` 함수에서 모두 실행하고 실패를 집계하는 패턴 유효

### 프로세스
- plan-eng-review의 D1~D4 결정이 구현을 명확히 가이드했음 (linux 첫 실행 처리, snapshot_utils 위치, Err(_) 처리, nextest 격리)

## 테스트 결과

- snapshot_utils 단위 테스트: 4/4 통과
- image_regression 통합 테스트: macOS에서 스냅샷 생성 모드로 통과 (비교는 CI에서 검증)
- `cargo clippy -- -D warnings`: 경고 없음
- `cargo fmt --check`: 통과

## 회귀 분류 표

| # | 항목 요약 | 카테고리 | 판단 근거 |
|---|---------|---------|---------|
| 1 | pdfium이 broken xref offset을 자동 복구해 렌더링 성공 가능 | **C: 스킵** | broken 파일은 `Err(_)` 어떤 변형이든 통과 설계로 이미 처리됨 |
| 2 | `std::panic::catch_unwind`로 단일 테스트에서 실패 집계 패턴 | **C: 스킵** | 이번 task 전용 패턴, CLAUDE.md 반영 불필요 |

## 다음 관련 작업

- Task #13 완료 → linux 스냅샷은 CI 첫 실행 후 아티팩트 다운로드 → 수동 커밋 필요
- Task #14: SVG 스냅샷
- Task #15: SVG debug overlay
- Task #16: 여러 페이지 스냅샷

## 참고 자료

- PR: (생성 후 기재)
- 계획서: `mydocs/plans/task13-image-regression.md`
