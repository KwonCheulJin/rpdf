# Task #26 계획서 — PDFium WASM 대체 전략

**Issue**: #50  
**브랜치**: `local/task26`  
**마일스톤**: M040 (v0.4 WASM 바인딩)  
**작성일**: 2026-05-19

---

## 목적

`pdfium-render`는 네이티브 PDFium 동적 라이브러리에 의존하므로 WASM 타겟에서 사용 불가하다.
웹 환경에서의 렌더링 전략을 확정하고 ADR로 기록한다.
아울러 `rpdf-wasm` Cargo.toml 메타데이터를 정비하고 Gotcha를 `CONTRIBUTING.md`에 기록한다.
(pkg/ 제외는 `.gitignore`의 전역 `pkg/` 규칙으로 이미 완료됨)

---

## 배경 조사 (사전 확인)

현재 `rpdf-wasm`의 의존 그래프:
```
rpdf-wasm
├── rpdf-core
├── rpdf-parser
├── rpdf-edit
├── rpdf-serializer
├── wasm-bindgen
├── js-sys
└── serde-wasm-bindgen
```
`rpdf-render`(pdfium 의존)가 포함되지 않아 현재도 `wasm-pack build --target web` 성공 (336KB gzip).

---

## 결정된 전략

**옵션 2 채택**: 웹 환경 렌더링은 pdf.js에 위임, Rust 코어는 파싱·편집·저장만 담당.

| 옵션 | 설명 | 기각 이유 |
|------|------|----------|
| A: PDFium → WASM 컴파일 | google/pdfium-wasm | 빌드 복잡, 번들 수 MB → 2MB 초과 위험 |
| B: pdf.js 위임 (채택) | pdf.js 렌더링, Rust 편집/저장 | 간단, 번들 최소 |
| C: SVG 렌더러 개선 | rpdf-svg 품질 향상 | 시간 소모 과다, v0.6 이후 재검토 |

---

## 범위

### 포함

1. ADR-004 작성 — WASM 렌더링 전략 결정 기록 (`--target web` 선택 근거 포함)
2. `crates/rpdf-wasm/Cargo.toml` 메타데이터 보강 (description, repository, license)
3. `CONTRIBUTING.md` Gotcha 추가 — rpdf-wasm에 rpdf-render 의존 추가 금지
4. `wasm-pack build crates/rpdf-wasm --target web` 재확인 + 번들 크기 측정

### 제외

- pdf.js 통합 코드 (Task #29 rpdf-studio)
- npm 패키지 구성 (Task #28)
- CI wasm-pack 빌드 파이프라인 (Task #27)

---

## 체크포인트

### CP-A: ADR + 환경 정비

**체크리스트**:
- [ ] `docs/decisions/ADR-004-wasm-rendering-strategy.md` 작성 (`--target web` 근거 포함)
- [ ] `crates/rpdf-wasm/Cargo.toml` — description, repository, license 추가
- [ ] `CONTRIBUTING.md` — Gotcha 추가 (rpdf-wasm ↔ rpdf-render 격리)

**완료 기준**: ADR 파일 존재, wasm-pack build 경고 메시지 사라짐, CONTRIBUTING.md에 Gotcha 항목

### CP-B: 빌드 재확인

**체크리스트**:
- [ ] `wasm-pack build crates/rpdf-wasm --target web` 통과
- [ ] gzip 번들 크기 2MB 이하 확인
- [ ] `cargo test -p rpdf-wasm` 통과 (18개)
- [ ] `cargo clippy -p rpdf-wasm -- -D warnings` 경고 없음

---

## 파일 목록

| 파일 | 액션 |
|------|------|
| `docs/decisions/ADR-004-wasm-rendering-strategy.md` | 신규 |
| `crates/rpdf-wasm/Cargo.toml` | 수정 — description/repository/license 추가 |
| `CONTRIBUTING.md` | 수정 — Gotcha 추가 (rpdf-wasm ↔ rpdf-render 격리) |

---

## 에러 처리

해당 없음 (코드 변경 없음, 빌드 검증만).

---

## 테스트 전략

- `wasm-pack build --target web` 성공 여부
- gzip 번들 크기 측정 (`gzip -c crates/rpdf-wasm/pkg/*.wasm | wc -c`)
- `cargo test -p rpdf-wasm` 기존 18개 회귀 통과

---

## 외부 크레이트 변경

없음.

---

## 위험 요소

| 위험 | 대응 |
|------|------|
| 향후 rpdf-wasm에 rpdf-render가 실수로 추가될 경우 | ADR-004 + CONTRIBUTING.md Gotcha로 명시 |
| wasm-opt 없이 번들이 2MB 초과할 경우 | wasm-pack이 자동으로 wasm-opt 실행 (이미 확인됨) |

---

## NOT in scope

- pdf.js 통합 코드 — Task #29
- npm 패키지 구성 — Task #28
- CI wasm-pack 빌드 파이프라인 — Task #27
- PDFium WASM 컴파일 전략 — 번들 크기 초과 위험으로 기각

## What already exists

- `.gitignore`의 전역 `pkg/` 규칙 — crates/rpdf-wasm/pkg/ 이미 제외됨
- `rpdf-wasm` 의존 그래프 — rpdf-render 없이 이미 wasm-pack build 성공
- ADR-001 — 크레이트 분리 전략으로 pdfium 격리 이미 결정됨

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAN (PLAN) | 2 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **OUTSIDE VOICE:** Gemini — `.gitignore` 중복 제거, wasm-pack 명령어 수정, CONTRIBUTING.md Gotcha 추가, ADR --target web 근거 명시
- **UNRESOLVED:** 0
- **VERDICT:** ENG CLEARED — 계획서 수정 완료, 구현 진행 가능
