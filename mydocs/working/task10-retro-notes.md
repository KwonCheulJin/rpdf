# Task #10 회고 채집 임시 메모

회고 분류 표 누락 방지용 체크포인트별 메모.
PR 본문 작성 시 이 메모를 활용해 A/B/C 분류 완성.

---

## Checkpoint A 끝 (samples/ 수집)

후보 1: 라이선스 검증 절차 — PDF commit 전 출처·라이선스 확인 표준화 패턴
후보 2: 손상 PDF 직접 생성 패턴 — 의도 손상 방법 (trailer 제거, offset 오염) 문서화 가치

## Checkpoint B 끝 (insta 스냅샷)

후보 1: insta 첫 도입 함정 — 첫 실행 pending, cargo insta accept 순서, INSTA_UPDATE=no 필수
후보 2: 스냅샷 대상 결정 근거 — content stream 제외 이유

## Checkpoint C 끝 (CI 강화)

후보 1: cargo nextest 도입 효과 (병렬 실행 속도 개선)
후보 2: taiki-e/install-action 패턴 (cargo 도구 CI 설치 표준 방법)

## Checkpoint D 끝 (성능 베이스라인)

후보 1: 단순 측정 vs criterion 분기 결정 (v0.1 vs v0.2)
후보 2: 측정 지표 선정 (load_document 시간만 vs 메모리)

## Checkpoint E 시작 (v0.1-completion.md)

- v0.1 전체 회고 (10개 타스크)
- 자율 진행 모드 평가 (Task #7-10 누적)
- 회고 분류 채집 패턴 자체가 A 항목 후보 (CLAUDE.md 반영)
