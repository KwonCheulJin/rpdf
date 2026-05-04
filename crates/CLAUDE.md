# crates/ — Rust 워크스페이스 크레이트

## 크레이트 구조

| 크레이트 | 역할 | 의존 |
|----------|------|------|
| `rpdf-core` | 도메인 타입 (`Document`, `Page`, `ObjRef` 등). 파싱 로직 없음. | 없음 |
| `rpdf-parser` | PDF 바이트 → `Document` IR. lopdf 없이 직접 파싱. | rpdf-core |
| `rpdf-render` | pdfium-render로 PNG 출력. 동적 라이브러리 런타임 로딩. | rpdf-core |
| `rpdf-cli` | CLI 진입점 (`rpdf` 바이너리). | rpdf-core, rpdf-parser, rpdf-render |

## 새 크레이트 추가 규칙

```bash
cargo new --lib crates/<name> --vcs none
```

- 루트 `Cargo.toml`의 `[workspace] members`에 추가
- `version.workspace = true`, `edition.workspace = true` 사용
- 크레이트명: `rpdf-<role>` 패턴

## 주의사항 (Gotcha)

- **Caveat**: `rpdf-core`에는 파싱·렌더링 로직 없음 — 값 객체(`Copy + Clone + PartialEq + Eq`)만
- `rpdf-render`는 `PDFIUM_DYNAMIC_LIB_PATH` 환경변수 필요 (런타임 동적 로딩)
- **Pitfall**: `rpdf-core` 타입 변경 → 모든 크레이트 재컴파일. 영향 범위 넓음.
- 테스트 회귀 케이스: `crates/rpdf-parser/tests/regression/`

## 관련 (See Also)

- [docs/decisions/ADR-001-crate-workspace-structure.md](../docs/decisions/ADR-001-crate-workspace-structure.md) — 크레이트 분리 결정
- [CONTRIBUTING.md](../CONTRIBUTING.md) — 네이밍 규칙·Known Issues
