# SVG 렌더러 (rpdf-svg)

## 개요

`rpdf-svg` 크레이트는 v0.1 IR(`ContentStreamOperator`) → SVG 문자열 변환을 담당한다.
`rpdf-parser::load_document()`로 파싱한 `Page` 구조체를 받아, PDF 콘텐츠 스트림 연산자를
순회하며 SVG 요소를 생성한다. pdfium 동적 라이브러리가 불필요한 경량 렌더링 경로다.

## 설계 결정

### y-flip 그룹

```svg
<g transform="matrix(1 0 0 -1 0 H)">
```

PDF 좌표계는 좌하단이 원점(y축 위 방향)이고, SVG 좌표계는 좌상단이 원점(y축 아래 방향)이다.
높이 H를 기준으로 y축을 반전(`scale Y = -1`)하고 `translate Y = H`로 오프셋을 맞춰
PDF 연산자를 좌표 변환 없이 그대로 사용할 수 있게 한다.

### `loose_cm_depth` 패턴

PDF `cm` 연산자(ConcatMatrix)가 `q`/`Q` 없이 단독 등장할 때,
각 `cm`마다 `<g transform="matrix(...)">` 태그를 열고 `loose_cm_depth`를 증가시킨다.

`Q`(RestoreState) 처리 시 스택을 pop하며 닫힌 `</g>` 태그를 출력한다.

**핵심 주의사항**: `q`(SaveState) 처리 시 현재 열려 있는 `loose_cm_depth`만큼
`</g>` 태그를 **먼저 닫은 뒤** 그래픽 상태를 push해야 한다.
이를 지키지 않으면 `cm → q` 패턴 PDF에서 SVG 구조가 파손된다.

```rust
// SaveState 진입 전 loose_cm 태그 먼저 닫기
for _ in 0..loose_cm_depth {
    out.push_str("</g>\n");
}
loose_cm_depth = 0;
// 이후 그래픽 상태 스택 push
```

### 오버레이 레이어 분리

`debug_overlay: true`일 때 생성되는 디버그 그리드·경계·마커는
y-flip 그룹 **바깥**에 독립 레이어(`id="debug-overlay"`)로 배치한다.

y-flip 변환이 적용된 좌표계가 아니라 SVG 원본 좌표로 디버그 정보를 표시하므로,
PDF 콘텐츠와 좌표 오염 없이 정확한 위치 확인이 가능하다.

## 지원 연산자 목록

### 그래픽 상태

| 연산자 | 동작 |
|--------|------|
| `q` | 그래픽 상태 저장 (SaveState) |
| `Q` | 그래픽 상태 복원 (RestoreState) |
| `cm` | 현재 변환 행렬에 연결 (ConcatMatrix) |
| `w` | 선 두께 설정 (SetLineWidth) |

### 색상

| 연산자 | 동작 |
|--------|------|
| `rg` | RGB 채우기 색상 설정 |
| `RG` | RGB 선 색상 설정 |
| `g` | 그레이스케일 채우기 색상 설정 |
| `G` | 그레이스케일 선 색상 설정 |

### 경로 구성

| 연산자 | 동작 |
|--------|------|
| `m` | 새 경로 시작 (MoveTo) |
| `l` | 직선 추가 (LineTo) |
| `c` | 3점 베지어 곡선 (CurveTo) |
| `v` | 시작점 = 현재 점 베지어 곡선 (CurveToInitialPoint) |
| `y` | 끝점 = 제어점 베지어 곡선 (CurveToFinalPoint) |
| `h` | 경로 닫기 (ClosePath) |
| `re` | 사각형 경로 추가 (Rectangle) |

### 경로 그리기

| 연산자 | 동작 |
|--------|------|
| `S` | 선만 그리기 (Stroke) |
| `s` | 경로 닫고 선 그리기 (CloseAndStroke) |
| `f`, `F` | 채우기 (Fill, 비영 규칙) |
| `f*` | 채우기 (Fill, 홀짝 규칙) |
| `B` | 채우기 + 선 (FillAndStroke, 비영 규칙) |
| `B*` | 채우기 + 선 (FillAndStroke, 홀짝 규칙) |
| `b` | 경로 닫고 채우기 + 선 (CloseFillAndStroke) |
| `b*` | 경로 닫고 채우기 + 선 (CloseFillAndStroke, 홀짝 규칙) |
| `n` | 경로 그리지 않고 종료 (EndPath) |

### 텍스트

| 연산자 | 동작 |
|--------|------|
| `BT` | 텍스트 블록 시작 |
| `ET` | 텍스트 블록 종료 |
| `Tm` | 텍스트 행렬 설정 |
| `Td` | 텍스트 위치 이동 |
| `TD` | 텍스트 위치 이동 + 줄 간격 설정 |
| `Tj` | 문자열 출력 |
| `TJ` | 개별 글리프 간격 조정 문자열 출력 |
| `'` | 줄 바꿈 후 문자열 출력 |

## 미지원 연산자

지원하지 않는 연산자는 SVG 주석으로 기록하고 계속 진행한다:

```svg
<!-- unsupported: Do -->
<!-- unsupported: SCN -->
```

오류 없이 최대한 렌더링을 완료하는 것을 우선한다.

## 공개 API 예제

```rust
use rpdf_svg::{RenderOptions, render_page_svg, render_page_svg_with_options};

// 기본 렌더링
let svg = render_page_svg(page);

// 디버그 오버레이 포함
let svg = render_page_svg_with_options(page, &RenderOptions { debug_overlay: true });
```

## 알려진 한계

- **디렉토리 출력 전제 조건**: `-o <dir>/` 형식은 디렉토리가 사전에 존재해야 한다.
  런타임 `is_dir()` 검사에 의존하며, 디렉토리 자동 생성 기능 없음.
- **텍스트 렌더링**: 텍스트 위치(`Tm`, `Td`)만 반영하며 폰트·인코딩 미지원.
  깨진 글자나 잘못된 문자가 표시될 수 있음.
- **이미지 미지원**: `Do` 연산자(XObject 그리기) 미지원. 주석으로 건너뜀.
- **CMYK 색상 미지원**: `k`/`K` 연산자 미지원. 주석으로 건너뜀.
- **클리핑 미지원**: `W`/`W*` 연산자 미지원. 클리핑 경로 없이 렌더링됨.
