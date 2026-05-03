# 합성 테스트 데이터 작성 원칙

## 실제 파일 vs 합성 데이터의 역할

| 구분 | 실제 PDF 파일 | 합성 데이터 |
|------|-------------|-----------|
| 목적 | end-to-end 회귀 검증 | 특정 엣지 케이스 정밀 제어 |
| 강점 | 실제 환경과 동일한 구조 | 임의 시나리오 구성 가능 |
| 약점 | 엣지 케이스가 없을 수 있음 | 실제 인코딩 오류 감지 불가 |

두 가지를 함께 사용한다. 실제 파일이 없는 시나리오는 합성 데이터로 보완한다.

## 합성 데이터가 더 효과적인 케이스

- **구조적 엣지 케이스**: 실제 PDF 생성 도구는 보통 "정상" 구조만 출력한다.
  경계값(빈 섹션, 최소/최대 필드, 혼재 포맷)은 합성 데이터로만 재현 가능.
- **오류 주입**: 손상된 오프셋, 잘린 스트림, 잘못된 딕셔너리 값 등.
- **미존재 시나리오**: 실제 파일에서 관측되지 않는 포맷 조합
  (예: traditional xref + xref 스트림 hybrid chain).

## Task #5 사례

### IT-8: Hybrid Chain (전통 xref + xref 스트림 혼재)

`fw4-2024.pdf`에는 `/Prev` chain을 가진 hybrid 구조가 없었다.
`make_hybrid_pdf_for_it8()` 헬퍼로 다음을 직접 구성:

1. 전통 xref 섹션 (`xref 0 3 \n ...`)
2. 해당 xref를 가리키는 xref 스트림 객체 (`/Prev` 포함)
3. 두 번째 `%%EOF`

이를 통해 `parse_xref_chain` 의 hybrid 분기가 실제로 실행되는지 검증.

### Predictor 변형 테스트 (C 단계)

Predictor 10(None), 12(Up), 15(Optimum) 등은 실제 PDF에서 12만 관측됨.
나머지는 수동으로 바이트 시퀀스를 계산하여 합성 입력 구성.

## 합성 데이터 헬퍼 작성 시 주의 사항

### xref 엔트리 포맷 버그 사례 (Task #5)

`make_entry("0000000000 65535 f\r\n")` 형태로 문자열 리터럴에서 바이트 생성 시
실수로 ` \r\n` (스페이스 포함)을 출력한 버그가 발견됨.

- 생산 파서는 공백을 구분자로 인식하므로 영향 없었음
- 그러나 헬퍼의 출력과 예상 출력을 직접 비교하는 테스트에서 실패
- 회귀 테스트 `reject_malformed_entry_space_before_cr_lf` 추가로 고정

**교훈**: 합성 데이터 헬퍼 자체도 테스트하거나, 헬퍼 출력을 직접 검증하는 어서션을 추가한다.

### 일반 원칙

- 헬퍼 함수는 `#[allow(dead_code)]` 없이 작성 — 사용되지 않으면 경고가 나야 한다.
- 바이트 리터럴(`b"..."`)과 문자열 변환은 UTF-8 이외 바이트를 포함하면 혼선이 생기므로
  바이너리 포맷은 `Vec<u8>` 직접 구성을 선호한다.
- 합성 데이터가 커지면 helper 파일을 분리하고 `tests/fixtures/` 에 바이너리로 저장한다.

## 관련 파일

- `tests/parser/xref_tests.rs` — `make_stream_entry_row`, `make_xref_stream_block` 헬퍼
- `tests/parser/integration_tests.rs` — `make_hybrid_pdf_for_it8` 헬퍼
- `mydocs/troubleshootings/xref-entry-format-spaces.md` — 헬퍼 버그 사례 상세
