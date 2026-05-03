# 트러블슈팅 — pnpm/action-setup@v4 버전 충돌로 CI 빌드 실패

**발생일**: 2026-05-03
**해결일**: 2026-05-03
**관련 Issue**: #1 (Task #1 프로젝트 초기 세팅)
**심각도**: 높음 (CI 전체 node job 실패)
**환경**: GitHub Actions / pnpm/action-setup@v4 / pnpm 10.13.1

## 증상

- GitHub Actions의 `Node (install)` job이 시작 직후 실패
- `cargo` 관련 job은 정상이나 `node` job만 실패해 CI 전체가 빨간 상태
- 에러 메시지:
  ```
  Error: Multiple versions of pnpm specified:
    - version 10 in the GitHub Action config with the key "version"
    - version pnpm@10.13.1 in the package.json with the key "packageManager"
  Remove one of these versions to avoid version mismatch errors like ERR_PNPM_BAD_PM_VERSION
  ```

## 재현 방법

`.github/workflows/ci.yml`의 node job에 아래와 같이 `version` 키가 있고,
`package.json`에도 `packageManager` 필드가 있을 때 재현됨.

```yaml
- uses: pnpm/action-setup@v4
  with:
    version: 10
```

```json
{
  "packageManager": "pnpm@10.13.1"
}
```

위 상태로 push하면 node job이 즉시 오류 종료.

## 원인 분석

### 1차 가설

CI `version` 키에 major만 지정(`10`)하고 `packageManager`는 패치 버전까지 지정(`10.13.1`)해서
버전 매칭 비교에 실패한다고 생각했으나, 이는 부분적으로만 맞음.

### 최종 원인

`pnpm/action-setup@v3`까지는 CI에서 `version` 키로 pnpm 버전을 지정하는 것이 표준이었음.
**v4부터** action이 `package.json`의 `packageManager` 필드를 자동으로 인식하도록 변경됨.
두 소스가 동시에 존재하면 어느 버전을 따를지 모호하다는 이유로 action이 에러로 종료함.

**관련 파일**: `.github/workflows/ci.yml:28-29`

**왜 이 문제가 발생했는가**:
CI 파일 작성 시 이전 버전(`@v3`) 방식대로 `version` 키를 유지한 채 action만 `@v4`로 올렸기 때문.
`pnpm init`이 생성한 `packageManager` 필드를 지우지 않은 것도 원인.

## 해결책

### 적용한 수정

CI에서 `version` 키를 제거. `pnpm/action-setup@v4`가 `package.json`의
`packageManager` 필드를 단일 소스로 읽도록 위임.

```yaml
# 변경 전
- uses: pnpm/action-setup@v4
  with:
    version: 10

# 변경 후
- uses: pnpm/action-setup@v4
```

`package.json`의 `packageManager` 필드는 그대로 유지:

```json
{
  "packageManager": "pnpm@10.13.1"
}
```

### 테스트 추가

CI 자체가 회귀 테스트 역할을 함. 수정 후 push 시 node job **success** 확인.

## 재발 방지

- `pnpm/action-setup`을 버전 업그레이드할 때 릴리즈 노트에서 breaking change 확인
- pnpm 버전은 `package.json`의 `packageManager` 필드를 **단일 소스**로 유지
- CI에서 pnpm 버전을 별도로 지정하지 않는다 (action이 자동으로 읽음)
- 새 프로젝트 세팅 시 이 문서를 체크리스트로 활용

## 배운 점

- `pnpm/action-setup` v3 → v4 마이그레이션 시 `version` 키 제거가 필수
- `packageManager` 필드(`pnpm init`이 자동 생성)가 있으면 CI에서 중복 지정 불필요
- CI 실패는 push 직후 빠르게 확인해야 함 — 이번 케이스는 22초 만에 실패 결과 확인 가능

## 참고 자료

- pnpm/action-setup v4 릴리즈 노트: <https://github.com/pnpm/action-setup/releases/tag/v4.0.0>
- 관련 커밋: `c1afedb` (fix: CI node job pnpm 버전 충돌 수정)
- 관련 Issue: #1
