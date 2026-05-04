---
title: assert_cmd이 predicates를 re-export하지 않음
date: 2026-05-04
task: "#9"
---

## 증상

`assert_cmd`를 dev-dependency로 추가하고 통합 테스트에서 `.stdout(predicates::str::contains(...))`를
사용하면 컴파일 에러 발생:

```
error[E0433]: cannot find module or crate `predicates` in this scope
```

## 원인

`assert_cmd`는 내부적으로 `predicates` 크레이트에 의존하지만 pub re-export하지 않는다.
`predicates::str::contains`를 직접 사용하려면 별도로 선언해야 한다.

## 해결

`Cargo.toml` dev-dependencies에 `predicates`를 명시적으로 추가:

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

## 교훈

`assert_cmd` 공식 예시 코드는 `predicates`를 함께 보여주는데, 예시에 항상
두 크레이트가 함께 선언되어 있다. 이걸 보고 재-export라고 착각하기 쉽다.
binary crate의 통합 테스트 scaffold 시 두 크레이트를 항상 함께 추가할 것.
