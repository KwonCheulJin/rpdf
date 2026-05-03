# Task #6 계획서 — 객체 스트림 파서 (/Type /ObjStm)

**Issue**: #10
**브랜치**: `local/task6`
**예상 소요**: 2세션
**선행 조건**: Task #5 완료 ✅

---

## 목표

`XrefEntry::Compressed { obj_stm_num, index }`를 실제 `PdfObject`로 해소한다.

PDF 1.5+에서는 페이지 딕셔너리 등 일반 객체를 `/Type /ObjStm` 스트림 안에 저장한다.
이를 파싱해야 Content Stream 파서(Task #7), Document IR(Task #8), CLI(Task #9)가
PDF 1.4/1.5 모두에서 완전히 동작한다.

---

## 배경 — 객체 스트림 구조 (ISO 32000 §7.5.7)

```
12 0 obj
<< /Type /ObjStm
   /N 3
   /First 18
   /Filter /FlateDecode
   /Length 42
>>
stream
<compressed>
endstream
endobj
```

압축 해제 후 스트림 본문:

```
3 0 17 9 25 18      ← 헤더: N개 "obj_num offset" 쌍 (offset은 /First 기준 상대)
<< /Type /Catalog >>  ← obj#3 본문 (First=18에서 시작, offset=0)
<< /Type /Pages >>    ← obj#17 본문 (offset=9)
true                  ← obj#25 본문 (offset=18)
```

### 핵심 규칙

- **헤더**: `2N`개 정수 — `obj_num₁ offset₁ obj_num₂ offset₂ ...`
- **본문**: `/First` 위치부터 각 객체가 `obj/endobj` 키워드 **없이** raw 형태로 연속
- **`/Extends`**: 다른 ObjStm 상속. 드문 케이스, v0.1 범위 외 (명시적 거부)
- **`/Filter`**: 보통 `FlateDecode`. 없으면 비압축 그대로 사용.

### XrefEntry::Compressed 연결

```
XrefEntry::Compressed { obj_stm_num: 12, index: 2 }
→ obj #12 파싱 → ObjStm 헤더에서 index=2번째 쌍 (obj_num=25, offset=18)
→ First+18 위치에서 parse_object 호출
→ 반환된 PdfObject 확인 (obj_num 일치 검증)
```

`index`는 헤더 쌍의 0-based 순서. 검증: 헤더의 `obj_num`이 XrefTable의 키와 다른 경우 **xref 우선 + `tracing::warn` 경고** — 손상 PDF 호환을 위해 엄격 거부 없이 로그만 남긴다.

---

## 범위

### 포함

- `/Type /ObjStm` 딕셔너리 파싱 (`/N`, `/First`, `/Filter`, `/Length`)
- FlateDecode 압축 해제 (`decompress_flate` 재사용)
- 헤더(`2N` 정수 쌍) 파싱
- 본문 객체 추출 (`parse_object` 재사용)
- `ParsedObjectStream::get(obj_num)` 메서드 — Compressed 엔트리 해소 API
- 통합 테스트: root가 `Compressed` 엔트리인 실제 PDF 또는 합성 PDF

### 제외

- `/Extends` 상속 체인 — `ObjStmExtendsUnsupported` 에러로 명시적 거부
- 객체 캐싱/메모이제이션 — Document IR(Task #8) 영역
- `get_object(xref_table, data, obj_id)` 통합 resolver — Task #8 영역
- FlateDecode 외 필터 (LZW 등) — `InvalidObjStmFilter` 에러

---

## 데이터 모델

### 신규 타입

```rust
// crates/rpdf-parser/src/object_stream.rs (신규)

/// ObjStm 파싱 결과. 객체 번호 → PdfObject 매핑.
pub struct ParsedObjectStream {
    /// ObjStm이 포함하는 객체 목록 (obj_num, object)
    pub objects: Vec<(u32, PdfObject)>,
}
```

`ParsedObjectStream`은 `rpdf-parser` 공개 API로 노출한다 (`pub`).

### 신규 에러 변형 (error.rs)

```rust
/// /Type /ObjStm 딕셔너리가 손상됨 (필수 키 누락 또는 잘못된 값).
#[error("객체 스트림 구조 오류 (오프셋 {offset}): {reason}")]
MalformedObjStm { offset: u64, reason: String },

/// /Extends 키가 발견됨 — 상속 ObjStm은 v0.1 범위 외.
#[error("객체 스트림 /Extends 미지원 (오프셋 {offset})")]
ObjStmExtendsUnsupported { offset: u64 },

/// /Filter가 FlateDecode 외 필터를 지정함.
#[error("지원하지 않는 ObjStm 필터 (오프셋 {offset}): {filter:?}")]
InvalidObjStmFilter { offset: u64, filter: String },

/// 헤더의 obj_num이 XrefTable의 키와 불일치.
///
/// **현재 정책**: 발생시키지 않음 — xref 우선 + `tracing::warn` 로그.
/// 향후 strict 모드 옵션 도입 시 활용 예약.
#[error("객체 스트림 헤더 번호 불일치: 헤더={header_num}, xref={xref_num}")]
ObjStmObjNumMismatch { header_num: u32, xref_num: u32 },
```

---

## API 설계

```rust
// crates/rpdf-parser/src/object_stream.rs

/// ObjStm 간접 객체를 파싱해 객체 목록을 반환한다.
///
/// `offset`은 xref table에서 읽은 ObjStm 객체의 파일 오프셋.
/// 반환된 `ParsedObjectStream.objects`는 (obj_num, PdfObject) 쌍 벡터.
pub(crate) fn parse_object_stream(
    data: &[u8],
    offset: u64,
) -> Result<ParsedObjectStream, ParseError>

impl ParsedObjectStream {
    /// Compressed 엔트리의 obj_num으로 PdfObject를 찾는다.
    ///
    /// `obj_num`이 stream에 없으면 `None` 반환.
    /// XrefTable::get()과 일관된 시그니처.
    pub fn get(&self, obj_num: u32) -> Option<&PdfObject> {
        self.objects.iter()
            .find(|(num, _)| *num == obj_num)
            .map(|(_, obj)| obj)
    }
}
```

### `lib.rs` 공개 사항

```rust
pub use object_stream::ParsedObjectStream;
pub(crate) use object_stream::parse_object_stream;
```

---

## 구현 체크포인트

### A: 모듈 뼈대 + 타입 + 에러 변형

1. `crates/rpdf-parser/src/object_stream.rs` 생성
   - `ParsedObjectStream` 구조체
   - `parse_object_stream` 시그니처 (`todo!()`)
   - `ParsedObjectStream::get()` 메서드 구현
2. `error.rs`에 에러 변형 4개 추가
3. `lib.rs`에 `mod object_stream` + 공개 사항 추가
4. `cargo build` 통과 확인

### B: ObjStm 딕셔너리 파싱 + 헤더 추출

`parse_object_stream` 구현:

1. `parse_indirect_object(data, offset)` 호출
2. `PdfObject::Stream` 확인, 아니면 `MalformedObjStm`
3. 딕셔너리에서 추출:
   - `/Type == /ObjStm` 확인, 아니면 `MalformedObjStm`
   - `/N`: 양의 정수 필수
   - `/First`: 양의 정수 필수
   - `/Filter`: `FlateDecode` 또는 없음. 그 외 `InvalidObjStmFilter`
   - `/Extends`: 있으면 `ObjStmExtendsUnsupported`
4. FlateDecode 압축 해제 (`decompress_flate` 재사용)
   - `/Filter` 없으면 raw 그대로
5. 헤더 파싱: `0..First` 영역에서 `2N`개 정수 추출 → `Vec<(u32, u64)>`
   - 정수 파싱 실패 또는 개수 불일치 → `MalformedObjStm`
   - 정수 사이 구분자: 화이트스페이스 (PDF §7.2.3) — `' '`, `'\t'`, `'\n'`, `'\r'`, `'\x0C'`, `'\0'`
   - `skip_whitespace_and_comments` 활용 권장 (% 주석 스킵 포함)

체크포인트 B 테스트 (~8개):
- `/N` + `/First` 파싱 성공
- `/Type /ObjStm` 없음 → `MalformedObjStm`
- `/N` 없음 → `MalformedObjStm`
- `/Extends` 있음 → `ObjStmExtendsUnsupported`
- 헤더 2N 정수 수 불일치 → `MalformedObjStm`
- `/Filter /LZWDecode` → `InvalidObjStmFilter`
- 비압축 ObjStm(Filter 없음) 헤더 파싱 성공

### C: 본문 객체 추출 + `resolve_compressed_entry`

1. 각 헤더 엔트리 `(obj_num, rel_offset)`에 대해:
   - `abs_offset = first + rel_offset`
   - `parse_object(decompressed_data, abs_offset)` 호출
   - 실패 시 `MalformedObjStm`
2. `ParsedObjectStream { objects: vec![(obj_num, object), ...] }` 반환
3. `resolve_compressed_entry(stream, obj_num)` 구현:
   - `objects`에서 `obj_num` 일치 항목 선형 탐색 → `Option<&PdfObject>`

체크포인트 C 테스트 (~10개):
- Dictionary 객체 추출 성공
- Integer, Boolean, Array 등 단순 객체 추출
- `/First` 오프셋 경계 값 (rel_offset = 0, 최대값)
- `resolve_compressed_entry` 존재하는 obj_num → `Some`
- `resolve_compressed_entry` 없는 obj_num → `None`
- FlateDecode 압축 ObjStm 전체 파이프라인
- 손상된 본문 → `MalformedObjStm`

### D: 통합 테스트 + proptest + 문서

1. IT-9 신규: Compressed 엔트리를 포함하는 합성 ObjStm PDF 통합 테스트
   - 합성 ObjStm을 포함하는 PDF 데이터 생성
   - `parse_xref` → `XrefEntry::Compressed` 해소 → `PdfObject` 검증
2. IT-10 신규 (선택): 실제 PDF에서 Compressed 엔트리 해소 가능하면 추가
3. proptest: `arbitrary_input_never_panics_parse_object_stream` (internal_tests 내)
4. `object_stream_tests.rs` 파일 배치 결정:
   - `parse_object_stream`, `resolve_compressed_entry`는 `pub(crate)` → internal_tests

> **파일 위치 결정 (CLAUDE.md 원칙)**:
> `pub(crate)` 함수는 external 테스트 파일에서 접근 불가.
> → 모든 단위 테스트는 `object_stream.rs` 내 `#[cfg(test)] mod internal_tests`에 위치.

5. 완료 보고서 작성 (`mydocs/working/task6-done.md`)

---

## 엣지 케이스

| 케이스 | 처리 방식 |
|--------|---------|
| `/Filter` 없음 | 비압축 — raw 그대로 사용 |
| `/N = 0` | 빈 ObjStm 허용, 빈 `objects` 반환 |
| `rel_offset` 가 `First`보다 큰 경우 | `MalformedObjStm` |
| `obj_num`이 xref table 키와 다른 경우 | **xref 우선** + `tracing::warn` 경고. `ObjStmObjNumMismatch` 변형 유지(미발생) — 향후 strict 모드 예약. |
| `/Extends` 존재 | `ObjStmExtendsUnsupported` — 명시적 거부 |
| 헤더 파싱 중 음수 offset | `MalformedObjStm` |

---

## 테스트 전략

### 단위 테스트 (internal_tests, object_stream.rs 내)

| 그룹 | 내용 | 예상 수 |
|------|------|--------|
| B: 딕셔너리 + 헤더 | /N, /First, /Filter, /Extends | ~8 |
| C: 객체 추출 | Dictionary/Array/Scalar, FlateDecode | ~10 |
| proptest | arbitrary_input panic 없음 | 1 |
| 경계값 | N=0, 단일 객체, 최대 offset | ~3 |
| **소계** | | **~22** |

### 통합 테스트 (integration_tests.rs)

| ID | 내용 | 방식 |
|----|------|------|
| IT-9 | Compressed 엔트리 → ObjStm 추출 → PdfObject 검증 | 합성 PDF |
| IT-10 | 실제 PDF (D 단계 진입 시 examples/ Compressed 엔트리 존재 확인 후 결정) | 실제 파일 또는 합성 |

---

## 파일 변경 목록

| 파일 | 변경 유형 |
|------|---------|
| `crates/rpdf-parser/src/object_stream.rs` | **신규** |
| `crates/rpdf-parser/src/error.rs` | 에러 변형 4개 추가 |
| `crates/rpdf-parser/src/lib.rs` | `mod object_stream` + 공개 사항 |
| `crates/rpdf-parser/tests/parser/integration_tests.rs` | IT-9, IT-10 추가 |
| `mydocs/working/task6-done.md` | **신규** (완료 보고서) |

---

## 의존성

신규 의존성 **없음** — `flate2`, `parse_object`, `parse_indirect_object`, `parse_dictionary` 모두 Task #4/#5에서 확보됨.
