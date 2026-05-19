# Task #24 계획서 — CLI 편집 명령 추가

**Issue**: #46  
**브랜치**: `local/task24`  
**선행 조건**: Task #23 (rpdf-serializer) 완료 ✅  
**마일스톤**: M030 (v0.3 편집 커맨드)

---

## 목적

v0.3에서 구현된 편집 커맨드(Task #17~#23)를 `rpdf` CLI에서 직접 호출 가능하게 연결한다.

```
rpdf merge a.pdf b.pdf c.pdf -o merged.pdf
rpdf split input.pdf --pages 1-3,5,7-10 -o out_dir/
rpdf rotate input.pdf --page 3 --degrees 90 -o rotated.pdf
rpdf delete input.pdf --pages 2,4,6 -o deleted.pdf
rpdf extract input.pdf --pages 5-10 -o excerpt.pdf
```

---

## 배경 — 현재 코드 구조

| 크레이트 | 역할 |
|----------|------|
| `rpdf-edit` | RotatePageCommand(Command), DeletePagesCommand(Command), MergeCommand(Command), SplitCommand(Query), ExtractPagesCommand(Query) |
| `rpdf-serializer` | `load_document_tracked(data)` → `(Document, Vec<PageSource>)`, `serialize_document(doc, sources)` → `Vec<u8>` |
| `rpdf-cli` | 현재 info/dump/dump-pages/render 명령만 존재. 편집 크레이트 의존 없음. |

### 핵심 페이지 번호 규칙

| 커맨드 | 입력 | 비고 |
|--------|------|------|
| `RotatePageCommand::new(idx, degrees)` | **0-based** idx | CLI에서 1-based 입력받아 변환 |
| `DeletePagesCommand::new(indices)` | **0-based** idx 목록 | CLI에서 1-based 입력받아 변환 |
| `MergeCommand::new(sources)` | Document 목록 | 페이지 번호 없음 |
| `SplitCommand::new(spec)` | **1-based** 명세 문자열 | CLI 입력 그대로 전달 |
| `ExtractPagesCommand::new(start, end)` | **1-based** 시작·끝 | CLI 입력 그대로 전달 |

---

## 변경 파일 목록

### 신규

```
crates/rpdf-cli/src/commands/edit/
  mod.rs
  merge.rs
  split.rs
  rotate.rs
  delete.rs
  extract.rs
```

### 수정

```
crates/rpdf-cli/Cargo.toml          — rpdf-edit, rpdf-serializer 의존성 추가
                                      dev-dependencies에 rpdf-parser 추가 (roundtrip 테스트용)
crates/rpdf-cli/src/main.rs         — 5개 서브커맨드 추가 (Merge, Split, Rotate, Delete, Extract)
crates/rpdf-cli/src/commands/mod.rs — pub mod edit 추가
crates/rpdf-cli/tests/cli_tests.rs  — 편집 명령 통합 테스트 추가
```

---

## API 설계

### clap 서브커맨드

```rust
// main.rs Commands enum에 추가
/// 여러 PDF를 하나로 합친다.
Merge {
    /// 입력 PDF 파일들 (2개 이상 필수).
    #[arg(value_name = "PDF", required = true, num_args = 2..)]
    inputs: Vec<PathBuf>,
    /// 출력 파일 경로.
    #[arg(short = 'o', long = "output", value_name = "PATH", required = true)]
    output: PathBuf,
},
/// 페이지 범위별로 PDF를 여러 파일로 분리한다.
Split {
    /// 입력 PDF 파일.
    #[arg(value_name = "PDF")]
    input: PathBuf,
    /// 1-based 페이지 범위 명세 (예: "1-3,5,7-10").
    #[arg(long = "pages", value_name = "SPEC")]
    pages: String,
    /// 출력 디렉토리 경로.
    #[arg(short = 'o', long = "output", value_name = "DIR", required = true)]
    output: PathBuf,
},
/// 페이지를 회전시킨다.
Rotate {
    /// 입력 PDF 파일.
    #[arg(value_name = "PDF")]
    input: PathBuf,
    /// 회전할 1-based 페이지 번호.
    #[arg(long = "page", value_name = "N")]
    page: usize,
    /// 회전 각도 (90의 배수; 양수=시계방향, 음수=반시계방향).
    #[arg(long = "degrees", value_name = "DEG")]
    degrees: i32,
    /// 출력 파일 경로.
    #[arg(short = 'o', long = "output", value_name = "PATH", required = true)]
    output: PathBuf,
},
/// 지정한 페이지를 삭제한다.
Delete {
    /// 입력 PDF 파일.
    #[arg(value_name = "PDF")]
    input: PathBuf,
    /// 삭제할 1-based 페이지 번호 목록 (쉼표 구분, 예: "2,4,6").
    #[arg(long = "pages", value_name = "PAGES")]
    pages: String,
    /// 출력 파일 경로.
    #[arg(short = 'o', long = "output", value_name = "PATH", required = true)]
    output: PathBuf,
},
/// 지정 범위 페이지를 새 PDF로 추출한다.
Extract {
    /// 입력 PDF 파일.
    #[arg(value_name = "PDF")]
    input: PathBuf,
    /// 추출 범위 (1-based, 예: "5-10").
    #[arg(long = "pages", value_name = "RANGE")]
    pages: String,
    /// 출력 파일 경로.
    #[arg(short = 'o', long = "output", value_name = "PATH", required = true)]
    output: PathBuf,
},
```

### 공통 유틸리티 (`commands/edit/mod.rs`)

```rust
/// "2,4,6" → Vec<usize> (0-based 변환, sort+dedup 포함). 1-based 입력 가정.
/// 빈 문자열이나 0 입력 시 에러. sort_unstable() + dedup()으로 정렬된 고유 인덱스를 반환.
/// DeletePagesCommand 내부 dedup 동작과 일치시켜 sources 동기화 버그를 방지한다.
pub(super) fn parse_page_list(spec: &str) -> anyhow::Result<Vec<usize>>;

/// "5-10" → (start, end) (1-based 그대로). ExtractPagesCommand에 전달.
/// 단일 숫자 "5" → (5, 5). "10-5" 등 start > end 시 에러.
pub(super) fn parse_single_range(spec: &str) -> anyhow::Result<(usize, usize)>;

/// split 출력 파일명: `{stem}_part{N}.pdf` (N은 1-based).
/// stem은 input Path::file_stem()에서 추출. None이면 "output"을 fallback으로 사용.
pub(super) fn split_output_path(dir: &Path, stem: &str, n: usize) -> PathBuf;
```

### 각 핸들러 로직

#### merge

```
1. inputs 각각 load_document_tracked → (doc_i, sources_i)
2. doc_0을 기준 doc으로 설정, sources = sources_0
3. MergeCommand::new(vec![doc_1, doc_2, ...]).execute(&mut doc_0)
4. sources = sources_0 + sources_1 + ... 순서로 연결
5. serialize_document(&doc_0, &sources) → bytes
6. output 경로에 bytes 저장
```

#### split

```
1. load_document_tracked(data) → (doc, sources)
2. SplitCommand::new(&pages_spec) — 파싱 에러는 bail!
3. cmd.execute(&doc) → Vec<Document> (sub_docs)
4. SplitCommand의 내부 ranges 순서에 맞게 sources 슬라이싱
   - sub_docs[i] ↔ ranges[i].start..=ranges[i].end
   - sub_sources_i = sources[ranges[i].start..=ranges[i].end].to_vec()
5. 각 (sub_docs[i], sub_sources_i)에 대해:
   serialize_document → bytes
   split_output_path(output_dir, stem, i+1) 경로에 저장
```

⚠️ `SplitCommand`의 `ranges` 필드가 `pub(crate)` 또는 private인 경우, 페이지 번호 명세를 재파싱하는 독립 유틸리티 `parse_page_spec` 를 `edit/mod.rs`에 구현한다 (SplitCommand 내부에 의존하지 않음).

#### rotate

```
1. load_document_tracked(data) → (doc, sources)
2. page_index = page - 1 (1-based → 0-based)
3. RotatePageCommand::new(page_index, degrees).execute(&mut doc)
4. serialize_document(&doc, &sources) → bytes
5. output 경로에 bytes 저장
```

#### delete

```
1. load_document_tracked(data) → (doc, sources)
2. parse_page_list(&pages_spec) → indices (0-based, sort+dedup 완료)
3. DeletePagesCommand::new(indices.clone()).execute(&mut doc)
4. 삭제 후 sources 동기화 (retain() 패턴):
   - let indices_set: HashSet<usize> = indices.into_iter().collect();
   - sources.retain_mut 대신 열거: sources = sources.into_iter().enumerate()
     .filter(|(i, _)| !indices_set.contains(i)).map(|(_, s)| s).collect()
   ⚠️ 오름차순 remove() 금지: 인덱스 이동 버그 발생. retain 패턴만 사용.
5. serialize_document(&doc, &sources) → bytes
6. output 경로에 bytes 저장
```

#### extract

```
1. load_document_tracked(data) → (doc, sources)
2. parse_single_range(&pages_spec) → (start, end) (1-based)
3. ExtractPagesCommand::new(start, end).execute(&doc) → new_doc
4. sources 슬라이싱: sources[(start-1)..=end-1].to_vec()
5. serialize_document(&new_doc, &sub_sources) → bytes
6. output 경로에 bytes 저장
```

---

## 에러 처리 표

| 상황 | 에러 메시지 패턴 | 발생 위치 |
|------|-----------------|-----------|
| 입력 파일 읽기 실패 | `"파일을 읽을 수 없습니다: {path}"` | read_file / std::fs::read |
| merge: 입력 파일 1개 이하 | clap `num_args = 2..`로 자동 거부 | clap 파싱 단계 |
| pages 명세 파싱 실패 | `"잘못된 페이지 명세: {spec}"` | parse_page_list / parse_single_range |
| pages 0 포함 (1-based 위반) | `"페이지 번호는 1부터 시작합니다"` | parse_page_list / parse_single_range |
| 커맨드 실행 실패 | `CommandError` → `anyhow::bail!` | execute() |
| 직렬화 실패 | `SerializeError` → `anyhow::bail!` | serialize_document() |
| 출력 파일 쓰기 실패 | `"파일을 쓸 수 없습니다: {path}"` | std::fs::write |
| split: 출력 디렉토리 없음 | `"출력 디렉토리가 존재하지 않습니다: {path}"` | split 핸들러 진입 시 검사 |
| extract: "5-10" 형식 아닌 경우 | `"extract --pages는 '시작-끝' 또는 '숫자' 형식이어야 합니다"` | parse_single_range |
| In-place edit (input == output) | v0.3 미보장 — 동작하나 write 실패 시 원본 truncation 위험. | 에러 처리 없음 (v0.4 TODO) |

---

## 테스트 전략

### 단위 테스트 (`commands/edit/` 각 파일 내 `#[cfg(test)]`)

- `parse_page_list`: 정상 케이스, 0 포함, 빈 문자열, 중복, 문자 혼합
- `parse_single_range`: "5-10", "5", "10-5"(에러), "0-5"(에러)
- `split_output_path`: 파일명 생성 규칙 검증

### 통합 테스트 (`tests/cli_tests.rs` 확장)

fixtures: `crates/rpdf-cli/tests/fixtures/` 에 기존 샘플 PDF 활용 (임의 PDF를 실제 열어본 뒤 맞는 것 선택)

```rust
// rotate: 3페이지 PDF의 1번 페이지를 90도 회전 → 저장 → 재파싱 → rotation 값 검증
#[test]
fn rotate_page_roundtrip()

// delete: 3페이지 PDF에서 2번 페이지 삭제 → 저장 → 재파싱 → page_count == 2 검증
#[test]
fn delete_page_roundtrip()

// merge: 두 PDF 합치기 → 저장 → 재파싱 → page_count == sum 검증
#[test]
fn merge_roundtrip()

// extract: 5페이지 PDF에서 2-4 추출 → 저장 → 재파싱 → page_count == 3 검증
#[test]
fn extract_pages_roundtrip()

// split: 5페이지 PDF를 "1-2,4-5"로 분리 → 출력 파일 2개 생성 검증
#[test]
fn split_roundtrip()

// CLI 에러: pages 0 입력 → exit code 1 검증
#[test]
fn delete_zero_page_errors()

// CLI 에러: merge 입력 파일 1개 → exit code 2 (clap 에러) 검증
#[test]
fn merge_requires_two_inputs()
```

---

## 구현 체크포인트

### CP-A: 의존성 추가 + 빌드 통과

- [ ] `rpdf-cli/Cargo.toml`에 `rpdf-edit`, `rpdf-serializer` 추가
- [ ] `cargo build -p rpdf-cli` 통과

### CP-B: 공통 유틸리티 + merge/rotate 구현

- [ ] `commands/edit/mod.rs`: parse_page_list, parse_single_range, split_output_path
- [ ] `merge.rs`, `rotate.rs` 구현
- [ ] `main.rs` Merge, Rotate 서브커맨드 연결
- [ ] `cargo test -p rpdf-cli` 통과

### CP-C: delete/extract/split 구현

- [ ] `delete.rs`, `extract.rs`, `split.rs` 구현
- [ ] `main.rs` Delete, Extract, Split 서브커맨드 연결
- [ ] 통합 테스트 전체 통과

### CP-D: 품질 관문 통과

- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo fmt --check`
- [ ] `cargo test --workspace`

---

## 완료 기준

1. 5개 편집 CLI 명령이 `rpdf --help`에 나타난다.
2. 각 명령이 실제 PDF 파일에 대해 정상 동작한다.
3. 잘못된 입력 시 exit code 1과 명확한 에러 메시지를 출력한다.
4. 전체 테스트 통과 + clippy 경고 없음.

---

## 범위 외 (NOT in scope)

- Undo/Redo CLI 노출 (v0.4 이후) — CQRS 설계상 CommandStack을 노출하지 않음
- 편집 회귀 테스트 (Task #25) — 이미지 스냅샷 비교는 별도 태스크
- 비대화형 배치 처리 파이프라인 — v0.5 이후
- In-place edit 안전성 (임시 파일 → rename 패턴) — v0.4 TODO
- split 출력 디렉토리 자동 생성 (`create_dir_all`) — 현재 에러로 명확 실패 선택

---

## What already exists (재사용)

| 기존 코드 | 재사용 여부 | 비고 |
|----------|------------|------|
| `main.rs`의 `read_file()` | ✅ 재사용 | 단일 파일 읽기에 사용 |
| `cli_tests.rs`의 `pdf()` 헬퍼 | ✅ 재사용 | examples/ PDF 경로 조회 |
| `examples/*.pdf` fixtures | ✅ 재사용 | fw4-2024.pdf(5p), irs-f1040.pdf 등 |
| `rpdf-edit` 모든 Command/Query | ✅ 래핑만 | 로직 중복 없음 |
| `rpdf-serializer` 두 함수 | ✅ 직접 호출 | CLI 핸들러에서 직접 사용 |

---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAR | 4 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | — |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | — | — |

- **UNRESOLVED:** 0
- **VERDICT:** ENG CLEARED — ready to implement
