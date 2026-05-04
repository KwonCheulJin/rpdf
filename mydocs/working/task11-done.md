# Task #11 완료 보고서: pdfium 환경 구축

**날짜**: 2026-05-04  
**Issue**: #20  
**브랜치**: local/task11

---

## 완료 기준 달성 여부

| # | 완료 기준 | 달성 |
|---|-----------|------|
| 1 | `crates/rpdf-render/` 신규 crate 생성, 워크스페이스 등록 | ✅ |
| 2 | `scripts/fetch-pdfium.sh` 작성 (macOS arm64/x64 + Linux x64) | ✅ |
| 3 | `cargo build --workspace` 성공 | ✅ |
| 4 | `rpdf-render`에 `Pdfium::bind_to_library()` 최소 테스트 작성 + 통과 | ✅ |
| 5 | CI (`.github/workflows/ci.yml`) pdfium 자동 설치 + `cargo nextest run -p rpdf-render` 통과 | ✅ |
| 6 | `LD_LIBRARY_PATH` + `PDFIUM_DYNAMIC_LIB_PATH` CI에 설정됨 | ✅ |
| 7 | `mydocs/tech/dev-tool-pdfium-render.md` 작성 | ✅ |
| 8 | `.gitignore`에 `pdfium/` 추가 | ✅ |

---

## 실제 사용된 버전 정보

| 항목 | 값 |
|------|-----|
| pdfium-render crate | **0.9.1** (명세 0.9.2는 crates.io 미존재) |
| pdfium 빌드번호 | **7763** (Chromium 148.0.7763.0, 명세 6721에서 수정) |
| pdfium 플랫폼 (로컬 검증) | mac-arm64 |

---

## 생성/수정된 파일

| 파일 | 변경 내용 |
|------|-----------|
| `crates/rpdf-render/Cargo.toml` | 신규 생성 (pdfium-render 0.9.1, image 0.25) |
| `crates/rpdf-render/src/lib.rs` | 최소 스텁 + pdfium_dynamic_links 테스트 |
| `scripts/fetch-pdfium.sh` | 플랫폼별 pdfium 다운로드 스크립트 |
| `.gitignore` | `pdfium/` 추가 |
| `.github/workflows/ci.yml` | pdfium 캐시/설치/env 설정 스텝 추가 |
| `mydocs/plans/task11-pdfium-env.md` | 계획서 |
| `mydocs/tech/dev-tool-pdfium-render.md` | 도구 도입 문서 |
| `mydocs/troubleshootings/pdfium-render-version-not-on-crates-io.md` | 트러블슈팅 |

---

## 트러블슈팅 발생/해결 내역

### T1: pdfium-render 0.9.2 crates.io 미존재

- **현상**: `cargo fetch` 실패 — 0.9.2 없음
- **원인**: 명세에 0.9.2가 지정됐으나 실제 최신 버전은 0.9.1
- **해결**: 0.9.1로 수정. README 확인으로 pdfium 빌드번호도 6721 → 7763 수정
- **문서**: `mydocs/troubleshootings/pdfium-render-version-not-on-crates-io.md`

---

## 검증 결과

```
# cargo build --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s)

# cargo nextest run -p rpdf-render (PDFIUM_DYNAMIC_LIB_PATH 설정 후)
PASS [0.010s] (1/1) rpdf-render tests::pdfium_dynamic_links
Summary: 1 test run: 1 passed, 0 skipped

# cargo clippy --all-targets -- -D warnings
Finished `dev` profile (경고 없음)

# cargo fmt --all -- --check
(출력 없음 = 포맷 정상)
```

---

## 회고 분류표

| 분류 | 항목 | 처리 |
|------|------|------|
| B | pdfium-render 버전 명세 ↔ 실제 crates.io 버전 불일치 | `mydocs/troubleshootings/pdfium-render-version-not-on-crates-io.md` 작성 완료 |
| C | CI `Set pdfium env` 스텝은 cache hit 여부와 무관하게 항상 실행해야 함 | 완료 보고서 메모 (구현에 반영됨) |

### A 항목 (CLAUDE.md 즉시 반영)

없음 — 기존 규칙(`외부 크레이트 사용 전 docs.rs에서 실제 버전 확인`)으로 커버됨.

---

## 다음 작업

Task #12: PDF → PNG 렌더링 (rpdf-render에 실제 렌더링 코드 추가)
