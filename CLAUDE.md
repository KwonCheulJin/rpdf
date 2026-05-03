# CLAUDE.md

이 문서는 AI 페어 프로그래밍(Claude Code) 사용 시의 **작업 규칙**입니다.
AI는 이 규칙을 따르고, 사람은 이 규칙이 지켜지는지 검토합니다.

## 기본 원칙

**이것은 바이브 코딩이 아닙니다.** AI가 생성한 코드는 반드시 사람이 읽고 이해하고 승인한 뒤에 merge합니다.

| 사람의 역할 | AI의 역할 |
| --- | --- |
| 방향 설정, 우선순위 결정 | 분석, 계획, 구현 |
| 계획 검토 및 승인 | 구현 계획서 초안 작성 |
| 아키텍처 결정 | 정밀한 실행 |
| 코드 리뷰 및 품질 판단 | 코드·문서·테스트 생성 |

## 작업 프로토콜

모든 타스크는 다음 순서를 따릅니다.

1. **Issue 생성** — `gh issue create`로 GitHub Issue 등록, 마일스톤 지정
2. **브랜치 생성** — `local/task{Issue번호}` 브랜치
3. **계획서 작성** — `mydocs/plans/task{N}-{slug}.md`
   - 데이터 모델 변경 사항
   - 새/변경되는 API
   - 엣지 케이스
   - 테스트 전략
4. **계획 승인** — 사람이 읽고 승인
5. **구현** — 계획서대로, 계획 외 변경 시 계획서부터 수정
6. **테스트** — `cargo test`, `cargo clippy`, `pnpm test` 통과 필수
7. **완료 보고서** — `mydocs/working/task{N}-done.md`
8. **PR 및 merge** — `devel` 브랜치로 PR, `closes #{N}`

## 금지 사항

다음은 AI가 절대 하지 말아야 합니다.

- 계획서 없이 구현을 시작하는 것
- 테스트 없이 새 기능을 추가하는 것
- 아키텍처 결정을 독자적으로 내리는 것
- `unsafe` 블록을 사람 확인 없이 추가하는 것
- 외부 크레이트를 사람 승인 없이 추가하는 것
- 마일스톤 범위 밖의 기능을 "겸사겸사" 구현하는 것

## 커밋 메시지 규칙

```
Task #{번호}: 한 줄 요약

상세 설명 (선택)
- 변경된 파일의 의미
- 왜 이 방식을 택했는지

closes #{번호}
```

예시:
```
Task #12: 페이지 추출 커맨드 구현

- document_core/commands/extract_pages.rs 추가
- CLI `rpdf extract --pages 1-3` 지원
- Undo를 위한 역연산 구현

closes #12
```

## 품질 관문

merge 전 모두 통과해야 합니다.

- `cargo test` — 전체 테스트 통과
- `cargo clippy -- -D warnings` — 경고 없음
- `cargo fmt --check` — 포맷 정리됨
- 웹/데스크톱: `pnpm test`, `pnpm lint`, `pnpm typecheck`
- E2E 회귀: 주요 워크플로우 Puppeteer 통과

## 문서화 규칙

- 모든 공개 API는 `///` Rust 문서 주석 작성
- 복잡한 로직은 이유를 주석으로 남김 (코드가 "무엇"을 하는지는 코드에서, "왜" 하는지는 주석에서)
- 새 기능은 `mydocs/tech/` 또는 `mydocs/manual/`에 기술 노트 추가
- 버그 수정은 `mydocs/troubleshootings/`에 원인 분석 기록

## 디버깅 프로토콜

PDF 관련 버그는 다음 순서로 진단합니다.

1. `rpdf info <file>` — 파일 메타데이터 확인
2. `rpdf dump <file> -p <page>` — 해당 페이지 IR 덤프
3. `rpdf export-svg <file> --debug-overlay` — 시각적 디버그
4. 재현 케이스를 `tests/regression/`에 추가
5. 수정 후 동일 케이스가 통과하는지 확인

## 파일 명명 규칙

- Rust 파일: `snake_case.rs`
- TypeScript 파일: `kebab-case.ts` 또는 `PascalCase.tsx` (컴포넌트)
- 문서: `kebab-case.md`, 단 `orders/`는 `yyyymmdd.md`
- 브랜치: `local/task{N}` 또는 `feature/{slug}`

## 참고

- 개발 방법론: `mydocs/manual/hyper-waterfall.md`
- 아키텍처: `mydocs/manual/architecture.md`
- 온보딩: `mydocs/manual/onboarding.md`
