# Task #27 완료 보고서 — wasm-pack CI 파이프라인 + `@rpdf/core` npm 패키지 구성

**Issue**: #52  
**브랜치**: `local/task27`  
**완료일**: 2026-05-19  
**마일스톤**: M040 (v0.4 WASM 바인딩)

---

## 구현 결과

wasm-pack 빌드 산출물을 `npm/core/`로 라우팅하는 파이프라인을 구성하고, CI에 독립 `wasm` job을 추가했다.

| 작업 | 결과 |
|------|------|
| `@rpdf/core` npm 패키지 구성 | `npm/core/package.json` — `"name": "@rpdf/core"` |
| wasm 빌드 산출물 git 제외 | `npm/core/.gitignore` — `*.wasm`, `*.js`, `*.d.ts`, `snippets/` |
| rename 후처리 스크립트 | `scripts/rename-wasm-package.sh` |
| CI wasm job | `taiki-e/install-action@v2`, rename 검증, gzip ≤ 2MB 체크 |
| pnpm workspace 등록 | `pnpm-lock.yaml` — `npm/core: {}` importer 추가 |

---

## 변경 파일

### 신규

```
npm/core/.gitignore              — 빌드 산출물 git 제외
npm/core/package.json            — @rpdf/core 패키지 메타데이터
scripts/rename-wasm-package.sh   — package.json name 후처리 스크립트 (chmod +x)
mydocs/plans/task27-wasm-npm-pipeline.md — 계획서
```

### 수정

```
.github/workflows/ci.yml   — wasm job 추가
pnpm-lock.yaml             — npm/core importer 추가
```

---

## 체크포인트별 결과

| CP | 내용 | 결과 |
|----|------|------|
| CP-A | npm/core 패키지 구성 + pnpm 등록 + 로컬 wasm-pack 빌드 검증 | ✅ |
| CP-B | CI wasm job 추가 + evaluator 검증 PASS | ✅ |

---

## 빌드 검증

```
wasm-pack build crates/rpdf-wasm --target web --out-dir ../../npm/core --out-name rpdf_core --scope rpdf
→ 성공

gzip npm/core/rpdf_core_bg.wasm: 344KB (2MB 이하 ✅)
pnpm install: "Scope: all 2 workspace projects" ✅
git add --dry-run npm/core/: package.json + .gitignore만 ✅
```

---

## plan-eng-review 발견 이슈 처리

| 이슈 | 처리 |
|------|------|
| jetli action → taiki-e/install-action 일관성 (D1) | ✅ taiki-e/install-action@v2 + tool: wasm-pack@0.15.0 |
| pnpm-lock.yaml 갱신 단계 누락 (D2) | ✅ CP-A에 pnpm install + 커밋 단계 추가 |
| rename 스크립트 silent failure (D3) | ✅ CI에 grep 검증 step 추가 |
| 번들 크기 체크 범위 명시 (D4) | ✅ .wasm 단독 + CI 코멘트에 JS ~50KB 명시 |

---

## 계획서와 다르게 구현된 사항

| 항목 | 계획서 | 실제 구현 | 이유 |
|------|--------|----------|------|
| `.gitignore` 선생성 후 보호 | 선생성만 언급 | wasm-pack이 `*` 내용으로 덮어씀 → 복원 필요 | wasm-pack 0.15.0이 out-dir의 .gitignore를 항상 덮어씀 |

---

## 트러블슈팅 후보 분류

| 항목 | 분류 | 처리 |
|------|------|------|
| wasm-pack이 `--out-dir`의 `.gitignore`를 `*`로 덮어씀 | A | CLAUDE.md 또는 CONTRIBUTING.md에 즉시 반영 |
| `pnpm ls --filter @rpdf/core` — 의존 패키지 없으면 node_modules 링크 없음 (정상) | C | 완료 보고서 메모 |
| `--scope rpdf` + `--out-name rpdf_core` 조합 시 package.json name은 `@rpdf/rpdf-wasm` (crate name 기반) | C | 완료 보고서 메모 (rename 스크립트로 처리) |

---

## 완료 기준 달성

1. ✅ `npm/core/package.json`에 `"name": "@rpdf/core"` 확인
2. ✅ wasm-pack 빌드 산출물 git 미포함 (`.gitignore` 검증)
3. ✅ pnpm workspace 패키지 인식 (`npm/core: {}` in pnpm-lock.yaml)
4. ✅ CI `wasm` job — taiki-e/install-action, rename 검증, gzip ≤ 2MB
5. ✅ evaluator PASS
