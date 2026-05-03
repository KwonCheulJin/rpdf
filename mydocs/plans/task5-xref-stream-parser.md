# Task #5 계획서 — Xref 스트림 파싱 (PDF 1.5+)

**Issue**: #8  
**브랜치**: `local/task5`  
**예상 소요**: 2–3 세션  
**선행 조건**: Task #4 완료 ✅

---

## 목표

PDF 1.5+ 형식의 cross-reference stream을 파싱한다.  
현재 `XrefStreamUnsupported` 에러를 반환하는 `parse_xref` 경로를 실제 파싱으로 교체하여,  
`fw4-2024.pdf`, `irs-f1040.pdf`를 정상 처리한다.

`parse_trailer`는 변경하지 않는다 — 역방향 탐색 구조상 xref 스트림 딕셔너리에  
`trailer` 키워드가 없어 `XrefStreamUnsupported` 반환이 계속 유효하다.

---

## 배경 — Xref 스트림 구조 (ISO 32000 §7.5.8)

전통 xref 테이블 대신 간접 객체 형식의 스트림으로 저장된다.

```
3594 0 obj
<< /Type /XRef
   /Size 3595
   /W [1 3 1]
   /Index [3540 1 3563 1 3589 2 3592 3]
   /Filter /FlateDecode
   /DecodeParms << /Predictor 12 /Columns 5 >>
   /Root 3540 0 R  /Info 3538 0 R
   /Length 44
>>
stream
<compressed bytes>
endstream
endobj
```

### /W 배열 (엔트리 필드 너비)

`[W1 W2 W3]` — 각 xref 엔트리의 세 필드가 차지하는 바이트 수.

| 타입 | 필드 1 (W1) | 필드 2 (W2) | 필드 3 (W3) |
|------|------------|------------|------------|
| 0 (free) | 타입=0 | 다음 free 번호 | generation |
| 1 (in-use) | 타입=1 | 파일 오프셋 | generation |
| 2 (compressed) | 타입=2 | 객체 스트림 번호 | 스트림 내 인덱스 |

W1=0이면 타입=1(default). W2=0 또는 W3=0이면 해당 필드=0.

**실측 (fw4-2024.pdf)**: `/W [1 3 1]` → 엔트리당 5바이트.

### /Index 배열

`[first_obj count first_obj count ...]` — 서브섹션 정의.  
없으면 기본값 `[0 /Size]` (단일 서브섹션, 객체 0번부터 /Size개).

### FlateDecode + PNG Predictor

실제 스트림 데이터는 zlib(deflate)으로 압축되고,  
`/DecodeParms << /Predictor 12 /Columns 5 >>` 형태로 PNG 예측 필터가 추가 적용된다.

**지원하는 Predictor 값** (ISO 32000 §7.4.4.4):

| 값 | 이름 | 처리 |
|----|------|------|
| 1 | 없음 | 그대로 |
| 10 | PNG None | 필터 바이트(0x00) 제거 후 그대로 |
| 11 | PNG Sub | 왼쪽 픽셀 차이 복원 |
| 12 | PNG Up | 위쪽 행 차이 복원 ← fw4 실측 |
| 13 | PNG Average | 평균 복원 |
| 14 | PNG Paeth | Paeth 복원 |
| 15 | PNG Optimum | 행마다 첫 바이트가 predictor 타입 |

**지원하지 않는 값**:
- 2 (TIFF Predictor) → `ParseError::UnsupportedPredictor { value: 2 }`
- 그 외 알 수 없는 값 → 동일 에러

각 PNG 예측 행: `filter_type(1) + data(Columns)` 바이트.  
`Columns = W1 + W2 + W3`.

---

## 데이터 모델 변경

### XrefEntry::Compressed — 처음으로 생성됨

`rpdf-core/src/types/xref.rs`의 `Compressed` variant가 이미 존재한다.  
Task #5에서 처음으로 실제 생성된다.

```rust
// 변경 없음 — 이미 정의됨
Compressed { obj_stm_num: u32, index: u32 }
```

> **범위 외**: `Compressed` 엔트리가 가리키는 객체 스트림(`/Type /ObjStm`)의  
> 실제 파싱은 이 Task의 범위 밖이다. Task #5는 `XrefEntry::Compressed`를 생성하고  
> XrefTable에 삽입하기만 한다. 압축 객체의 실제 추출은 후속 Task에서 처리한다.

### 새 에러 변형 (error.rs)

각 변형마다 반드시 발생시키는 단위 테스트가 있어야 한다 (CLAUDE.md 원칙).

```rust
/// xref 스트림 /W 배열 누락 또는 형식 오류 (3개 양의 정수 배열 아님).
#[error("xref 스트림 /W 배열 오류: {reason}")]
XrefStreamInvalidW { reason: String },

/// xref 스트림 /Index 배열 형식 오류 (홀수 개수, 비정수 등).
#[error("xref 스트림 /Index 배열 오류: {reason}")]
XrefStreamInvalidIndex { reason: String },

/// zlib/FlateDecode 압축 해제 실패.
#[error("xref 스트림 압축 해제 실패 (오프셋 {offset}): {reason}")]
XrefStreamDecompressError { offset: usize, reason: String },

/// xref 스트림 엔트리 수가 /Index가 선언한 수와 불일치.
#[error("xref 스트림 엔트리 수 불일치: 예상 {expected}, 실제 {actual}")]
XrefStreamEntryCountMismatch { expected: usize, actual: usize },

/// /W 배열 크기와 스트림 데이터 길이 불일치 (행 경계에서 잘림).
#[error("xref 스트림 W 필드 크기 불일치 (오프셋 {offset})")]
XrefStreamWFieldMismatch { offset: u64 },

/// xref 스트림 /Filter가 FlateDecode 외 필터를 지정함.
#[error("지원하지 않는 xref 스트림 필터 (오프셋 {offset}): {filter:?}")]
InvalidXrefStreamFilter { offset: u64, filter: String },

/// /DecodeParms /Predictor 값이 지원 범위(1, 10–15) 밖.
#[error("지원하지 않는 Predictor 값: {value}")]
UnsupportedPredictor { value: u8 },

/// xref 스트림 간접 객체 또는 스트림 딕셔너리가 손상됨.
#[error("xref 스트림 구조 오류 (오프셋 {offset}): {reason}")]
MalformedXrefStream { offset: u64, reason: String },
```

`XrefStreamUnsupported`는 유지 — `parse_trailer`에서 계속 반환된다.  
이 변형이 살아 있는 한 dead variant 아님.

---

## API 설계

### 신규 함수: `parse_xref_stream`

```rust
// crates/rpdf-parser/src/xref_stream.rs (신규 파일)

/// xref 스트림 간접 객체를 파싱해 엔트리 목록, PdfTrailer, 섹션 정보를 반환한다.
///
/// 반환 타입이 `parse_xref_section`과 동일해 `parse_xref_chain`에서 투명 교체 가능.
pub(crate) fn parse_xref_stream(
    data: &[u8],
    xref_offset: u64,
) -> Result<(Vec<(u32, XrefEntry)>, PdfTrailer, XrefSectionInfo), ParseError>
```

반환 타입 구조:
- `Vec<(u32, XrefEntry)>` — 객체 번호 → 엔트리 쌍 목록
- `PdfTrailer` — xref 스트림 딕셔너리에서 추출한 trailer 필드
- `XrefSectionInfo` — 오프셋과 엔트리 수 (디버그용)

xref 스트림 딕셔너리에 trailer 필드(`/Root`, `/Info`, `/Prev`, `/Size`)가 통합되어 있어  
`extract_trailer_fields`를 그대로 재사용할 수 있다.

### `parse_xref_section` 반환 타입 변경

기존 `parse_xref_section`도 `XrefSectionInfo`를 반환하도록 시그니처 통일:

```rust
// 변경 전
fn parse_xref_section(...) -> Result<(Vec<(u32, XrefEntry)>, PdfTrailer), ParseError>

// 변경 후
fn parse_xref_section(...) -> Result<(Vec<(u32, XrefEntry)>, PdfTrailer, XrefSectionInfo), ParseError>
```

### `parse_xref_chain` 알고리즘 (Hybrid Chain 지원)

PDF 1.5+ 파일은 incremental update에서 전통 xref와 xref 스트림이 혼재할 수 있다.  
`parse_xref_chain`이 각 오프셋에서 형식을 판별해 분기한다.

```
parse_xref_chain:
  current = start_offset

  LOOP:
    visited 검사 → XrefChainCycle (depth 검사보다 반드시 먼저)
    depth 검사   → XrefChainTooDeep
    visited.insert(current)
    depth += 1

    if is_xref_stream(data, current):
      (entries, trailer, section_info) = parse_xref_stream(data, current)
    else:
      (entries, trailer, section_info) = parse_xref_section(data, current)

    sections.push(section_info)

    for (obj_num, entry) in entries:
      table.insert_if_absent(obj_num, entry)

    if first_trailer.is_none():
      first_trailer = Some(trailer.clone())

    match trailer.prev:
      Some(prev_offset) → current = prev_offset
      None              → BREAK

  return ParsedXref { table, trailer: first_trailer, sections }
```

visited/depth 검사 순서는 Task #3에서 확립된 규칙 유지  
(참고: `mydocs/troubleshootings/xref-chain-check-order.md`).

---

## 구현 체크포인트

### A: 의존성 추가 + 모듈 뼈대

1. `flate2 = "1.1"` 워크스페이스 의존성 추가 (`Cargo.toml`)
2. `rpdf-parser/Cargo.toml`에 `flate2.workspace = true` 추가
3. `crates/rpdf-parser/src/xref_stream.rs` 생성 — `parse_xref_stream` 시그니처만 (`todo!()`)
4. `lib.rs`에 `mod xref_stream` 추가 (비공개)
5. 에러 변형 8개 추가 (`error.rs`)
6. `cargo build` 통과 확인

### B: 스트림 딕셔너리 파싱

`parse_xref_stream` 구현:

1. `parse_indirect_object`로 간접 객체 파싱
2. `object`가 `PdfObject::Stream`인지 확인, 아니면 `MalformedXrefStream`
3. 스트림 딕셔너리에서 추출:
   - `/Type` == `/XRef` 확인 (없거나 다르면 `MalformedXrefStream`)
   - `/W [W1 W2 W3]` — 3개 비음수 정수 배열 필수 (`XrefStreamInvalidW`)
   - `/Index [...]` — 선택, 없으면 `[0, /Size]` 기본값
   - `/Filter` — Name 또는 1개짜리 배열 허용, FlateDecode 외 `InvalidXrefStreamFilter`
   - `/DecodeParms` — 없으면 Predictor=1, Columns는 W1+W2+W3
4. `extract_trailer_fields` 재사용해 `PdfTrailer` 구성
5. 체크포인트 B 테스트 (~10개):
   - /W 3개 양의 정수 파싱 성공
   - /W 원소 수 != 3 → `XrefStreamInvalidW`
   - /Type /XRef 없음 → `MalformedXrefStream`
   - /Filter LZWDecode → `InvalidXrefStreamFilter`
   - /Index 없으면 [0, Size] 기본값 적용
   - /Index 홀수 원소 → `XrefStreamInvalidIndex`

### C: FlateDecode + PNG 언필터링

1. `PdfStream.data` (raw bytes)에서 FlateDecode 압축 해제:
   - `flate2::read::ZlibDecoder` 사용
   - 실패 시 `XrefStreamDecompressError`
2. PNG 예측 필터 적용 (`/DecodeParms /Predictor` 값 기반):
   - row_size = `Columns + 1` (filter byte 포함)
   - Predictor 1: 필터 처리 없음 (ZlibDecoder 출력 그대로)
   - Predictor 10: 각 행 첫 바이트 제거 후 그대로
   - Predictor 11 (Sub): `row[i] += row[i - 1]`
   - Predictor 12 (Up): `row[i] += prev_row[i]`
   - Predictor 13 (Average): `row[i] += (row[i-1] + prev_row[i]) / 2`
   - Predictor 14 (Paeth): Paeth predictor 공식
   - Predictor 15 (Optimum): 각 행 첫 바이트가 predictor 타입으로 10–14 분기
   - Predictor 2 또는 기타 → `UnsupportedPredictor`
3. 체크포인트 C 테스트 (~10개):
   - FlateDecode 압축 해제 성공 (실제 fw4-2024.pdf 스트림 44바이트 사용)
   - Predictor 12 (Up) 언필터링 단위 테스트 (수동 계산값 검증)
   - Predictor 1 (없음) — 그대로 통과
   - Predictor 15 (Optimum) — 행마다 다른 타입
   - 손상된 zlib 스트림 → `XrefStreamDecompressError`
   - Predictor 2 → `UnsupportedPredictor`

### D: 엔트리 디코딩 + xref 체인 통합

1. 디코딩된 바이트를 W1+W2+W3 크기로 분할, 엔트리 타입별 파싱:
   - W1=0: 타입=1(default), W1 필드 읽지 않음
   - 타입 0 → `XrefEntry::Free { next_free_obj_num, generation }`
   - 타입 1 → `XrefEntry::InUse { offset, generation }`
   - 타입 2 → `XrefEntry::Compressed { obj_stm_num, index }`
   - 그 외 타입 → 무시 (ISO 32000 §7.5.8.3: "reserved for future use")
2. `/Index` 서브섹션에 따라 객체 번호 할당
3. 엔트리 수 검증: `/Index`의 합계 != 실제 엔트리 수 → `XrefStreamEntryCountMismatch`
4. W 경계 불일치 검증: 디코딩 바이트가 행 크기 배수 아님 → `XrefStreamWFieldMismatch`
5. `parse_xref_section` 시그니처에 `XrefSectionInfo` 추가 반환
6. `parse_xref_chain` 알고리즘 업데이트 (Hybrid Chain 처리)
7. 체크포인트 D 테스트 (~12개):
   - 타입 0/1/2 엔트리 파싱 단위 테스트
   - W1=0(default type) 처리
   - /Index 없는 경우(기본값) 파싱
   - /Index 다수 서브섹션 처리
   - 엔트리 수 불일치 → `XrefStreamEntryCountMismatch`
   - W 경계 불일치 → `XrefStreamWFieldMismatch`

### E: 통합 테스트 + proptest

1. IT-6 수정: `fw4-2024.pdf`가 `ParsedXref` 정상 반환 + `/Root` Catalog 파싱
2. IT-7 추가: `irs-f1040.pdf` — xref 스트림 파싱, `/Root` `/Info` 객체 파싱
3. IT-8 추가: `fw4-2024.pdf` — `/Prev` chain이 있는 xref 스트림 순회  
   (`parsed_xref.sections.len() >= 2` 검증)
4. proptest: `arbitrary_input_never_panics_parse_xref_stream` 추가
5. `XrefStreamUnsupported` 도달 가능성 재확인:
   - `parse_trailer`에서 여전히 반환됨 → 유지 (IT-6 수정 후에도 parse_trailer 경로는 에러)

---

## 엣지 케이스

| 케이스 | 처리 방식 |
|--------|---------|
| `/W [0 4 2]` — W1=0 | default 타입=1 고정 |
| `/W [1 0 0]` — W2·W3=0 | 해당 필드=0 고정 |
| `/Filter` 없음 | 비압축, 그대로 사용 |
| `/Filter [/FlateDecode]` (배열 형식) | 배열 1원소도 동일 처리 |
| `/DecodeParms` 없음 | Predictor=1 (no-op) |
| `/Predictor 2` (TIFF) | `UnsupportedPredictor { value: 2 }` |
| xref 스트림 + `/Prev` chain | `parse_xref_chain` 자동 처리 |
| 전통 xref + xref 스트림 혼재 | `is_xref_stream` 감지 후 분기 |
| 타입 0/1/2 이외 타입 | 무시 (ISO 스펙) |

---

## 테스트 전략

### 단위 테스트 (`xref_stream_tests.rs` 신규)

| 그룹 | 내용 | 예상 수 |
|------|------|--------|
| B: 딕셔너리 파싱 | /W, /Index, /Type, /Filter 파싱 및 에러 | ~10 |
| C: 압축 해제 | FlateDecode, Predictor 1/10/11/12/13/14/15, 손상 | ~10 |
| D: 엔트리 디코딩 | 타입 0/1/2, W1=0, /Index 기본값, 불일치 에러 | ~12 |
| 경계값 | W=[0,4,2], 빈 스트림, 단일 엔트리 | ~5 |
| **소계** | | **~37** |

### 통합 테스트 수정/추가

| ID | 파일 | 검증 내용 |
|----|------|---------|
| IT-6 수정 | fw4-2024.pdf | XrefStreamUnsupported → ParsedXref, /Root Catalog |
| IT-7 신규 | irs-f1040.pdf | xref 스트림 파싱, /Root /Info 파싱 |
| IT-8 신규 | fw4-2024.pdf | /Prev chain 순회 (sections.len() >= 2) |

### proptest (fuzz_tests.rs 추가)

- `arbitrary_input_never_panics_parse_xref_stream` — 1개

**예상 신규 테스트**: 단위 ~37 + 통합 3 + proptest 1 = **~41개**

---

## 의존성 추가

**`flate2 = "1.1.9"`** — zlib/deflate 압축 해제.

- docs.rs 공개 API 확인 완료: `flate2::read::ZlibDecoder` — `Read` trait 구현체
- 순수 Rust (`miniz_oxide` 기반), 시스템 라이브러리 의존 없음
- MIT/Apache-2 라이선스, 최근 유지보수 활성

---

## 범위 외

- `parse_trailer`의 xref 스트림 처리 (역방향 탐색 구조 유지)
- 객체 스트림(`/Type /ObjStm`) 파싱 — `Compressed` 엔트리 생성만, 실제 추출은 후속 Task
- LZW, JBIG2 등 FlateDecode 외 스트림 필터
- TIFF Predictor (값 2) — `UnsupportedPredictor` 에러로 명시적 거부

---

## 파일 변경 목록

| 파일 | 변경 유형 |
|------|---------|
| `Cargo.toml` (workspace) | `flate2 = "1.1"` 추가 |
| `crates/rpdf-parser/Cargo.toml` | `flate2.workspace = true` 추가 |
| `crates/rpdf-parser/src/xref_stream.rs` | **신규** |
| `crates/rpdf-parser/src/xref.rs` | `parse_xref_chain` 알고리즘 + `parse_xref_section` 시그니처 |
| `crates/rpdf-parser/src/error.rs` | 에러 변형 8개 추가 |
| `crates/rpdf-parser/src/lib.rs` | `mod xref_stream` 추가 |
| `crates/rpdf-parser/tests/parser/xref_stream_tests.rs` | **신규** |
| `crates/rpdf-parser/tests/parser/mod.rs` | `xref_stream_tests` 등록 |
| `crates/rpdf-parser/tests/parser/integration_tests.rs` | IT-6 수정, IT-7·IT-8 추가 |
| `crates/rpdf-parser/tests/parser/fuzz_tests.rs` | proptest 1개 추가 |
