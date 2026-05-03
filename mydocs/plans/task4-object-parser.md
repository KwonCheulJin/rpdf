# Task #4 — PDF 객체 파서 계획서

**Issue**: v0.1 Task #4 / #6
**브랜치**: `local/task4`
**마일스톤**: M010 / v0.1
**상태**: 승인 대기
**작성일**: 2026-05-03

---

## 목표

PDF의 모든 기본 객체 타입을 파싱한다. Task #2의 미니 파서(`object_parser.rs`)를 확장하여 전체 PDF 객체 트리를 처리할 수 있게 하고, Task #5(xref 스트림)·Task #7(Document IR)의 기반을 만든다.

Task #3이 끝나면 "어디에 무엇이 있는지"(`XrefTable`)를 알게 됐다. Task #4가 끝나면 "그 위치에서 실제 객체를 읽을 수 있는" 상태가 된다.

---

## 범위 결정 (확정)

### 포함

| 타입 | 비고 |
|------|------|
| Boolean | `true`, `false` |
| Integer | `42`, `-3`, `+10` — `i64` |
| Real | `3.14`, `-.5`, `+1.0`; 지수 표기법(`1.5e2`) 미지원 |
| LiteralString | `(...)` — 이스케이프 디코딩 후 raw bytes |
| HexString | `<...>` — hex 디코딩 후 raw bytes |
| Name | `/Foo`, `/#20With` — `#HH` 이스케이프 디코딩 후 raw bytes |
| Array | `[...]`, 재귀, 깊이 제한 |
| Dictionary | `<<...>>`, 재귀, 깊이 제한 |
| Stream | `<< /Length N >> stream ... endstream` — 구조 인식 + raw bytes 추출 |
| Null | `null` |
| Reference | `N G R` — 간접 참조, 해결(resolve) 아님 |
| IndirectObject | `N G obj ... endobj` — 최상위 래퍼 |
| 깊이 제한 | `MAX_OBJECT_DEPTH = 50` — 배열·딕셔너리 재귀 보호 |

### 제외 (이유)

| 항목 | Task |
|------|------|
| 스트림 필터 디코딩 (`FlateDecode` 등) | #5 — `flate2` 의존 도입 시점에 통합 |
| 문자열 인코딩 해석 (PDFDocEncoding, UTF-16BE) | #7 — Document IR에서 의미 해석 |
| 간접 참조 해결 (`XrefTable`→오프셋→객체) | #7 — Document IR의 책임 |

---

## 설계 결정 사항

### 문자열 분리: `LiteralString` + `HexString`

단일 `String(Vec<u8>)` 대신 두 변형으로 분리한다.

이유:
1. **손실 없는 직렬화**: PDF 편집 후 저장 시 원본 형식(`(Hello)` vs `<48656C6C6F>`) 보존 필요. 단일 타입은 저장 시 어느 형식으로 쓸지 정보를 잃는다.
2. **디버깅 용이성**: "이 문자열이 hex였나 literal이었나"가 진단 단서가 될 수 있다.
3. **Task #7 분기 명확성**: LiteralString → PDFDocEncoding 디코딩 시도, HexString + BOM `<FEFF...>` → UTF-16BE 디코딩. 입력 단계에서 구분되어야 이 분기가 자연스럽다.

Helper 메서드를 통해 두 형식을 통합 처리할 수 있다:

```rust
impl PdfObject {
    /// LiteralString 또는 HexString의 raw bytes 반환. 다른 타입이면 None.
    pub fn as_string_bytes(&self) -> Option<&[u8]> { ... }

    pub fn string_format(&self) -> Option<StringFormat> { ... }
}

pub enum StringFormat { Literal, Hex }
```

### offset 타입: `usize`로 통일

`parse_object`, `parse_indirect_object`의 offset은 `usize`를 사용한다.

- 타겟 플랫폼: 64비트 전용 (Tauri 데스크톱)
- `usize`가 슬라이싱(`data[offset..]`)과 자연스러움; 변환 비용 제거
- 기존 Task #2~#3 함수들도 대부분 `usize` 입력을 사용 (일관성 유지)

### IndirectObject vs PdfObject 분리

`IndirectObject { id: ObjectId, object: PdfObject }`는 PdfObject 변형이 아니라 **객체에 ID와 세대를 부여하는 컨테이너**다. PDF 스펙의 "indirect object"는 파일 내 특정 위치에 ID와 함께 저장된 객체를 의미한다.

- `PdfObject::Reference(ObjectId)`: 다른 indirect object를 가리키는 참조값
- `IndirectObject`: `N G obj ... endobj` 구조 전체를 파싱한 결과

따라서:
- `parse_object` → `PdfObject` 반환
- `parse_indirect_object` → `IndirectObject` 반환

---

## 데이터 모델 변경 (`rpdf-core`)

### 새 파일: `crates/rpdf-core/src/types/object.rs`

```rust
use crate::types::ObjectId;

/// PDF 기본 객체 타입 (ISO 32000-1 §7.3).
///
/// 문자열은 raw bytes로 저장한다. 인코딩 해석(PDFDocEncoding, UTF-16BE)은
/// Task #7 Document IR에서 담당한다.
/// 스트림 raw bytes는 필터 적용 전 원본 데이터다. 필터 디코딩은 Task #5.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    // 스칼라
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),

    // 문자열류 (raw bytes — 출처 형식 보존)
    LiteralString(Vec<u8>), // 괄호 형식, 이스케이프 처리 후
    HexString(Vec<u8>),     // hex 디코딩 후
    Name(Vec<u8>),          // '/' 제외, '#HH' 이스케이프 처리 후

    // 컨테이너
    Array(Vec<PdfObject>),
    Dictionary(PdfDict),

    // 특수 (구조)
    Stream(PdfStream),
    Reference(ObjectId),    // 간접 참조 — 해결(resolve) 아님
}

impl PdfObject {
    /// LiteralString 또는 HexString의 raw bytes 반환. 다른 타입이면 None.
    pub fn as_string_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::LiteralString(bytes) | PdfObject::HexString(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn string_format(&self) -> Option<StringFormat> {
        match self {
            PdfObject::LiteralString(_) => Some(StringFormat::Literal),
            PdfObject::HexString(_) => Some(StringFormat::Hex),
            _ => None,
        }
    }
}

/// 문자열 출처 형식 — 저장 시 어느 형식으로 쓸지 결정할 때 사용.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringFormat { Literal, Hex }

/// PDF 딕셔너리 (`<< key value ... >>`).
///
/// 키는 Name raw bytes(Vec<u8>), 값은 PdfObject.
/// 삽입 순서를 유지하기 위해 Vec 사용.
/// 중복 키는 PDF 스펙(ISO 32000-1 §7.3.7)상 비허용이나 실제로 발생하므로
/// 두 가지 조회 메서드를 모두 제공한다.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PdfDict(pub Vec<(Vec<u8>, PdfObject)>);

impl PdfDict {
    /// 첫 번째 매칭 항목 반환 (ISO 32000-1 §7.3.7 권장).
    pub fn get(&self, key: &[u8]) -> Option<&PdfObject> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 마지막 매칭 항목 반환 (일부 PDF 처리기 호환).
    pub fn get_last(&self, key: &[u8]) -> Option<&PdfObject> {
        self.0.iter().rfind(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Vec<u8>, PdfObject)> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn len(&self) -> usize { self.0.len() }
}

/// PDF 스트림 객체 (`<< ... >> stream ... endstream`).
///
/// `data`는 필터 적용 전 raw bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfStream {
    pub dict: PdfDict,
    pub data: Vec<u8>,
}

/// 간접 객체 컨테이너 (`N G obj ... endobj`).
///
/// PDF 스펙의 "indirect object"는 파일 내 특정 위치에 ID와 함께 저장된 객체다.
/// PdfObject의 한 변형이 아니라, 객체에 ID와 세대를 부여하는 최상위 래퍼다.
#[derive(Debug, Clone, PartialEq)]
pub struct IndirectObject {
    pub id: ObjectId,
    pub object: PdfObject,
}
```

### 수정: `crates/rpdf-core/src/types/mod.rs`

```rust
pub mod object;
pub mod object_id;
pub mod pdf_version;
pub mod xref;

pub use object::{IndirectObject, PdfDict, PdfObject, PdfStream, StringFormat};
pub use object_id::ObjectId;
pub use pdf_version::PdfVersion;
pub use xref::{XrefEntry, XrefTable};
```

---

## API 설계 (`rpdf-parser`)

### `crates/rpdf-parser/src/object_parser.rs` 확장

기존 `pub(crate)` 헬퍼들을 유지하면서 새 공개 함수와 헬퍼를 추가한다.

#### 새로 추가할 공개 함수

```rust
/// `data[offset..]`에서 PDF 객체 하나를 파싱한다.
/// offset: usize (64비트 플랫폼 전용, usize == u64 범위 보장)
pub fn parse_object(data: &[u8], offset: usize) -> Result<(PdfObject, usize), ParseError>

/// `data[offset..]`에서 간접 객체(`N G obj ... endobj`)를 파싱한다.
/// XrefEntry::InUse { offset, .. }의 offset을 as usize로 변환해 넘긴다.
pub fn parse_indirect_object(data: &[u8], offset: usize) -> Result<(IndirectObject, usize), ParseError>
```

#### 내부 재귀 함수

```rust
fn parse_object_with_depth(data: &[u8], offset: usize, depth: usize)
    -> Result<(PdfObject, usize), ParseError>
```

depth 증가 규칙:
- Array, Dictionary 진입 시만 `depth + 1` 전달
- 스칼라(Boolean, Integer, Real, String, Name, Null, Reference)는 depth 변경 없음
- Stream 헤더 dict는 일반 dict로 처리 (depth + 1); raw bytes는 파싱 아님 — depth 무관
- `depth >= MAX_OBJECT_DEPTH`이면 즉시 `ObjectTooDeep` 반환

#### 새 헬퍼 함수

```rust
/// 화이트스페이스와 주석(`%` ~ 줄 끝)을 건너뛴다.
/// 다음 비-화이트스페이스 위치까지 소비된 바이트 수 반환.
pub(crate) fn skip_whitespace_and_comments(data: &[u8]) -> usize
```

표준 화이트스페이스 6개: `\0`, `\t`, `\n`, `\x0C` (`\f`), `\r`, `' '`
주석: `%` 부터 `\n` 또는 `\r` 이전까지

기존 `skip_whitespace`는 주석을 처리하지 않는다. 객체 파서는 모든 토큰 경계에서 `skip_whitespace_and_comments`를 사용한다.

#### 상수

```rust
pub(crate) const MAX_OBJECT_DEPTH: usize = 50;
```

### `crates/rpdf-parser/src/lib.rs` 공개 추가

```rust
pub use object_parser::{parse_indirect_object, parse_object};
```

---

## 에러 변형 추가 (`rpdf-parser/src/error.rs`)

```rust
/// 배열 또는 딕셔너리 중첩이 허용 깊이(MAX_OBJECT_DEPTH)를 초과함.
#[error("객체 중첩 깊이 초과: {max_depth}")]
ObjectTooDeep { max_depth: usize },

/// 객체 파싱 실패 (예: 예상치 못한 토큰, 잘못된 이름 이스케이프).
#[error("오프셋 {offset}에서 잘못된 객체: {found:?}")]
InvalidObject { offset: usize, found: String },

/// 스트림 구조 오류 (`stream` 키워드 없음, `endstream` 없음, `/Length` 없음 또는 간접 참조).
#[error("스트림 구조 오류 (오프셋 {offset}): {reason}")]
MalformedStream { offset: usize, reason: String },

/// `endobj` 키워드 없음.
#[error("오프셋 {offset}에서 endobj 없음")]
MissingEndobj { offset: usize },
```

---

## 구현 세부사항

### Boolean 파싱

`true` → `PdfObject::Boolean(true)`, `false` → `PdfObject::Boolean(false)`.

경계 조건: `truefoo`는 Boolean이 아님 — 뒤에 이름 문자가 오면 거부. `null`도 동일.

### Number 파싱

PDF 숫자 문법 (ISO 32000 §7.3.3):
- 정수: `[+|-]?[0-9]+` → `PdfObject::Integer(i64)`
- 실수: `[+|-]?[0-9]*\.[0-9]*` → `PdfObject::Real(f64)` (소수점 앞 또는 뒤에 숫자 없어도 됨: `.5`, `3.`)
- 지수 표기법(`1.5e2`) 미지원 — PDF 스펙 외; `e` 앞까지만 숫자로 인식

파싱 전략:
1. 부호 확인 (`+`, `-`)
2. 정수 부분 파싱
3. `.` 있으면 실수, 없으면 정수

### String 파싱

#### LiteralString `(...)`

이스케이프 처리 (ISO 32000 §7.3.4.2):
- `\n` → 0x0A, `\r` → 0x0D, `\t` → 0x09
- `\b` → 0x08, `\f` → 0x0C
- `\\` → `\`, `\(` → `(`, `\)` → `)`
- `\ddd` — octal 1~3 자리 → 해당 바이트
- `\<newline>` — 줄 계속(line continuation), 결과에서 제거
- 그 외 `\X` → `X` (백슬래시 무시)

중첩 괄호: 짝이 맞으면 그대로 포함 (기존 `skip_literal_string`과 동일한 depth 카운팅).

#### HexString `<...>`

- 공백·탭·개행 무시
- 16진수 두 자리씩 → 바이트
- 홀수 자리면 마지막 자리 뒤에 `0` 추가 (ISO 32000 §7.3.4.3)
- `>` 전에 비16진수 문자 → `InvalidObject`

### Name 파싱

- `/` 이후 이름 문자열; 공백·구분자에서 종료
- `#HH` → 해당 바이트 (ISO 32000 §7.3.5 — PDF 1.2+)
- `/` 단독 → 빈 Name `vec![]` (유효)
- `#` 뒤 비16진수 또는 `#`로 끝남 → `InvalidObject`
- 결과에 `/` 미포함

### Array 파싱

- `[` ~ `]` 사이 값들의 시퀀스
- 빈 배열 `[]` 허용
- `parse_object_with_depth(data, pos, depth + 1)` 재귀 호출

### Dictionary 파싱

- `<<` ~ `>>` 사이 key-value 쌍
- 키는 반드시 Name; 비Name 키 → `InvalidObject`
- 중복 키 허용, 순서 보존 (Vec에 그대로 추가)
- `parse_object_with_depth(data, pos, depth + 1)` 재귀 호출

### Stream 파싱

```
<< /Length N ... >> stream <newline> <N bytes> endstream
```

1. 딕셔너리 파싱 (`depth + 1`)
2. `skip_whitespace_and_comments` 후 `stream` 키워드 확인
3. 줄바꿈 소비: `\r\n` 또는 `\n`만 허용 (`\r` 단독 → `MalformedStream`)
4. `/Length` 값 읽기:
   - 없으면 `MalformedStream { reason: "missing /Length" }`
   - `Reference`이면 `MalformedStream { reason: "indirect /Length not resolvable at parse time" }`
5. N 바이트 복사 → `PdfStream::data`
6. 선택적 공백 + `endstream` 확인

**제약**: `/Length`가 간접 참조인 PDF는 Task #4에서 지원하지 않는다. Task #7에서 resolve 후 재파싱 가능.

### IndirectObject 파싱

```
N G obj <whitespace> <value> <whitespace> endobj
```

1. `N`, `G` 정수 파싱 → `ObjectId`
2. `skip_whitespace_and_comments` + `obj` 키워드 확인
3. `parse_object_with_depth(data, pos, 0)` 호출
4. `skip_whitespace_and_comments` + `endobj` 확인 → 없으면 `MissingEndobj`

---

## 깊이 제한

`MAX_OBJECT_DEPTH = 50`. 컨테이너(Array, Dictionary, Stream 헤더 dict)에 진입할 때만 depth를 1씩 증가시킨다.

```rust
fn parse_object_with_depth(data: &[u8], offset: usize, depth: usize)
    -> Result<(PdfObject, usize), ParseError> {
    if depth >= MAX_OBJECT_DEPTH {
        return Err(ParseError::ObjectTooDeep { max_depth: MAX_OBJECT_DEPTH });
    }
    // ...
}
```

XrefChainTooDeep와의 일관성: depth 검사를 컨테이너 진입 첫 번째 검사로 배치.

---

## 테스트 전략

### 파일 배치

```
crates/rpdf-parser/tests/parser/
├── object_tests.rs     ← Task #4 단위 테스트 (단일 파일로 시작)
└── mod.rs              ← object_tests 등록
```

150개 이상이 되면 타입별 분리 검토. 처음부터 분리하면 `mod.rs` 관리 부담만 늘어난다.

### 예상 단위 테스트 수 (~100개)

체크포인트별 분배:

| 체크포인트 | 타입 | 예상 수 |
|-----------|------|---------|
| B (스칼라) | Boolean·Null 각 3개, Integer 8개, Real 10개, LiteralString 15개, HexString 10개, Name 10개 | ~約60개 |
| C (컨테이너) | Array 10개, Dictionary 15개, Reference 5개 | ~30개 |
| D (스트림 + IndirectObject) | Stream 8개, IndirectObject 8개 | ~16개 |

합계: 약 106개 단위 테스트 + proptest + 통합 테스트 확장

### 에러 변형 도달 가능성 확인 (Task #3 교훈)

4개 변형 모두 테스트로 도달:
- `ObjectTooDeep`: 깊이 51짜리 배열
- `InvalidObject`: `#GG` 이름, 비Name 딕셔너리 키 등
- `MalformedStream`: `endstream` 없음, `/Length` 없음, `\r` 단독 줄바꿈
- `MissingEndobj`: `endobj` 없는 간접 객체

### proptest

```rust
fn arbitrary_input_never_panics_parse_object(data in vec(...), offset in 0..65536usize) {
    let _ = parse_object(&data, offset.min(data.len()));
    let _ = parse_indirect_object(&data, offset.min(data.len()));
}
```

### 통합 테스트: 기존 IT-1, IT-3, IT-5 확장

IT-7, IT-8, IT-9로 신설하지 않고 기존 IT에 `parse_indirect_object` 호출을 추가한다 (전체 파이프라인 검증 의도 유지).

```rust
// IT-1에 추가 (pdfjs-tracemonkey.pdf — /Root 객체 확인)
let root_entry = parsed_xref.table.get(root_id.number).unwrap();
if let XrefEntry::InUse { offset, .. } = root_entry {
    let (root_obj, _) = parse_indirect_object(data, *offset as usize).unwrap();
    // /Root는 Catalog 딕셔너리여야 함
    assert!(matches!(root_obj.object, PdfObject::Dictionary(_)));
    let dict = match &root_obj.object { PdfObject::Dictionary(d) => d, _ => panic!() };
    let type_val = dict.get(b"Type").unwrap();
    assert_eq!(type_val.as_name_bytes(), Some(b"Catalog".as_ref()));
}
```

---

## 체크포인트 계획

### Checkpoint A — 데이터 타입 정의
- `rpdf-core/src/types/object.rs` 생성
- `rpdf-core/src/types/mod.rs` 업데이트
- `cargo build` 통과

### Checkpoint B — 스칼라 파서 (자가 검토 포함)
- `skip_whitespace_and_comments` 헬퍼 정의
- Boolean, Null, Integer, Real, LiteralString, HexString, Name 파싱
- 단위 테스트 ~60개
- **자가 검토**: `+0`, `-0`, `.`, `3.`, `/.`, `#`로 끝나는 Name, `truefoo`, `nullX`

### Checkpoint C — 컨테이너 파서 (자가 검토 포함)
- Array, Dictionary, Reference 파싱
- 깊이 제한 (`MAX_OBJECT_DEPTH = 50`)
- 단위 테스트 ~30개
- **자가 검토**: 빈 배열/딕셔너리, 중첩 51단계, 중복 키, 비Name 딕셔너리 키

### Checkpoint D — 스트림 + IndirectObject (자가 검토 포함)
- Stream 구조 인식 + raw bytes 추출
- IndirectObject 래퍼
- 단위 테스트 ~16개
- **자가 검토**: `/Length` 간접 참조, `\r` 단독 줄바꿈, `endobj` 없음, `endstream` 없음

### Checkpoint E — 통합 및 마무리
- `lib.rs` 공개 추가
- IT-1, IT-3, IT-5에 `parse_indirect_object` 검증 추가
- proptest 추가
- `cargo clippy -- -D warnings`, `cargo fmt --check` 통과

---

## 파일 변경 요약

### 새로 추가
- `crates/rpdf-core/src/types/object.rs`
- `crates/rpdf-parser/tests/parser/object_tests.rs`

### 수정
- `crates/rpdf-core/src/types/mod.rs` — object 모듈 공개
- `crates/rpdf-parser/src/object_parser.rs` — `parse_object`, `parse_indirect_object`, `skip_whitespace_and_comments` 추가; 기존 헬퍼 유지
- `crates/rpdf-parser/src/error.rs` — 에러 변형 4개 추가
- `crates/rpdf-parser/src/lib.rs` — `parse_object`, `parse_indirect_object` 공개
- `crates/rpdf-parser/tests/parser/mod.rs` — `object_tests` 등록
- `crates/rpdf-parser/tests/parser/integration_tests.rs` — IT-1, IT-3, IT-5 확장

---

## 참고 자료

- ISO 32000-1:2008 §7.3 (Objects)
- 관련 Issue: #6
- Task #3 계획서: `mydocs/plans/task3-xref-parser.md`
