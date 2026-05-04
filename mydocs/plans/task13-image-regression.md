# Task #13 계획서: samples/ 28개 PNG 회귀 검증

**Issue**: #24  
**브랜치**: local/task13  
**날짜**: 2026-05-04  
**선행 조건**: Task #12 완료 (render_page 함수, PR #23 머지)

---

## 목표

`samples/` 28개 PDF 각 첫 페이지를 렌더링해 기준 PNG와 비교하는 이미지 회귀 테스트를 구축한다.  
OS별 픽셀 차이를 허용 오차로 흡수하고, CI에서 diff 이미지를 아티팩트로 업로드한다.

---

## 완료 기준

| # | 기준 |
|---|------|
| 1 | `crates/rpdf-render/tests/snapshots/linux/` · `macos/` 기준 PNG 각각 생성 (broken 제외 26개씩) |
| 2 | `cargo nextest run -p rpdf-render` — 이미지 회귀 테스트 28개 통과 |
| 3 | 허용 오차 정책: 픽셀 채널 평균 차이 ≤ 0.1% (전체 픽셀 × 채널 기준) |
| 4 | `UPDATE_SNAPSHOTS=1` 환경변수로 기준 PNG 재생성 가능 |
| 5 | CI: 테스트 실패 시 diff PNG를 GitHub Actions 아티팩트로 업로드 |
| 6 | `cargo test`, `cargo clippy`, `cargo fmt --check` 전체 통과 |

---

## broken PDF 처리 방침

`samples/` 에 파싱 실패 가능 파일 2개 존재:
- `broken-bad-xref-offset.pdf`
- `broken-missing-trailer.pdf`

이 파일들은 pdfium이 로딩 자체를 실패할 수 있다. 회귀 테스트에서 **기대 실패(expected failure)** 로 처리한다.

```rust
// 렌더링 성공 → 기준 PNG 비교
// 렌더링 실패(Err(_)) → 에러 종류 무관하게 예상된 에러로 통과
```

> 에러 타입을 `RenderError::FileOpen`으로 단정하지 않는다 — pdfium이 반환하는 에러 변형이 달라질 수 있다. (plan-eng-review D3 결정)

---

## 이미지 비교 인프라 설계

### 신규 외부 크레이트 없음

`image` 크레이트(이미 승인됨, Task #12)로 직접 픽셀 diff를 구현한다.  
`insta`는 텍스트 스냅샷 전용이며 이미지 feature 없음 (insta 1.47.2 확인).

### 허용 오차 계산

```
normalized_diff = Σ|a_channel - b_channel| / (width × height × channels × 255)
통과 조건: normalized_diff ≤ 0.001  (0.1%)
```

허용 오차 상수는 `snapshot_utils.rs`에 `const DIFF_THRESHOLD: f64 = 0.001;`로 선언해 조정 가능하게 한다.

### 기준 PNG 저장 위치

```
crates/rpdf-render/tests/snapshots/
  linux/          ← CI (ubuntu-latest)
    trad-xref-basicapi_p0.png
    ...           (broken PDF 2개는 파일 없음)
  macos/          ← 로컬 개발
    trad-xref-basicapi_p0.png
    ...
```

OS는 `std::env::consts::OS` ("linux" | "macos")로 자동 선택.

### 기준 PNG 생성 방법

`UPDATE_SNAPSHOTS=1` 설정 시 비교 대신 기준 PNG를 덮어쓴다.

```bash
UPDATE_SNAPSHOTS=1 PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib \
  cargo nextest run -p rpdf-render -- image_regression
```

---

## 모듈 설계

### 신규 파일: `crates/rpdf-render/tests/image_regression.rs`

```rust
// 테스트 진입점: samples/ 28개 순회
// render_page() 호출 → 기준 PNG 비교 or 업데이트
mod snapshot_utils;
```

### 신규 파일: `crates/rpdf-render/tests/snapshot_utils.rs`

테스트 전용 유틸리티 — 크레이트 공개 API에 노출하지 않는다 (plan-eng-review D2 결정):

```rust
pub const DIFF_THRESHOLD: f64 = 0.001;

/// 두 DynamicImage의 정규화 픽셀 diff를 반환한다 (0.0 ~ 1.0).
/// 크기가 다른 경우 1.0(최대 불일치) 반환.
pub fn normalized_diff(a: &DynamicImage, b: &DynamicImage) -> f64;

/// diff 픽셀을 시각화한 DynamicImage를 반환한다 (CI 아티팩트용).
pub fn diff_image(a: &DynamicImage, b: &DynamicImage) -> DynamicImage;

// 단위 테스트 (#[cfg(test)])
// - 동일 이미지 → normalized_diff == 0.0
// - 완전 다른 이미지(흑백) → normalized_diff ≈ 1.0
// - 크기 다른 이미지 → normalized_diff == 1.0 (패닉 없음)
// - diff_image 출력 크기 == 입력 크기
```

> `src/snapshot.rs` 및 `lib.rs`의 `pub mod snapshot` 추가 불필요

---

## CI 변경 사항 (`.github/workflows/ci.yml`)

```yaml
- name: Run image regression tests
  run: |
    PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib \
    cargo nextest run -p rpdf-render -- image_regression

- name: Upload diff artifacts on failure
  if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: image-diff-${{ runner.os }}
    path: /tmp/rpdf-diff/
    retention-days: 7

- name: Upload linux snapshots for first-run commit
  if: success()
  uses: actions/upload-artifact@v4
  with:
    name: snapshots-linux-${{ github.sha }}
    path: crates/rpdf-render/tests/snapshots/linux/
    retention-days: 3
```

diff 이미지는 테스트 실패 시 `/tmp/rpdf-diff/{pdf_stem}_diff.png`에 저장된다.  
`RPDF_DIFF_DIR` 환경변수로 경로 오버라이드 가능.

---

## 기준 PNG 생성 절차 (초회)

**Step 1 — macOS 스냅샷 생성 (로컬)**
```bash
UPDATE_SNAPSHOTS=1 PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib \
  cargo nextest run -p rpdf-render -- image_regression
git add crates/rpdf-render/tests/snapshots/macos/
git commit -m "test: macOS 기준 PNG 스냅샷 초기 생성"
```

**Step 2 — CI push 후 linux 스냅샷 아티팩트 다운로드**

1. PR push → CI 실행 → `snapshots-linux-{sha}` 아티팩트 생성
2. GitHub Actions UI에서 아티팩트 다운로드
3. `crates/rpdf-render/tests/snapshots/linux/`에 압축 해제
4. 커밋 후 push → 이후 CI에서 linux 스냅샷 비교 활성화

> linux 스냅샷이 없는 CI 첫 실행에서는 `UPDATE_SNAPSHOTS=1`로 자동 생성 후 아티팩트 업로드만 한다 (비교 skip). (plan-eng-review D1 결정)

---

## 파일 목록 (예상)

| 파일 | 변경 |
|------|------|
| `crates/rpdf-render/tests/image_regression.rs` | 신규 — 회귀 테스트 28개 |
| `crates/rpdf-render/tests/snapshot_utils.rs` | 신규 — diff 유틸리티 + 단위 테스트 |
| `crates/rpdf-render/tests/snapshots/macos/*.png` | 신규 — macOS 기준 PNG (로컬 생성) |
| `crates/rpdf-render/tests/snapshots/linux/*.png` | 신규 — linux 기준 PNG (CI 아티팩트 → 수동 커밋) |
| `.github/workflows/ci.yml` | 이미지 회귀 테스트 + diff/snapshot 아티팩트 업로드 추가 |

> `src/lib.rs` 변경 없음. `*.png` 파일은 git LFS 없이 직접 커밋. 26개 × 2 × ~150KB ≈ 7.8MB 예상.

---

## 에지 케이스

| 케이스 | 처리 |
|--------|------|
| broken PDF 렌더링 실패 | `Err(_)` 어떤 변형이든 예상된 에러로 통과 |
| 크기 다른 이미지 비교 | `normalized_diff` = 1.0 (패닉 없음) → 테스트 실패 |
| `PDFIUM_DYNAMIC_LIB_PATH` 미설정 | 전체 이미지 회귀 테스트 skip |
| 스냅샷 파일 없음 + `UPDATE_SNAPSHOTS` 미설정 | 테스트 실패 (기준 PNG 생성 필요 메시지 출력) |
| linux 스냅샷 없음 (CI 첫 실행) | `UPDATE_SNAPSHOTS=1` 자동 적용 → 비교 skip, 아티팩트 업로드 |

---

## 테스트 전략

- **이미지 회귀 테스트** (`tests/image_regression.rs`): samples/ 28개 순회, 허용 오차 비교
- **snapshot 유틸리티 단위 테스트** (`tests/snapshot_utils.rs` `#[cfg(test)]`):
  - 동일 이미지 → `normalized_diff` == 0.0
  - 완전 다른 이미지(흑백) → `normalized_diff` ≈ 1.0
  - 크기 다른 이미지 → `normalized_diff` == 1.0 (패닉 없음)
  - `diff_image` 출력 크기 == 입력 크기
- **병렬 실행**: cargo nextest 프로세스 격리 활용 — 각 테스트가 별도 프로세스에서 실행되므로 pdfium OnceCell 충돌 없음 (plan-eng-review D4 결정)

---

## 미포함 (이후 Task)

- 여러 페이지 스냅샷 — Task #16
- SVG 스냅샷 — Task #14·15
