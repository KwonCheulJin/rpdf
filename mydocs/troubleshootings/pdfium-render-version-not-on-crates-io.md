# pdfium-render 버전 crates.io 미존재

**발생 Task**: #11  
**날짜**: 2026-05-04

---

## 증상

`Cargo.toml`에 `pdfium-render = "0.9.2"`를 지정했을 때 `cargo fetch` 실패:

```
error: failed to select a version for the requirement `pdfium-render = "^0.9.2"`
candidate versions found which didn't match: 0.9.1, 0.9.0, 0.8.37, ...
```

---

## 원인

명세서(CLAUDE.md 태스크 지시)에 0.9.2가 명시되어 있었으나,
실제 crates.io에는 0.9.2가 등록되지 않았다. 최신 버전은 0.9.1이었다.

---

## 해결

`pdfium-render = "0.9.1"`로 수정.

0.9.1 README에 따르면 최신 호환 pdfium 빌드번호는 **7763** (feature `pdfium_7763`).
빌드번호도 명세의 6721에서 7763으로 수정했다.

---

## 예방

외부 크레이트 버전 지정 시 사전에 `cargo search <crate>` 또는 crates.io에서 실제 최신 버전 확인 후 기재한다.
명세 문서의 버전은 "최소 요구사항"이 아닌 "실제 설치된 버전"을 기재한다.

---

## pdfium-render ↔ pdfium 빌드번호 대응표

| pdfium-render | pdfium 빌드번호 |
|---------------|----------------|
| 0.9.1 | 7763 |
| 0.8.37 | 7543 |
| 0.8.x | README `pdfium_latest` feature 참조 |
