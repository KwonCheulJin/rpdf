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

impl ContentStreamOperator {
    /// PDF 스펙 키워드를 반환한다 (`&'static str`).
    ///
    /// `rpdf dump` 출력 및 PDF 스펙 대조 목적. `Debug` 포맷과 별개.
    /// `Unknown` 변형은 `"?"` 반환 — raw bytes 포함 표현은 [`display_name`] 사용.
    pub fn pdf_keyword(&self) -> &'static str {
        match self {
            Self::BeginText => "BT",
            Self::EndText => "ET",
            Self::SetCharSpacing => "Tc",
            Self::SetWordSpacing => "Tw",
            Self::SetHorizontalScale => "Tz",
            Self::SetLeading => "TL",
            Self::SetFont => "Tf",
            Self::SetRenderingMode => "Tr",
            Self::SetTextRise => "Ts",
            Self::MoveText => "Td",
            Self::MoveTextSetLeading => "TD",
            Self::SetTextMatrix => "Tm",
            Self::MoveToNextLine => "T*",
            Self::ShowText => "Tj",
            Self::ShowTextAdjusted => "TJ",
            Self::MoveShowText => "'",
            Self::MoveSetShowText => "\"",
            Self::SaveState => "q",
            Self::RestoreState => "Q",
            Self::ConcatMatrix => "cm",
            Self::SetLineWidth => "w",
            Self::SetLineCap => "J",
            Self::SetLineJoin => "j",
            Self::SetMiterLimit => "M",
            Self::SetDashPattern => "d",
            Self::SetFlatness => "i",
            Self::SetGraphicsState => "gs",
            Self::SetRenderingIntent => "ri",
            Self::MoveTo => "m",
            Self::LineTo => "l",
            Self::CurveTo => "c",
            Self::CurveToV => "v",
            Self::CurveToY => "y",
            Self::ClosePath => "h",
            Self::Rect => "re",
            Self::Stroke => "S",
            Self::CloseStroke => "s",
            Self::Fill => "f",
            Self::FillObsolete => "F",
            Self::FillEvenOdd => "f*",
            Self::FillStroke => "B",
            Self::FillStrokeEvenOdd => "B*",
            Self::CloseFillStroke => "b",
            Self::CloseFillStrokeEvenOdd => "b*",
            Self::EndPath => "n",
            Self::Clip => "W",
            Self::ClipEvenOdd => "W*",
            Self::SetStrokeColorSpace => "CS",
            Self::SetFillColorSpace => "cs",
            Self::SetStrokeColor => "SC",
            Self::SetStrokeColorN => "SCN",
            Self::SetFillColor => "sc",
            Self::SetFillColorN => "scn",
            Self::SetStrokeGray => "G",
            Self::SetFillGray => "g",
            Self::SetStrokeRGB => "RG",
            Self::SetFillRGB => "rg",
            Self::SetStrokeCMYK => "K",
            Self::SetFillCMYK => "k",
            Self::InvokeXObject => "Do",
            Self::Shading => "sh",
            Self::InlineImage => "BI/ID/EI",
            Self::MarkedContentPoint => "MP",
            Self::MarkedContentPointProp => "DP",
            Self::BeginMarkedContent => "BMC",
            Self::BeginMarkedContentProp => "BDC",
            Self::EndMarkedContent => "EMC",
            Self::BeginCompatibility => "BX",
            Self::EndCompatibility => "EX",
            Self::Unknown(_) => "?",
        }
    }

    /// 사용자 출력용 표현을 반환한다.
    ///
    /// 대부분의 변형은 [`pdf_keyword`]와 동일하지만, `Unknown` 변형은
    /// raw bytes를 포함한 `"?<bytes>"` 형식의 `String`을 반환한다.
    pub fn display_name(&self) -> String {
        match self {
            Self::Unknown(bytes) => format!("?{}", String::from_utf8_lossy(bytes)),
            other => other.pdf_keyword().to_string(),
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    // BT-1: BeginText.pdf_keyword() == "BT"
    #[test]
    fn bt1_begin_text_keyword() {
        assert_eq!(ContentStreamOperator::BeginText.pdf_keyword(), "BT");
    }

    // BT-2: EndText.display_name() == "ET"
    #[test]
    fn bt2_end_text_display_name() {
        assert_eq!(ContentStreamOperator::EndText.display_name(), "ET");
    }

    // BT-3: Unknown raw bytes 포함 display_name
    #[test]
    fn bt3_unknown_display_name_includes_raw_bytes() {
        let op = ContentStreamOperator::Unknown(b"foo".to_vec());
        assert_eq!(op.display_name(), "?foo");
    }

    // BT-4: 모든 비-Unknown 변형의 pdf_keyword()가 빈 문자열 아님
    #[test]
    fn bt4_all_known_operators_have_nonempty_keyword() {
        let known: &[ContentStreamOperator] = &[
            ContentStreamOperator::BeginText,
            ContentStreamOperator::EndText,
            ContentStreamOperator::SetCharSpacing,
            ContentStreamOperator::SetWordSpacing,
            ContentStreamOperator::SetHorizontalScale,
            ContentStreamOperator::SetLeading,
            ContentStreamOperator::SetFont,
            ContentStreamOperator::SetRenderingMode,
            ContentStreamOperator::SetTextRise,
            ContentStreamOperator::MoveText,
            ContentStreamOperator::MoveTextSetLeading,
            ContentStreamOperator::SetTextMatrix,
            ContentStreamOperator::MoveToNextLine,
            ContentStreamOperator::ShowText,
            ContentStreamOperator::ShowTextAdjusted,
            ContentStreamOperator::MoveShowText,
            ContentStreamOperator::MoveSetShowText,
            ContentStreamOperator::SaveState,
            ContentStreamOperator::RestoreState,
            ContentStreamOperator::ConcatMatrix,
            ContentStreamOperator::SetLineWidth,
            ContentStreamOperator::SetLineCap,
            ContentStreamOperator::SetLineJoin,
            ContentStreamOperator::SetMiterLimit,
            ContentStreamOperator::SetDashPattern,
            ContentStreamOperator::SetFlatness,
            ContentStreamOperator::SetGraphicsState,
            ContentStreamOperator::SetRenderingIntent,
            ContentStreamOperator::MoveTo,
            ContentStreamOperator::LineTo,
            ContentStreamOperator::CurveTo,
            ContentStreamOperator::CurveToV,
            ContentStreamOperator::CurveToY,
            ContentStreamOperator::ClosePath,
            ContentStreamOperator::Rect,
            ContentStreamOperator::Stroke,
            ContentStreamOperator::CloseStroke,
            ContentStreamOperator::Fill,
            ContentStreamOperator::FillObsolete,
            ContentStreamOperator::FillEvenOdd,
            ContentStreamOperator::FillStroke,
            ContentStreamOperator::FillStrokeEvenOdd,
            ContentStreamOperator::CloseFillStroke,
            ContentStreamOperator::CloseFillStrokeEvenOdd,
            ContentStreamOperator::EndPath,
            ContentStreamOperator::Clip,
            ContentStreamOperator::ClipEvenOdd,
            ContentStreamOperator::SetStrokeColorSpace,
            ContentStreamOperator::SetFillColorSpace,
            ContentStreamOperator::SetStrokeColor,
            ContentStreamOperator::SetStrokeColorN,
            ContentStreamOperator::SetFillColor,
            ContentStreamOperator::SetFillColorN,
            ContentStreamOperator::SetStrokeGray,
            ContentStreamOperator::SetFillGray,
            ContentStreamOperator::SetStrokeRGB,
            ContentStreamOperator::SetFillRGB,
            ContentStreamOperator::SetStrokeCMYK,
            ContentStreamOperator::SetFillCMYK,
            ContentStreamOperator::InvokeXObject,
            ContentStreamOperator::Shading,
            ContentStreamOperator::InlineImage,
            ContentStreamOperator::MarkedContentPoint,
            ContentStreamOperator::MarkedContentPointProp,
            ContentStreamOperator::BeginMarkedContent,
            ContentStreamOperator::BeginMarkedContentProp,
            ContentStreamOperator::EndMarkedContent,
            ContentStreamOperator::BeginCompatibility,
            ContentStreamOperator::EndCompatibility,
        ];
        for op in known {
            let kw = op.pdf_keyword();
            assert!(!kw.is_empty(), "{op:?} has empty pdf_keyword");
        }
    }
}
