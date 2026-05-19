# Task #27 계획서 — wasm-pack CI 파이프라인 + `@rpdf/core` npm 패키지 구성

**Issue**: #52  
**브랜치**: `local/task27`  
**마일스톤**: M040 (v0.4 WASM 바인딩)  
**선행 조건**: Task #26 완료 ✅ (ADR-004, wasm-pack build 성공 336KB)

---

## 목표

1. `npm/core/`를 `@rpdf/core` npm 패키지로 구성 (wasm-pack 출력을 직접 이 디렉터리에 저장)
2. CI에 `wasm` job 추가 — wasm-pack build + 번들 크기 검증 (gzip 2MB 이하)

---

## 현재 상태

| 항목 | 현황 |
|------|------|
| `crates/rpdf-wasm/pkg/` | wasm-pack 기본 출력 (gitignore됨, 로컬 확인용) |
| `npm/core/` | README.md 플레이스홀더만 존재 (디렉터리 없음) |
| CI | `rust` job + `node` job (npm install만) |
| `pnpm-workspace.yaml` | `npm/*` 이미 포함 |

---

## 설계 결정

### wasm-pack 출력 경로

`crates/rpdf-wasm/pkg/`는 로컬 확인용(gitignore)으로 유지하고,
`npm/core/`를 **npm 패키지 배포 경로**로 분리한다.

```bash
# --out-dir은 crate 기준 상대경로
# crates/rpdf-wasm/ + ../../npm/core = npm/core/ (workspace root 기준)
# ⚠️ CP-A에서 로컬 실행 후 npm/core/에 파일이 생성됨을 반드시 확인
wasm-pack build crates/rpdf-wasm --target web \
  --out-dir ../../npm/core \
  --out-name rpdf_core \
  --scope rpdf
```

- `--out-name rpdf_core` → `rpdf_core.js`, `rpdf_core_bg.wasm`, `rpdf_core.d.ts`
- `--scope rpdf` + `--out-name rpdf_core` → `package.json` name = `@rpdf/rpdf-wasm`
  (wasm-pack은 crate name 기반으로 name 생성, --out-name은 파일명만 변경)
- name을 `@rpdf/core`로 변경하기 위한 post-process 스크립트를 추가한다

### npm/core 디렉터리 관리 방침

- wasm-pack 빌드 산출물(`.wasm`, `.js`, `.d.ts`)은 **git에 커밋하지 않는다** (CI에서 매번 재생성)
- `npm/core/.gitignore`로 빌드 산출물 제외 — **반드시 wasm-pack build 전에 생성**
- `npm/core/package.json`은 git에 커밋 (wasm-pack 생성본을 post-process 후 저장)

> **이유**: WASM 바이너리는 빌드 재현 가능 산출물이므로 git에 포함할 이유가 없다.
> package.json은 버전/의존성 선언 파일이므로 커밋한다.

### post-process 방식

wasm-pack이 생성한 `package.json`의 `name` 필드를 `@rpdf/core`로 수정한다.

```bash
# scripts/rename-wasm-package.sh
node -e "
  const fs = require('fs');
  const p = JSON.parse(fs.readFileSync('npm/core/package.json', 'utf8'));
  p.name = '@rpdf/core';
  fs.writeFileSync('npm/core/package.json', JSON.stringify(p, null, 2) + '\n');
"
```

---

## 파일 변경 목록

### 신규

```
npm/core/.gitignore          — 빌드 산출물(*.wasm, *.js, *.d.ts) 제외
npm/core/package.json        — @rpdf/core 패키지 메타데이터 (post-process 결과 커밋)
scripts/rename-wasm-package.sh — package.json name 후처리 스크립트
```

### 수정

```
.github/workflows/ci.yml     — wasm job 추가
pnpm-lock.yaml               — @rpdf/core workspace 패키지 추가로 변경됨
```

---

## 체크포인트

### CP-A: npm/core 패키지 구성

**작업**:
1. `npm/core/.gitignore` 생성 ← **wasm-pack build보다 먼저**
   ```
   *.wasm
   *.js
   *.d.ts
   snippets/
   ```
2. `scripts/rename-wasm-package.sh` 생성 (위 설계 참조)
3. 로컬에서 wasm-pack build 실행, **`npm/core/`에 파일이 생성됐는지 확인** (경로 해석 검증 필수)
   ```bash
   wasm-pack build crates/rpdf-wasm --target web \
     --out-dir ../../npm/core \
     --out-name rpdf_core \
     --scope rpdf
   ls npm/core/  # ← rpdf_core.js, rpdf_core_bg.wasm, rpdf_core.d.ts 확인
   ```
4. `scripts/rename-wasm-package.sh` 실행, `package.json` name 확인
   ```bash
   bash scripts/rename-wasm-package.sh
   grep '"name": "@rpdf/core"' npm/core/package.json  # 반드시 확인
   ```
5. `pnpm install` 실행 후 `pnpm-lock.yaml` 업데이트 확인
   ```bash
   pnpm install
   git diff pnpm-lock.yaml  # @rpdf/core importer 추가 확인
   ```
6. `pnpm ls --filter @rpdf/core` — `@rpdf/core` 인식 여부 확인
7. `npm/core/package.json` 및 `pnpm-lock.yaml` 커밋 (빌드 산출물 제외 확인)
   ```bash
   git status  # *.wasm, *.js, *.d.ts가 표시되지 않음을 확인
   ```

**완료 기준**:
- `npm/core/package.json`에 `"name": "@rpdf/core"` 확인
- `pnpm ls --filter @rpdf/core`에서 패키지 인식
- `npm/core/*.wasm` 등 산출물이 git에 포함되지 않음
- `pnpm-lock.yaml` 업데이트됨 (커밋 포함)

### CP-B: CI wasm job 추가

**작업**: `.github/workflows/ci.yml`에 `wasm` job 추가

```yaml
wasm:
  name: WASM (wasm-pack build)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: wasm32-unknown-unknown
    - uses: Swatinem/rust-cache@v2
    - uses: taiki-e/install-action@v2
      with:
        tool: wasm-pack@0.15.0

    - name: wasm-pack build
      run: |
        wasm-pack build crates/rpdf-wasm --target web \
          --out-dir ../../npm/core \
          --out-name rpdf_core \
          --scope rpdf
        bash scripts/rename-wasm-package.sh
        # rename 결과 검증 — silent failure 방지
        grep '"name": "@rpdf/core"' npm/core/package.json || exit 1

    - name: Bundle size check (gzip ≤ 2MB)
      run: |
        # .wasm 단독 체크 (.js 글루 코드 ~50KB 포함해도 여유 충분)
        SIZE=$(gzip -c npm/core/rpdf_core_bg.wasm | wc -c)
        echo "gzip 번들 크기: ${SIZE} bytes"
        if [ "$SIZE" -gt 2097152 ]; then
          echo "❌ 번들 크기 초과: ${SIZE} bytes (상한: 2097152)"
          exit 1
        fi
        echo "✅ 번들 크기 통과: ${SIZE} bytes"
```

**완료 기준**:
- CI `wasm` job 통과
- `cargo test -p rpdf-wasm` (네이티브)는 기존 `rust` job에서 이미 실행됨 → 별도 추가 불필요
- 번들 크기 로그에서 gzip 바이트 수 확인

---

## 에러 처리 표

| 에러 상황 | 대응 |
|----------|------|
| `wasm32-unknown-unknown` 타겟 미설치 | `dtolnay/rust-toolchain@stable` + `targets: wasm32-unknown-unknown` |
| `wasm-pack --out-dir` 경로 오류 (npm/core 미생성) | CP-A step 3에서 `ls npm/core/` 확인 필수 |
| `npm/core/package.json` name 미변경 | `grep` 검증 실패로 CI exit 1 → rename 스크립트 확인 |
| gzip 크기 초과 | `wasm-opt` 적용 또는 feature flag로 코드 축소 검토 |
| pnpm이 `@rpdf/core` 미인식 | `pnpm-workspace.yaml`의 `npm/*` 패턴 확인 |
| CI `node` job `--frozen-lockfile` 실패 | CP-A step 5 `pnpm install` 후 `pnpm-lock.yaml` 커밋 누락 여부 확인 |

---

## 테스트 전략

- **CP-A**: `pnpm ls --filter @rpdf/core`로 패키지 인식 확인
- **CP-B**: CI `wasm` job 통과 + 번들 크기 로그 확인 + `grep` 검증 통과
- wasm-bindgen TypeScript 타입(`npm/core/rpdf_core.d.ts`) 유효성은 Task #28(웹 에디터)에서 실사용으로 검증

---

## 도구 버전 (실제 설치)

| 도구 | 버전 |
|------|------|
| wasm-pack | 0.15.0 |
| Rust stable | 1.87.0 (CI: latest stable) |
| Node | v22.22.0 |
| pnpm | 10.13.1 |

---

## 범위 외

- npm publish (`@rpdf/core` 실제 배포) → v0.4 완료 후 별도 Task
- Web Worker / Comlink 래퍼 → Task #28
- `rpdf-studio` 웹 에디터 → Task #28

---

## 참고

- wasm-pack 설치 Action: `taiki-e/install-action@v2` + `tool: wasm-pack@0.15.0`
- ADR-004: `docs/decisions/ADR-004-wasm-rendering-strategy.md`
- v0.4 개요: `mydocs/plans/v0.4-wasm-web.md`

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 4 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0 (D1~D4 모두 결정 완료)
- **VERDICT:** ENG CLEARED — 계획서 구현 준비 완료
