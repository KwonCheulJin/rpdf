# Runbook: CI 실패 대응

## pdfium 관련 실패

### 증상: `PDFIUM_DYNAMIC_LIB_PATH not set`

```
PDFIUM_DYNAMIC_LIB_PATH not set — run scripts/fetch-pdfium.sh first
```

**원인**: CI의 "Set pdfium env" 스텝이 실행되지 않았거나 cache-hit 후 env 미설정.  
**해결**: `Set pdfium env` 스텝이 `if: always()` 없이 캐시 히트 분기에서 스킵되는지 확인.

### 증상: `symbol not found` / `cannot open shared object file`

**원인**: pdfium 빌드번호 ↔ pdfium-render 버전 불일치.  
**해결**: `scripts/CLAUDE.md` 호환표 확인. `PDFIUM_BUILD` 변수 동기화.

## insta 스냅샷 실패

### 증상: `snapshot assertion failed`

```
── snapshot assertion failed ──────────────────────────────────
snapshot name: t1_cross_ref_table
input file: crates/rpdf-parser/tests/regression/mod.rs
```

**원인 A**: 파서 변경으로 IR 출력이 달라짐 → 의도적 변경이면 로컬에서 `cargo insta accept`.  
**원인 B**: 신규 샘플 추가 후 `.snap` 커밋 누락 → `git add *.snap` 후 재푸시.  
**CI 환경**: `INSTA_UPDATE=no` 설정 필수 — 없으면 `*.snap.new` 생성 후 통과처럼 보이다 다음 실행 실패.

## clippy 실패

```bash
cargo clippy -- -D warnings
```

경고를 에러로 처리. 로컬에서 동일 명령 실행 후 수정.  
`approx_constant` 경고: 테스트 픽스처 값이 π/e 근사면 `use std::f64::consts::PI` 사용.

## 관련 문서

- `.github/workflows/ci.yml` — CI 전체 스텝 정의
- `scripts/fetch-pdfium.sh` — pdfium 다운로드 스크립트
