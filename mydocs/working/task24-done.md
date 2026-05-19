# Task #24 완료 보고서 — CLI 편집 명령 추가

**Issue**: #46  
**브랜치**: `local/task24`  
**완료일**: 2026-05-19  
**마일스톤**: M030 (v0.3 편집 커맨드)

---

## 구현 결과

5개 편집 서브커맨드가 `rpdf` CLI에 추가되었다.

```
rpdf merge a.pdf b.pdf c.pdf -o merged.pdf
rpdf split input.pdf --pages 1-3,5,7-10 -o out_dir/
rpdf rotate input.pdf --page 3 --degrees 90 -o rotated.pdf
rpdf delete input.pdf --pages 2,4,6 -o deleted.pdf
rpdf extract input.pdf --pages 5-10 -o excerpt.pdf
```

---

## 변경 파일

### 신규

```
crates/rpdf-cli/src/commands/edit/mod.rs    — parse_page_list, parse_single_range, split_output_path, parse_ranges_for_split + 단위 테스트
crates/rpdf-cli/src/commands/edit/merge.rs  — merge 핸들러
crates/rpdf-cli/src/commands/edit/rotate.rs — rotate 핸들러
crates/rpdf-cli/src/commands/edit/delete.rs — delete 핸들러
crates/rpdf-cli/src/commands/edit/extract.rs — extract 핸들러
crates/rpdf-cli/src/commands/edit/split.rs  — split 핸들러
```

### 수정

```
crates/rpdf-cli/Cargo.toml          — rpdf-edit, rpdf-serializer 추가; dev-dep rpdf-parser 추가
crates/rpdf-cli/src/main.rs         — 5개 서브커맨드 추가
crates/rpdf-cli/src/commands/mod.rs — pub mod edit 추가
crates/rpdf-cli/tests/cli_tests.rs  — IT-F1~F7 roundtrip 테스트 추가
```

---

## 체크포인트별 결과

| CP | 내용 | 결과 |
|----|------|------|
| CP-A | Cargo.toml 수정 + 빌드 통과 | ✅ |
| CP-B | 유틸리티 + merge/rotate 구현 | ✅ |
| CP-C | delete/extract/split 구현 + 통합 테스트 | ✅ |
| CP-D | clippy + fmt + 전체 테스트 | ✅ |

---

## 테스트 결과

```
cargo test -p rpdf-cli
  unit (24): ok
  cli_tests (19): ok
  render_tests (14): ok
```

---

## 계획서와 다르게 구현된 사항

| 항목 | 계획서 | 실제 구현 | 이유 |
|------|--------|----------|------|
| `PageSource` 슬라이싱 | `.to_vec()` 패턴 | `into_iter().enumerate().filter()` + 수동 `PageSource` 재구성 | `PageSource: !Clone` |
| split ranges 접근 | SplitCommand.ranges 재사용 | `parse_ranges_for_split` 헬퍼로 spec 재파싱 | `ranges` 필드 private |
| rotate page 0 검증 | 명시 없음 | 명시적 bail! 추가 | evaluator 지적 — silent failure 방지 |

---

## 트러블슈팅 후보 분류

| 항목 | 분류 | 처리 |
|------|------|------|
| `PageSource: !Clone` — `.to_vec()` 패턴 불가, into_iter 수동 재구성 필요 | B | 트러블슈팅 문서 작성 |
| rotate `saturating_sub` 사용 → `page=0` silent failure — `bail!`로 수정 | A | CLAUDE.md "외부 입력 검증" 사례로 기록 (기존 원칙에 부합, 추가 룰 불필요) |
| split `SplitCommand.ranges` private → CLI에서 spec 재파싱 필요 — DRY 약한 위반 | C | 완료 보고서 메모 (v0.4에서 접근자 노출 고려) |

---

## 완료 기준 달성

1. ✅ 5개 편집 CLI 명령이 `rpdf --help`에 나타남
2. ✅ 각 명령이 실제 PDF에 대해 정상 동작 (roundtrip 테스트)
3. ✅ 잘못된 입력 시 exit 1 + 명확한 에러 메시지
4. ✅ 전체 테스트 통과 + clippy 경고 없음
