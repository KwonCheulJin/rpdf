# samples/

rpdf 회귀 테스트용 PDF 모음. 파서 변경 시 자동으로 스냅샷과 비교된다.

- **28개** git commit (3MB 이하)
- **2개** `samples/large/` — git 제외, `scripts/fetch-samples.sh`로 다운로드

> `examples/`는 개발·디버깅용. `samples/`는 회귀 테스트 전용.

## 파일 목록

### 전통 xref (T1~T8) — PDF 1.3~1.5, `xref` 키워드 + 20바이트 고정 항목

| 파일 | 크기 | 출처 | 라이선스 | 특징 |
|------|------|------|---------|------|
| `trad-xref-basicapi.pdf` | 103KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/basicapi.pdf) | Apache-2.0 | 기본 API 테스트용 표준 파일 |
| `trad-xref-tracemonkey.pdf` | 993KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/tracemonkey.pdf) | Apache-2.0 | 14페이지, `xref` 체인 |
| `trad-xref-canvas.pdf` | 147KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/canvas.pdf) | Apache-2.0 | Canvas 렌더링 테스트 파일 |
| `trad-xref-cmyk-jpeg.pdf` | 365KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/cmykjpeg.pdf) | Apache-2.0 | CMYK JPEG 이미지 포함 |
| `trad-xref-attachment.pdf` | 14KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/attachment.pdf) | Apache-2.0 | 파일 첨부 주석 포함 |
| `trad-xref-find-all.pdf` | 10KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/find_all.pdf) | Apache-2.0 | 텍스트 검색 테스트용 |
| `trad-xref-pages-tree-refs.pdf` | 1KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/Pages-tree-refs.pdf) | Apache-2.0 | 페이지 트리 간접 참조 구조 |
| `trad-xref-issue1293r.pdf` | 621B | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/issue1293r.pdf) | Apache-2.0 | 최소 PDF, 버그 재현 케이스 |

### xref stream (S1~S8) — PDF 1.5+, `/XRef` 객체, FlateDecode 압축

| 파일 | 크기 | 출처 | 라이선스 | 특징 |
|------|------|------|---------|------|
| `xref-stream-doc-13-pages.pdf` | 11KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/doc_1_3_pages.pdf) | Apache-2.0 | 3페이지, ObjStm 포함 |
| `xref-stream-extract-link.pdf` | 9KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/extract_link.pdf) | Apache-2.0 | 2페이지, 링크 주석 |
| `xref-stream-form-two-pages.pdf` | 22KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/form_two_pages.pdf) | Apache-2.0 | 2페이지 폼, xref stream |
| `xref-stream-zapfdingbats.pdf` | 17KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/ZapfDingbats.pdf) | Apache-2.0 | ZapfDingbats 심볼 폰트 |
| `xref-stream-irs-f1040nr.pdf` | 173KB | [IRS](https://www.irs.gov/pub/irs-pdf/f1040nr.pdf) | 미국 정부 공개 도메인 | Form 1040-NR (비거주자), PDF 1.7 |
| `xref-stream-irs-f1040es.pdf` | 323KB | [IRS](https://www.irs.gov/pub/irs-pdf/f1040es.pdf) | 미국 정부 공개 도메인 | Form 1040-ES (분기 세금), PDF 1.7 |
| `xref-stream-irs-f1120.pdf` | 332KB | [IRS](https://www.irs.gov/pub/irs-pdf/f1120.pdf) | 미국 정부 공개 도메인 | Form 1120 (법인세), PDF 1.7 |
| `xref-stream-irs-f941.pdf` | 822KB | [IRS](https://www.irs.gov/pub/irs-pdf/f941.pdf) | 미국 정부 공개 도메인 | Form 941 (고용세), PDF 1.7 |

### 다국어/유니코드 (M1~M4)

| 파일 | 크기 | 출처 | 라이선스 | 특징 |
|------|------|------|---------|------|
| `multilang-french-diacritics.pdf` | 10KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/french_diacritics.pdf) | Apache-2.0 | 프랑스어 악센트 문자, xref stream |
| `multilang-german-umlaut.pdf` | 14KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/german-umlaut-r.pdf) | Apache-2.0 | 독일어 움라우트, 전통 xref |
| `multilang-arabic-cidfont.pdf` | 12KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/arial_unicode_ab_cidfont.pdf) | Apache-2.0 | 아랍어 CID 폰트, CIDToGIDMap |
| `multilang-korean-metadata.pdf` | 488B | 직접 생성 (`scripts/make-corrupted-samples.py`) | 저작권 없음 (생성물) | UTF-16BE 한국어 메타데이터, 최소 PDF |

> **참고**: `multilang-korean-metadata.pdf`의 한국어 메타데이터는 현재 rpdf가 UTF-16BE BOM을 완전히 지원하지 않아 일부 깨져 표시된다. 스냅샷은 현재 상태를 기록하며, v0.2에서 수정 시 스냅샷이 업데이트된다.

### 손상 / 의도적 에러 (B1~B2) — ParseError 종류 검증

| 파일 | 크기 | 생성 방법 | 손상 종류 | 기대 ParseError |
|------|------|---------|---------|----------------|
| `broken-missing-trailer.pdf` | 103KB | `trad-xref-basicapi.pdf`에서 `trailer` 이후 제거 | trailer + EOF 누락 | `MissingEof` |
| `broken-bad-xref-offset.pdf` | 103KB | `trad-xref-basicapi.pdf`에서 `startxref` 오프셋을 99999999로 변경 | xref 오프셋 범위 초과 | xref 오프셋 범위 오류 |

### 비표준 / 관용 처리 (N1~N2) — 다양한 PDF 생성기 호환 검증

| 파일 | 크기 | 출처 | 라이선스 | 특징 |
|------|------|------|---------|------|
| `nonstandard-helloworld-bad.pdf` | 745B | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/helloworld-bad.pdf) | Apache-2.0 | 비표준 구조, 하지만 일부 뷰어가 허용 |
| `nonstandard-bad-page-labels.pdf` | 798B | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/bad-PageLabels.pdf) | Apache-2.0 | 잘못된 `/PageLabels` 구조 |

### 특수 케이스 (X1~X4)

| 파일 | 크기 | 출처 | 라이선스 | 특징 |
|------|------|------|---------|------|
| `special-acroform-calc.pdf` | 1KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/acroform_calculation_order.pdf) | Apache-2.0 | AcroForm 계산 순서, 1페이지 |
| `special-irs-f4868.pdf` | 517KB | [IRS](https://www.irs.gov/pub/irs-pdf/f4868.pdf) | 미국 정부 공개 도메인 | Form 4868 (신고 연장), 복잡한 폼 |
| `special-irs-f1099msc.pdf` | 532KB | [IRS](https://www.irs.gov/pub/irs-pdf/f1099msc.pdf) | 미국 정부 공개 도메인 | Form 1099-MISC (기타 소득) |
| `special-unicode-en-cidfont.pdf` | 15KB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/arial_unicode_en_cidfont.pdf) | Apache-2.0 | 영어 Arial Unicode CID 폰트 |

## 대용량 파일 (samples/large/) — git 제외

다음 2개 파일은 크기가 크므로 git에 포함되지 않는다.  
`scripts/fetch-samples.sh`로 로컬에 다운로드 후 `--features samples-large`로 테스트한다.

| 파일 | 크기 | 출처 | 라이선스 | 용도 |
|------|------|------|---------|------|
| `large/large-pdfjs-9279.pdf` | ~3MB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/PDFJS-9279-reduced.pdf) | Apache-2.0 | 성능 베이스라인 측정 L1 |
| `large/large-pdfjs-issue12841.pdf` | ~5.7MB | [Mozilla pdf.js](https://github.com/mozilla/pdf.js/blob/master/test/pdfs/issue12841_reduced.pdf) | Apache-2.0 | 성능 베이스라인 측정 L2 |

```bash
# 대용량 PDF 다운로드
bash scripts/fetch-samples.sh

# 전체 회귀 테스트 (대용량 포함)
cargo nextest run --all --features samples-large
```

## 파일 추가 기준

- 3MB 이하 파일만 `samples/`에 git commit한다 (large는 제외)
- 라이선스: Apache-2.0, MIT, CC0, 미국 정부 공개 도메인만 허용
- CC-BY-NC, GPL 파일은 추가 불가
- 파일 추가 시 이 README 라이선스 컬럼에 반드시 기록한다
- 모호한 라이선스는 commit 안 함 → 대체 파일 탐색

## 손상 PDF 생성 방법

```bash
# 손상 PDF 재생성 (원본에서 새로 생성)
python3 scripts/make-corrupted-samples.py
```
