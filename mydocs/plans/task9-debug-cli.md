# Task #9 — 디버그 CLI 계획서

**Issue**: #16
**브랜치**: `local/task9`
**작성일**: 2026-05-04
**마일스톤**: M010 v0.1

---

## 목표

Task #2-8에서 구축한 파서를 사용자가 손에 쥘 수 있는 CLI 도구로 노출한다.
첫 번째 binary crate(`rpdf-cli`)를 도입하며, v0.1 파서 품질 검증 도구 역할을 한다.

## 완료 기준

- [ ] `rpdf info <pdf>` — 메타데이터 + 페이지 수 출력 (인간 가독 + `--json`)
- [ ] `rpdf dump-pages [-p PAGE] <pdf>` — 페이지 메타데이터 목록 출력 (인간 가독 + `--json`)
- [ ] `rpdf dump [-p PAGE] <pdf>` — content stream 연산자 시퀀스 출력 (인간 가독 + `--json`)
- [ ] `ContentStreamOperator::pdf_keyword()` + `display_name()` 메서드 (`rpdf-core`)
- [ ] `cargo test --workspace` 전체 통과
- [ ] `cargo clippy -- -D warnings` 경고 없음
- [ ] `examples/` 5개 PDF에 대해 3개 명령 모두 동작

## 범위

### 포함

- `rpdf-cli` binary crate (workspace member)
- 3개 CLI 명령 (`info`, `dump-pages`, `dump`)
- 인간 가독 텍스트 출력 (기본) + `--json` 플래그
- `ContentStreamOperator::pdf_keyword()` + `display_name()` (`rpdf-core` 확장)
- 단위 테스트 (포맷팅 함수)
- 통합 테스트 (`assert_cmd`, `examples/` 기반)
- `v0.1-parser-skeleton.md` Task #9 항목 명세 갱신

### 제외 (v0.2+ 영역)

- `rpdf export-svg` — 실제 렌더링 필요 (v0.2)
- 색상 터미널 출력 (`anstream`, `colored` 등)
- 인터랙티브 모드
- 암호화된 PDF 처리
- `rpdf merge`, `rpdf split`, `rpdf rotate` (v0.3+)

---

## 새 크레이트 구조

```
crates/rpdf-cli/
├── Cargo.toml
└── src/
    └── main.rs
```

**의존성 방향**:
```
rpdf-cli (binary)
  ├── rpdf-parser  (path = "crates/rpdf-parser")  — load_document, ParseError
  ├── rpdf-core    (path = "crates/rpdf-core")    — Document, Page, ContentStreamOperator
  ├── clap = "4.6"  (features = ["derive"])       — 서브커맨드 인자 파싱
  ├── anyhow = "1"  (workspace)                   — CLI 에러 처리
  └── serde_json = "1.0"                          — --json 출력
```

**dev-dependencies**:
```
assert_cmd = "2.2"    — CLI 통합 테스트 (assert stderr/stdout/exit code)
```

**workspace Cargo.toml 추가**:
```toml
[workspace.dependencies]
serde_json = "1"
assert_cmd = "2"
clap = { version = "4", features = ["derive"] }
rpdf-cli = { path = "crates/rpdf-cli" }
```

---

## 명령 시그니처

### `rpdf info <pdf>`

```
USAGE:
    rpdf info [OPTIONS] <PDF>

ARGS:
    <PDF>    PDF 파일 경로

OPTIONS:
    --json    JSON 형식으로 출력
    -h, --help
```

**인간 가독 출력 예시**:
```
File:     fw4-2024.pdf
Pages:    5
Title:    Form W-4 (2024)
Author:   Internal Revenue Service
Producer: Adobe PDF Library 21.7
Created:  D:20240101000000-05'00'
Modified: D:20240115000000-05'00'
```

**JSON 출력 예시** (공통 최상위 구조):
```json
{
  "page_count": 5,
  "metadata": {
    "title": "Form W-4 (2024)",
    "author": "Internal Revenue Service",
    "subject": null,
    "creator": null,
    "producer": "Adobe PDF Library 21.7",
    "creation_date": "D:20240101000000-05'00'",
    "modification_date": "D:20240115000000-05'00'"
  }
}
```

> metadata가 None이면 `"metadata": null`.

---

### `rpdf dump-pages [-p PAGE] <pdf>`

```
USAGE:
    rpdf dump-pages [OPTIONS] <PDF>

ARGS:
    <PDF>    PDF 파일 경로

OPTIONS:
    -p, --page <PAGE>    출력할 페이지 (0-based 인덱스). 미지정 시 전체 페이지.
    --json               JSON 형식으로 출력
    -h, --help
```

**인간 가독 출력 예시** (페이지 1개):
```
Page 0:
  MediaBox: [0.0, 0.0, 612.0, 792.0]
  CropBox:  none
  Rotation: 0
  Ops:      247
```

**JSON 출력 예시** (공통 최상위 구조):
```json
{
  "page_count": 5,
  "filtered_page": null,
  "pages": [
    {
      "index": 0,
      "media_box": [0.0, 0.0, 612.0, 792.0],
      "crop_box": null,
      "rotation": 0,
      "op_count": 247
    }
  ]
}
```

> `-p 0` 지정 시 `"filtered_page": 0`, 미지정 시 `"filtered_page": null`.
> **`resources` 필드**: `PdfDict`가 Serialize 미구현이므로 JSON 출력에서 제외.
> 리소스 키 목록이 필요하면 v0.2에서 추가.

---

### `rpdf dump [-p PAGE] <pdf>`

```
USAGE:
    rpdf dump [OPTIONS] <PDF>

ARGS:
    <PDF>    PDF 파일 경로

OPTIONS:
    -p, --page <PAGE>    출력할 페이지 (0-based 인덱스). 미지정 시 전체 페이지.
    --json               JSON 형식으로 출력
    -h, --help
```

**인간 가독 출력 예시**:
```
=== Page 0 (247 ops) ===
  q
  0.5 w
  /GS1 gs
  BT
  /F1 12 Tf
  100 700 Td
  (Hello World) Tj
  ET
  Q
```

> 피연산자 → 연산자 순 (PDF 스펙 순서). PdfObject 출력은 `Debug` 포맷 단순화.
> `Unknown` 연산자: `?<raw_bytes>` 형식.

**JSON 출력 예시** (공통 최상위 구조):
```json
{
  "page_count": 5,
  "filtered_page": null,
  "pages": [
    {
      "index": 0,
      "op_count": 247,
      "ops": [
        { "op": "q", "operands": [] },
        { "op": "w", "operands": [0.5] },
        { "op": "Tf", "operands": ["/F1", 12] },
        { "op": "Tj", "operands": ["Hello World"] }
      ]
    }
  ]
}
```

> `PdfObject` JSON 직렬화: `Serialize` 미구현이므로 별도 직렬화 헬퍼 작성.
> v0.1 한정: Boolean/Integer/Real/Name/String만 처리. Array 재귀. Dict/Stream/Reference는 `"<complex>"` 대체.
> 더 정밀한 객체 덤프는 v0.2 이후 별도 명령(`rpdf inspect` 등)으로 분리 예정.

---

## ContentStreamOperator 확장 (rpdf-core)

`crates/rpdf-core/src/types/content_stream.rs`에 추가:

```rust
impl ContentStreamOperator {
    /// PDF 스펙 키워드 반환. `&'static str`.
    ///
    /// CLI `rpdf dump` 출력 및 스펙 대조 목적. Debug 포맷과 별개.
    pub fn pdf_keyword(&self) -> &'static str { ... }

    /// 사용자 출력용 표현. Unknown은 raw bytes 포함한 String 반환.
    pub fn display_name(&self) -> String { ... }
}
```

매핑 (전체):
- `BeginText` → `"BT"`, `EndText` → `"ET"`
- `ShowText` → `"Tj"`, `ShowTextAdjusted` → `"TJ"`
- `MoveText` → `"Td"`, `MoveTextSetLeading` → `"TD"`, `SetTextMatrix` → `"Tm"`, `MoveToNextLine` → `"T*"`
- `MoveShowText` → `"'"`, `MoveSetShowText` → `"\""`
- `SetCharSpacing` → `"Tc"`, `SetWordSpacing` → `"Tw"`, `SetHorizontalScale` → `"Tz"`, `SetLeading` → `"TL"`, `SetFont` → `"Tf"`, `SetRenderingMode` → `"Tr"`, `SetTextRise` → `"Ts"`
- `SaveState` → `"q"`, `RestoreState` → `"Q"`, `ConcatMatrix` → `"cm"`
- `SetLineWidth` → `"w"`, `SetLineCap` → `"J"`, `SetLineJoin` → `"j"`, `SetMiterLimit` → `"M"`, `SetDashPattern` → `"d"`, `SetFlatness` → `"i"`, `SetGraphicsState` → `"gs"`, `SetRenderingIntent` → `"ri"`
- `MoveTo` → `"m"`, `LineTo` → `"l"`, `CurveTo` → `"c"`, `CurveToV` → `"v"`, `CurveToY` → `"y"`, `ClosePath` → `"h"`, `Rect` → `"re"`
- `Stroke` → `"S"`, `CloseStroke` → `"s"`, `Fill` → `"f"`, `FillObsolete` → `"F"`, `FillEvenOdd` → `"f*"`, `FillStroke` → `"B"`, `FillStrokeEvenOdd` → `"B*"`, `CloseFillStroke` → `"b"`, `CloseFillStrokeEvenOdd` → `"b*"`, `EndPath` → `"n"`
- `Clip` → `"W"`, `ClipEvenOdd` → `"W*"`
- `SetStrokeColorSpace` → `"CS"`, `SetFillColorSpace` → `"cs"`, `SetStrokeColor` → `"SC"`, `SetStrokeColorN` → `"SCN"`, `SetFillColor` → `"sc"`, `SetFillColorN` → `"scn"`, `SetStrokeGray` → `"G"`, `SetFillGray` → `"g"`, `SetStrokeRGB` → `"RG"`, `SetFillRGB` → `"rg"`, `SetStrokeCMYK` → `"K"`, `SetFillCMYK` → `"k"`
- `InvokeXObject` → `"Do"`, `Shading` → `"sh"`, `InlineImage` → `"BI/ID/EI"`
- `MarkedContentPoint` → `"MP"`, `MarkedContentPointProp` → `"DP"`, `BeginMarkedContent` → `"BMC"`, `BeginMarkedContentProp` → `"BDC"`, `EndMarkedContent` → `"EMC"`
- `BeginCompatibility` → `"BX"`, `EndCompatibility` → `"EX"`
- `Unknown(_)` → `"?"` (pdf_keyword), `"?<raw>"` (display_name)

---

## 체크포인트

### Checkpoint A — crate 스캐폴딩 + 인자 파서

1. `cargo new --bin crates/rpdf-cli --vcs none`
2. 루트 `Cargo.toml` workspace members 추가: `"crates/rpdf-cli"`
3. workspace dependencies에 `clap`, `serde_json`, `assert_cmd` 추가
4. `crates/rpdf-cli/Cargo.toml` 작성 (의존성 명세)
5. `main.rs`: `Cli` 구조체 + `Commands` enum (clap derive), 빈 match dispatch
6. `cargo build --bin rpdf` 성공
7. `cargo clippy -- -D warnings` 통과

**테스트**: `rpdf --help`가 3개 서브커맨드를 출력하는지 수동 확인.

### Checkpoint B — ContentStreamOperator 확장

1. `rpdf-core/src/types/content_stream.rs`에 `pdf_keyword()` + `display_name()` 추가
2. 단위 테스트 (`#[cfg(test)] mod internal_tests`):
   - `BT-1`: `BeginText.pdf_keyword() == "BT"`
   - `BT-2`: `EndText.display_name() == "ET"`
   - `BT-3`: `Unknown(b"foo".to_vec()).display_name() == "?foo"`
   - `BT-4`: 모든 비-Unknown 변형의 `pdf_keyword()`가 빈 문자열 아님 (리스트 순회)
3. `cargo test -p rpdf-core` 통과

### Checkpoint C — `rpdf info` 구현

1. `src/main.rs` (또는 `src/commands/info.rs`): `run_info()` 함수
2. `InfoOutput` 구조체 (Serialize 가능)
3. 인간 가독 출력 함수 (`format_info_human()`)
4. `--json`: `serde_json::to_string_pretty` 출력
5. 단위 테스트:
   - `CI-1`: `format_info_human` — title 있을 때 출력 형식
   - `CI-2`: `format_info_human` — 모든 필드 None일 때
   - `CI-3`: `InfoOutput` JSON 직렬화 — page_count 포함 여부
6. 통합 테스트 (`assert_cmd`):
   - `IT-C1`: `rpdf info examples/fw4-2024.pdf` → exit 0, 출력에 "Pages:" 포함
   - `IT-C2`: `rpdf info examples/fw4-2024.pdf --json` → exit 0, JSON 파싱 가능, `page_count` 필드 존재

### Checkpoint D — `rpdf dump-pages` 구현

1. `run_dump_pages()` 함수
2. `PageInfoOutput` 구조체 (`Serialize`)
3. 인간 가독 출력 + `--json`
4. `-p PAGE` 필터링 (범위 초과 시 에러)
5. 단위 테스트:
   - `CD-1`: `-p 0` → 1개 페이지만 출력
   - `CD-2`: 범위 초과 `-p 99` → 에러 메시지
   - `CD-3`: `PageInfoOutput` JSON 직렬화
6. 통합 테스트:
   - `IT-D1`: `rpdf dump-pages examples/fw4-2024.pdf` → 5개 페이지 출력
   - `IT-D2`: `rpdf dump-pages -p 0 examples/fw4-2024.pdf --json` → JSON array 길이 1

### Checkpoint E — `rpdf dump` 구현 + 통합 테스트 + 보고서 + PR

1. `run_dump()` 함수
2. `PdfObject` 직렬화 헬퍼 (`operand_to_json_value()`):
   - Boolean/Integer/Real: 해당 JSON 타입
   - Name: string (leading `/` 제거 여부는 v0.1에서 포함 유지)
   - String: hex 또는 UTF-8 시도 후 fallback `"<binary>"`
   - Array: 재귀
   - Dict/Stream/Reference: `"<complex>"`
3. 인간 가독 출력:
   - 각 연산: `  <operand1> <operand2> <operator_keyword>` (들여쓰기 2칸)
   - 페이지 헤더: `=== Page N (K ops) ===`
4. `--json` 출력: `{ "pages": [{ "index": N, "ops": [...] }] }`
5. 단위 테스트:
   - `CE-1`: `operand_to_json_value` — Integer, Real, Name, String
   - `CE-2`: `operand_to_json_value` — Array 재귀
   - `CE-3`: `operand_to_json_value` — Dict/Stream → `"<complex>"`
   - `CE-4`: 인간 가독 단일 연산 포맷 (`format_op_line`)
6. 통합 테스트:
   - `IT-E1`: `rpdf dump -p 0 examples/fw4-2024.pdf` → exit 0, "BT" 포함
   - `IT-E2`: `rpdf dump --json examples/pdfjs-basicapi.pdf` → exit 0, JSON 파싱 가능
   - `IT-E3`: `rpdf dump examples/pdfjs-tracemonkey.pdf` → exit 0 (14페이지, 전체 출력)
7. proptest: `arbitrary_input_never_panics_rpdf_dump` (fuzz bytes → 크래시 없음, exit 0 or 1)
8. 완료 보고서 작성 (`mydocs/working/task9-done.md`)
9. PR 작성 후 멈춤 (조건 B)

---

## 테스트 전략

### 단위 테스트

- `rpdf-core`: `pdf_keyword()` / `display_name()` 메서드 (내부 `#[cfg(test)]`)
- `rpdf-cli`: 출력 포맷팅 함수 (`format_info_human`, `format_op_line`, `operand_to_json_value`)
  - `rpdf-cli`는 binary crate이므로 `src/main.rs`에 inline `#[cfg(test)]` 또는 `src/` 내 모듈로 분리

### 통합 테스트

- `assert_cmd`로 실제 CLI 바이너리 실행
- `examples/` 기존 5개 PDF 전부 사용
- exit code, stdout 내용, JSON 파싱 가능 여부 검증
- `crates/rpdf-cli/tests/cli_tests.rs`에 배치

### proptest

- `crates/rpdf-cli/tests/cli_tests.rs`에 포함
- 임의 바이트 파일 → `rpdf dump` 실행 → panic/abort 없음 (exit 0 or 1 허용)

---

## 에러 처리 정책

- CLI에서 `anyhow::Error` 사용: `ParseError`는 `?`로 전파 → `anyhow::Error`로 변환
- 사용자 노출 메시지: `eprintln!("Error: {err}")` 후 exit code 1
- `-p PAGE` 범위 초과: "page N not found (total: M)" 메시지 + exit 1
- 파일 읽기 실패: `anyhow::Context`로 파일명 포함 메시지

---

## 외부 의존성 결정 (공개 API 확인 완료)

| 크레이트 | 버전 | 용도 | 확인 |
|--------|------|------|------|
| `clap` | 4.6.1 | 서브커맨드 인자 파싱, derive 매크로 | docs.rs 공개 API 확인 |
| `anyhow` | 1 (workspace) | CLI 에러 체인 | 이미 workspace에 존재 |
| `serde_json` | 1.0.149 | --json 출력 | docs.rs 공개 API 확인 |
| `assert_cmd` | 2.2.1 | CLI 통합 테스트 | docs.rs 공개 API 확인 |

색상 출력 크레이트(`anstream`, `colored`) — **v0.1 제외**.

---

## v0.1-parser-skeleton.md 갱신 항목

Task #9 항목에 `rpdf dump` vs `rpdf dump-pages` 명세를 추가:

```markdown
### #9 디버그 CLI 구현 (기존 #8)
- `rpdf info <file>` — 메타데이터 + 페이지 수 (인간 가독 + --json)
- `rpdf dump <file> [-p PAGE]` — content stream 연산자 시퀀스
- `rpdf dump-pages <file> [-p PAGE]` — 페이지 메타데이터 목록
- JSON 출력 옵션 (`--json`)
```
