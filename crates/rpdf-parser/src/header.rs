use rpdf_core::types::PdfVersion;

use crate::error::ParseError;

/// PDF 헤더 파싱 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfHeader {
    /// PDF 버전 (`%PDF-X.Y`에서 추출).
    pub version: PdfVersion,
    /// 파일 시작점 기준으로 `%PDF-` 시그니처가 시작하는 절대 바이트 오프셋.
    /// 대부분 0이지만, BOM이나 다른 데이터가 앞에 있으면 0이 아닐 수 있다.
    pub byte_offset: usize,
    /// 헤더 바로 다음 줄에 0x80 이상의 바이트가 4개 이상이면 `true`.
    /// 이진 파일임을 나타내는 선택적 주석 (PDF 스펙 §7.5.2).
    pub has_binary_marker: bool,
}

/// `data`에서 PDF 헤더를 파싱한다.
///
/// 파일 처음 1KB 이내에서 `%PDF-` 시그니처를 탐색하고 버전·이진 마커 여부를 반환한다.
///
/// 버전 형식은 `X.Y`로 X와 Y 각각 단일 ASCII digit(PDF 스펙 §7.5.2).
/// 버전 뒤에는 반드시 `\r` 또는 `\n`이 와야 한다.
///
/// # Errors
///
/// - [`ParseError::HeaderNotFound`] — 처음 1KB 내에 `%PDF-` 없음
/// - [`ParseError::InvalidVersion`] — 버전이 `digit.digit` 형식이 아니거나
///   줄바꿈으로 끝나지 않음
pub fn parse_header(data: &[u8]) -> Result<PdfHeader, ParseError> {
    const SEARCH_LIMIT: usize = 1024;
    const PDF_MARKER: &[u8] = b"%PDF-";

    let search_end = data.len().min(SEARCH_LIMIT);
    let marker_offset = data[..search_end]
        .windows(PDF_MARKER.len())
        .position(|w| w == PDF_MARKER)
        .ok_or(ParseError::HeaderNotFound {
            searched_bytes: search_end,
        })?;

    // `version_start`는 `%PDF-` 바로 다음 바이트 위치.
    // `InvalidVersion`의 `offset`은 이 위치를 기준으로 실패 지점을 가리킨다.
    let version_start = marker_offset + PDF_MARKER.len();

    // 버전은 정확히 3바이트: ASCII digit, '.', ASCII digit (PDF 스펙)
    if version_start + 3 > data.len() {
        return Err(ParseError::InvalidVersion {
            offset: version_start,
            found: truncate_for_error(&data[version_start..], 16),
        });
    }

    let vb = &data[version_start..version_start + 3];

    let major = ascii_digit(vb[0]).ok_or_else(|| ParseError::InvalidVersion {
        offset: version_start,
        found: truncate_for_error(vb, 16),
    })?;

    if vb[1] != b'.' {
        return Err(ParseError::InvalidVersion {
            offset: version_start + 1,
            found: truncate_for_error(vb, 16),
        });
    }

    let minor = ascii_digit(vb[2]).ok_or_else(|| ParseError::InvalidVersion {
        offset: version_start + 2,
        found: truncate_for_error(vb, 16),
    })?;

    // 버전 바로 다음은 반드시 줄바꿈(\r 또는 \n)이어야 함 (PDF 스펙 §7.5.2)
    let after_version = version_start + 3;
    if after_version >= data.len() {
        return Err(ParseError::InvalidVersion {
            offset: after_version,
            found: String::new(),
        });
    }
    match data[after_version] {
        b'\n' | b'\r' => {}
        _ => {
            return Err(ParseError::InvalidVersion {
                offset: after_version,
                found: truncate_for_error(&data[after_version..], 16),
            });
        }
    }

    let version = PdfVersion::from_bytes(major, minor);
    let has_binary_marker = detect_binary_marker(data, after_version);

    Ok(PdfHeader {
        version,
        byte_offset: marker_offset,
        has_binary_marker,
    })
}

/// `bytes[..bytes.len().min(max)]`를 UTF-8 손실 변환하여 반환한다.
/// 에러 메시지에 긴 raw bytes가 그대로 포함되지 않도록 절단한다.
fn truncate_for_error(bytes: &[u8], max: usize) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(max)]).to_string()
}

/// ASCII 숫자 바이트를 그 값(0~9)으로 변환한다. 숫자가 아니면 `None`.
fn ascii_digit(b: u8) -> Option<u8> {
    if b.is_ascii_digit() {
        Some(b - b'0')
    } else {
        None
    }
}

/// 버전 줄 다음 줄에 0x80 이상 바이트가 4개 이상이면 이진 마커로 간주한다.
fn detect_binary_marker(data: &[u8], after_version_end: usize) -> bool {
    let rest = &data[after_version_end..];

    let line_start = rest
        .iter()
        .position(|&b| b != b'\r' && b != b'\n')
        .unwrap_or(rest.len());
    let line_data = &rest[line_start..];

    let line_end = line_data
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(line_data.len());

    line_data[..line_end].iter().filter(|&&b| b >= 0x80).count() >= 4
}
