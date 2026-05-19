# Task #25 완료 보고서 — wasm_api 모듈 작성

**Issue**: #48  
**브랜치**: `local/task25`  
**완료일**: 2026-05-19  
**마일스톤**: M040 (v0.4 WASM 바인딩)

---

## 구현 결과

`rpdf-wasm` crate를 신규 생성해 JS/TypeScript에서 PDF 파싱·편집·저장 API를 제공한다.

```typescript
const pdf = new PdfDocument(bytes);          // PDF 로드
pdf.page_count();                            // 페이지 수
pdf.page_info(0);                            // { index, rotation, media_box, crop_box }
pdf.rotate_page(0, 90);                      // 0번 페이지 90도 회전
pdf.delete_pages([1, 3]);                    // 1, 3번 페이지 삭제
pdf.undo(); pdf.redo();                      // 실행 취소/다시 실행
const out = pdf.save();                      // Vec<u8> → Uint8Array
```

---

## 변경 파일

### 신규

```
crates/rpdf-wasm/Cargo.toml    — cdylib+rlib, wasm-bindgen 의존
crates/rpdf-wasm/src/lib.rs    — PdfDocument 전체 구현 (18 테스트)
mydocs/plans/task25-wasm-api.md
```

### 수정

```
Cargo.toml                          — workspace.dependencies에 wasm-bindgen/serde-wasm-bindgen/js-sys 추가
crates/rpdf-serializer/src/types.rs — PageSource에 #[derive(Clone)] 추가
```

---

## 체크포인트별 결과

| CP | 내용 | 결과 |
|----|------|------|
| CP-A | crate 생성 + wasm-pack 빌드 | ✅ |
| CP-B | new / page_count / save / page_info | ✅ |
| CP-C | rotate_page / delete_pages / undo / redo | ✅ |
| CP-D | wasm-pack release 빌드 + gzip 크기 | ✅ 344KB |

---

## 테스트 결과

```
cargo test -p rpdf-wasm
  18 passed; 0 failed
cargo test (전체 워크스페이스)
  전체 통과
wasm-pack build --target web
  pkg/rpdf_wasm_bg.wasm gzip: 344KB (2MB 이하)
```

---

## 계획서와 다르게 구현된 사항

| 항목 | 계획서 | 실제 구현 | 이유 |
|------|--------|----------|------|
| execute_cmd sources_undo push 방식 | rotate=None, delete=Some 구분 | 항상 push | evaluator 지적: 혼합 커맨드 탈동기화 방지 |
| UT-07 구현 | delete_pages API 직접 호출 | validate_delete_indices 헬퍼 테스트 | JsError::new 비-wasm 환경 격리 |
| 테스트 수 | 12개 | 18개 | UT-13 혼합 시나리오 + 내부 헬퍼 테스트 추가 |

---

## plan-eng-review 발견 이슈 처리

| 이슈 | 계획서 반영 | 구현 반영 |
|------|------------|----------|
| sources_undo/redo 스냅샷 패턴 | ✅ | ✅ |
| execute_cmd 타이밍 버그 (Gemini) | ✅ | ✅ |
| 내부 헬퍼 분리 (Gemini) | ✅ | ✅ |
| js_sys::Error 사용 (Gemini) | ✅ | ✅ |
| Vec<u32> 타입 (Gemini) | ✅ | ✅ |
| 혼합 커맨드 탈동기화 (evaluator) | — | ✅ |

---

## 트러블슈팅 후보 분류

| 항목 | 분류 | 처리 |
|------|------|------|
| undo/redo 시 CommandStack과 sources 스택 높이를 항상 일치시켜야 함 — rotate처럼 sources 불변 커맨드도 sentinel push 필요 | A | CLAUDE.md "스택 기반 상태 관리" 사례 추가 |
| JsError::new()는 비-wasm 네이티브 테스트에서 사용 불가 — 내부 헬퍼를 JsValue-free로 분리해야 네이티브 cargo test 가능 | B | 트러블슈팅 문서 작성 |
| wasm-pack build 시 Cargo.toml에 description/repository/license 없으면 경고 — npm 배포 전 필수 | C | 완료 보고서 메모 (Task #28에서 처리) |

---

## 완료 기준 달성

1. ✅ `rpdf-wasm` crate 신규 생성, `wasm-pack build --target web` 성공
2. ✅ `PdfDocument` 공개 API — new, page_count, page_info, rotate_page, delete_pages, save, undo, redo, undo_len, redo_len
3. ✅ 네이티브 단위 테스트 UT-01~UT-13 (18개) 모두 통과
4. ✅ `cargo clippy -p rpdf-wasm -- -D warnings` 경고 없음
5. ✅ WASM 번들 크기 344KB (2MB 이하)
