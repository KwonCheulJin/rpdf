# Task #7 계획서 — Content Stream 파서 (토큰화 + 연산자 분류)

**Issue**: #12
**브랜치**: `local/task7`
**예상 소요**: 2세션
**선행 조건**: Task #6 완료 ✅

---

## 목표

페이지 content stream(`&[u8]`)을 파싱하여 연산자 시퀀스(`Vec<ContentStreamOperation>`)로 표현한다.

의미 해석(폰트 매핑, 좌표 변환 누적, 색상 모델 해석)은 **범위 외**다. Task #8 Document IR과 v0.2 렌더링이 담당한다. 이 태스크는 "어떤 연산자가 어떤 피연산자와 등장하는가"를 구조화된 타입으로 돌려주는 것이 전부다.

---

## 배경 — Content Stream 구조 (ISO 32000 §7.8, §8~§9)

```
BT
  /F1 12 Tf
  72 720 Td
  (Hello World) Tj
ET
q
  200 0 0 200 100 100 cm
  /Im0 Do
Q
```

- **토큰 구조**: 피연산자(숫자, 문자열, 이름, 배열 등) 0개 이상이 먼저 스택에 쌓이고, 마지막에 연산자 키워드가 등장 → 스택을 소비하여 하나의 `ContentStreamOperation`을 생성
- **피연산자**: `PdfObject`로 표현 가능한 범위(숫자, 문자열, 이름, 배열, 딕셔너리). Indirect Reference는 content stream 안에 등장하지 않음(스펙 §7.8.2).
- **인라인 이미지**: `BI ... ID <rawbytes> EI` — 특수 처리 필요

---

## 범위

### 포함

- 연산자 토큰화: 피연산자 스택 누적 + 키워드 인식
- 연산자 그룹 분류 (8개 그룹, 아래 연산자 표 참조)
- 피연산자 파싱: `parse_object` 재사용
- 인라인 이미지 처리: `BI`~`ID`~`EI` 파싱 (dict key-value + raw bytes)
- q/Q 스택 깊이 검증: 음수 즉시 에러 + 파싱 완료 후 양수 에러
- 공개 API: `parse_content_stream(data: &[u8]) -> Result<Vec<ContentStreamOperation>, ParseError>`

### 제외 (의미 해석 X)

| 제외 항목 | 담당 단계 |
|---------|---------|
| 폰트 이름 → 실제 폰트 해석 | Task #8, v0.2 |
| Tf 크기 단위 → 렌더링 좌표 | v0.2 렌더링 |
| 색상 공간 해석 (CS/cs/G/g 등) | v0.2 렌더링 |
| 좌표 변환 행렬(cm) 누적 | v0.2 렌더링 |
| XObject 재귀 해소 (Do) | Task #8 |
| gs 인자 딕셔너리 의미 해석 | v0.2 |

---

## 연산자 분류 표 (ISO 32000 §8~§9, PDF 키워드 71개 / enum 변형 69개)

| 그룹 | PDF 키워드 | enum 변형 |
|-----|---------|---------|
| **Text 객체** | `BT` `ET` | `BeginText` `EndText` |
| **Text 상태** | `Tc` `Tw` `Tz` `TL` `Tf` `Tr` `Ts` | `SetCharSpacing` `SetWordSpacing` `SetHorizontalScale` `SetLeading` `SetFont` `SetRenderingMode` `SetTextRise` |
| **Text 위치** | `Td` `TD` `Tm` `T*` | `MoveText` `MoveTextSetLeading` `SetTextMatrix` `MoveToNextLine` |
| **Text 표시** | `Tj` `TJ` `'` `"` | `ShowText` `ShowTextAdjusted` `MoveShowText` `MoveSetShowText` |
| **Graphics 상태** | `q` `Q` `cm` `w` `J` `j` `M` `d` `i` `gs` `ri` | `SaveState` `RestoreState` `ConcatMatrix` `SetLineWidth` `SetLineCap` `SetLineJoin` `SetMiterLimit` `SetDashPattern` `SetFlatness` `SetGraphicsState` `SetRenderingIntent` |
| **경로 구성** | `m` `l` `c` `v` `y` `h` `re` | `MoveTo` `LineTo` `CurveTo` `CurveToV` `CurveToY` `ClosePath` `Rect` |
| **경로 그리기** | `S` `s` `f` `F` `f*` `B` `B*` `b` `b*` `n` | `Stroke` `CloseStroke` `Fill` `FillObsolete` `FillEvenOdd` `FillStroke` `FillStrokeEvenOdd` `CloseFillStroke` `CloseFillStrokeEvenOdd` `EndPath` |
| **클리핑** | `W` `W*` | `Clip` `ClipEvenOdd` |
| **색상** | `CS` `cs` `SC` `SCN` `sc` `scn` `G` `g` `RG` `rg` `K` `k` | `SetStrokeColorSpace` `SetFillColorSpace` `SetStrokeColor` `SetStrokeColorN` `SetFillColor` `SetFillColorN` `SetStrokeGray` `SetFillGray` `SetStrokeRGB` `SetFillRGB` `SetStrokeCMYK` `SetFillCMYK` |
| **XObject/셰이딩** | `Do` `sh` | `InvokeXObject` `Shading` |
| **인라인 이미지** | `BI` `ID` `EI` | `InlineImage` (단일 복합 연산) |
| **Marked Content** | `MP` `DP` `BMC` `BDC` `EMC` | `MarkedContentPoint` `MarkedContentPointProp` `BeginMarkedContent` `BeginMarkedContentProp` `EndMarkedContent` |
| **호환성** | `BX` `EX` | `BeginCompatibility` `EndCompatibility` |
| **알 수 없음** | (기타) | `Unknown(Vec<u8>)` — 에러 아님, 보존 |

> **명명 원칙**: PDF 키워드를 변형 이름으로 직접 차용하지 않는다. 모든 변형은 의도가 드러나는 의미 기반 이름을 사용한다. 키워드 → enum 매핑은 `keyword_to_operator` 함수 안에서만 처리된다.

---

## 데이터 모델

### 설계 결정

**Q1. `ContentStreamOperator`를 하나의 flat enum으로 vs 그룹별 enum으로?**

→ **단일 flat enum** 선택.
- 69개 enum 변형은 하나의 `match`로 소화 가능. 그룹 분기는 메서드(`is_text()` 등)로 표현.
- 그룹별 enum은 이중 래핑(`ContentStreamOperation::Text(TextOperator)`)을 강제하며, Task #8 소비자가 더 번거롭다. KISS 원칙 준수.

**Q2. 피연산자를 `Vec<PdfObject>`로 vs 연산자별 구체 타입으로?**

→ **`Vec<PdfObject>`** 선택. v0.1은 "분류"가 목표. 의미 해석(타입 검증)은 Task #8. YAGNI.

**Q3. B와 C 사이의 중간 표현**

→ **B에서 keyword bytes 보관, C에서 enum으로 변환**.
- B는 `parse_content_stream` 내부에서 `Keyword(Vec<u8>)` 토큰을 인식하여 피연산자 스택과 함께 보관.
- C에서 `keyword_to_operator(keyword: &[u8]) -> ContentStreamOperator`를 구현하고 `ContentStreamOperation`으로 변환.
- 이 분리 덕분에 B 단계 테스트에서 토큰화만 검증 가능하고, C 단계 테스트에서 분류 정확성만 검증 가능.

**Q4. q/Q 불균형 검증 정책**

→ **옵션 B: 음수 즉시 검증 + offset 포함**.
- `Q` 만났을 때 `depth == 0` → 즉시 `UnbalancedGraphicsState { offset, depth: -1 }` 에러
- 파싱 완료 후 `depth > 0` → `UnbalancedGraphicsState { offset: data.len(), depth }` 에러
- 사용자가 어디서 처음 어긋났는지 파악 가능. 완료 후 검증만 하면 위치 정보를 잃는다.

### `rpdf-core`에 추가될 타입

파일: `crates/rpdf-core/src/types/content_stream.rs`

```rust
/// PDF content stream 연산자 (ISO 32000 §8~§9).
///
/// 모든 변형은 의미 기반 이름을 사용한다. PDF 키워드 → enum 매핑은
/// `rpdf_parser::content_stream::keyword_to_operator`에서만 처리된다.
///
/// `Unknown(Vec<u8>)`: 스펙에 없는 키워드. 무시하지 않고 보존하여
/// 디버깅 시 확인 가능. 파싱 에러가 아님.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentStreamOperator {
    // ── Text 객체 ──────────────────────────────────────
    BeginText, EndText,
    // ── Text 상태 ─────────────────────────────────────
    SetCharSpacing, SetWordSpacing, SetHorizontalScale, SetLeading,
    SetFont, SetRenderingMode, SetTextRise,
    // ── Text 위치 ─────────────────────────────────────
    MoveText, MoveTextSetLeading, SetTextMatrix, MoveToNextLine,
    // ── Text 표시 ─────────────────────────────────────
    ShowText, ShowTextAdjusted, MoveShowText, MoveSetShowText,
    // ── 그래픽 상태 ────────────────────────────────────
    SaveState, RestoreState, ConcatMatrix,
    SetLineWidth, SetLineCap, SetLineJoin, SetMiterLimit,
    SetDashPattern, SetFlatness, SetGraphicsState, SetRenderingIntent,
    // ── 경로 구성 ─────────────────────────────────────
    MoveTo, LineTo, CurveTo, CurveToV, CurveToY, ClosePath, Rect,
    // ── 경로 그리기 ───────────────────────────────────
    Stroke, CloseStroke, Fill, FillObsolete, FillEvenOdd,
    FillStroke, FillStrokeEvenOdd, CloseFillStroke, CloseFillStrokeEvenOdd,
    EndPath,
    // ── 클리핑 ────────────────────────────────────────
    Clip, ClipEvenOdd,
    // ── 색상 ──────────────────────────────────────────
    SetStrokeColorSpace, SetFillColorSpace,
    SetStrokeColor, SetStrokeColorN, SetFillColor, SetFillColorN,
    SetStrokeGray, SetFillGray,
    SetStrokeRGB, SetFillRGB,
    SetStrokeCMYK, SetFillCMYK,
    // ── XObject / 셰이딩 ───────────────────────────────
    InvokeXObject, Shading,
    // ── 인라인 이미지 (BI...ID...EI 통합) ─────────────
    InlineImage,
    // ── 마킹된 콘텐츠 ─────────────────────────────────
    MarkedContentPoint, MarkedContentPointProp,
    BeginMarkedContent, BeginMarkedContentProp, EndMarkedContent,
    // ── 호환성 ────────────────────────────────────────
    BeginCompatibility, EndCompatibility,
    // ── 알 수 없는 연산자 (보존) ──────────────────────
    Unknown(Vec<u8>),
}

/// content stream의 단일 연산 — 연산자 + 피연산자 목록.
///
/// 인라인 이미지(`InlineImage`)의 경우:
/// - `operands`: dict key-value 쌍 (`PdfObject::Name, value, ...` 순서)
/// - `inline_data`: `Some(raw_bytes)` — ID와 EI 사이의 원본 이미지 데이터
///
/// 나머지 연산자의 경우 `inline_data`는 항상 `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentStreamOperation {
    pub operator: ContentStreamOperator,
    /// 피연산자. Indirect Reference는 content stream 안에 등장하지 않음 (§7.8.2).
    pub operands: Vec<PdfObject>,
    /// 인라인 이미지 raw bytes (InlineImage 연산자 전용).
    pub inline_data: Option<Vec<u8>>,
}
```

### 공개 API (`rpdf-parser`)

```rust
// crates/rpdf-parser/src/content_stream.rs

/// `data` 전체를 content stream으로 파싱한다.
///
/// 피연산자는 `parse_object`로 파싱된다. 연산자 키워드는 ASCII 이름
/// 토큰으로 인식된 후 `ContentStreamOperator`로 분류된다.
///
/// q/Q 깊이 검증:
/// - `Q` 만났을 때 depth == 0 → `UnbalancedGraphicsState { offset, depth: -1 }` 즉시 에러
/// - 파싱 완료 후 depth > 0 → `UnbalancedGraphicsState { offset: data.len(), depth }` 에러
///
/// 알 수 없는 키워드는 `Unknown(bytes)` 변형으로 보존 (에러 아님).
///
/// ISO 32000-1 §7.8.2
pub fn parse_content_stream(
    data: &[u8],
) -> Result<Vec<ContentStreamOperation>, ParseError>
```

### 새 `ParseError` 변형

```rust
/// content stream 구조 오류 (피연산자/연산자 불일치, 예상치 못한 EOF 등).
MalformedContentStream { offset: usize, reason: String },

/// 인라인 이미지 (BI...ID...EI) 파싱 오류.
MalformedInlineImage { offset: usize, reason: String },

/// q/Q 상태 스택 불균형.
/// - Q 만났을 때 depth == 0 → depth = -1, offset = 해당 Q 위치
/// - 파싱 완료 후 depth > 0 → offset = data.len()
UnbalancedGraphicsState { offset: usize, depth: i32 },
```

---

## 체크포인트

### Checkpoint A — 타입 + 에러 변형 + 모듈 뼈대

**목표**: 컴파일만 통과.

1. `crates/rpdf-core/src/types/content_stream.rs` 신규:
   - `ContentStreamOperator` enum (전체 변형, 의미 기반 이름)
   - `ContentStreamOperation` struct
2. `crates/rpdf-core/src/types/mod.rs` 에 `pub mod content_stream` + re-export
3. `crates/rpdf-parser/src/error.rs`:
   - `MalformedContentStream`, `MalformedInlineImage`, `UnbalancedGraphicsState` 추가
4. `crates/rpdf-parser/src/content_stream.rs` 신규:
   - `parse_content_stream` stub (항상 `Ok(vec![])` 반환)
5. `crates/rpdf-parser/src/lib.rs`:
   - `mod content_stream; pub use content_stream::parse_content_stream;`
6. `crates/rpdf-parser/tests/parser/content_stream_tests.rs` 신규:
   - `mod.rs` 등록
   - `empty_input_returns_empty_vec()` 테스트 1개

**완료 기준**: `cargo test` 통과, `cargo clippy -- -D warnings` 경고 없음.

---

### Checkpoint B — 토큰화 (피연산자 + 키워드 분리)

**목표**: content stream을 "피연산자 토큰" + "연산자 키워드"로 분리. keyword는 아직 enum 변환 없이 `Vec<u8>` 보관.

내부 표현:
```rust
// content_stream.rs 내부 (pub(crate) 아님, 모듈 내부 전용)
enum Token {
    Operand(PdfObject),
    Keyword(Vec<u8>),  // ASCII keyword bytes, enum 변환은 C에서
}
```

구현:
1. `next_token(data: &[u8], pos: usize) -> Result<Option<(Token, usize)>, ParseError>` 내부 함수
   - `Token::Operand(PdfObject)`: `parse_object` 호출 (실패 시 `MalformedContentStream`)
   - `Token::Keyword(Vec<u8>)`: ASCII 키워드 바이트 수집 (`is_keyword_char` 사용)
   - 화이트스페이스·주석 처리: `skip_whitespace_and_comments` 재사용
2. `parse_content_stream` 구현 (B 단계):
   - 피연산자 누적 (`Vec<PdfObject>`) → Keyword 만나면 `ContentStreamOperation { operator: Unknown(keyword), operands, inline_data: None }` 생성 + 스택 클리어
   - BI는 아직 미지원 — `BI` 키워드 만나면 `MalformedContentStream` 반환 (Checkpoint D에서 교체)
   - q/Q 깊이 추적: B에서 시작 (`depth: i32`)

테스트 (B 단계 단위 테스트 목표: 10개):
- 빈 입력 → `Ok([])`
- 단일 숫자 피연산자 + 키워드 → `operands.len() == 1`
- 다중 피연산자 + 키워드 → `operands.len()` 검증
- 피연산자 없는 키워드 (`BT`) → `operands` 비어 있음
- 주석 포함 스트림 → 정상 무시
- 여러 연산 연속 → 순서 보장
- 배열 피연산자 (`TJ` 인자 `[(text)10(more)]`) → `PdfObject::Array` 정상 파싱
- 피연산자만 있고 키워드 없음 → 남은 스택 무시, `Ok([])`
- 잘못된 피연산자 시작 바이트 → `MalformedContentStream`
- q/Q 균형 스트림 → `Ok`

**완료 기준**: 위 단위 테스트 통과, `cargo clippy -- -D warnings` 경고 없음.

---

### Checkpoint C — 연산자 분류 (`keyword_to_operator`)

**목표**: 모든 73개 PDF 키워드를 `ContentStreamOperator` 변형으로 매핑. B에서 `Unknown(bytes)`로 저장하던 것을 의미 기반 enum으로 교체.

구현:
1. `keyword_to_operator(keyword: &[u8]) -> ContentStreamOperator` 내부 함수
   ```rust
   match keyword {
       b"BT" => BeginText,   b"ET" => EndText,
       b"Tc" => SetCharSpacing, b"Tw" => SetWordSpacing,
       b"cm" => ConcatMatrix, b"w"  => SetLineWidth,
       b"W"  => Clip,        b"W*" => ClipEvenOdd,
       // ... 전체 73개
       _ => Unknown(keyword.to_vec()),
   }
   ```
2. B 단계 `parse_content_stream`에서 Keyword → `keyword_to_operator` 호출로 교체
3. q/Q: B에서 `Unknown(b"q")`/`Unknown(b"Q")`로 쌓이던 것이 `SaveState`/`RestoreState`로 정확히 분류되므로, depth 추적 로직도 C에서 완성

테스트 (C 단계 단위 테스트 목표: 12개, 각 그룹 대표):
- Text: `BT` → `BeginText`, `Tj` → `ShowText`, `TJ` → `ShowTextAdjusted`, `'` → `MoveShowText`
- Graphics: `q` → `SaveState`, `Q` → `RestoreState`, `cm` → `ConcatMatrix`, `gs` → `SetGraphicsState`
- Path: `m` → `MoveTo`, `re` → `Rect`, `f*` → `FillEvenOdd`, `W*` → `ClipEvenOdd`
- Color: `RG` → `SetStrokeRGB`, `k` → `SetFillCMYK`
- XObject: `Do` → `InvokeXObject`
- MarkedContent: `BMC` → `BeginMarkedContent`, `EMC` → `EndMarkedContent`
- Compatibility: `BX` → `BeginCompatibility`
- Unknown: `xyz` → `Unknown(b"xyz".to_vec())`

**완료 기준**: 단위 테스트 통과, `cargo clippy -- -D warnings` 경고 없음.

---

### Checkpoint D-1 — 인라인 이미지 + q/Q 검증

**목표**: 인라인 이미지 완전 처리 + q/Q 불균형 에러 완성.

구현:
1. **인라인 이미지 파서** (`parse_inline_image`):
   - `BI` 키워드 감지 → dict key-value 쌍 파싱 (`Name + PdfObject` 반복)
   - `ID` 키워드 → raw bytes 수집 (`EI` 앞 공백(SP 또는 LF) 필수 규칙으로 탐지)
   - 결과: `ContentStreamOperation { operator: InlineImage, operands: [key-value...], inline_data: Some(bytes) }`
   - 실패: `ID` 없음, `EI` 없음, EOF → `MalformedInlineImage`
2. **q/Q 즉시 검증 완성** (C에서 partial 구현 → D-1에서 완성):
   - `Q`(RestoreState) 처리 시 `depth == 0` → `UnbalancedGraphicsState { offset, depth: -1 }` 즉시 에러
   - 파싱 완료 후 `depth > 0` → `UnbalancedGraphicsState { offset: data.len(), depth }` 에러
3. B에서의 `BI` 임시 에러 제거, `parse_inline_image` 통합

테스트 (D-1 단계 단위 테스트 목표: 10개):
- 인라인 이미지 정상 파싱 (합성 `BI /W 10 /H 10 /CS /RGB /BPC 8\nID <raw> EI`)
- `ID` 없는 BI → `MalformedInlineImage`
- `EI` 없는 BI → `MalformedInlineImage`
- raw bytes 내 `EI` 같은 시퀀스 처리 (공백 없으면 EI 아님)
- q/Q 균형 → `Ok`
- q 2개 Q 2개 → `Ok`
- q 2개 Q 1개 파싱 완료 → `UnbalancedGraphicsState { depth: 1 }`
- Q 먼저 → `UnbalancedGraphicsState { depth: -1 }`의 `offset` 검증
- 전체 통합: BT/ET + q/Q + m/l/S 포함 스트림 → 순서, 연산자 분류, 피연산자 검증
- 빈 BT ET 쌍 → `operands` 비어 있는 `BeginText`, `EndText`

**완료 기준**: 단위 테스트 통과, `cargo clippy -- -D warnings` 경고 없음.

---

### Checkpoint D-2 — 실제 PDF content stream 사전 확인

**목표**: 실제 PDF에서 content stream 바이트 추출 가능 여부 확인. IT-12 케이스 결정 근거 확보.

작업:
1. `examples/` 샘플 PDF 중 적합한 파일 선택 (fw4-2024.pdf 또는 다른 샘플)
2. 1페이지 `/Contents` 객체 추출:
   - xref 파싱 → page tree 순회 → `/Contents` 간접 참조 해소 → stream 바이트 추출
   - 필요 시 FlateDecode 압축 해제
3. 추출된 스트림에 `parse_content_stream` 적용, 연산자 수 및 첫 10개 연산자 확인
4. 발견 사항 보고 → IT-12 기대 값 결정

**완료 기준**: 실제 PDF에서 content stream 추출 성공, IT-12에 사용할 파일 + 기대 값 확정.

---

### Checkpoint E-1 — IT-11 합성 통합 테스트

**목표**: 복합 합성 content stream으로 통합 테스트.

테스트:
- 텍스트 + 경로 + 색상 + q/Q + 인라인 이미지 포함 수작업 스트림
- 연산자 수, 순서, 피연산자 값, 그룹 분류 모두 검증
- 파일: `tests/parser/integration_tests.rs` (IT-11)

---

### Checkpoint E-2 — IT-12 실제 PDF 통합 테스트

**목표**: D-2 사전 확인으로 결정된 실제 PDF + 기대 값으로 통합 테스트.

테스트:
- 실제 PDF content stream → 연산자 수 + 첫 몇 개 연산자 검증
- 파일: `tests/parser/integration_tests.rs` (IT-12)

---

### Checkpoint E-3 — proptest

**목표**: 임의 입력에 panic 없음 확인.

테스트:
- `arbitrary_input_never_panics_parse_content_stream` 1종
- 파일: `tests/parser/fuzz_tests.rs` 확장

---

### Checkpoint E-4 — 완료 보고서 + PR

1. `mydocs/working/task7-done.md` 완료 보고서 작성
2. `cargo test` 전체 통과, `cargo clippy -- -D warnings` 경고 없음, `cargo fmt --check` 통과 확인
3. PR 생성 (`closes #12`)

---

## 테스트 전략 요약

| 단계 | 테스트 수 | 파일 |
|-----|---------|-----|
| A | 1 | `tests/parser/content_stream_tests.rs` |
| B | 10 | 동일 |
| C | 12 | 동일 |
| D-1 | 10 | 동일 |
| E-1 | 1 (IT-11) | `tests/parser/integration_tests.rs` |
| E-2 | 1 (IT-12) | 동일 |
| E-3 | 1 (proptest) | `tests/parser/fuzz_tests.rs` |
| **합계** | **36개 이상** | |

private/`pub(crate)` 함수 테스트가 필요하면 `content_stream.rs` 내부 `#[cfg(test)] mod internal_tests {}` 사용.

---

## 의존성

| 크레이트 | 사용 목적 | 신규 여부 |
|--------|--------|--------|
| `rpdf-core` | `ContentStreamOperator`, `ContentStreamOperation`, `PdfObject` | 신규 타입 추가 |
| `rpdf-parser` (내부) | `parse_object`, `skip_whitespace_and_comments` 재사용 | 기존 |
| `proptest` | 임의 입력 테스트 | 기존 dev-dep |

**새 외부 크레이트 없음.**

---

## 엣지 케이스 목록

| 케이스 | 처리 |
|------|-----|
| 빈 스트림 | `Ok(vec![])` |
| 피연산자만 있고 연산자 없음 | 남은 스택 무시 (엔트리 생성 안 함) |
| 알 수 없는 연산자 | `Unknown(bytes)` 보존 |
| `'` `"` 연산자 | 피연산자 파싱 완료 후 키워드 위치에서만 등장 — `MoveShowText`, `MoveSetShowText`로 분류 |
| 인라인 이미지 raw bytes 안에 `EI`처럼 보이는 시퀀스 | `EI` 앞 공백(SP/LF) 필수 규칙 적용 |
| `Q` 만났을 때 depth == 0 | `UnbalancedGraphicsState` 즉시 에러 |
| 파싱 완료 후 depth > 0 | `UnbalancedGraphicsState { offset: data.len() }` |
| Indirect Reference(`N G R`) | `parse_object`가 Reference를 파싱하면 그대로 담기 — 사용하지 않음 |
| 스트림 필터 미적용 raw bytes | 호출자 책임 (Task #8에서 FlateDecode 후 전달) |

---

## 파일 변경 요약 (예상)

| 파일 | 변경 |
|-----|-----|
| `crates/rpdf-core/src/types/content_stream.rs` | 신규 (~90줄) |
| `crates/rpdf-core/src/types/mod.rs` | `pub mod content_stream` + re-export |
| `crates/rpdf-parser/src/content_stream.rs` | 신규 (~400줄) |
| `crates/rpdf-parser/src/error.rs` | 변형 3개 추가 |
| `crates/rpdf-parser/src/lib.rs` | `mod content_stream` + `pub use` |
| `crates/rpdf-parser/tests/parser/content_stream_tests.rs` | 신규 (~270줄) |
| `crates/rpdf-parser/tests/parser/mod.rs` | `mod content_stream_tests` 등록 |
| `crates/rpdf-parser/tests/parser/integration_tests.rs` | IT-11, IT-12 추가 |
| `crates/rpdf-parser/tests/parser/fuzz_tests.rs` | proptest 1개 추가 |

---

## 위험 요소

| 위험 | 가능성 | 대응 |
|-----|------|-----|
| 인라인 이미지 `EI` 탐지 오류 | 중 | 공백 선행 규칙 엄격 적용 + 테스트 케이스 |
| `'` `"` 연산자 파싱 충돌 | 낮음 | content stream에서 `(string)`과 `'`/`"` 연산자는 위치가 다름 (피연산자 파싱 후 키워드 위치에서만 등장) |
| 실제 PDF content stream 예상치 못한 구조 | 중 | D-2 사전 확인 후 IT-12 케이스 결정 |

---

## 참고

- ISO 32000-1 §7.8 (Content Streams), §8 (Graphics), §9 (Text)
- Task #6 완료 보고서: `mydocs/working/task6-done.md`
- Task #8 계획(예정): `mydocs/plans/task8-document-ir.md`
