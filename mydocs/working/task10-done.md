# Task #10 — 회귀 테스트 인프라 완료 보고서

**Issue**: #18  
**브랜치**: `local/task10`  
**완료일**: 2026-05-04  
**소요 시간**: 1세션 (자율 진행)

## 완료된 작업

계획서 체크포인트 대비 결과:

- [x] **A: samples/ 수집** — 28개 git commit + 2개 large (scripts/fetch-samples.sh)
- [x] **B: insta 스냅샷** — 28개 스냅샷 생성 및 승인
- [x] **C: CI 강화** — cargo nextest + INSTA_UPDATE=no 도입
- [x] **D: 성능 베이스라인** — perf-baseline.md 작성
- [x] **E: v0.1 완성 문서** — v0.1-completion.md 작성

## 실제 변경 사항

### 새로 추가된 파일

- `samples/` — 28개 PDF (T8+S8+M4+B2+N2+X4)
  - Apache-2.0 (Mozilla pdf.js 18개)
  - 미국 정부 공개도메인 (IRS 7개)
  - 직접 생성 (손상 B1~B2, 한국어 M4: 3개)
- `samples/README.md` — 라이선스 표 포함 전체 목록
- `scripts/fetch-samples.sh` — large/ 다운로드 스크립트 (macOS/Linux)
- `scripts/make-corrupted-samples.py` — B1/B2 손상 PDF 재생성
- `crates/rpdf-parser/tests/regression/mod.rs` — 30개 스냅샷 테스트
- `crates/rpdf-parser/tests/regression/snapshots/` — 28개 `.snap` 파일
- `mydocs/report/perf-baseline.md` — 성능 베이스라인
- `mydocs/report/v0.1-completion.md` — v0.1 완성 문서
- `mydocs/troubleshootings/pages-tree-circular-ref-stack-overflow.md`

### 수정된 파일

- `Cargo.toml` — `insta = "1" (features=yaml)` workspace dev-dep 추가
- `crates/rpdf-parser/Cargo.toml` — `insta`, `serde` dev-dep, `samples-large` feature
- `crates/rpdf-parser/tests/lib.rs` — `mod regression` 추가
- `.github/workflows/ci.yml` — nextest + INSTA_UPDATE=no 도입
- `.gitignore` — `samples/large/` 제외
- `mydocs/report/v0.1-progress.md` — 최종 갱신

## 테스트 결과

| 종류 | 수량 | 결과 |
|------|------|------|
| 스냅샷 테스트 (T/S/M/B/N/X) | 28개 | 전체 통과 |
| L1/L2 대용량 (samples-large feature) | 2개 | ignore (expected) |
| **기존 테스트 (Task #1~9)** | 311개 | **전체 유지** |
| **합계 (nextest)** | **339개 + 2 skip** | **전체 통과** |

## 설계 결정 기록

- **samples/ 분류 8+8+4+2+2+2+4**: 각 분류의 테스트 의도를 명확히 분리
- **대용량 2개 cfg feature 분리**: CI가 외부 URL에 의존하지 않도록
- **insta yaml 스냅샷**: content stream 제외, 메타데이터+페이지 메타만
- **`.snap.new → .snap` 수동 rename**: cargo-insta 도구 없이 초기 승인
- **UTF-16BE 메타데이터 현재 상태 기록**: v0.2 수정 후 스냅샷 업데이트 예정
- **perf-baseline.md**: criterion 없이 rpdf info 3회 평균으로 거친 기준선

## 트러블슈팅

- [페이지 트리 순환 참조 스택 오버플로우](../troubleshootings/pages-tree-circular-ref-stack-overflow.md) — `Pages-tree-refs.pdf`가 스택 오버플로우 유발. T7에서 제거하고 issue1155r.pdf로 교체. 파서 순환 감지 로직 추가는 v0.2.

## 셀프 리뷰

- [x] 28개 samples/ 파일 모두 유효 PDF (`%PDF-` 확인)
- [x] 라이선스: Apache-2.0, 공개도메인, 직접생성만 사용
- [x] 스냅샷 28개 생성 및 승인 완료 (pending 없음)
- [x] `INSTA_UPDATE=no cargo nextest run --all` → 339 passed, 2 skipped
- [x] `cargo clippy -- -D warnings` 경고 없음
- [x] CI workflow 수정: nextest + INSTA_UPDATE=no
- [x] perf-baseline.md 작성 (fw4 139ms 이상치 포함)
- [x] v0.1-completion.md 작성

## 회고 분류

| 후보 | 분류 | 근거 |
|------|------|------|
| 라이선스 3종 분류 패턴 (Apache-2.0/공개도메인/직접생성) | **B** | 트러블슈팅 문서로 기록 가치. PDF 수집 시 표준 체크리스트. |
| `cargo-insta` 도구 없이 `.snap.new → .snap` rename | **B** | insta 첫 도입 시 발생. 트러블슈팅 후보. |
| 페이지 트리 순환 참조 스택 오버플로우 발견 | **B** | 트러블슈팅 즉시 작성 완료. v0.2 수정 예정. |
| 회고 채집 `retro-notes.md` 패턴 | **A** | CLAUDE.md 자율 진행 모드 섹션에 반영 권장. |
| insta 스냅샷 대상: 메타+페이지 메타만 (content 제외) | **C** | 완료 보고서 메모. 스냅샷이 너무 커지지 않게 하는 설계 결정. |
| `taiki-e/install-action@cargo-nextest` CI 패턴 | **A** | CLAUDE.md CI 섹션에 표준 도구 설치 방법으로 추가 권장. |

## 다음 작업

**v0.2 렌더링 뼈대**:
- `pdfium-render` crate 도입 (docs.rs 공개 API 확인 선행)
- `rpdf-render` crate 신규 추가
- v0.1 parser 출력 → v0.2 렌더링 연결
- 수정 우선순위: `/Length` indirect reference, 페이지 트리 순환 참조

**v0.1 이슈 백로그** (v0.2에서 해소):
- Stream `/Length` indirect reference (canvas.pdf, 다수 파일)
- 페이지 트리 순환 참조 감지
- UTF-16BE 메타데이터 디코딩 개선
