# Playbook: PDF 파싱 실패 디버깅

## 증상

`load_document` 또는 `rpdf info <file>` 실행 시 오류 반환 또는 패닉.

## 1단계: 오류 유형 확인

```bash
rpdf info <file>
```

| 오류 | 다음 단계 |
|------|-----------|
| `ParseError::InvalidHeader` | 파일이 PDF인지 확인 (`%PDF-` 시작) |
| `ParseError::XrefNotFound` | xref 스트림 여부 확인 (3단계) |
| `ParseError::CircularPageTree` | 페이지 트리 순환 참조 — 아직 미구현, 스택 오버플로우 발생 |
| `ParseError::UnsupportedEncryption` | 암호화 PDF — 현재 미지원 |

## 2단계: IR 덤프로 파싱 범위 확인

```bash
rpdf dump <file> -p 1
```

페이지 IR이 나오면 파싱 성공 — 렌더링 문제로 넘어간다.

## 3단계: 샘플 추가 후 회귀 테스트

```bash
cp <problematic.pdf> samples/
# crates/rpdf-parser/tests/regression/mod.rs 에 테스트 케이스 추가
cargo nextest run -p rpdf-parser
```

## 4단계: 스냅샷 업데이트

새 샘플의 첫 실행은 `*.snap.new` 파일 생성됨:

```bash
cargo insta accept   # 또는 cargo insta review (대화형)
git add crates/rpdf-parser/tests/regression/snapshots/
```

## 알려진 함정 (Known Pitfalls)

- **Caveat**: 암호화 PDF는 `ParseError::UnsupportedEncryption` — 현재 지원 없음, workaround 없음
- **Gotcha**: 순환 참조 페이지 트리는 스택 오버플로우 유발 (v0.2 미수정, deprecated 동작)
- **Hidden issue**: `load_document` 성공해도 일부 페이지 IR이 빈 경우 있음 — content stream 인코딩 문제

## 관련 문서 (See Also)

- [mydocs/troubleshootings/](../../mydocs/troubleshootings/) — 과거 파싱 버그 목록
- [docs/decisions/ADR-003-parser-no-lopdf.md](../decisions/ADR-003-parser-no-lopdf.md) — 파서 설계 결정
