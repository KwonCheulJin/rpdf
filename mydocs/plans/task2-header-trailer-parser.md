# Task #2 계획서 — PDF Header·Trailer 파서

**Issue**: #2
**브랜치**: `local/task2`
**마일스톤**: M010 v0.1 Parser Skeleton
**작성일**: 2026-05-03
**상태**: 승인됨

---

## 목적

PDF 파일의 Header와 Trailer를 raw byte 수준에서 파싱하여 타입 안전한 Rust 구조체로 변환한다.

이 타스크가 완료되면:
- PDF 버전(예: `1.7`, `2.0`)을 읽을 수 있다
- xref 테이블의 파일 내 오프셋(`startxref`)을 알 수 있다
- 카탈로그(Root), 전체 객체 수(Size) 등 Trailer 핵심 필드를 읽을 수 있다

> **parse_trailer 설계 결정 (2026-05-03)**
>
> 1. **파싱 전략**: trailer 영역을 직접 탐색 + 미니 파서로 4가지 타입만 파싱.
>    lopdf의 내부 파서 API(`parser::dictionary`)는 외부에 비공개이며,
>    `Document::load_mem`은 전체 PDF를 파싱하므로 합성 바이트 슬라이스 단위 테스트에 부적합.
>    trailer dict는 정수·간접 참조·이름·중첩 딕셔너리 4가지만 처리하면 충분 (~100줄).
>    Task #4에서 전체 객체 파서가 완성되면 이 미니 파서를 대체한다.
>
> 2. **탐색 알고리즘**: `search_end`에서 4096바이트 역방향 탐색 → `trailer` 키워드 →
>    `<<` 위치 탐색 → `<<`/`>>` 깊이 카운팅으로 매칭 `>>` 탐색.
>    단순화 정책: 괄호 문자열 `(...)` 안의 `<<`/`>>`는 무시;
>    단일 `<hex>`는 `<<`와 구분(2바이트 연속 여부).
>
> 3. **PdfTrailer 필드**: `size: u32`, `root: ObjectId`, `info: Option<ObjectId>`,
>    `prev: Option<u64>`. 이 4개만 Task #2 범위. `/ID`, `/Encrypt` 등은 무시(skip).
>
> 4. **lopdf 의존 추가 없음**: 미니 파서로 충분하므로 Cargo.toml 변경 없음.
>
> 5. **시그니처 변경**: `parse_trailer(data: &[u8], search_end: usize)` — `search_end`는
>    보통 `find_eof()` 결과. 내부에서 `parse_startxref(data, search_end)` 호출.

---

## 완료 기준

- [ ] `rpdf-parser` 크레이트 생성 및 워크스페이스 등록
- [ ] `parse_header()`, `find_eof()`, `parse_startxref()`, `parse_trailer()` 4개 함수 구현
- [ ] `examples/`의 샘플 5개 파일 모두 header + trailer 파싱 성공
- [ ] 단위 테스트 23개 + 통합 테스트 6개 + proptest 1개 = **총 30개 이상**
- [ ] `cargo nextest run --all` 통과
- [ ] `cargo clippy -- -D warnings` 경고 0개
- [ ] `cargo fmt --check` 통과

---

## 크레이트 구조

```
crates/
  rpdf-core/                 (기존 — 도메인 타입)
    src/
      lib.rs
      types/
        mod.rs               (신규)
        pdf_version.rs       (신규)
        object_id.rs         (신규)
  rpdf-parser/               (신규 — 파싱 로직)
    src/
      lib.rs
      header.rs
      trailer.rs
      error.rs
    tests/
      parser/
        header_tests.rs      (단위 10개+)
        trailer_tests.rs     (단위 13개+)
        integration_tests.rs (통합 6개)
        fuzz_tests.rs        (proptest 1개)
```

`rpdf-core`는 도메인 타입만 보유한다. 파싱 로직은 `rpdf-parser`에만 존재한다.
의존 방향: `rpdf-parser` → `rpdf-core` (역방향 의존 금지).

---

## 데이터 모델

### `rpdf-core`에 추가되는 타입

#### `PdfVersion` (enum)

알려진 버전은 열거형 변형으로, 미지원 버전은 `Other`로 수용한다.

```rust
// crates/rpdf-core/src/types/pdf_version.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PdfVersion {
    V1_0,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V2_0,
    /// 스펙에 없는 미래 버전 또는 비표준 버전 (예: major=10, minor=5)
    Other { major: u8, minor: u8 },
}

impl PdfVersion {
    pub fn from_bytes(major: u8, minor: u8) -> Self {
        match (major, minor) {
            (1, 0) => Self::V1_0,
            (1, 1) => Self::V1_1,
            (1, 2) => Self::V1_2,
            (1, 3) => Self::V1_3,
            (1, 4) => Self::V1_4,
            (1, 5) => Self::V1_5,
            (1, 6) => Self::V1_6,
            (1, 7) => Self::V1_7,
            (2, 0) => Self::V2_0,
            _ => Self::Other { major, minor },
        }
    }
}

impl std::fmt::Display for PdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_0 => write!(f, "1.0"),
            Self::V1_1 => write!(f, "1.1"),
            Self::V1_2 => write!(f, "1.2"),
            Self::V1_3 => write!(f, "1.3"),
            Self::V1_4 => write!(f, "1.4"),
            Self::V1_5 => write!(f, "1.5"),
            Self::V1_6 => write!(f, "1.6"),
            Self::V1_7 => write!(f, "1.7"),
            Self::V2_0 => write!(f, "2.0"),
            Self::Other { major, minor } => write!(f, "{}.{}", major, minor),
        }
    }
}
```

#### `ObjectId`

```rust
// crates/rpdf-core/src/types/object_id.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub number: u32,
    pub generation: u16,
}
```

### `rpdf-parser`에 추가되는 타입

#### `PdfHeader`

```rust
// crates/rpdf-parser/src/header.rs

pub struct PdfHeader {
    /// PDF 버전. PdfVersion::Other로 미지원 버전도 보존.
    pub version: PdfVersion,

    /// 파일 내에서 `%PDF-` 시그니처가 발견된 바이트 오프셋.
    /// 스펙은 첫 1 024 바이트 이내 어디든 허용한다.
    /// 대부분의 파일은 0이지만 0이 아닌 경우도 유효하다.
    pub byte_offset: usize,

    /// 헤더 직후 줄에 0x80 이상 바이트가 4개 이상 존재하는지.
    /// Adobe 스펙 권장: 이진 데이터를 포함하는 파일임을 알리기 위해 사용.
    pub has_binary_marker: bool,
}
```

#### `PdfTrailer` + `ParsedTrailer`

```rust
// crates/rpdf-parser/src/trailer.rs

pub struct PdfTrailer {
    /// /Size — xref 엔트리 총 개수 (필수)
    pub size: u32,
    /// /Root — 카탈로그 딕셔너리 ObjectId (필수)
    pub root: ObjectId,
    /// /Info — 문서 정보 딕셔너리 ObjectId (선택)
    pub info: Option<ObjectId>,
    /// /Prev — 이전 revision의 xref 오프셋 (점진적 업데이트 파일에서만 존재)
    pub prev: Option<u64>,
}

/// parse_trailer()의 반환 타입.
/// trailer 딕셔너리와 xref 테이블 오프셋을 함께 묶어 가독성을 높인다.
pub struct ParsedTrailer {
    pub trailer: PdfTrailer,

    /// `startxref` 키워드 다음에 기재된 값.
    /// xref 테이블(또는 xref 스트림)이 파일 내에서 시작하는 **절대 바이트 오프셋**.
    /// 이 값을 사용해 Task #3에서 xref 테이블을 읽는다.
    pub xref_offset: u64,
}
```

---

## 에러 타입

모든 엣지 케이스에 대응하는 변형을 사전 정의한다. 구현 중 분류 시간을 줄이기 위함이다.

```rust
// crates/rpdf-parser/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    // ── Header ──────────────────────────────────────────────────────────────

    /// `%PDF-` 시그니처를 첫 N바이트 이내에서 찾지 못함.
    /// 빈 파일, 비PDF 파일, 너무 긴 프리앰블이 원인.
    #[error("PDF 헤더(%PDF-)를 파일 첫 {searched_bytes}바이트 이내에서 찾을 수 없음")]
    HeaderNotFound { searched_bytes: usize },

    /// `%PDF-` 뒤의 버전 문자열이 "major.minor" 숫자 형식이 아님.
    /// 예: `%PDF-1.` (minor 없음), `%PDF-x.y` (비숫자)
    #[error("잘못된 PDF 버전 문자열: '{found}'")]
    InvalidVersion { found: String },

    // ── EOF / startxref ──────────────────────────────────────────────────────

    /// `%%EOF` 마커가 존재하지 않음.
    /// 손상된 파일이거나 전송 도중 잘린 파일.
    #[error("%%EOF 마커를 찾을 수 없음 (파일이 손상되었거나 잘렸을 수 있음)")]
    MissingEof,

    /// `%%EOF` 이전에 `startxref` 키워드가 없음.
    #[error("startxref 키워드를 찾을 수 없음")]
    MissingStartXref,

    /// `startxref` 다음의 값이 유효한 양의 정수가 아님.
    /// `found`가 "0"이면 오프셋이 0인 잘못된 파일.
    #[error("startxref 오프셋 값이 유효하지 않음: '{found}'")]
    InvalidStartXref { found: String },

    // ── Trailer 딕셔너리 ─────────────────────────────────────────────────────

    /// `%%EOF` 앞에서 `trailer` 키워드를 찾지 못함.
    /// xref 스트림 방식(PDF 1.5+) 파일이거나 손상된 파일.
    #[error("trailer 키워드를 찾을 수 없음")]
    MissingTrailer,

    /// trailer 딕셔너리의 필수 키(`/Size`, `/Root`)가 없음.
    #[error("trailer 필수 키 누락: {key}")]
    MissingRequiredKey { key: &'static str },

    /// `<num> <gen> R` 형식이 아닌 indirect reference.
    #[error("indirect reference 형식 오류 (기대: '<num> <gen> R'): '{found}'")]
    InvalidObjectRef { found: String },

    // ── 미지원 기능 ──────────────────────────────────────────────────────────

    /// xref 스트림 방식(PDF 1.5+) 파일.
    /// 이 형식은 `trailer` 키워드 없이 xref와 trailer를 스트림으로 통합한다.
    /// Task #3에서 지원 예정.
    #[error("xref 스트림 형식(PDF 1.5+)은 Task #3에서 지원 예정")]
    XrefStreamUnsupported,

    // ── 크기 제한 ────────────────────────────────────────────────────────────

    /// trailer 영역이 역방향 버퍼 한도를 초과.
    /// 정상적인 PDF에서는 발생하지 않으나 비정상 파일 방어용.
    #[error("trailer 영역이 버퍼 한도({limit_kb}KB)를 초과")]
    TrailerTooLarge { limit_kb: usize },

    // ── 일반 ─────────────────────────────────────────────────────────────────

    #[error("예상치 못한 형식: {0}")]
    UnexpectedFormat(String),
}
```

---

## 함수 시그니처

모든 함수는 `&[u8]`을 받는다. 파일 I/O와 파싱 로직을 분리하기 위함이다 —
테스트에서 파일을 열지 않고 바이트 리터럴로 바로 검증할 수 있다.

```rust
// crates/rpdf-parser/src/header.rs

/// 파일 앞부분 바이트에서 PDF Header를 파싱한다.
/// `data`는 파일 전체 또는 최소 첫 1 024 바이트여야 한다.
pub fn parse_header(data: &[u8]) -> Result<PdfHeader, ParseError>;


// crates/rpdf-parser/src/trailer.rs

/// 파일 끝부분 바이트에서 `%%EOF` 마커의 위치를 찾는다.
/// 반환값은 `data` 슬라이스 기준 바이트 오프셋이다 (파일 절대 오프셋 아님).
/// 점진적 업데이트 파일에서 `%%EOF`가 여러 개면 **가장 마지막** 것을 반환한다.
pub fn find_eof(data: &[u8]) -> Result<usize, ParseError>;

/// 파일 끝부분 바이트에서 `startxref` 키워드와 그 뒤에 오는 오프셋을 파싱한다.
/// 반환값은 xref 테이블의 파일 내 절대 바이트 오프셋이다.
pub fn parse_startxref(data: &[u8]) -> Result<u64, ParseError>;

/// 파일 끝부분 바이트에서 trailer 딕셔너리를 파싱한다.
/// `search_end`는 `find_eof()`의 반환값(%%EOF 시작 오프셋)을 넘긴다.
/// 내부에서 `parse_startxref(data, search_end)`를 호출해 xref_offset을 구한다.
/// `ParsedTrailer.xref_offset`에 startxref 오프셋이 포함된다.
pub fn parse_trailer(data: &[u8], search_end: usize) -> Result<ParsedTrailer, ParseError>;
```

---

## 파싱 전략

### Header 파싱

```
입력: data[0..min(1024, len)]
1. b"%PDF-" 시그니처를 선형 탐색 (최대 1 024 바이트 이내)
   → 없으면 HeaderNotFound { searched_bytes: min(1024, len) }
2. 시그니처 다음 바이트들을 "major.minor" 형태로 파싱
   - '.' 구분자 좌우가 모두 ASCII 숫자여야 함 (여러 자리도 허용)
   → 형식 불일치 시 InvalidVersion
3. 시그니처 발견 위치를 byte_offset으로 기록
4. 헤더 다음 줄에서 0x80 이상 바이트가 4개 이상이면 has_binary_marker = true
```

스펙(ISO 32000-1 §7.5.2): `%PDF-` 이전에 최대 1 024 바이트의 데이터가 올 수 있다.
즉 헤더 offset이 0이 아닐 수 있으며, 이는 완전히 유효한 파일이다.

### Trailer 파싱

PDF trailer는 파일 **끝**에 있다. 역방향 탐색이 필수다.

```
입력: data[max(0, len-4096)..len]  (최대 4 KB 역방향 버퍼)

1. find_eof()
   - b"%%EOF" 역방향 탐색
   - 여러 개면 마지막 위치 반환

2. parse_startxref()
   - find_eof() 결과 앞에서 b"startxref" 역방향 탐색
   - 다음 줄 숫자 파싱 → xref_offset
   - 0이거나 숫자가 아니면 InvalidStartXref

3. parse_trailer() 내부
   a. b"trailer" 키워드 역방향 탐색 (%%EOF 기준 앞쪽)
      → 없으면 xref stream 여부 확인 → XrefStreamUnsupported 또는 MissingTrailer
   b. "<<" ~ ">>" 딕셔너리 블록 추출 (중첩 << >> 고려)
   c. /Size <int>, /Root <int> <int> R, /Info <int> <int> R, /Prev <int> 파싱
      → /Size, /Root 없으면 MissingRequiredKey
```

**주의**: `trailer` 키워드는 xref 스트림(PDF 1.5+) 방식에서는 존재하지 않는다.
`trailer`가 없고 xref 스트림 시그니처가 보이면 `XrefStreamUnsupported`를 반환한다.
(Task #3에서 처리 예정이라는 안내 포함.)

---

## 파일 I/O 전략

### 현재 결정: 파일 전체 읽기 (`std::fs::read`)

```rust
let data = std::fs::read(path)?;
let header = parse_header(&data)?;
let trailer = parse_trailer(&data)?;
```

**근거**
- 구현이 단순하고 테스트가 쉽다 (함수가 `&[u8]`만 받으면 됨)
- Task #2의 목표는 파서 로직 검증이지 I/O 최적화가 아니다
- 샘플 5개 파일 크기: 88KB ~ 992KB — 전부 메모리에 올려도 무방하다

**한계**
- 수백 MB 파일에서는 메모리 낭비가 크다 (Header는 앞 1KB, Trailer는 끝 4KB만 필요)
- 파일 크기에 비례해 `read()` 시간이 선형 증가한다

**미래 마이그레이션 경로**

Task #8(회귀 테스트 인프라) 이후 대용량 파일 지원이 필요해지면
`BufReader + Seek` 방식으로 전환한다.

```rust
// 미래 시그니처 (Task #8 이후 검토)
pub fn parse_header_from<R: Read>(reader: &mut R) -> Result<PdfHeader, ParseError>;
pub fn parse_trailer_from<R: Read + Seek>(reader: &mut R) -> Result<ParsedTrailer, ParseError>;
```

현재 `&[u8]` 기반 API는 이 전환에서 내부 구현 변경만으로 충분하다.
공개 시그니처를 `&[u8]`로 유지한 채 내부에서 `Cursor<&[u8]>`를 사용하면
전환 시 외부 코드 수정이 없다.

---

## 엣지 케이스 — Must / Should / Could 분류

### Must — Task #2에서 반드시 처리

샘플 5개 파일 통과 또는 계약(contract)의 핵심에 해당하는 케이스.

| # | 케이스 | 동작 | 에러 변형 |
|---|--------|------|-----------|
| 1 | `%PDF-` 없음 | 에러 반환 | `HeaderNotFound` |
| 2 | 헤더가 offset 0이 아닌 곳에서 시작 (1 024B 이내) | 정상 파싱, `byte_offset` 기록 | — |
| 3 | `%%EOF` 없음 | 에러 반환 | `MissingEof` |
| 4 | `%%EOF` 뒤에 공백/개행 | 정상 파싱 (tolerate) | — |
| 5 | 점진적 업데이트 — `%%EOF` 여러 개 | 마지막 `%%EOF` 기준 파싱 | — |
| 6 | `startxref` 없음 | 에러 반환 | `MissingStartXref` |
| 7 | trailer `/Root` 없음 | 에러 반환 | `MissingRequiredKey { key: "/Root" }` |
| 8 | 비-ASCII 바이트가 헤더 앞에 위치 | 정상 파싱 (byte scan이 처리) | — |
| 9 | Windows `\r\n` 줄바꿈 | 정상 파싱 | — |
| 10 | xref 스트림 방식 파일 (PDF 1.5+) | 에러 반환 + Task #3 안내 | `XrefStreamUnsupported` |
| 11 | 빈 파일 (0 bytes) | 에러 반환 | `HeaderNotFound { searched_bytes: 0 }` |

### Should — Task #2 처리 시도, 시간 부족 시 별도 Issue 등록

| # | 케이스 | 동작 | 에러 변형 |
|---|--------|------|-----------|
| 12 | `%PDF-1.` 뒤에 숫자 없음 | 에러 반환 | `InvalidVersion` |
| 13 | 버전 major/minor가 여러 자리 (예: `%PDF-10.5`) | 정상 파싱 | `PdfVersion::Other` |
| 14 | `startxref` 값이 0 | 에러 반환 | `InvalidStartXref { found: "0" }` |
| 15 | trailer `/Size` 없음 | 에러 반환 | `MissingRequiredKey { key: "/Size" }` |
| 16 | `3 0 R` 형식이 아닌 indirect ref | 에러 반환 | `InvalidObjectRef` |

### Could — 별도 GitHub Issue 등록 후 백로그

| # | 케이스 | 동작 | 비고 |
|---|--------|------|------|
| 17 | 4 KB를 넘는 trailer 영역 | 에러 반환 | `TrailerTooLarge` — 실제로 거의 없음 |

---

## 테스트 전략

### 단위 테스트 (`tests/parser/`)

**header_tests.rs** (10개)
```rust
#[test] fn parse_pdf_1_7_header()
#[test] fn parse_pdf_2_0_header()
#[test] fn parse_pdf_1_4_header()
#[test] fn header_not_at_offset_zero()          // Must #2
#[test] fn detect_binary_marker_four_bytes()
#[test] fn reject_non_pdf_magic()               // Must #1
#[test] fn reject_incomplete_version_no_minor() // Should #12
#[test] fn reject_version_non_numeric()
#[test] fn header_with_crlf_line_ending()       // Must #9
#[test] fn header_beyond_1024_bytes_fails()     // Must #1 (경계)
```

**trailer_tests.rs** (13개)
```rust
#[test] fn find_eof_basic()
#[test] fn find_eof_with_trailing_newline()     // Must #4
#[test] fn find_eof_crlf()                      // Must #9
#[test] fn find_eof_missing_returns_error()     // Must #3
#[test] fn find_eof_multiple_takes_last()       // Must #5
#[test] fn parse_startxref_basic()
#[test] fn parse_startxref_missing_returns_error()   // Must #6
#[test] fn parse_startxref_zero_returns_error()      // Should #14
#[test] fn parse_trailer_basic_required_fields()
#[test] fn parse_trailer_with_info()
#[test] fn parse_trailer_with_prev()
#[test] fn parse_trailer_missing_root_returns_error()  // Must #7
#[test] fn parse_trailer_xref_stream_returns_unsupported() // Must #10
```

### 통합 테스트 (`tests/parser/integration_tests.rs`) (6개)

각 테스트의 시나리오, 사용 데이터, 검증 항목을 사전에 명시한다.

---

**IT-1: 표준 PDF 1.7 파일 — 4개 함수 모두 정상 동작**

```rust
#[test]
fn it1_standard_pdf17_all_four_functions() {
    // 파일: examples/fw4-2024.pdf (IRS W-4, PDF 1.7, 204KB)
    // 시나리오: parse_header / find_eof / parse_startxref / parse_trailer 연속 호출
    // 검증:
    //   - version == PdfVersion::V1_7
    //   - byte_offset == 0
    //   - xref_offset > 0
    //   - trailer.root.number > 0
    //   - trailer.size > 0
}
```

---

**IT-2: 헤더 오프셋 != 0 — `byte_offset`이 정확히 기록되는지 확인**

```rust
#[test]
fn it2_header_not_at_offset_zero() {
    // 데이터: 합성 (실제 fw4-2024.pdf 앞에 20바이트 프리앰블을 붙인 슬라이스)
    //   let mut data = b"SOME PREAMBLE DATA  ".to_vec();  // 20 bytes
    //   data.extend_from_slice(&fw4_bytes);
    // 시나리오: 헤더가 offset 20에서 시작하는 경우
    // 검증:
    //   - parse_header(&data).unwrap().byte_offset == 20
    //   - version은 프리앰블 없는 경우와 동일
}
```

---

**IT-3: 점진적 업데이트 파일 — `find_eof`가 마지막 `%%EOF` 반환**

```rust
#[test]
fn it3_incremental_update_last_eof_used() {
    // 데이터: 합성 — 유효한 PDF body 뒤에 %%EOF, 추가 revision, %%EOF 순서
    //   b"...%%EOF\nstartxref\n100\n%%EOF\nstartxref\n200\n%%EOF"
    // 시나리오: %%EOF가 3개 존재하는 점진적 업데이트 파일
    // 검증:
    //   - find_eof()가 마지막 %%EOF 위치 반환
    //   - parse_startxref()가 200 반환 (마지막 revision 기준)
}
```

---

**IT-4: 잘린 파일 — `ParseError` 적절히 반환**

```rust
#[test]
fn it4_truncated_file_returns_parse_error() {
    // 데이터: fw4-2024.pdf의 앞 500 바이트만 (파일 중간에서 잘림)
    //   let truncated = &fw4_bytes[..500];
    // 시나리오: %%EOF, startxref, trailer가 모두 없는 불완전 파일
    // 검증:
    //   - parse_header(&truncated) → Ok(...)  (헤더는 앞 1KB에 있으므로 성공)
    //   - parse_trailer(&truncated) → Err(ParseError::MissingEof)
}
```

---

**IT-5: `/Info` 딕셔너리 포함 파일 — `trailer.info`가 `Some`으로 추출**

```rust
#[test]
fn it5_trailer_info_field_extracted() {
    // 데이터: examples/fw4-2024.pdf 또는 examples/irs-f1040.pdf
    //   (IRS 폼 PDF는 대부분 /Info 딕셔너리를 포함함 — 실행 전 확인)
    //   확인 불가 시: 합성 trailer 바이트로 대체
    //     b"trailer\n<< /Size 10 /Root 1 0 R /Info 2 0 R >>\nstartxref\n100\n%%EOF"
    // 시나리오: trailer에 /Info 키가 있는 파일
    // 검증:
    //   - trailer.info == Some(ObjectId { number: N, generation: 0 })
    //
    // 비고: 한글 메타데이터 포함 PDF는 현재 샘플 없음.
    //       /Info ObjectId 추출을 검증하는 것으로 목적 달성.
    //       한글 포함 샘플은 Task #8(회귀 테스트 인프라) 단계에서 추가.
}
```

---

**IT-6: xref 스트림 방식 파일 — `XrefStreamUnsupported` 에러 반환**

```rust
#[test]
fn it6_xref_stream_pdf_returns_unsupported_error() {
    // 데이터: 합성 — xref 스트림 시그니처를 포함하는 최소 PDF 끝부분
    //   b"1 0 obj\n<< /Type /XRef /Size 5 >>\nstream\n...endstream\nendobj\n\
    //     startxref\n9\n%%EOF"
    //   (trailer 키워드 없이 xref 스트림이 있는 구조)
    //
    // 시나리오: PDF 1.5+ 스타일 xref 스트림 파일
    // 검증:
    //   - parse_trailer(&data) → Err(ParseError::XrefStreamUnsupported)
    //   - 에러 메시지에 "Task #3" 언급 포함
    //
    // 비고: pdfjs-annotation-border.pdf(PDF 1.5)가 xref stream을 쓸 수도 있음.
    //       실제 파일로 확인되면 합성 데이터 대신 파일 사용.
}
```

### proptest (`tests/parser/fuzz_tests.rs`) (1개)

임의 입력에서 패닉이 없음을 확인한다. 더 정교한 시나리오는 Task #8로 유보.

```rust
use proptest::prelude::*;

proptest! {
    /// 어떤 바이트 시퀀스도 panic을 유발해서는 안 된다.
    /// 파서는 항상 Ok(...) 또는 Err(...)로만 종료해야 한다.
    #[test]
    fn arbitrary_input_never_panics(data in proptest::collection::vec(any::<u8>(), 0..65536)) {
        let _ = parse_header(&data);
        let _ = parse_trailer(&data);
    }
}
```

`dev-dependencies`에 `proptest = "1"` 추가 필요 → 워크스페이스에 등록 (사람 승인됨).

---

## 워크스페이스 변경

### 루트 `Cargo.toml`

```toml
[workspace]
members = [
    "crates/rpdf-core",
    "crates/rpdf-parser",   # 추가
]

[workspace.dependencies]
rpdf-core = { path = "crates/rpdf-core" }  # 추가
thiserror = "2"
anyhow = "1"
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
proptest = "1"                              # 추가 (dev)
```

### `crates/rpdf-parser/Cargo.toml`

```toml
[package]
name = "rpdf-parser"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
rpdf-core.workspace = true
thiserror.workspace = true

[dev-dependencies]
proptest.workspace = true
```

---

## 구현 순서

1. `cargo new --lib crates/rpdf-parser --vcs none`
2. 루트 `Cargo.toml`에 `rpdf-parser` 멤버, `rpdf-core` path, `proptest` dev dep 등록
3. `rpdf-core/src/types/` 모듈 추가 (`PdfVersion` enum, `ObjectId`)
4. `rpdf-parser/src/error.rs` 작성 (위 enum 그대로)
5. `rpdf-parser/src/header.rs` — `parse_header()` 구현 + 단위 테스트 10개
6. `rpdf-parser/src/trailer.rs` — `find_eof()`, `parse_startxref()`, `parse_trailer()` 구현 + 단위 테스트 13개
7. `tests/parser/integration_tests.rs` — 5개 샘플 파일 통합 테스트 6개
8. `tests/parser/fuzz_tests.rs` — proptest 1개
9. `cargo nextest run --all` + `cargo clippy -- -D warnings` + `cargo fmt --check`
10. 완료 보고서 작성

---

## 예상 소요 시간

| 단계 | 예상 |
|------|------|
| 크레이트 생성 + 워크스페이스 연결 | 15분 |
| 도메인 타입 (`PdfVersion`, `ObjectId`) | 20분 |
| `error.rs` 작성 | 10분 |
| `parse_header()` + 단위 테스트 10개 | 45분 |
| `find_eof()` + `parse_startxref()` + 테스트 | 45분 |
| `parse_trailer()` + 테스트 | 60분 |
| 통합 테스트 6개 + proptest 1개 | 30분 |
| clippy/fmt 정리 + 완료 보고서 | 20분 |
| **합계** | **약 4시간** |

AI 페어 프로그래밍 기준 실제 소요 1~1.5시간 예상.
