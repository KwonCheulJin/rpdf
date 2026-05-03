# Task #3 — Xref 테이블 파싱 계획서

**Issue**: v0.1 Task #3
**브랜치**: `local/task3`
**마일스톤**: M010 / v0.1
**상태**: 승인 대기
**작성일**: 2026-05-03

---

## 목표

`startxref`가 가리키는 오프셋으로 이동하여 전통적 xref 테이블을 파싱하고, `/Prev` 체인을 따라 점진적 업데이트 PDF의 완전한 객체 위치 맵(`XrefTable`)을 반환한다.

Task #2에서는 "EOF → startxref → trailer" 역방향 탐색만 완성했고, xref 오프셋으로는 아직 이동하지 않는다. Task #3이 끝나면 파일 내 모든 객체의 파일 오프셋을 알 수 있는 상태가 된다. Task #4에서 이 맵을 활용해 실제 객체를 읽는다.

---

## 범위 결정 (검토 확정)

### xref 스트림 — 미포함 (Task #5로 이동)

xref 스트림 파싱(PDF 1.5+)은 Task #3에 포함하지 않는다.

| 이유 | 설명 |
|------|------|
| 선행 의존 | 스트림 본문 파싱에 Task #4 객체 파서가 필요함 |
| 압축 객체 항목 | Type 2 entry는 `/ObjStm` 처리를 동반 — Task #5 영역 |
| 학습 순서 | 전통 xref를 먼저 완전히 이해한 뒤 압축 변형 다루는 것이 자연스러움 |
| v0.1 목표 | 마일스톤 목표는 "PDF 구조를 이해하는 툴체인"이지 모든 PDF 처리가 아님 |

xref 스트림 PDF(`fw4-2024.pdf` 등)는 Task #3 이후에도 `XrefStreamUnsupported { xref_offset }` 반환. 에러에 `xref_offset` 필드를 추가해 명확한 진단 정보를 제공한다.

마일스톤 재배치 (`v0.1-parser-skeleton.md` 반영 완료):
```
#3 전통적 xref 테이블 + chain 순회  (이 작업)
#4 PDF 객체 파서
#5 xref 스트림 (신규, #4 선행 필요)
#6 Content stream 파서
#7 Document IR
#8 디버그 CLI
#9 회귀 테스트 인프라
```

### TrailerTooLarge 보강 — 미포함 (별도 Issue 등록)

Task #2의 `TrailerTooLarge { found_bytes }` 보강은 Task #3 범위 밖이다. 별도 GitHub Issue로만 등록한다.

### object_parser.rs — Task #3에서 절대 수정 금지

`object_parser.rs`는 Task #4 객체 파서 확장의 대상이다. Task #3에서는 어떤 수정도 하지 않는다. xref 파싱 중 추가 헬퍼가 필요하면 `xref.rs` 내부의 private 함수로 만들고, Task #4에서 통합 가치가 있으면 그때 이동한다.

---

## 데이터 모델 변경 (`rpdf-core`)

### 새 파일: `crates/rpdf-core/src/types/xref.rs`

```rust
use std::collections::BTreeMap;

/// PDF 교차 참조 테이블.
/// 객체 번호 → 파일 내 위치 또는 상태 매핑.
#[derive(Debug, Clone, PartialEq)]
pub struct XrefTable {
    entries: BTreeMap<u32, XrefEntry>,
}

impl XrefTable {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new() }
    }

    /// 항목 추가. 이미 있으면 덮어쓰지 않는다 (최신 값 우선).
    /// xref chain 병합 시 가장 최근 섹션을 먼저 삽입하면
    /// 이후 `insert_if_absent` 호출은 자동으로 무시된다.
    pub fn insert_if_absent(&mut self, obj_num: u32, entry: XrefEntry) {
        self.entries.entry(obj_num).or_insert(entry);
    }

    pub fn get(&self, obj_num: u32) -> Option<&XrefEntry> {
        self.entries.get(&obj_num)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// xref 항목 하나의 상태.
#[derive(Debug, Clone, PartialEq)]
pub enum XrefEntry {
    /// 사용 중 — 파일 오프셋에 객체가 위치함.
    InUse { offset: u64, generation: u16 },

    /// 해제됨 — 이 번호의 다음 사용 가능한 객체 번호와 세대를 가리킴.
    Free { next_free_obj_num: u32, generation: u16 },

    /// 객체 스트림 내 압축됨 (PDF 1.5+ xref 스트림 전용).
    /// Task #3에서는 생성하지 않음. Task #5에서 활용.
    Compressed { obj_stm_num: u32, index: u32 },
}
```

### `types/mod.rs` 수정

```rust
pub mod xref;
pub use xref::{XrefEntry, XrefTable};
```

---

## 파서 설계 (`rpdf-parser`)

### 공개 API

```rust
/// PDF xref 체인 전체를 파싱해 `ParsedXref`를 반환한다.
///
/// `xref_offset`은 `parse_startxref`가 반환한 값을 그대로 전달한다.
/// 전통적 xref 테이블과 `/Prev` chain 순회를 처리한다.
/// xref 스트림 형식은 `XrefStreamUnsupported { xref_offset }` 에러를 반환한다.
pub fn parse_xref(
    data: &[u8],
    xref_offset: u64,
) -> Result<ParsedXref, ParseError>

/// xref 파싱 결과.
#[derive(Debug)]
pub struct ParsedXref {
    /// 병합된 xref 테이블. 동일 객체 번호가 여러 섹션에 있으면 최신 값 우선.
    pub table: XrefTable,

    /// 가장 최근 섹션(xref_offset 위치)의 trailer.
    /// /Root, /Info 등 문서 수준 메타를 가짐.
    pub trailer: PdfTrailer,

    /// 순회한 xref 섹션 정보 (디버그·진단용).
    /// sections.len() = incremental update 횟수.
    /// Task #8 디버그 CLI에서 활용.
    pub sections: Vec<XrefSectionInfo>,
}

/// 단일 xref 섹션의 위치 정보 (디버그용).
#[derive(Debug)]
pub struct XrefSectionInfo {
    pub offset: u64,
    pub entry_count: usize,
}
```

### 호출 흐름 (사용 예시)

```rust
let eof_offset  = find_eof(&data)?;
let xref_offset = parse_startxref(&data, eof_offset)?;
let parsed_xref = parse_xref(&data, xref_offset)?;

// 객체 5의 위치 조회
match parsed_xref.table.get(5) {
    Some(XrefEntry::InUse { offset, .. }) => { /* offset 위치에서 객체 읽기 */ }
    Some(XrefEntry::Free { .. })          => { /* 해제된 객체 */ }
    None                                  => { /* 알 수 없는 객체 */ }
}

// /Root 참조로 Catalog 찾기
let root_id = parsed_xref.trailer.root; // ObjectId
```

### 내부 구조 (`crates/rpdf-parser/src/xref.rs`)

```
parse_xref(data, xref_offset)
  └─ parse_xref_chain(data, xref_offset)
       └─ [loop] parse_xref_section(data, current_offset)
             └─ parse_traditional_xref(data, offset)
                  ├─ parse_xref_subsection_header(...)
                  ├─ parse_xref_entries(...)
                  └─ parse_trailer_at(data, start_offset)  // xref 직후 순방향
```

모든 보조 함수는 `pub(crate)` 또는 `fn`(모듈 내부). 공개 API는 `parse_xref`와 `ParsedXref`·`XrefSectionInfo` 타입뿐이다.

### trailer 파싱 함수 두 가지

xref 위치에서 trailer를 파싱하는 경로는 기존 `parse_trailer`와 다르다. 두 함수를 분리 유지한다.

| 함수 | 위치 | 시그니처 | 사용처 |
|------|------|----------|--------|
| `parse_trailer` | `trailer.rs` | `(data, search_end)` | 파일 끝에서 역방향 탐색 (Task #2) |
| `parse_trailer_at` | `xref.rs` | `(data, start_offset)` | xref 섹션 직후 순방향 파싱 (Task #3) |

두 함수는 `object_parser` 헬퍼(`parse_indirect_ref`, `skip_value` 등)를 공유한다. 인터페이스가 다른 이유는 사용 컨텍스트가 다르기 때문이다. `parse_trailer`는 변경 없이 유지한다.

### xref 스트림 감지 책임

`parse_trailer`(Task #2)는 이미 `is_xref_stream` 헬퍼로 감지하고 `XrefStreamUnsupported`를 반환한다. `parse_xref`(Task #3)도 동일 헬퍼를 사용해 같은 검사를 추가한다.

```
[xref 스트림 PDF 호출 경로]
parse_trailer(data, eof) → is_xref_stream 감지 → XrefStreamUnsupported { xref_offset }
parse_xref(data, xref_offset) → is_xref_stream 감지 → XrefStreamUnsupported { xref_offset }
```

두 함수가 독립적으로 감지하므로 어느 경로로 호출해도 동일 에러를 반환한다. IT-6에서 양쪽 모두 검증한다.

### `lib.rs` 추가

```rust
mod xref;
pub use xref::{ParsedXref, XrefSectionInfo, parse_xref};
```

---

## 전통 xref 테이블 파싱

### 포맷

```
xref                              ← 키워드
0 8                               ← 섹션 헤더: 시작 객체 번호  항목 수
0000000000 65535 f \r\n           ← 해제 항목 (정확히 20바이트)
0000000009 00000 n \r\n           ← 사용 중 항목
0000000058 00000 n \r\n
...
trailer
<< /Size 8 /Root 1 0 R /Info 4 0 R >>
```

섹션이 여러 개 연속될 수 있음:
```
xref
0 1
0000000000 65535 f
3 2
0000000100 00000 n
0000000250 00000 n
trailer
<< /Size 5 ... >>
```

### 항목 포맷 (20바이트 고정)

```
oooooooooo ggggg k EOL
│          │     │ └── \r\n 또는 ' '\n (2바이트)
│          │     └── n(in-use) 또는 f(free)
│          └── 5자리 세대 번호 (앞 공백 포함)
└── 10자리 오프셋 또는 다음 해제 번호
```

바이트 위치:
- `data[0..10]` → `offset_or_next_free: u64`
- `data[10]` → 공백
- `data[11..16]` → `generation: u16`
- `data[16]` → 공백
- `data[17]` → `n` 또는 `f`
- `data[18..20]` → EOL

### EOL 정책

- **정상**: `\r\n` 또는 ` \n`(공백+줄바꿈) 각 2바이트 → 항목 총 20바이트 고정
- **비표준 줄바꿈** (`\n` 단독, `\r` 단독): `MalformedXref { reason: "비표준 항목 EOL" }` 반환
- 19바이트나 21바이트 항목도 `MalformedXref`
- 향후 관용 처리가 필요한 실 파일이 발견되면 별도 Issue로 등록 후 도입 검토

### 섹션 헤더 줄바꿈

섹션 헤더(`first count`) 다음 줄바꿈은 `\r\n` 또는 `\n` 모두 허용한다. 첫 항목 시작 위치는 "헤더 줄바꿈 직후"로 계산해야 한다. `\n` 단독인지 `\r\n`인지에 따라 1바이트 어긋나지 않도록 `parse_xref_subsection_header`가 소비한 끝 위치를 정확히 반환한다.

---

## Xref Chain 순회 알고리즘

### 에러 우선순위

순환 참조 검사(`visited` set)가 깊이 검사(`depth`)보다 먼저 수행된다.

- **`XrefChainCycle`**: 동일 오프셋 재방문 시 즉시 발생. 순환 chain은 항상 이 에러로 보고된다.
- **`XrefChainTooDeep`**: 모든 오프셋이 서로 다르지만 깊이가 `MAX_XREF_CHAIN_DEPTH`를 초과할 때만 발생.

```rust
/// xref chain의 최대 허용 깊이.
///
/// 일반 PDF는 1-3 단계, 형식 채우기 PDF도 10-50 단계 이내.
/// 100을 초과하는 chain은 비정상 또는 손상된 파일로 간주.
const MAX_XREF_CHAIN_DEPTH: usize = 100;
```

### 순회 방향 및 병합 규칙

가장 최근 섹션(파일 끝에 가까울수록)이 우선한다. 순회는 최신 → 오래된 순서.

```
parse_xref_chain(data, start_offset):
  table    = XrefTable::new()
  sections = Vec::new()
  visited  = HashSet::new()   // 순환 참조 방지
  trailer  = None
  depth    = 0

  current = start_offset
  LOOP:
    if depth >= MAX_XREF_CHAIN_DEPTH → XrefChainTooDeep { max_depth: MAX_XREF_CHAIN_DEPTH }
    if visited.contains(current) → XrefChainCycle { offset: current }
    visited.insert(current)
    depth += 1

    (entries, section_trailer) = parse_xref_section(data, current)?
    sections.push(XrefSectionInfo { offset: current, entry_count: entries.len() })

    // 최신 항목 우선: 이미 있는 객체 번호는 무시 (or_insert)
    for (obj_num, entry) in entries:
      table.insert_if_absent(obj_num, entry)

    // 가장 최신 섹션(첫 순회)의 trailer를 보존한다.
    // 이전 섹션들의 trailer는 /Prev chain을 잇는 용도이며,
    // /Root, /Info 등 문서 수준 메타는 최신 trailer가 권위를 가진다.
    if trailer.is_none():
      trailer = Some(section_trailer.clone())

    match section_trailer.prev:
      Some(prev_offset) → current = prev_offset
      None              → BREAK

  return ParsedXref { table, trailer: trailer.unwrap(), sections }
```

**병합 규칙 요약**: `or_insert`를 사용하므로, 동일 객체 번호가 여러 섹션에 등장하면 가장 먼저 처리한(= 가장 최근 = 파일 끝에 가까운) 값이 채택된다.

---

## ParseError 보강

### 기존 변형 수정

```rust
// 변경 전
XrefStreamUnsupported,

// 변경 후 — xref 오프셋 포함 (Task #5에서 스트림 파싱 시 활용)
#[error("xref 스트림 형식은 미지원 (오프셋 {xref_offset}): Task #5에서 처리 예정")]
XrefStreamUnsupported { xref_offset: u64 },
```

`XrefStreamUnsupported` 변형 시그니처 변경은 `trailer.rs`의 `is_xref_stream` 반환 경로에서 `xref_offset`을 함께 전달하도록 `trailer.rs`를 소폭 수정해야 한다.

### 신규 변형

```rust
/// xref 항목 구조 오류 (항목 길이, 잘못된 타입, 섹션 헤더 불일치 등).
#[error("xref 테이블 손상 (파일 오프셋 {offset}): {reason}")]
MalformedXref { offset: u64, reason: String },

/// /Prev chain에서 순환 참조 감지.
#[error("xref /Prev chain 순환 참조: 오프셋 {offset} 재방문")]
XrefChainCycle { offset: u64 },

/// /Prev chain 깊이 초과 (비정상 PDF).
#[error("xref chain 최대 깊이({max_depth}) 초과")]
XrefChainTooDeep { max_depth: usize },

/// xref 오프셋이 파일 크기를 벗어남.
#[error("xref 오프셋 {offset}이 파일 크기 {file_size}를 초과함")]
XrefOffsetOutOfBounds { offset: u64, file_size: u64 },

/// xref 오프셋 위치에 xref 테이블이 없음 (헤더 등 다른 내용).
/// `startxref = 0` 케이스도 이 에러로 처리됨 (오프셋 0 = %PDF- 헤더 위치).
#[error("오프셋 {offset}에 xref 없음: {found:?}")]
InvalidXrefAtOffset { offset: u64, found: String },
```

**`MalformedXref` 설계 결정**: `reason` 필드는 `String`으로 구현한다.
- 동적 정보(항목 인덱스, 실제 바이트 값 등)를 포함하면 디버깅이 훨씬 명확해짐
- 향후 구현 중 reason 패턴이 4~5가지로 굳어지면 회고에서 세분화 검토
  (예: `MalformedXrefEntryLength { found: usize }`, `MalformedXrefEntryType { found: char }`)
- Task #3 범위에서는 단일 변형으로 진행

**`startxref = 0` 책임 분리**:
- `parse_startxref`: 숫자 0 파싱 → `Ok(0)` 반환 (의미 검증 없음 — Task #2 책임 범위)
- `parse_xref`: 오프셋 0으로 이동 → `%PDF-` 발견 → `InvalidXrefAtOffset { offset: 0, found: "%PDF-..." }` 반환

### 에러 도달 가능성 보장

새 변형마다 단위 테스트로 발생시켜야 한다 (CLAUDE.md 원칙):

| 변형 | 발생시키는 테스트 방법 |
|------|--------------------|
| `XrefStreamUnsupported { xref_offset }` | `fw4-2024.pdf` 또는 합성 xref 스트림 바이트 |
| `MalformedXref` | 항목 길이 19바이트, 타입 `x`, 비표준 EOL 등 합성 데이터 |
| `XrefChainCycle` | `/Prev`가 자기 자신을 가리키는 합성 데이터 |
| `XrefChainTooDeep` | 101개 이상 서로 다른 오프셋의 `/Prev` chain |
| `XrefOffsetOutOfBounds` | `xref_offset = file_size + 1` |
| `InvalidXrefAtOffset` | `xref_offset = 0` (헤더 위치) |

---

## 모듈 구조

```
crates/rpdf-parser/src/
├── lib.rs              ← parse_xref, ParsedXref, XrefSectionInfo 추가
├── error.rs            ← ParseError 보강 (기존 수정 1개 + 신규 5개)
├── header.rs           ← Task #2 (변경 없음)
├── eof.rs              ← Task #2 (변경 없음)
├── startxref.rs        ← Task #2 (변경 없음)
├── trailer.rs          ← XrefStreamUnsupported에 xref_offset 추가 (소폭 수정)
├── object_parser.rs    ← Task #2 (Task #3에서 절대 수정 금지, Task #4에서 확장)
└── xref.rs             ← Task #3 신규

crates/rpdf-core/src/types/
├── mod.rs              ← xref 모듈 추가
├── object_id.rs        ← Task #2 (변경 없음)
├── pdf_version.rs      ← Task #2 (변경 없음)
└── xref.rs             ← Task #3 신규 (XrefTable, XrefEntry)
```

---

## 엣지 케이스

| 케이스 | 처리 |
|--------|------|
| xref 오프셋이 파일 크기 초과 | `XrefOffsetOutOfBounds` |
| xref 오프셋에 `xref` 키워드 없음 (헤더 등) | `InvalidXrefAtOffset` |
| `startxref = 0` | `InvalidXrefAtOffset { offset: 0, found: "%PDF-..." }` |
| xref 스트림 감지 | `XrefStreamUnsupported { xref_offset }` |
| 항목 19바이트 (비표준 `\n` 단독) | `MalformedXref { reason: "비표준 항목 EOL" }` |
| 항목 21바이트 | `MalformedXref { reason: "비표준 항목 EOL" }` |
| `n`/`f` 이외의 항목 타입 | `MalformedXref { reason: "알 수 없는 항목 타입" }` |
| 섹션 헤더 숫자 파싱 실패 | `MalformedXref { reason: "섹션 헤더 파싱 실패" }` |
| 실제 항목 수 < 헤더 선언 항목 수 | `MalformedXref { reason: "항목 수 불일치" }` |
| `/Prev` 순환 참조 | `XrefChainCycle { offset }` |
| `/Prev` chain 깊이 100 초과 (비순환) | `XrefChainTooDeep { max_depth: 100 }` |
| 존재하지 않는 `/Prev` 오프셋 | `XrefOffsetOutOfBounds` |
| 동일 객체 번호 중복 (chain 내) | 최신 항목 우선 (`or_insert`), 에러 아님 |
| 빈 xref 섹션 (`0 0\n`) | `Ok`, 항목 없음 — 에러 아님 |

---

## 테스트 전략

### 테스트 파일 구조

```
crates/rpdf-parser/tests/parser/
├── mod.rs              ← xref_tests 등록
├── xref_tests.rs       ← Task #3 신규 단위 테스트
├── integration_tests.rs ← IT-1~IT-5에 parse_xref 검증 추가, IT-6 업데이트
├── trailer_tests.rs    ← XrefStreamUnsupported 에러 시그니처 업데이트
└── fuzz_tests.rs       ← parse_xref proptest 추가
```

### 합성 데이터 헬퍼 (`xref_tests.rs` 상단)

단위 테스트 23개+를 위해 헬퍼를 미리 정의한다.

```rust
/// (offset_or_next: u64, generation: u16, kind: 'n'|'f') → 20바이트 xref 항목
fn make_entry(offset_or_next: u64, generation: u16, kind: char) -> Vec<u8> {
    format!("{:010} {:05} {}\r\n", offset_or_next, generation, kind)
        .into_bytes()
}

/// 완전한 xref 섹션 바이트 생성
/// entries: &[(offset_or_next, generation, kind)]
fn make_xref_section(start_obj: u32, entries: &[(u64, u16, char)], trailer_dict: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("{} {}\n", start_obj, entries.len()).as_bytes());
    for &(off, gen, k) in entries {
        buf.extend_from_slice(&make_entry(off, gen, k));
    }
    buf.extend_from_slice(b"trailer\n");
    buf.extend_from_slice(trailer_dict.as_bytes());
    buf
}
```

### 단위 테스트 (`xref_tests.rs`) 카테고리별 최소 목표

| 카테고리 | 최소 | 주요 케이스 |
|---------|------|------------|
| 기본 파싱 | 8 | 단일 섹션, 복수 섹션, f/n 혼합, 다중 서브섹션 |
| xref chain | 5 | Prev 없음, 2단계, 3단계, 병합 우선순위 확인 |
| 에러 케이스 | 6 | 에러 도달 가능성 표의 6개 변형 각 1개 이상 |
| 경계 케이스 | 4 | 빈 섹션, 오프셋 0, 파일 끝 직전 항목, 19바이트 항목 |

목표 테스트 수: 23개 이상 → 누적 70 + 23 = 93개 이상.

### 통합 테스트 확장 정책

**IT 확장 방식: Option A** — 기존 IT 함수에 `parse_xref` 호출 추가.

IT-N의 목적은 "전체 파이프라인이 동작함" 검증이므로 분리하지 않는다.

```rust
// IT-N: 기존 parse_header / parse_trailer 검증 뒤 parse_xref 추가
let parsed_xref = parse_xref(&data, xref_offset).unwrap();
assert!(!parsed_xref.table.is_empty(), "xref 항목이 비어있음");
let root_num = parsed_xref.trailer.root.number;
assert!(parsed_xref.table.get(root_num).is_some(), "/Root 참조가 xref에 없음");
```

**IT-6 (`fw4-2024.pdf`) 업데이트**: `parse_trailer`와 `parse_xref` 모두 동일 에러 반환 확인.

```rust
#[test]
fn it_6_xref_stream_pdf_returns_xref_stream_unsupported() {
    let data = include_bytes!("../../examples/fw4-2024.pdf");
    let eof = find_eof(data).unwrap();
    let xref_offset = parse_startxref(data, eof).unwrap();

    // 두 경로 모두 동일 에러 반환
    let trailer_result = parse_trailer(data, eof);
    let xref_result    = parse_xref(data, xref_offset);

    assert!(matches!(
        trailer_result,
        Err(ParseError::XrefStreamUnsupported { xref_offset: _ })
    ));
    assert!(matches!(
        xref_result,
        Err(ParseError::XrefStreamUnsupported { xref_offset: _ })
    ));
}
```

### proptest 확장 (`fuzz_tests.rs`)

```rust
proptest! {
    #[test]
    fn arbitrary_input_never_panics_parse_xref(data: Vec<u8>) {
        let _ = parse_xref(&data, 0);
    }
}
```

---

## 구현 체크포인트

**A. 타입 정의 완료 → 시그니처 검토 후 B 진행**

- [ ] `rpdf-core/src/types/xref.rs` — `XrefTable`, `XrefEntry`
- [ ] `rpdf-parser/src/error.rs` — `ParseError` 신규 변형 5개 + 기존 수정 1개
- [ ] `rpdf-parser/src/xref.rs` — `ParsedXref`, `XrefSectionInfo`, `parse_xref` 시그니처 (구현 없음, `todo!()`)
- [ ] `rpdf-parser/src/lib.rs` — 타입 및 함수 공개 선언
- [ ] `cargo build --all` 컴파일 성공

**B. 단일 xref 섹션 파싱 (chain 없이)**

- [ ] `parse_traditional_xref` 구현 (서브섹션 헤더 + 항목 + `parse_trailer_at`)
- [ ] `parse_trailer_at` 구현 (xref 섹션 직후 순방향 trailer 파싱)
- [ ] 에러 도달 가능성: `MalformedXref`, `InvalidXrefAtOffset`, `XrefOffsetOutOfBounds`
- [ ] 기본 파싱 단위 테스트 8개 통과

**C. xref chain 순회**

- [ ] `parse_xref_chain` 구현 (`/Prev` 순회 + `or_insert` 병합 + 방문 set + 깊이 제한)
- [ ] 에러 도달 가능성: `XrefChainCycle`, `XrefChainTooDeep`
- [ ] chain 단위 테스트 5개 통과

**D. 전체 테스트 통과**

- [ ] 단위 테스트 23개 이상 통과
- [ ] IT-1~IT-5에 `parse_xref` 검증 추가 통과
- [ ] IT-6 양쪽 에러 일치 확인
- [ ] proptest `arbitrary_input_never_panics_parse_xref` 통과
- [ ] `cargo clippy -- -D warnings` 경고 없음
- [ ] `cargo fmt --check` 통과

---

## Must/Should/Could 처리 현황

| 구분 | 항목 |
|------|------|
| **Must** | 전통 xref 파싱, xref chain 순회, ParseError 보강, 에러 도달 가능성 테스트, trailer 함수 분리 |
| **Should** | `XrefSectionInfo` 디버그 정보, 합성 데이터 헬퍼 |
| **Could** | `reject_empty_input` 테스트 이름 중복 정리 (백로그 낮음) |
| **Out of scope** | xref 스트림 파싱 (Task #5), `TrailerTooLarge { found_bytes }` 보강 (별도 Issue), `object_parser.rs` 수정 |

---

## 공개 API 확인 체크리스트

- [x] 신규 외부 크레이트 없음 (Task #3 범위 내)
- [x] xref 스트림 미포함 → `flate2` 승인 불필요
- [x] `XrefTable`/`XrefEntry` — 내부 설계, 외부 의존 없음
- [x] 에러 변형마다 발생시키는 단위 테스트 계획 포함
- [x] `object_parser.rs` 미수정 확약

---

## 다음 관련 작업

- Task #4: `object_parser.rs` 확장 (실수, 스트림, boolean, null) — `XrefTable::get`으로 오프셋 확인 후 객체 파싱
- Task #5: xref 스트림 파싱 (`flate2` 도입) — Task #4 객체 파서 활용

closes #(생성 예정)
