# Task #26 완료 보고서 — PDFium WASM 대체 전략

**Issue**: #50  
**브랜치**: `local/task26`  
**완료일**: 2026-05-19  
**마일스톤**: M040 (v0.4 WASM 바인딩)

---

## 구현 결과

pdfium-render의 WASM 불가 문제를 해결하기 위한 전략을 ADR로 기록하고, rpdf-wasm 빌드 환경을 정비했다.

**채택된 전략**: pdf.js 렌더링 위임 + Rust 코어 파싱·편집·저장

| 작업 | 담당 |
|------|------|
| 페이지 렌더링, 썸네일 | pdf.js |
| 문서 구조 파싱 | Rust (rpdf-parser) |
| 편집 커맨드 | Rust (rpdf-edit) |
| 저장 | Rust (rpdf-serializer) |

---

## 변경 파일

### 신규

```
docs/decisions/ADR-004-wasm-rendering-strategy.md  — pdf.js 위임 전략 ADR
mydocs/plans/task26-wasm-rendering-strategy.md     — 계획서
```

### 수정

```
crates/rpdf-wasm/Cargo.toml  — description/repository/license 추가
CONTRIBUTING.md              — rpdf-wasm ↔ rpdf-render 격리 Gotcha 추가
```

---

## 체크포인트별 결과

| CP | 내용 | 결과 |
|----|------|------|
| CP-A | ADR-004 작성 + Cargo.toml 메타데이터 + CONTRIBUTING.md Gotcha | ✅ |
| CP-B | wasm-pack build 재확인, gzip 336KB, cargo test 18개 통과 | ✅ |

---

## 빌드 검증

```
wasm-pack build crates/rpdf-wasm --target web  →  성공 (Done in 1.43s)
gzip 번들 크기: 336KB (2MB 이하)
cargo test -p rpdf-wasm: 18개 통과
cargo clippy -p rpdf-wasm -- -D warnings: 경고 없음
```

---

## 계획서와 다르게 구현된 사항

| 항목 | 계획서 | 실제 구현 | 이유 |
|------|--------|----------|------|
| gitignore 추가 | 포함 | 제거 | 전역 `pkg/` 규칙으로 이미 완료됨 |
| wasm-pack 명령어 | `-p` 플래그 사용 | 디렉토리 경로 사용 | wasm-pack은 `-p` 미지원 (Gemini 지적) |

---

## plan-eng-review 발견 이슈 처리

| 이슈 | 처리 |
|------|------|
| .gitignore 중복 제거 (Gemini) | ✅ 계획서에서 제거 |
| wasm-pack 명령어 수정 (Gemini) | ✅ 계획서 + CP-B 수정 |
| CONTRIBUTING.md Gotcha 누락 (Gemini) | ✅ 파일 목록 추가 및 구현 |
| ADR --target web 근거 명시 (Gemini) | ✅ ADR-004에 포함 |

---

## 트러블슈팅 후보 분류

| 항목 | 분류 | 처리 |
|------|------|------|
| wasm-pack은 `-p` 플래그 미지원, 디렉토리 경로 필요 | C | 완료 보고서 메모 (cargo 명령어와 혼동하기 쉬움) |
| Cargo.toml에 license 있어도 루트 LICENSE 파일 없으면 wasm-pack 경고 | C | 완료 보고서 메모 (Task #28에서 처리) |

---

## 완료 기준 달성

1. ✅ ADR-004 작성 — pdf.js 위임 전략, --target web 근거, 기각 대안 포함
2. ✅ rpdf-wasm이 pdfium-render 없이 wasm-pack build 성공 (이미 달성됨, 재확인)
3. ✅ CONTRIBUTING.md Gotcha — rpdf-wasm ↔ rpdf-render 격리 명시
4. ✅ wasm-pack build 통과, 번들 336KB
