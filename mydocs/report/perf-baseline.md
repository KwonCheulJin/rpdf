# v0.1 성능 베이스라인

**측정일**: 2026-05-04  
**환경**: macOS 25.4.0, aarch64-apple-darwin, release 빌드  
**측정 방법**: `rpdf info <file>` 3회 평균 (프로세스 시작 포함)  
**측정 대상**: `load_document` 포함 전체 파이프라인 (xref 파싱 + 페이지 트리 + content stream)

## examples/ 5개

| 파일 | 크기 | 페이지 | 평균 시간 | xref 타입 | 비고 |
|------|------|--------|---------|---------|------|
| `fw4-2024.pdf` | 203KB | 5 | **139.1ms** | xref stream (chain) | 이상치 ⚠️ 페이지 4: 11,376 ops |
| `irs-f1040.pdf` | 215KB | 2 | 7.4ms | xref stream (chain) | |
| `pdfjs-basicapi.pdf` | 103KB | 3 | 3.6ms | 전통 xref | |
| `pdfjs-tracemonkey.pdf` | 992KB | 14 | 6.2ms | 전통 xref | |
| `pdfjs-annotation-border.pdf` | 87KB | 1 | 2.7ms | 전통 xref (incremental) | |

## samples/ 대표 파일

| 파일 | 크기 | 페이지 | 평균 시간 | xref 타입 | 비고 |
|------|------|--------|---------|---------|------|
| `trad-xref-tracemonkey.pdf` | 992KB | 14 | 5.4ms | 전통 xref | |
| `xref-stream-irs-f941.pdf` | 821KB | 3 | 4.2ms | xref stream | |
| `xref-stream-irs-f1040es.pdf` | 323KB | N/A | N/A | xref stream | ⚠️ Contents not a stream (비표준) |
| `trad-xref-canvas.pdf` | 147KB | N/A | N/A | 전통 xref | ⚠️ /Length 간접참조 미지원 (Task #7 백로그) |

## 파싱 불가 파일 (v0.1)

| 파일 | 에러 | 원인 |
|------|------|------|
| `xref-stream-irs-f1040es.pdf` | `Contents object ... is not a stream` | 비표준 Contents 구조 |
| `trad-xref-canvas.pdf` | `/Length is an indirect reference` | Task #7 백로그 항목 |

## 이상치 분석: fw4-2024.pdf (139ms)

다른 파일들이 2~8ms인 반면 fw4-2024.pdf는 139ms로 약 20배 느리다.

원인 후보:
1. 페이지 4의 content stream ops 11,376개 (다른 페이지 대비 2~6배)
2. xref stream chain × 2 (`/Prev`) 추가 I/O
3. 폼 필드(AcroForm) 처리

v0.2에서 content stream 렌더링 추가 시 이 파일의 성능을 집중 관찰 권장.

## v0.2 비교 기준

v0.2 렌더링 기능 추가 후 다음 파일들의 `load_document` 시간이 크게 변하면 파서 성능 회귀로 판단:

| 파일 | v0.1 기준 | 허용 상한 |
|------|---------|---------|
| `pdfjs-tracemonkey.pdf` | 6.2ms | 20ms |
| `irs-f1040.pdf` | 7.4ms | 25ms |
| `fw4-2024.pdf` | 139.1ms | 500ms (이미 느림, 별도 최적화 필요) |

> **참고**: criterion 기반 정밀 벤치마크는 v0.2에서 도입. 현재는 릴리즈 빌드 + 3회 평균으로 거친 기준선만 설정.
