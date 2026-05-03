# mydocs — 개발 과정 문서

이 디렉터리는 rpdf 프로젝트의 **개발 과정 기록**입니다. 단순히 "코드 설명 문서"가 아니라 **AI 페어 프로그래밍으로 소프트웨어를 만드는 방법**의 기록입니다.

## 디렉터리 구조

```
mydocs/
├── orders/              # 오늘 할일 (yyyymmdd.md)
├── plans/               # 구현 계획서 (task{N}-{slug}.md)
│   └── archives/        # 완료된 계획서 보관 (선택)
├── working/             # 완료 보고서
├── report/              # 마일스톤/정기 보고서
├── feedback/            # 실사용자 피드백, 코드 리뷰
├── tech/                # 기술 연구, 의사결정 기록
├── manual/              # 온보딩, 아키텍처, 방법론
└── troubleshootings/    # 문제 해결 기록
```

## 문서 유형별 쓰임

### 타스크 시작 시
1. `orders/yyyymmdd.md`에 오늘의 타스크 추가
2. `plans/task{N}-{slug}.md` 계획서 작성
3. 사람 승인 후 구현 시작

### 타스크 완료 시
1. `working/task{N}-done.md` 완료 보고서 작성
2. 트러블슈팅이 있었다면 `troubleshootings/`에 기록
3. `orders/yyyymmdd.md`의 체크박스 업데이트

### 마일스톤 완료 시
1. `report/v{버전}.md` 마일스톤 보고서 작성
2. 다음 마일스톤의 계획서 검토 및 조정

### 실사용자 피드백 수집 시
1. `feedback/user-{기간}.md` 관찰 기록 작성
2. 발견된 이슈를 GitHub Issue로 전환
3. 다음 마일스톤 계획에 반영

### 기술 조사 시
1. `tech/{주제}.md` 기술 노트 작성
2. 의사결정은 근거와 함께 기록
3. 이후 선택이 바뀌면 같은 파일에 업데이트 이력 추가

## 주요 문서 목록

### 방법론 및 가이드 (manual/)
- [온보딩 가이드](manual/onboarding.md)
- [아키텍처](manual/architecture.md)
- [Hyper-Waterfall 방법론](manual/hyper-waterfall.md)

### 기술 문서 (tech/)
- [PDF 스펙 요약](tech/pdf-spec-summary.md)
- [크레이트 선택 의사결정](tech/crate-decisions.md)

### 로드맵 계획서 (plans/)
- [v0.1 — 파서 뼈대](plans/v0.1-parser-skeleton.md)
- [v0.2 — 렌더링 뼈대](plans/v0.2-rendering-skeleton.md)
- [v0.3 — 편집 커맨드](plans/v0.3-editing-commands.md)
- [v0.4 — WASM 바인딩](plans/v0.4-wasm-bindings.md)
- [v0.5 — Tauri 데스크톱](plans/v0.5-tauri-desktop.md)

### 템플릿
- [타스크 완료 보고서](working/template-task-done.md)
- [마일스톤 보고서](report/template-milestone-report.md)
- [실사용자 관찰](feedback/template-user-observation.md)
- [코드 리뷰](feedback/template-code-review.md)
- [트러블슈팅](troubleshootings/template-troubleshooting.md)

## 문서 작성 원칙

### 한국어로 작성
모든 내부 문서는 한국어로. 공개 API 문서(README 등)는 영문 병기 고려.

### 왜를 남긴다
"무엇을" 했는지는 코드와 커밋이 보여준다. 문서는 "왜" 그렇게 했는지를 남긴다.

### 실패도 기록한다
잘못된 선택과 수정 이력을 지우지 않는다. 다시 같은 실수를 하지 않도록.

### 짧고 구체적으로
길이보다 내용. 추상적 원칙보다 구체적 예시.

### 링크로 연결
문서끼리 상호 참조. 이슈, PR, 커밋, 외부 자료 모두 링크.

## 참고

이 문서화 방식은 [rhwp](https://github.com/edwardkim/rhwp)에서 영감을 받았습니다.
