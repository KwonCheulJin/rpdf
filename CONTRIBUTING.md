# 기여 가이드

rpdf 프로젝트에 기여해주셔서 감사합니다. 이 문서는 이슈 제보와 PR 제출 방법을 안내합니다.

## 이 프로젝트의 특성

rpdf는 **개인 학습과 실사용 도구 제작**이 주 목적인 프로젝트입니다. 상용 제품이 아니기 때문에 다음 원칙을 지킵니다.

- 기능 추가는 실사용 피드백 우선
- 포크 환영, 방향성 토론도 환영
- 빠른 응답은 약속하지 않음

## 이슈 제보

### 버그 제보 시 필요한 정보

1. 재현 가능한 PDF 파일 (민감 정보 제거)
2. 실행한 커맨드 또는 클릭 순서
3. 기대한 결과 vs 실제 결과
4. OS, 버전 정보
5. `rpdf info <file>` 결과

### 기능 제안 시 필요한 정보

1. 어떤 상황에서 필요한가
2. 현재는 어떻게 우회하고 있는가
3. 비슷한 기능을 가진 다른 제품이 있다면 그 레퍼런스

## PR 제출

### 사전 준비

1. 작업 전에 Issue를 먼저 열어 방향 합의
2. `local/task{Issue번호}` 브랜치에서 작업
3. `mydocs/plans/task{N}-{slug}.md` 계획서 작성

### 품질 요구사항

- `cargo test` 통과
- `cargo clippy -- -D warnings` 경고 없음
- `cargo fmt` 포맷 정리
- 새 기능에는 테스트 포함
- 공개 API에는 `///` 문서 주석

### 커밋 메시지

```
Task #{번호}: 한 줄 요약

상세 설명 (필요시)

closes #{번호}
```

### PR 대상 브랜치

- `devel` 브랜치로 PR 제출
- `main`은 릴리즈 태깅 전용

## 코드 스타일

- Rust: 표준 `rustfmt` 설정
- TypeScript: 프로젝트 `.prettierrc` 준수
- import 순서: 표준 라이브러리 → 외부 크레이트 → 내부 모듈

## 명명·네이밍 규칙 (Naming Convention / Pattern)

PDF 스펙(ISO 32000) 용어를 코드에 그대로 반영한다. Don't use translated or invented names.

| 개념 | 올바른 이름 | 피해야 할 이름 |
|------|------------|---------------|
| xref 테이블 | `XrefTable` | `CrossReferenceTable` |
| trailer 딕셔너리 | `PdfTrailer` | `PdfFooter`, `TrailerDict` |
| 오브젝트 참조 | `ObjRef` | `ObjectReference`, `PdfRef` |
| 페이지 미디어 박스 | `media_box` | `page_bounds`, `page_rect` |

## 알려진 Gotcha (이미 빠진 함정)

### pdfium-render 버전 ↔ 빌드번호 미스매치
- **증상**: 런타임 `symbol not found` 또는 `cannot open shared object file`
- **원인**: pdfium-render 버전과 PDFium 바이너리 빌드번호가 맞지 않음
- **해결**: `scripts/CLAUDE.md` 호환표 확인 후 `fetch-pdfium.sh`의 `PDFIUM_BUILD` 동기화

### macOS Gatekeeper quarantine
- **증상**: `libpdfium.dylib cannot be opened because the developer cannot be verified`
- **해결**: `scripts/fetch-pdfium.sh`가 자동 해제 (`xattr -d com.apple.quarantine`)

### insta 스냅샷 첫 도입
- **증상**: CI에서 스냅샷 불일치 오류 (`*.snap.new` 파일만 생성, `.snap` 없음)
- **해결**: 로컬에서 `cargo insta accept` 실행 후 `.snap` 파일 커밋

### `rpdf-core`에 파싱 로직 추가 금지
- `rpdf-core`는 값 객체(`Copy + Clone + PartialEq + Eq`)만 — 파싱은 `rpdf-parser`
- 위반 시 WASM 타겟에서 링크 오류 발생 가능

### `PDFIUM_DYNAMIC_LIB_PATH`는 파일이 아닌 디렉토리
- 올바름: `export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib`
- 잘못됨: `export PDFIUM_DYNAMIC_LIB_PATH=$(pwd)/pdfium/lib/libpdfium.dylib`

### rpdf-wasm에 rpdf-render 의존 추가 금지

`rpdf-wasm`의 `Cargo.toml`에 `rpdf-render` 의존을 추가하면 wasm-pack build가 실패한다.
`rpdf-render`는 네이티브 PDFium 동적 라이브러리를 런타임에 로딩하므로 WASM 타겟에서 컴파일 불가.

웹 환경 렌더링은 ADR-004에 따라 pdf.js가 담당한다. Rust 코어는 파싱·편집·저장만.

## 라이선스

기여한 코드는 MIT 라이선스로 배포됩니다.
