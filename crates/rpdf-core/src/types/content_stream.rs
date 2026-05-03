use crate::types::PdfObject;

/// PDF content stream 연산자 (ISO 32000 §8~§9).
///
/// 모든 변형은 의미 기반 이름을 사용한다. PDF 키워드 → enum 매핑은
/// `rpdf_parser::content_stream::keyword_to_operator`에서만 처리된다.
///
/// `Unknown(Vec<u8>)`: 스펙에 없는 키워드. 무시하지 않고 보존하여
/// 디버깅 시 확인 가능. 파싱 에러가 아님.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentStreamOperator {
    // ── Text 객체 ──────────────────────────────────────
    /// `BT` — Begin text object.
    BeginText,
    /// `ET` — End text object.
    EndText,
    // ── Text 상태 ─────────────────────────────────────
    /// `Tc` — Set character spacing.
    SetCharSpacing,
    /// `Tw` — Set word spacing.
    SetWordSpacing,
    /// `Tz` — Set horizontal text scaling.
    SetHorizontalScale,
    /// `TL` — Set text leading.
    SetLeading,
    /// `Tf` — Set font and size.
    SetFont,
    /// `Tr` — Set text rendering mode.
    SetRenderingMode,
    /// `Ts` — Set text rise.
    SetTextRise,
    // ── Text 위치 ─────────────────────────────────────
    /// `Td` — Move text position.
    MoveText,
    /// `TD` — Move text position and set leading.
    MoveTextSetLeading,
    /// `Tm` — Set text matrix and text line matrix.
    SetTextMatrix,
    /// `T*` — Move to start of next text line.
    MoveToNextLine,
    // ── Text 표시 ─────────────────────────────────────
    /// `Tj` — Show text string.
    ShowText,
    /// `TJ` — Show text with individual glyph positioning.
    ShowTextAdjusted,
    /// `'` — Move to next line and show text string.
    MoveShowText,
    /// `"` — Set word/char spacing, move to next line, show text string.
    MoveSetShowText,
    // ── 그래픽 상태 ────────────────────────────────────
    /// `q` — Save graphics state.
    SaveState,
    /// `Q` — Restore graphics state.
    RestoreState,
    /// `cm` — Modify current transformation matrix.
    ConcatMatrix,
    /// `w` — Set line width.
    SetLineWidth,
    /// `J` — Set line cap style.
    SetLineCap,
    /// `j` — Set line join style.
    SetLineJoin,
    /// `M` — Set miter limit.
    SetMiterLimit,
    /// `d` — Set line dash pattern.
    SetDashPattern,
    /// `i` — Set flatness tolerance.
    SetFlatness,
    /// `gs` — Set parameters from graphics state parameter dictionary.
    SetGraphicsState,
    /// `ri` — Set color rendering intent.
    SetRenderingIntent,
    // ── 경로 구성 ─────────────────────────────────────
    /// `m` — Begin new subpath (moveto).
    MoveTo,
    /// `l` — Append straight line segment (lineto).
    LineTo,
    /// `c` — Append cubic Bezier curve (all control points).
    CurveTo,
    /// `v` — Append cubic Bezier curve (first control point = current point).
    CurveToV,
    /// `y` — Append cubic Bezier curve (second control point = final point).
    CurveToY,
    /// `h` — Close current subpath.
    ClosePath,
    /// `re` — Append rectangle to path.
    Rect,
    // ── 경로 그리기 ───────────────────────────────────
    /// `S` — Stroke path.
    Stroke,
    /// `s` — Close and stroke path.
    CloseStroke,
    /// `f` — Fill path (nonzero winding number rule).
    Fill,
    /// `F` — Fill path (obsolete, same as f).
    FillObsolete,
    /// `f*` — Fill path (even-odd rule).
    FillEvenOdd,
    /// `B` — Fill and stroke path (nonzero winding).
    FillStroke,
    /// `B*` — Fill and stroke path (even-odd rule).
    FillStrokeEvenOdd,
    /// `b` — Close, fill, and stroke path (nonzero winding).
    CloseFillStroke,
    /// `b*` — Close, fill, and stroke path (even-odd rule).
    CloseFillStrokeEvenOdd,
    /// `n` — End path without filling or stroking.
    EndPath,
    // ── 클리핑 ────────────────────────────────────────
    /// `W` — Modify clipping path (nonzero winding rule).
    Clip,
    /// `W*` — Modify clipping path (even-odd rule).
    ClipEvenOdd,
    // ── 색상 ──────────────────────────────────────────
    /// `CS` — Set stroke color space.
    SetStrokeColorSpace,
    /// `cs` — Set fill color space.
    SetFillColorSpace,
    /// `SC` — Set stroke color.
    SetStrokeColor,
    /// `SCN` — Set stroke color (supports Pattern/Separation/DeviceN).
    SetStrokeColorN,
    /// `sc` — Set fill color.
    SetFillColor,
    /// `scn` — Set fill color (supports Pattern/Separation/DeviceN).
    SetFillColorN,
    /// `G` — Set stroke color in DeviceGray.
    SetStrokeGray,
    /// `g` — Set fill color in DeviceGray.
    SetFillGray,
    /// `RG` — Set stroke color in DeviceRGB.
    SetStrokeRGB,
    /// `rg` — Set fill color in DeviceRGB.
    SetFillRGB,
    /// `K` — Set stroke color in DeviceCMYK.
    SetStrokeCMYK,
    /// `k` — Set fill color in DeviceCMYK.
    SetFillCMYK,
    // ── XObject / 셰이딩 ───────────────────────────────
    /// `Do` — Invoke named XObject.
    InvokeXObject,
    /// `sh` — Paint area defined by shading pattern.
    Shading,
    // ── 인라인 이미지 (BI...ID...EI 통합) ─────────────
    /// `BI`...`ID`...`EI` — Inline image (parsed as one compound operation).
    InlineImage,
    // ── 마킹된 콘텐츠 ─────────────────────────────────
    /// `MP` — Designate marked-content point.
    MarkedContentPoint,
    /// `DP` — Designate marked-content point with property list.
    MarkedContentPointProp,
    /// `BMC` — Begin marked-content sequence.
    BeginMarkedContent,
    /// `BDC` — Begin marked-content sequence with property list.
    BeginMarkedContentProp,
    /// `EMC` — End marked-content sequence.
    EndMarkedContent,
    // ── 호환성 ────────────────────────────────────────
    /// `BX` — Begin compatibility section.
    BeginCompatibility,
    /// `EX` — End compatibility section.
    EndCompatibility,
    // ── 알 수 없는 연산자 (보존) ──────────────────────
    /// 스펙에 없는 키워드. 파싱 에러 아님.
    Unknown(Vec<u8>),
}

/// content stream의 단일 연산 — 연산자 + 피연산자 목록.
///
/// 인라인 이미지(`InlineImage`)의 경우:
/// - `operands`: dict key-value 쌍 (`PdfObject::Name, value, ...` 순서)
/// - `inline_data`: `Some(raw_bytes)` — ID와 EI 사이의 원본 이미지 데이터
///
/// 나머지 연산자는 `inline_data`가 항상 `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentStreamOperation {
    pub operator: ContentStreamOperator,
    /// 피연산자. Indirect Reference는 content stream 안에 등장하지 않음 (§7.8.2).
    pub operands: Vec<PdfObject>,
    /// 인라인 이미지 raw bytes (InlineImage 연산자 전용).
    pub inline_data: Option<Vec<u8>>,
}

impl ContentStreamOperation {
    /// 새 연산을 생성한다.
    pub fn new(operator: ContentStreamOperator, operands: Vec<PdfObject>) -> Self {
        Self {
            operator,
            operands,
            inline_data: None,
        }
    }

    /// 인라인 이미지 연산을 생성한다.
    pub fn inline_image(operands: Vec<PdfObject>, data: Vec<u8>) -> Self {
        Self {
            operator: ContentStreamOperator::InlineImage,
            operands,
            inline_data: Some(data),
        }
    }
}
