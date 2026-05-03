# 트러블슈팅 — {제목}

**발생일**: yyyy-mm-dd
**해결일**: yyyy-mm-dd
**관련 Issue**: #{N}
**심각도**: 치명 / 높음 / 중간 / 낮음
**환경**: Rust 1.75 / Windows 11 / PDF 버전 1.7

## 증상

실제로 관찰된 현상:

- 증상 1
- 증상 2
- 에러 메시지: `...`

## 재현 방법

최소 재현 케이스:

```bash
cargo run -- info samples/problematic.pdf
```

```
{실제 출력}
```

## 원인 분석

### 1차 가설
`...`이라고 생각했으나, 디버깅 결과 다름.

### 2차 가설
`...` 가능성을 확인하기 위해 {방법}으로 검증.

### 최종 원인
근본 원인: {원인 설명}

**관련 코드 위치**: `src/parser/xref.rs:123`

**왜 이 문제가 발생했는가**:
{맥락과 설계 의사결정의 실수 또는 스펙 해석 오류 등}

## 해결책

### 적용한 수정
```rust
// 변경 전
fn parse_xref(&mut self) -> Result<()> {
    let offset = self.find_startxref()?;
    self.seek(offset);
    // ...
}

// 변경 후
fn parse_xref(&mut self) -> Result<()> {
    let offset = self.find_startxref()?;
    if offset >= self.buffer.len() {
        return Err(RpdfError::CorruptedXref);
    }
    self.seek(offset);
    // ...
}
```

### 테스트 추가
회귀 방지용 테스트:

```rust
#[test]
fn corrupted_xref_offset_returns_error() {
    let bytes = include_bytes!("../samples/corrupted_xref.pdf");
    let result = Document::from_bytes(bytes);
    assert!(matches!(result, Err(RpdfError::CorruptedXref)));
}
```

## 재발 방지

- 코드 리뷰 체크리스트에 "offset 범위 검증" 추가
- 속성 기반 테스트에 랜덤 바이트 fuzz 강화
- 문서 업데이트: `mydocs/tech/pdf-spec-summary.md`에 주의사항 추가

## 배운 점

- PDF 스펙의 startxref 관련 서브섹션을 다시 읽으면서 발견: "{해석 요점}"
- `lopdf`의 기본 동작과 다름에 주의 필요
- 외부 라이브러리의 관대한 파싱 동작에 의존하면 위험

## 참고 자료

- ISO 32000-1:2008 §7.5.8
- lopdf issue: https://github.com/.../issues/{N}
- 관련 PR: #{N}
