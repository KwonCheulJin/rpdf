# Task #10 — 회귀 테스트 인프라

**Issue**: #18  
**브랜치**: `local/task10`  
**작성일**: 2026-05-04  
**마일스톤**: v0.1 (마지막 타스크)

## 목표

v0.1 성공 기준 #6 "샘플 PDF 30개 수집 및 회귀 테스트 인프라 구축"을 달성하여 v0.1 마일스톤을 완성한다. 파서 변경 시 회귀를 자동 검출하는 스냅샷 테스트 인프라를 구축한다.

## 범위

### 포함

- `samples/` 28개 PDF git commit + 2개 대용량 cfg feature 분리
- `insta` 기반 스냅샷 테스트 (Page 메타데이터 + ParseError 종류)
- CI 강화: `cargo nextest` 도입 + insta 스냅샷 검증
- 성능 베이스라인: 단순 측정 + `mydocs/report/perf-baseline.md` 기록
- v0.1 완성 문서: `mydocs/report/v0.1-completion.md`

### 제외 (v0.2+)

- criterion 벤치마크 crate 도입 및 성능 회귀 자동 알림
- Windows용 fetch-samples.ps1
- 손상 PDF 복구 (v0.1 방침: 에러 반환)

## 새 의존성

| 크레이트 | 추가 위치 | 버전 | 목적 |
|---|---|---|---|
| `insta` | workspace dev-dependency | `"1"` | 스냅샷 테스트 |
| `cargo-nextest` | CI workflow | 최신 stable | 병렬 테스트 실행 |

> **공개 API 확인**: insta 1.x docs.rs — `assert_snapshot!`, `assert_yaml_snapshot!` 매크로 확인 완료.  
> cargo-nextest는 CI에서 `cargo install cargo-nextest --locked`로 설치.

## samples/ 분류 상세 (총 30개)

### 수집 정책

- **출처 우선순위**: Mozilla pdf.js 테스트 코퍼스 (Apache 2.0) → IRS 공개 도메인 → LaTeX/LibreOffice 직접 생성
- **크기 기준**: 3MB 이하 → git commit (`samples/`), 초과 → cfg feature 분리 (`samples/large/`)
- **라이선스 검증**: 모든 파일에 출처·라이선스 README 기록. 모호하면 commit 안 함.
- **이름 규칙**: `{분류코드}-{설명}.pdf` (예: `xref-traditional-basic.pdf`, `xref-stream-adobe.pdf`)

### 분류 (8+8+4+2+2+2+4=30)

| 번호 | 분류 | 개수 | 기술적 특성 | 테스트 의도 |
|------|------|------|-------------|-------------|
| T1~T8 | 전통 xref | 8 | PDF 1.3~1.5, `xref` 키워드, 20바이트 고정 항목 | 기본 파이프라인 |
| S1~S8 | xref stream | 8 | PDF 1.5+, `/XRef`, `/W` 필드, ObjStm 포함 | 스트림 파이프라인 |
| M1~M4 | 다국어/한글 | 4 | UTF-16BE 메타데이터, CJK 폰트 | 메타데이터 인코딩 |
| B1~B2 | 손상 (의도 에러) | 2 | Trailer 누락, xref 오프셋 깨짐 | ParseError 검증 |
| N1~N2 | 비표준 (관용) | 2 | Mac Preview, 구형 Acrobat 출력 비표준 | 관용 처리 검증 |
| L1~L2 | 대용량 | 2 | 10MB+ (cfg feature 분리, git 제외) | 성능 베이스라인 |
| X1~X4 | 특수 | 4 | incremental update, 빈 페이지, 폼 필드, 암호화 헤더만 | 엣지 케이스 |

### samples/large/ 정책

```toml
# Cargo.toml (workspace)
[features]
samples-large = []
```

```rust
#[test]
#[cfg_attr(not(feature = "samples-large"), ignore)]
fn test_large_pdf_load_time() { ... }
```

CI는 `samples-large` feature 없이 실행 → 대용량 2개 자동 제외.  
로컬: `cargo nextest run --features samples-large` 으로 전체 검증.

### scripts/fetch-samples.sh

```bash
#!/usr/bin/env bash
# samples/large/ PDF 다운로드 (macOS/Linux)
set -euo pipefail
# L1: govdocs1 대용량 PDF
# L2: Mozilla pdf.js 대형 파일
```

macOS/Linux 우선. Windows는 별도 Issue.

## 스냅샷 테스트 설계

### 스냅샷 대상

```
samples/<name>.pdf → snapshot:
  page_count: N
  metadata:
    title: "..."
    author: "..."
    producer: "..."
  pages:
    - index: 0
      media_box: [0.0, 0.0, 612.0, 792.0]
      rotation: 0
      op_count: 1234
    ...
```

content stream 전체 dump는 수십 KB → 제외.

### 손상 PDF 스냅샷 (B1~B2)

```
samples/broken-no-trailer.pdf → snapshot:
  error: "MissingTrailer"
```

ParseError 변형 이름만 기록 (세부 메시지는 변경 가능성).

### insta 설정

```toml
# .config/insta.yaml (또는 Cargo.toml)
[insta]
snapshot_path = "tests/__snapshots__"
```

CI 환경: `INSTA_UPDATE=no` 환경변수 → 스냅샷 불일치 시 즉시 fail.  
로컬: `cargo insta review` 로 인터랙티브 승인.

> **insta 첫 도입 주의**: 첫 실행 시 스냅샷 파일이 없으면 pending 상태로 생성됨.  
> `cargo insta accept` 로 초기 스냅샷 일괄 승인 후 commit 필요.  
> CI에서 pending 스냅샷 = fail.

## CI 강화

### 변경 내용 (.github/workflows/ci.yml)

```yaml
- name: Install cargo-nextest
  uses: taiki-e/install-action@cargo-nextest

- name: Test (nextest)
  run: cargo nextest run --all
  env:
    INSTA_UPDATE: no
```

`cargo test --all` → `cargo nextest run --all`로 교체.  
insta가 `INSTA_UPDATE=no` 없으면 pending 파일 생성 후 통과할 수 있어 명시 필수.

### 예상 CI 시간

| 단계 | 현재 | 변경 후 |
|------|------|---------|
| cargo fmt | ~5s | ~5s |
| cargo clippy | ~30s | ~30s |
| cargo test (nextest) | ~60s | ~50s (병렬 가속) |
| 스냅샷 검증 | - | 포함됨 (별도 step 없음) |
| **합계** | ~2분 | ~2분 미만 목표 |

30개 PDF 스냅샷 테스트는 nextest 병렬 실행으로 흡수.

## 성능 베이스라인

### 측정 방법

```rust
// tests/perf_baseline.rs (또는 별도 binary)
let start = std::time::Instant::now();
let _ = load_document(&bytes).unwrap();
println!("{}: {:?}", file_name, start.elapsed());
```

수동 실행 후 결과를 `mydocs/report/perf-baseline.md`에 기록.

### 측정 대상

- examples/ 5개 (fw4-2024, irs-f1040, pdfjs-basicapi, pdfjs-tracemonkey, pdfjs-annotation-border)
- samples/ 대용량 2개 (L1, L2) — `--features samples-large`

criterion 도입은 v0.2.

## 체크포인트

### A: samples/ 수집 + README (완료 기준: 28개 commit, README 라이선스 기록)

1. Mozilla pdf.js GitHub corpus에서 후보 파일 목록 확인
2. 분류별 파일 선정 (T1~T8, S1~S8, M1~M4, N1~N2, X1~X4)
3. 손상 PDF 2개 직접 생성 (Python/Rust로 최소 구조 생성 후 의도 훼손)
4. 대용량 2개 URL 확인 → `scripts/fetch-samples.sh` 작성
5. `samples/README.md` 작성 (라이선스 표 포함)
6. `cargo test --all` 통과 확인 (기존 테스트 깨지지 않음)

**셀프 리뷰**: 라이선스 모호한 파일 없는지 README 재검토.

### B: insta 도입 + 스냅샷 생성 (완료 기준: 30개 스냅샷 commit)

1. `insta = "1"` workspace dev-dependency 추가
2. `tests/regression/` 디렉터리 생성
3. `tests/regression/mod.rs` — 28개 samples/ 스냅샷 테스트
4. `tests/regression/broken_tests.rs` — B1~B2 ParseError 스냅샷
5. 초기 실행 → `cargo insta accept` → commit
6. `cargo test --all` + `cargo clippy -- -D warnings` 통과

**셀프 리뷰**: 스냅샷 파일이 pending 없이 모두 accepted 상태인지 확인.

### C: CI 강화 (완료 기준: CI 통과 + nextest 사용)

1. `.github/workflows/ci.yml` 수정
   - `taiki-e/install-action@cargo-nextest` step 추가
   - `cargo test --all` → `cargo nextest run --all`
   - `env: INSTA_UPDATE: no` 추가
2. `devel` 브랜치에 push → CI 통과 확인
3. CI 총 시간 5분 이내 확인

**셀프 리뷰**: CI 로그에서 `cargo nextest` 병렬 실행 확인. insta pending 없음.

### D: 성능 베이스라인 (완료 기준: perf-baseline.md 기록)

1. examples/ 5개 `load_document` 시간 측정 (로컬 `cargo test -- --nocapture`)
2. samples/ L1, L2 측정 (`--features samples-large`)
3. `mydocs/report/perf-baseline.md` 작성 (파일, 크기, 시간)
4. commit

### E: v0.1-completion.md + 보고서 + PR (완료 기준: PR 오픈)

1. `mydocs/report/v0.1-completion.md` 작성
2. `mydocs/working/task10-done.md` 작성 (회고 분류 표 포함)
3. `mydocs/report/v0.1-progress.md` 최종 갱신
4. PR 작성 (closes #18)

**PR 본문 필수 항목** (Task #7-9 누락 패턴 방지):
- 셀프 리뷰 체크리스트
- 트러블슈팅 작성 명시
- 다음 작업 (v0.2 렌더링 진입)
- **회고 분류 표** (A/B/C 분류)

## 테스트 전략

```
tests/
  regression/
    mod.rs               ← 28개 samples/ 스냅샷 테스트
    broken_tests.rs      ← B1~B2 ParseError 스냅샷
  parser/
    integration_tests.rs ← 기존 (변경 없음)
  __snapshots__/
    regression__*.snap   ← insta 자동 생성
```

### 스냅샷 테스트 작성 패턴

```rust
// tests/regression/mod.rs
use insta::assert_yaml_snapshot;
use rpdf_parser::load_document;

fn snapshot_pdf(path: &str) -> serde_yaml::Value {
    let bytes = std::fs::read(path).unwrap();
    match load_document(&bytes) {
        Ok(doc) => serde_yaml::to_value(DocSnapshot::from(doc)).unwrap(),
        Err(e)  => serde_yaml::to_value(ErrorSnapshot { error: format!("{:?}", e) }).unwrap(),
    }
}

#[test]
fn snapshot_xref_traditional_basic() {
    assert_yaml_snapshot!(snapshot_pdf("samples/xref-traditional-basic.pdf"));
}
```

### 스냅샷 타입 정의

```rust
// 스냅샷용 경량 타입 (serde::Serialize)
#[derive(Serialize)]
struct DocSnapshot {
    page_count: usize,
    metadata: MetadataSnapshot,
    pages: Vec<PageSnapshot>,
}

#[derive(Serialize)]
struct PageSnapshot {
    index: usize,
    media_box: [f64; 4],
    rotation: i32,
    op_count: usize,
}

#[derive(Serialize)]
struct ErrorSnapshot {
    error: String,
}
```

## 의존성 관리

### workspace Cargo.toml 추가

```toml
[workspace.dependencies]
insta = { version = "1", features = ["yaml"] }
```

### rpdf-parser/Cargo.toml dev-dependencies

```toml
[dev-dependencies]
insta = { workspace = true }
```

## 라이선스 검증 절차

samples/ 각 파일 추가 시:

1. 출처 URL 기록
2. 라이선스 파일/페이지 확인
3. 상업적 이용·재배포 허용 여부 확인
4. `samples/README.md` 라이선스 컬럼에 명시
5. 모호하면 추가 중단 → 대체 파일 탐색

Apache 2.0, CC0, Public Domain, MIT는 자동 승인.  
CC-BY 계열은 출처 표시 README에 명시 후 OK.  
CC-BY-NC, GPL은 사용 불가.

## 엣지 케이스

- 손상 PDF B1~B2 직접 생성: Python `struct.pack`으로 최소 PDF 생성 후 trailer 또는 xref 헤더 제거
- 다국어 메타데이터 M1~M4: LaTeX로 직접 생성 (ko, ja, zh-TW, ar)
- X1 incremental update: `examples/pdfjs-annotation-border.pdf` 복사 활용 가능 (Apache 2.0)
