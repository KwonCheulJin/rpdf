# .ai-tests/ — AI 협업 시나리오 테스트

AI 에이전트와의 페어 프로그래밍에서 자주 발생하는 작업 시나리오.
각 `.prompt` 파일은 AI가 이 리포지토리에서 올바르게 작업할 수 있는지 검증하는 프롬프트.

## 시나리오 목록

| 파일 | 시나리오 | 성공 기준 |
|------|----------|-----------|
| `parse-new-pdf.prompt` | 새 PDF 파싱 코드 추가 | `rpdf-parser`에 구현, 테스트 포함 |
| `debug-xref.prompt` | xref 파싱 오류 디버깅 | `rpdf info`로 재현 후 픽스 |
| `add-cli-command.prompt` | 새 CLI 서브커맨드 추가 | `rpdf-cli/src/main.rs` 수정, 테스트 추가 |

## 실행

```bash
# 특정 시나리오를 Claude Code로 실행
cat .ai-tests/parse-new-pdf.prompt | pbcopy  # 클립보드에 복사 후 Claude Code에 붙여넣기
```
