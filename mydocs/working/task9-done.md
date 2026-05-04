# Task #9 — 디버그 CLI 완료 보고서

**Issue**: #16
**브랜치**: `local/task9`
**완료일**: 2026-05-04
**소요 시간**: 계획 1세션 / 실제 1세션

## 완료된 작업

계획서 완료 기준 대비 결과:

- [x] `rpdf info <pdf>` — 메타데이터 + 페이지 수 (인간 가독 + `--json`)
- [x] `rpdf dump-pages [-p PAGE] <pdf>` — 페이지 메타데이터 (인간 가독 + `--json`)
- [x] `rpdf dump [-p PAGE] <pdf>` — content stream 연산자 시퀀스 (인간 가독 + `--json`)
- [x] `ContentStreamOperator::pdf_keyword()` + `display_name()` 메서드 추가 (rpdf-core)
- [x] `cargo test --workspace` 전체 통과
- [x] `cargo clippy -- -D warnings` 경고 없음
- [x] `examples/` 5개 PDF 모두 3개 명령 동작

## 실제 변경 사항

### 새로 추가된 파일

- `crates/rpdf-cli/` — 새 binary crate (workspace member)
  - `Cargo.toml` — 의존성: clap 4.6, anyhow, serde, serde_json, + dev: assert_cmd, predicates, proptest, tempfile
  - `src/main.rs` — CLI 진입점, clap derive 서브커맨드 구조
  - `src/commands/mod.rs` — 3개 명령 모듈 선언
  - `src/commands/info.rs` — `rpdf info` 구현 + CI-1~3 단위 테스트
  - `src/commands/dump_pages.rs` — `rpdf dump-pages` 구현 + CD-1~3 단위 테스트
  - `src/commands/dump.rs` — `rpdf dump` 구현 + CE-1~5 단위 테스트
  - `tests/cli_tests.rs` — IT-C1~3, IT-D1~3, IT-E1~5 통합 테스트 + proptest

### 수정된 파일

- `Cargo.toml` — workspace dependencies에 clap, serde_json, assert_cmd, predicates, tempfile 추가
- `crates/rpdf-core/src/types/content_stream.rs` — `pdf_keyword()` + `display_name()` 메서드 + BT-1~4 단위 테스트

### 갱신된 문서

- `mydocs/plans/v0.1-parser-skeleton.md` — Task #9 항목 명세 명확화

## 테스트 결과

| 종류 | 수량 | 결과 |
|------|------|------|
| rpdf-cli 단위 테스트 (CI/CD/CE) | 11개 | 전체 통과 |
| rpdf-cli 통합 테스트 (IT-C/D/E + proptest) | 12개 | 전체 통과 |
| rpdf-core 새 단위 테스트 (BT-1~4) | 4개 | 전체 통과 |
| **기존 테스트 (Task #1~8)** | 284개 | **전체 유지** |
| **합계** | **311개** | **전체 통과** |

## 실제 동작 확인

```sh
$ rpdf info examples/fw4-2024.pdf
Pages:    5
Title     2026 Form W-4
Author    C:DC:TS:CAR:MP
...

$ rpdf dump-pages -p 0 examples/fw4-2024.pdf
Page 0:
  MediaBox: [0, 0, 611.976, 791.968]
  CropBox:  [0, 0, 611.976, 791.968]
  Rotation: 0
  Ops:      1769

$ rpdf dump -p 0 examples/irs-f1040.pdf | head -5
=== Page 0 (7124 ops) ===
  0.863 1 0.984 rg
  /RelativeColorimetric ri
  91.599 738 491.601 29.999 re
  f
```

## 설계 결정 기록

- **`rpdf-cli` 별도 binary crate**: architecture.md 바인딩 레이어 원칙 준수. 라이브러리에 바이너리 혼용 대신 명시적 분리.
- **`[[bin]] name = "rpdf"`**: crate name(`rpdf-cli`)과 binary name(`rpdf`)을 분리. Cargo.toml `[[bin]]` 섹션 필수.
- **`serde` 직접 선언 필요**: `serde_json`이 `serde`에 의존하지만 `use serde::Serialize`는 별도 선언 없이 사용 불가. workspace serde를 명시적으로 추가.
- **`PdfObject::String` 없음**: 이 코드베이스는 `LiteralString`/`HexString`으로 분리. `as_string_bytes()`가 두 변형을 통합해 반환하지만, `dump.rs`에서 직접 match시 양쪽 모두 처리 필요.
- **`predicates` 별도 선언**: `assert_cmd`가 `predicates`를 내부 사용하지만 re-export 안 함. 통합 테스트에서 `predicates::str::contains` 직접 사용 시 별도 선언 필수.
- **JSON 출력 스키마 일관성**: 3개 명령 모두 `{ "page_count", "filtered_page"?, "metadata"|"pages": ... }` 공통 최상위 구조.
- **`PdfObject` JSON 직렬화 헬퍼**: `Serialize` 미구현이므로 `operand_to_json_value()` 직접 작성. v0.1 한정 Dict/Stream → `"<complex>"`. doc comment에 v0.1 한정 정책 명시.

## 트러블슈팅

- [assert_cmd predicates not reexported](../troubleshootings/assert-cmd-predicates-not-reexported.md) — assert_cmd 통합 테스트 scaffold 시 predicates 별도 선언 필요

## 셀프 리뷰

- [x] 3개 명령 모두 5개 PDF에서 동작
- [x] `--json` 출력 JSON 파싱 가능 (IT-C2, IT-D2, IT-E2, IT-E4)
- [x] 범위 초과 `-p 99` → exit 1 + 에러 메시지 (IT-D3, IT-E5)
- [x] proptest: 임의 바이트 입력 → panic/abort 없음
- [x] vacuous pass 없음: BT 실제 존재 확인 (IT-E1)
- [x] `pdf_keyword()` 전체 69개 enum 변형 (Unknown 제외) 매핑 → BT-4 전수 검증

## 회고 분류

| 후보 | 분류 | 근거 |
|------|------|------|
| `[[bin]] name = "rpdf"` 패턴 | C | binary crate 첫 도입 시 필수 패턴. Task #10+ 에서도 재사용. |
| `serde` 직접 선언 | C | workspace serde 있어도 `use serde::*` 사용 시 별도 선언 필요. 규칙으로 인식해야 함. |
| `predicates` 별도 선언 | **A** | 트러블슈팅 즉시 작성. `assert_cmd` 예시에 항상 함께 나오지만 re-export 아님. 다음 타스크에서 통합 테스트 추가 시 반복될 패턴. |
| `PdfObject::LiteralString/HexString` 분리 | C | 코드베이스 아키텍처 결정. `as_string_bytes()`로 통합 접근 가능하지만 match 직접 시 양쪽 처리 필요. onboarding 문서에 추가 검토. |
| `<complex>` JSON 대체 정책 | C | v0.1 충분. doc comment에 명시함. |

## 다음 작업

Task #10 — 회귀 테스트 인프라 (CI 강화, `samples/` 30개 PDF, insta 스냅샷 테스트)
