# PDF 스펙 요약

이 문서는 PDF 포맷(ISO 32000)의 핵심을 rpdf 구현에 필요한 수준으로 정리한 것입니다. 전체 스펙은 1,000페이지가 넘으므로, 편집기에 꼭 필요한 것만 발췌합니다.

## 참고 자료

- ISO 32000-1:2008 (PDF 1.7) — 공개 표준
- ISO 32000-2:2020 (PDF 2.0)
- Adobe PDF Reference 1.7 (비공식적으로 통용되는 레퍼런스)
- `lopdf` 크레이트 소스 (Rust 구현 레퍼런스)

## PDF 파일의 기본 구조

PDF 파일은 다음 4개 영역으로 구성됩니다.

```
┌─────────────────────────────┐
│  Header                     │  예: %PDF-1.7
├─────────────────────────────┤
│  Body (객체들)               │  1 0 obj, 2 0 obj, ...
│                             │
├─────────────────────────────┤
│  Cross-Reference Table      │  xref 테이블
├─────────────────────────────┤
│  Trailer                    │  루트 객체 참조, 파일 메타
└─────────────────────────────┘
                   %%EOF
```

### Header
`%PDF-1.x` 또는 `%PDF-2.0` 형태. 첫 줄에 위치.

### Body
번호가 매겨진 객체들의 모음. 각 객체는 고유한 `object_id generation_id obj` ... `endobj` 형태.

### Xref Table
각 객체가 파일의 어느 오프셋에 위치하는지 매핑. 파싱의 시작점.

### Trailer
- `Root`: 루트 객체 (Catalog)의 참조
- `Size`: 전체 객체 수
- `Info`: 메타데이터 딕셔너리 참조
- `startxref`: xref 테이블의 오프셋

## 기본 객체 타입

PDF는 8가지 기본 객체 타입을 가집니다.

| 타입 | 예시 | Rust 표현 |
| --- | --- | --- |
| Boolean | `true`, `false` | `bool` |
| Number | `42`, `3.14` | `f64` 또는 `i64` |
| String | `(hello)`, `<48656c6c6f>` | `Vec<u8>` (바이너리일 수 있음) |
| Name | `/Type`, `/Page` | `String` with `/` 접두 |
| Array | `[1 2 3]` | `Vec<Object>` |
| Dictionary | `<< /Key /Value >>` | `BTreeMap<String, Object>` |
| Stream | `<< ... >> stream ... endstream` | `(Dictionary, Vec<u8>)` |
| Null | `null` | `Option::None` |

+ Indirect Reference: `3 0 R` (객체 ID 3, 세대 0 참조)

## 도큐먼트 구조 트리

```
Catalog (Root)
  ├─ Pages (Page Tree Root)
  │    ├─ Kids
  │    │   ├─ Page 1
  │    │   │    ├─ Contents (content stream)
  │    │   │    ├─ Resources (폰트, 이미지 등)
  │    │   │    └─ MediaBox, CropBox
  │    │   ├─ Page 2
  │    │   └─ ...
  │    └─ Count
  ├─ Metadata (XMP)
  ├─ Outlines (북마크)
  ├─ Names (링크, 임베디드 파일)
  └─ AcroForm (폼 필드)
```

## Content Stream

각 페이지의 실제 내용은 content stream 안의 **연산자 시퀀스**로 표현됩니다.

### 주요 연산자

```
BT              % Begin Text
  /F1 12 Tf     % 폰트 F1, 크기 12
  100 200 Td    % 좌표 (100, 200)으로 이동
  (Hello) Tj    % "Hello" 출력
ET              % End Text

q               % save graphics state
  1 0 0 1 50 50 cm   % 변환 행렬 (이동)
  100 100 m          % moveto (100, 100)
  200 100 l          % lineto (200, 100)
  200 200 l
  100 200 l
  h                  % close path
  S                  % stroke
Q               % restore graphics state
```

### 좌표계

- 원점: 페이지 **왼쪽 하단** (일반 이미지와 반대)
- 단위: 1/72 인치 (1 포인트)
- A4 페이지: 595.28 × 841.89 포인트

## 주요 딕셔너리

### Page 딕셔너리

```
<<
  /Type /Page
  /Parent 3 0 R
  /MediaBox [0 0 595.28 841.89]
  /Contents 10 0 R
  /Resources << /Font << /F1 20 0 R >> >>
  /Rotate 0
  /Annots [30 0 R 31 0 R]
>>
```

### 주요 필드

| 필드 | 타입 | 설명 |
| --- | --- | --- |
| `/MediaBox` | Array | 페이지 물리적 크기 |
| `/CropBox` | Array | 표시 영역 (선택) |
| `/Rotate` | Number | 회전 각도 (0, 90, 180, 270) |
| `/Contents` | Stream or Array | 페이지 내용 |
| `/Resources` | Dict | 폰트·이미지·패턴 등 |
| `/Annots` | Array | 주석 목록 |

## Annotation 타입

편집기에서 다룰 주요 주석 타입:

| Subtype | 용도 |
| --- | --- |
| `/Text` | 스티키 노트 |
| `/Highlight` | 형광펜 |
| `/Underline` | 밑줄 |
| `/StrikeOut` | 취소선 |
| `/FreeText` | 텍스트 박스 |
| `/Stamp` | 도장 이미지 |
| `/Ink` | 자유 그리기 |
| `/Square`, `/Circle` | 도형 |
| `/Link` | 하이퍼링크 |
| `/Widget` | 폼 필드 |
| `/Signature` | 디지털 서명 |

## 편집 시 주의할 점

### 1. Incremental Update

PDF의 강점 중 하나는 **기존 파일을 건드리지 않고 뒤에 추가만** 해서 업데이트할 수 있다는 점입니다.

```
[원본 내용]
[새 xref]
[새 trailer with /Prev 포인터]
%%EOF
```

이렇게 하면:
- 원본 보존 (디지털 서명 유지)
- 저장 빠름
- 실행 취소 이력 복구 가능

rpdf는 기본적으로 incremental update를 사용합니다.

### 2. 객체 참조 관리

객체 A가 B를 참조하는데 B를 지우면 안 됩니다. 편집 시에는 **참조 그래프**를 추적해야 합니다.

### 3. Content Stream 재작성

페이지 내용을 수정하려면 content stream을 파싱하고 다시 생성해야 합니다. 주석 추가는 상대적으로 쉽지만, 기존 텍스트 수정은 매우 어렵습니다 (Phase 2 이후).

### 4. 폰트 임베딩

PDF가 폰트를 포함(`embedded`)하는 경우와 시스템 폰트에 의존하는 경우가 있습니다. 편집 시 새 텍스트를 추가하려면 폰트 임베딩 또는 표준 14 폰트 사용이 필요합니다.

### 표준 14 폰트

이 14개 폰트는 모든 PDF 뷰어가 기본 지원해야 합니다.

```
Times-Roman, Times-Bold, Times-Italic, Times-BoldItalic
Helvetica, Helvetica-Bold, Helvetica-Oblique, Helvetica-BoldOblique
Courier, Courier-Bold, Courier-Oblique, Courier-BoldOblique
Symbol, ZapfDingbats
```

한글은 표준 14에 없으므로 한글 텍스트를 추가하려면 반드시 폰트를 임베드해야 합니다.

## 암호화

PDF는 다음 암호화 방식을 지원합니다.

- RC4 40-bit (deprecated)
- RC4 128-bit
- AES 128-bit
- AES 256-bit (PDF 2.0)

rpdf는 일단 복호화만 지원하고, 새로 암호화하는 기능은 Phase 후반에 고려합니다.

## rpdf에서 지원할 범위 (v0.1~v0.5)

**지원**
- PDF 1.4 ~ 2.0
- 기본 객체 및 xref 파싱
- 페이지 회전·삭제·병합·분할
- 썸네일 및 SVG 렌더링 (pdfium-render 위임)
- 주석 추가 (Highlight, FreeText, Stamp)
- Incremental save

**보류**
- 암호화된 PDF의 암호 해제 (v0.3 이후 고려)
- 폼 필드 편집 (v0.7 이후)
- 기존 텍스트 직접 편집 (v1.0 이후)
- 디지털 서명 검증/생성 (장기 보류)
- XFA 폼 (보류)

## 참고 크레이트

| 크레이트 | 용도 | 비고 |
| --- | --- | --- |
| `lopdf` | 파싱, 저장, 편집 | 기본 선택 |
| `pdfium-render` | 렌더링 | PDFium 바인딩, 렌더링 품질 최고 |
| `pdf` | 대안 파서 | 일부 케이스에서 lopdf보다 견고 |
| `printpdf` | PDF 생성 | 편집기에는 부적합 |

## 외부 참고 링크

- ISO 32000 무료 공개: https://www.iso.org/standard/75839.html
- PDF 구조 시각화: https://blog.idrsolutions.com/understanding-the-pdf-file-format/
- lopdf 문서: https://docs.rs/lopdf/
