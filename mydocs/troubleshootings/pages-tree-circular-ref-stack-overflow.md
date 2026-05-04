# 페이지 트리 순환 참조 → 스택 오버플로우

**발견 시점**: Task #10 (회귀 테스트 인프라), 2026-05-04  
**발견 경로**: `samples/trad-xref-pages-tree-refs.pdf` 스냅샷 테스트 실행 중

## 현상

`trad-xref-pages-tree-refs.pdf` 파일을 `load_document`로 파싱하면 스택 오버플로우(`SIGABRT`)가 발생한다.

```
thread 'regression::t7_trad_xref_pages_tree_refs' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

## 원인

파일 내부에 페이지 트리 순환 참조가 존재한다 (pdf.js 테스트 케이스 `Testcase: 'Pages loop'`):

```
Object 3 → /Kids [4 0 R]
Object 4 → /Kids [5 0 R]
Object 5 → /Kids [3 0 R]  ← 3 → 4 → 5 → 3 순환!
```

rpdf-parser의 페이지 트리 순회 로직(`load_document` 내부)이 순환 참조를 감지하지 못하고 무한 재귀에 빠진다.

## 조치

- **단기**: `samples/trad-xref-pages-tree-refs.pdf` 제거. `trad-xref-issue1155r.pdf`로 교체.
- **장기**: 페이지 트리 순회 시 방문한 객체 번호를 추적하여 순환 참조 감지 후 `ParseError::CircularPageTree` 반환.

## 수정 가이드

`load_document` (또는 페이지 트리 순회 함수) 내부에 방문 집합 추가:

```rust
fn traverse_page_tree(
    xref: &XrefTable,
    raw: &[u8],
    node_ref: ObjRef,
    visited: &mut HashSet<ObjRef>,  // 추가
    ...
) -> Result<Vec<Page>, ParseError> {
    if !visited.insert(node_ref) {
        return Err(ParseError::CircularPageTree { obj: node_ref });
    }
    ...
}
```

새 `ParseError::CircularPageTree { obj_num: u64 }` 변형 추가.

## 재현 케이스

```rust
// 재현: pdf.js test/pdfs/Pages-tree-refs.pdf
// 원본 URL: https://github.com/mozilla/pdf.js/blob/master/test/pdfs/Pages-tree-refs.pdf
// Apache 2.0
let bytes = std::fs::read("samples/trad-xref-pages-tree-refs.pdf").unwrap();
let result = load_document(&bytes);  // 현재: 스택 오버플로우, 기대: Err(CircularPageTree)
```

수정 완료 후 위 케이스가 `Err(ParseError::CircularPageTree { ... })`를 반환하면 테스트 통과.

## 관련

- `v0.1-parser-skeleton.md`: "깨진 xref: 에러 반환 (복구 없음)" 방침과 일관됨
- 수정 Issue: v0.2 또는 별도 버그 수정 Issue 등록 권장
