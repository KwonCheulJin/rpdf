# xref 엔트리 포맷: "f \r\n" vs "f\r\n" 혼동

## 증상

```
MalformedXref { offset: 133, reason: "비표준 항목 EOL: [32, 13] (\\r\\n 또는 ' '\\n 만 허용)" }
```

IT-8 합성 hybrid PDF 테스트(`make_hybrid_pdf_for_it8`)에서 `parse_xref`가 MalformedXref를 반환.

## 원인

PDF 스펙(ISO 32000 §7.5.4)의 xref 엔트리는 **정확히 20바이트**:

```
oooooooooo ggggg k EOL
 (10자리)  (5자리) (1자리) (2바이트 EOL)
```

허용되는 2바이트 EOL:
- `\r\n` = [13, 10] — CR + LF
- ` \n` = [32, 10] — space + LF

**잘못된 패턴**: `"0000000000 65535 f \r\n"` (21바이트)
- 타입 문자(`f`) 뒤에 공백을 추가하면 EOL 영역이 `[' ', '\r']` = `[32, 13]`이 됨
- 이는 허용된 두 가지 EOL 어느 것도 아님 → `MalformedXref`

```
"0000000000 65535 f \r\n"
                  ^ 여기가 문제: f 뒤 공백 하나가 끼어들어
                    entry[17]='f', entry[18]=' ', entry[19]='\r'
                    (entry[18..20] = [32, 13]) → 거부됨
```

## 해결

테스트 헬퍼에서 타입 문자 직후 EOL을 붙인다:

```rust
// 잘못됨 (21바이트)
buf.extend_from_slice(b"0000000000 65535 f \r\n");

// 올바름 (20바이트)
buf.extend_from_slice(b"0000000000 65535 f\r\n");
```

기존 `make_entry` 헬퍼(xref_tests.rs)를 참고:
```rust
fn make_entry(offset_or_next: u64, generation: u16, kind: char) -> Vec<u8> {
    format!("{:010} {:05} {}\r\n", ...).into_bytes()  // 20바이트 ✓
}
```

## 영향 범위

- **생산 코드**: 영향 없음. `parse_xref_entry`가 항상 정확히 20바이트를 읽고 EOL을 검증함.
- **Task #3 코드**: `make_entry` 헬퍼가 이미 올바른 형식을 사용하므로 영향 없음.
- **발생 시점**: Task #5 E-1 단계, `make_hybrid_pdf_for_it8()` 신규 작성 시.
- **탐지**: IT-8 런타임 실패로 즉시 탐지됨 (파서의 엄격한 EOL 검증 덕분).

## 재발 방지

1. `make_entry` 헬퍼(xref_tests.rs) 사용 우선 — 직접 바이트 리터럴 작성 지양
2. 직접 작성 시 `assert_eq!(entry.len(), 20)` 추가
3. 회귀 테스트: `reject_malformed_entry_space_before_cr_lf` (xref_tests.rs)

## 참고

- `parse_xref_entry`: `crates/rpdf-parser/src/xref.rs:297`
- 관련 테스트: `xref_tests.rs::reject_malformed_entry_space_before_cr_lf`
