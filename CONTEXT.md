# rpdf — AI 네비게이션 컴패스

PDF 파서·렌더러·편집기 (Rust + WASM + Tauri). 모듈 구조(module structure)와 entry point를 중심으로 빠르게 탐색하기 위한 문서.

## 핵심 진입점 (Entry Points)

| 파일 | 용도 |
|------|------|
| `crates/rpdf-parser/src/document.rs` | 파싱 진입점 `load_document()` |
| `crates/rpdf-cli/src/main.rs` | CLI 바이너리 진입점 |
| `CLAUDE.md` | 전체 개발 규칙 (TDD, DDD, 금지사항) |
| `docs/decisions/` | 주요 기술 결정 ADR |
| `docs/playbooks/` | 디버깅·CI 대응 절차서 |

## 빠른 시작

```bash
cargo build --workspace
cargo nextest run --all
```

## 품질 검증

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

## AI 페어 프로그래밍 성과 (개선 전/후)

- v0.1 task 성공률: 100% (tool call 최소화, 재작업 <10%)
- AI-Ready Score: 29 → 65+ (토큰 효율 향상 목표)
- 성공률 측정 기준: `docs/ai-metrics.md`

## 관련 문서 (See Also)

- 아키텍처: [mydocs/manual/architecture.md](mydocs/manual/architecture.md)
- 온보딩: [mydocs/manual/onboarding.md](mydocs/manual/onboarding.md)
- Gotcha·함정: [CONTRIBUTING.md](CONTRIBUTING.md#알려진-gotcha)
- AI 성과 지표: [docs/ai-metrics.md](docs/ai-metrics.md)
