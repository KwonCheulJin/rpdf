# examples/

rpdf 개발 및 디버깅에 사용하는 샘플 PDF 모음입니다.
모두 **공개 도메인 또는 오픈 라이선스** 파일입니다.

## 파일 목록

| 파일 | 출처 | 특징 | 라이선스 | 테스트 용도 |
| --- | --- | --- | --- | --- |
| `fw4-2024.pdf` | [IRS Form W-4](https://www.irs.gov/pub/irs-pdf/fw4.pdf) | xref stream 방식 (trailer 없음), PDF 1.7 | 미국 정부 공개 도메인 | IT-6 (xref stream 에러) |
| `irs-f1040.pdf` | [IRS Form 1040](https://www.irs.gov/pub/irs-pdf/f1040.pdf) | xref stream 방식 (trailer 없음), PDF 1.7 | 미국 정부 공개 도메인 | 예비 |
| `pdfjs-basicapi.pdf` | [PDF.js 테스트 스위트](https://github.com/mozilla/pdf.js) | 표준 trailer, `/Info` 포함, PDF 1.6 | Apache 2.0 | 예비 |
| `pdfjs-tracemonkey.pdf` | [PDF.js 테스트 스위트](https://github.com/mozilla/pdf.js) | 표준 trailer, `/Info 996 0 R`, PDF 1.4 | Apache 2.0 | IT-1 (전체 연동), IT-5 (/Info 추출) |
| `pdfjs-annotation-border.pdf` | [PDF.js 테스트 스위트](https://github.com/mozilla/pdf.js) | incremental update (trailer 2개, CRLF), PDF 1.5 | Apache 2.0 | IT-3 (마지막 trailer) |

## 향후 추가 예정

Task #3 (xref 테이블 파서) 이후 필요해질 수 있는 파일 유형:

| 특징 | 용도 | 추가 시점 |
| --- | --- | --- |
| xref 섹션 3개 이상 (대형 incremental update) | xref 체인 탐색 테스트 | Task #3 |
| 손상된 xref 오프셋 (복구 테스트용) | 에러 복구 검증 | Task #3 |
| `/Type /ObjStm` (object stream) | object stream 파싱 | Task #4 |

## 파일 추가 기준

- 1MB 이하 파일만 git에 커밋한다
- 명확한 공개 도메인 또는 오픈 라이선스 확인 필수
- 파일 추가 시 이 README에 출처와 특징을 기록한다
- 특정 버그 재현용 샘플은 `samples/`에, 일반 예시는 `examples/`에 둔다
