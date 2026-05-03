# Task #2 계획서 — PDF Header·Trailer 파싱

**Issue**: #2
**브랜치**: `local/task2`
**마일스톤**: M010 v0.1 Parser Skeleton
**작성일**: 2026-05-03
**상태**: 초안 (승인 대기)

---

## 목표

PDF 파일에서 Header와 Trailer를 파싱하여 타입 안전한 Rust 구조체로 변환한다.
이 타스크가 완료되면 PDF 버전, Root 객체 오프셋, 기본 메타데이터 키를 읽을 수 있게 된다.

---

## 배경: PDF 파일 구조

PDF 파일은 네 영역으로 구성된다.

```
%PDF-1.7          ← Header
...objects...     ← Body
xref              ← Cross-reference table (Task #3)
%%EOF             ← EOF 마커

trailer           ← Trailer 딕셔너리
  << /Size 10
     /Root 1 0 R
     /Info 2 0 R
  >>
startxref
12345             ← xref 테이블 시작 오프셋
%%EOF
```

---

## 크레이트 구조

Task #2에서 파서 크레이트를 새로 추가한다.

```
crates/
  rpdf-core/          (기존 — 도메인 타입)
  rpdf-parser/        (신규 — 파싱 로직)
    src/
      lib.rs
      header.rs       ← PdfHeader 파서
      trailer.rs      ← PdfTrailer 파서
      error.rs        ← ParseError
    tests/
      parser/
        header_tests.rs
        trailer_tests.rs
```

`rpdf-core`는 도메인 타입만 보유하고 파싱 로직은 가지지 않는다.
`rpdf-parser`가 `rpdf-core`에 의존하는 방향을 유지한다.

---

## 데이터 모델

### `rpdf-core` — 도메인 타입 추가

```rust
// crates/rpdf-core/src/types/mod.rs (신규)
pub mod pdf_version;
pub mod object_id;

// crates/rpdf-core/src/types/pdf_version.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfVersion {
    pub major: u8,
    pub minor: u8,
}

impl PdfVersion {
    pub fn new(major: u8, minor: u8) -> Self { Self { major, minor } }
}

impl std::fmt::Display for PdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

// crates/rpdf-core/src/types/object_id.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub number: u32,
    pub generation: u16,
}
```

### `rpdf-parser` — 파싱 결과 타입

```rust
// crates/rpdf-parser/src/header.rs
pub struct PdfHeader {
    pub version: PdfVersion,
    /// 헤더 바로 다음 줄에 오는 고바이트(≥128) 주석 여부
    /// PDF 스펙: 이진 파일임을 표시하기 위해 권장됨
    pub has_binary_marker: bool,
}

// crates/rpdf-parser/src/trailer.rs
pub struct PdfTrailer {
    /// /Size — xref 엔트리 총 개수 (필수)
    pub size: u32,
    /// /Root — 카탈로그 딕셔너리 ObjectId (필수)
    pub root: ObjectId,
    /// /Info — 문서 정보 딕셔너리 ObjectId (선택)
    pub info: Option<ObjectId>,
    /// /Prev — 이전 xref 오프셋 (점진적 업데이트 시 존재)
    pub prev: Option<u64>,
}

// crates/rpdf-parser/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("PDF 헤더를 찾을 수 없음: 파일이 %PDF-로 시작하지 않음")]
    MissingHeader,
    #[error("잘못된 PDF 버전: {0}")]
    InvalidVersion(String),
    #[error("%%EOF 마커를 찾을 수 없음")]
    MissingEof,
    #[error("startxref 오프셋을 찾을 수 없음")]
    MissingStartXref,
    #[error("trailer 딕셔너리를 찾을 수 없음")]
    MissingTrailer,
    #[error("필수 키 누락: {0}")]
    MissingRequiredKey(&'static str),
    #[error("예상치 못한 형식: {0}")]
    UnexpectedFormat(String),
    #[error("IO 오류: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 파싱 전략

### Header 파싱

```
입력: 파일 처음 1024 바이트 (스펙: 헤더는 파일 첫 1KB 이내)
1. "%PDF-" 시그니처 탐색
2. 이어지는 숫자를 major.minor로 파싱
3. 다음 줄에 0x80 이상 바이트가 4개 이상이면 has_binary_marker = true
```

### Trailer 파싱

PDF trailer는 파일 **끝**에서부터 역방향으로 탐색한다.

```
1. 파일 끝 1024 바이트 읽기
2. "%%EOF" 탐색 (역방향)
3. "startxref" 탐색 (역방향)
4. startxref 다음 숫자를 xref 오프셋으로 파싱
5. "trailer" 키워드 이후 딕셔너리 << ... >> 파싱
   - /Size <int>
   - /Root <int> <int> R
   - /Info <int> <int> R  (있으면)
   - /Prev <int>          (있으면)
```

**메모리 제약**: 역방향 스캔 버퍼를 최대 4KB로 제한한다.
실제 trailer가 4KB를 넘는 파일은 매우 드물며, 넘으면 `UnexpectedFormat` 반환.

---

## 새로운/변경되는 API (공개 인터페이스)

```rust
// rpdf-parser 공개 API
pub fn parse_header(data: &[u8]) -> Result<PdfHeader, ParseError>;
pub fn parse_trailer(data: &[u8]) -> Result<(PdfTrailer, u64), ParseError>;
//                                            ^^^^^^^^^^^  ^^^
//                                            trailer      startxref 오프셋
```

함수 시그니처는 `&[u8]`을 받아서 파일 I/O와 파싱 로직을 분리한다.
(호출자가 파일을 읽어서 슬라이스를 전달 → 테스트에서 리터럴 바이트 배열 사용 가능)

---

## 엣지 케이스

| 케이스 | 예상 동작 |
|--------|----------|
| `%PDF-` 없음 | `ParseError::MissingHeader` |
| 버전이 숫자가 아님 (`%PDF-x.y`) | `ParseError::InvalidVersion` |
| `%%EOF` 없음 | `ParseError::MissingEof` |
| `startxref` 없음 | `ParseError::MissingStartXref` |
| trailer에 `/Root` 없음 | `ParseError::MissingRequiredKey("/Root")` |
| 점진적 업데이트 파일 (여러 `%%EOF`) | 마지막 `%%EOF` 기준으로 파싱 |
| 빈 파일 (0 bytes) | `ParseError::MissingHeader` |

---

## 테스트 전략

### 단위 테스트 (`tests/parser/`)

```rust
// header_tests.rs
#[test] fn parse_pdf_1_7_header()
#[test] fn parse_pdf_1_4_header()
#[test] fn reject_non_pdf_bytes()
#[test] fn detect_binary_marker()
#[test] fn reject_invalid_version_format()

// trailer_tests.rs
#[test] fn parse_basic_trailer()
#[test] fn parse_trailer_with_info()
#[test] fn parse_trailer_with_prev()
#[test] fn reject_missing_root()
#[test] fn reject_missing_eof()
```

### 통합 테스트 — 샘플 파일 검증

```rust
// tests/parser/integration_tests.rs
// examples/의 5개 파일 모두 header/trailer 파싱 성공 검증
#[test] fn parse_fw4_2024()          // PDF 1.7
#[test] fn parse_irs_f1040()         // PDF 1.7
#[test] fn parse_pdfjs_basicapi()    // PDF 1.6
#[test] fn parse_pdfjs_tracemonkey() // PDF 1.4
#[test] fn parse_pdfjs_annotation()  // PDF 1.5
```

**최소 테스트 목표**: 20개 이상

---

## 워크스페이스 변경

### Cargo.toml (루트)

```toml
[workspace]
members = [
    "crates/rpdf-core",
    "crates/rpdf-parser",  # 추가
]

[workspace.dependencies]
rpdf-core = { path = "crates/rpdf-core" }
thiserror = "2"
# ... 기존 유지
```

`rpdf-parser`의 `Cargo.toml`:

```toml
[package]
name = "rpdf-parser"
version = "0.1.0"
edition.workspace = true

[dependencies]
rpdf-core.workspace = true
thiserror.workspace = true
```

---

## 구현 순서

1. `cargo new --lib crates/rpdf-parser --vcs none`
2. 루트 `Cargo.toml`에 멤버 추가, `workspace.dependencies`에 `rpdf-core` path 등록
3. `rpdf-core`에 `PdfVersion`, `ObjectId` 도메인 타입 추가
4. `rpdf-parser/src/error.rs` 작성
5. `rpdf-parser/src/header.rs` — `parse_header()` 구현 + 단위 테스트
6. `rpdf-parser/src/trailer.rs` — `parse_trailer()` 구현 + 단위 테스트
7. `tests/parser/integration_tests.rs` — 5개 샘플 파일 통합 테스트
8. `cargo nextest run --all`, `cargo clippy -- -D warnings`, `cargo fmt --check`
9. 완료 보고서 작성

---

## 완료 기준 체크리스트

- [ ] `rpdf-parser` 크레이트 생성 및 워크스페이스 등록
- [ ] `PdfHeader` / `PdfTrailer` / `ObjectId` / `PdfVersion` 타입 정의
- [ ] `parse_header()` / `parse_trailer()` 구현
- [ ] 단위 테스트 10개 이상
- [ ] 통합 테스트: 5개 샘플 파일 모두 통과
- [ ] `cargo nextest run --all` 통과
- [ ] `cargo clippy -- -D warnings` 경고 0개
- [ ] `cargo fmt --check` 통과
