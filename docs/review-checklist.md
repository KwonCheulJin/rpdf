# PR 리뷰 체크리스트

PR 병합 전 리뷰어가 확인해야 할 항목.

## 필수 (Mandatory)

- [ ] `cargo nextest run --all` 통과
- [ ] `cargo clippy -- -D warnings` 경고 없음
- [ ] `cargo fmt --check` 포맷 정리됨
- [ ] 새 기능에 테스트 포함 여부
- [ ] 공개 API에 `///` 문서 주석 있는지

## 아키텍처

- [ ] `rpdf-core`에 파싱 로직 없음 (값 객체만)
- [ ] 새 에러 변형에 실제 발생 코드 경로 존재
- [ ] PDF 스펙 용어 준수 (CONTRIBUTING.md 네이밍 규칙 참고)

## 보안 / 안정성

- [ ] 외부 입력 정수 변환 시 범위 검증 있음
- [ ] `unsafe` 블록에 사람 확인 코멘트 있음

## 문서

- [ ] 변경된 공개 API에 `///` 주석 업데이트
- [ ] Gotcha/함정 발견 시 `CONTRIBUTING.md` 또는 `mydocs/troubleshootings/` 반영

## 관련 (See Also)

- [CONTRIBUTING.md](../CONTRIBUTING.md) — 기여 규칙 전체
- [CLAUDE.md](../CLAUDE.md) — 품질 관문·금지 사항
