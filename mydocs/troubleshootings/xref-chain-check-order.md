# 트러블슈팅 — xref chain 순환 감지: visited 검사와 depth 검사 순서 오류

**발생일**: 2026-05-03
**해결일**: 2026-05-03
**관련 Issue**: #4
**심각도**: 높음
**환경**: Rust 2024 edition / rpdf-parser Task #3

## 증상

- xref chain에 순환이 존재하고 chain 길이가 정확히 `MAX_XREF_CHAIN_DEPTH`(100)인 경우, `XrefChainCycle` 대신 `XrefChainTooDeep`이 반환됨
- 설계 명세: "순환 chain은 항상 `XrefChainCycle`로 보고된다"가 이 엣지 케이스에서 위반됨

## 재현 방법

100개의 고유한 오프셋으로 이루어진 순환 xref chain을 구성한다:

```rust
let (data, start) = build_chain(100, true); // 100개 섹션, 마지막이 처음을 /Prev로 참조
let err = parse_xref(&data, start).unwrap_err();
// 기대: XrefChainCycle
// 실제(버그): XrefChainTooDeep { max_depth: 100 }
```

## 원인 분석

### 최종 원인

`parse_xref_chain` 함수 내 루프에서 `depth >= MAX_XREF_CHAIN_DEPTH` 검사를 `visited.contains(&current)` 검사보다 **먼저** 수행했다.

**버그 코드:**
```rust
loop {
    if depth >= MAX_XREF_CHAIN_DEPTH {       // ← 먼저 실행
        return Err(ParseError::XrefChainTooDeep { max_depth: MAX_XREF_CHAIN_DEPTH });
    }
    if visited.contains(&current) {          // ← 나중에 실행 (도달 못함)
        return Err(ParseError::XrefChainCycle { offset: current });
    }
    visited.insert(current);
    depth += 1;
    // ...
}
```

100개의 고유 오프셋이 있는 순환 chain에서:
- 첫 번째 오프셋부터 시작해 99개를 순회하면 `depth == 99`
- 100번째 반복에서 루프 시작 시 `depth == 100 >= MAX_XREF_CHAIN_DEPTH(100)`이 먼저 평가되어 `XrefChainTooDeep` 반환
- `visited` 검사에 도달하지 못하므로 순환임에도 `XrefChainCycle`이 반환되지 않음

**관련 코드 위치**: `crates/rpdf-parser/src/xref.rs` — `parse_xref_chain`

## 해결책

### 적용한 수정

`visited` 검사를 `depth` 검사보다 먼저 수행하도록 순서를 변경한다.

```rust
// 변경 전
loop {
    if depth >= MAX_XREF_CHAIN_DEPTH {
        return Err(ParseError::XrefChainTooDeep { max_depth: MAX_XREF_CHAIN_DEPTH });
    }
    if visited.contains(&current) {
        return Err(ParseError::XrefChainCycle { offset: current });
    }
    visited.insert(current);
    depth += 1;
    // ...
}

// 변경 후
loop {
    // visited 검사를 먼저: 순환 chain은 항상 XrefChainCycle로 보고
    if visited.contains(&current) {
        return Err(ParseError::XrefChainCycle { offset: current });
    }
    // depth 검사를 나중에: 비순환 비정상 chain에만 적용
    if depth >= MAX_XREF_CHAIN_DEPTH {
        return Err(ParseError::XrefChainTooDeep { max_depth: MAX_XREF_CHAIN_DEPTH });
    }
    visited.insert(current);
    depth += 1;
    // ...
}
```

### 테스트 추가

회귀 방지용 테스트 2개를 `tests/parser/xref_tests.rs`에 추가:

```rust
#[test]
fn cycle_with_exactly_100_unique_offsets_returns_cycle_not_too_deep() {
    // 정확히 100개 고유 오프셋을 가진 순환 chain
    let (data, start) = build_chain(100, true);
    let err = parse_xref(&data, start).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefChainCycle { .. }),
        "100개 순환 chain은 XrefChainCycle이어야 함, 실제: {err:?}"
    );
}

#[test]
fn non_cyclic_chain_of_101_returns_too_deep() {
    // 101개 비순환 chain → XrefChainTooDeep
    let (data, start) = build_chain(101, false);
    let err = parse_xref(&data, start).unwrap_err();
    assert!(
        matches!(err, ParseError::XrefChainTooDeep { max_depth: 100 }),
        "101개 비순환 chain은 XrefChainTooDeep이어야 함, 실제: {err:?}"
    );
}
```

## 재발 방지

- 루프 내에서 "정확한 경계값" 케이스(`depth == MAX`)를 항상 수동으로 추적한다.
- 서로 다른 에러 변형이 우선순위를 가지는 경우, **우선순위가 높은 검사를 먼저** 배치한다.
- 새로운 에러 변형을 추가할 때 "경계값 + 1" 케이스를 반드시 테스트한다.
- CLAUDE.md의 "에러 변형 도달 가능성" 원칙에 따라, 모든 에러 변형에 대해 발생 가능한 단위 테스트가 있어야 한다.

## 배운 점

- 에러 우선순위가 있는 다중 조건 루프에서 검사 순서는 의미론적으로 중요하다.
- 경계값(`depth == MAX`)에서의 동작을 명시적으로 테스트하지 않으면 이런 종류의 버그는 발견되지 않는다.
- 코드 리뷰(Checkpoint B 자가 검토)가 이 버그를 발견했다 — 로직의 "우선순위 역전" 패턴을 리뷰 항목으로 유지한다.

## 참고 자료

- ISO 32000-1:2008 §7.5.4 (Cross-Reference Table)
- 관련 Issue: #4
