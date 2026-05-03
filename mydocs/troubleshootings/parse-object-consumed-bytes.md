# parse_object 두 번째 반환값: consumed bytes 오용

**발생 태스크**: Task #7 (Content Stream 파서)
**발생 파일**: `crates/rpdf-parser/src/content_stream.rs` — `next_token` 함수

---

## 증상

`parse_content_stream`이 `q` 연산자 없이도 `UnbalancedGraphicsState { depth: -1 }` 에러를 반환.
또는 `parse_content_stream`이 토큰을 건너뛰거나 이미 처리한 위치로 역방향 이동.

단위 테스트 `graphics_state_operators`, `move_show_text_operators`가 예상 연산자 대신
`UnbalancedGraphicsState` 에러로 실패.

## 원인

`parse_object(data, pos)` 시그니처:

```rust
pub fn parse_object(data: &[u8], offset: usize) -> Result<(PdfObject, usize), ParseError>
```

두 번째 반환값 `consumed`는 **`offset`부터 소비된 바이트 수(상대값)**.  
절대 위치(다음 읽기 시작 지점)가 아님.

오용 코드:

```rust
// WRONG: consumed는 절대 위치가 아님
let (obj, consumed) = parse_object(data, pos)?;
Ok(Some((Token::Operand(obj), consumed)))  // next pos = consumed (잘못된 값)
```

`pos = 50`, `consumed = 5`이면 실제 다음 위치는 `55`인데 `5`를 반환.  
이후 루프에서 `pos = 5`가 되어 이미 처리한 토큰을 재처리하거나 엉뚱한 위치를 읽음.

## 해결

```rust
// CORRECT: pos + consumed로 절대 위치 복원
let (obj, consumed) = parse_object(data, pos)?;
Ok(Some((Token::Operand(obj), pos + consumed)))
```

`parse_object` doc comment에 명시:

```
/// `Ok((object, consumed))` — `consumed`는 `offset`부터 소비된 총 바이트 수
/// (선행 화이트스페이스 포함). **절대 위치가 필요하면 `offset + consumed`로 계산.**
```

## 확인 방법

`parse_object_with_depth` 구현 내부:

```rust
let ws = pos - offset;            // 선행 공백 바이트 수
Ok((obj, ws + consumed))          // 상대 오프셋 합산
```

반환값이 상대값임을 소스에서 직접 확인 가능.

## 영향 범위

- `parse_object` 자체는 정상. Task #4 구현 및 기존 동작 무관.
- `parse_object`를 **래퍼 루프 내에서 호출**하는 모든 코드가 취약점 대상:
  - `next_token` (Task #7, 수정 완료)
  - Task #8, #9에서 동일 패턴 사용 시 주의 필요

## 재발 방지

- `parse_object` doc comment에 `offset + consumed` 계산 명시 (완료)
- 회귀 테스트: Task #7 단위 테스트 29개 (`graphics_state_operators` 등)가 커버
- 새 코드에서 `parse_object` 결과를 다음 위치로 직접 사용하면 코드 리뷰 시 지적
