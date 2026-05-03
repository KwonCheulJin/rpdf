//! PDF 1.5+ cross-reference stream 파싱 (ISO 32000 §7.5.8).
//!
//! xref 스트림은 전통 xref 테이블 대신 간접 객체 형식의 스트림으로 저장된다.
//! FlateDecode 압축과 PNG 예측 필터를 지원한다.
//!
//! 책임 범위:
//! - xref 스트림 딕셔너리 파싱 (/W, /Index, /Filter, /DecodeParms)
//! - FlateDecode 압축 해제
//! - PNG Predictor 1·10–15 언필터링
//! - xref 엔트리 타입 0·1·2 디코딩
//!
//! 범위 외:
//! - 객체 스트림(/Type /ObjStm) 파싱 — 후속 Task
//! - FlateDecode 외 필터 — `InvalidXrefStreamFilter` 에러
//! - TIFF Predictor(값 2) — `UnsupportedPredictor` 에러

use std::io::Read;

use flate2::read::ZlibDecoder;
use rpdf_core::types::{IndirectObject, ObjectId, PdfObject, PdfStream, XrefEntry};

use crate::error::ParseError;
use crate::objects::parse_indirect_object;
use crate::trailer::PdfTrailer;
use crate::xref::{XrefSectionInfo, XrefSectionResult};

/// xref 스트림 간접 객체를 파싱해 엔트리 목록, PdfTrailer, 섹션 정보를 반환한다.
///
/// `xref_offset`: `startxref` 또는 `/Prev`가 가리키는 간접 객체(`N G obj`)의 파일 오프셋.
/// 반환 타입이 `parse_xref_section`과 동일해 `parse_xref_chain`에서 투명 교체 가능.
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — 간접 객체가 스트림이 아니거나 구조 손상
/// - [`ParseError::XrefStreamInvalidW`] — `/W` 배열 누락 또는 형식 오류
/// - [`ParseError::XrefStreamInvalidIndex`] — `/Index` 배열 형식 오류
/// - [`ParseError::InvalidXrefStreamFilter`] — FlateDecode 외 필터
/// - [`ParseError::UnsupportedPredictor`] — TIFF 또는 알 수 없는 Predictor
/// - [`ParseError::XrefStreamDecompressError`] — zlib 압축 해제 실패
/// - [`ParseError::XrefStreamWFieldMismatch`] — W 크기와 데이터 길이 불일치
/// - [`ParseError::XrefStreamEntryCountMismatch`] — 엔트리 수가 /Index 선언과 불일치
pub(crate) fn parse_xref_stream(
    data: &[u8],
    xref_offset: u64,
) -> Result<XrefSectionResult, ParseError> {
    let (_indirect, stream, _body_offset) = parse_xref_stream_dict(data, xref_offset)?;
    let metadata = extract_xref_stream_metadata(&stream.dict, xref_offset)?;
    let trailer = build_trailer_from_xref_stream_metadata(&metadata)?;

    let decompressed = match metadata.filter {
        Some(XrefStreamFilter::FlateDecode) => decompress_flate(&stream.data, xref_offset)?,
        None => stream.data.clone(),
    };

    let unpredicted = match metadata.predictor {
        None | Some(1) => decompressed,
        Some(p) if (10..=15).contains(&p) => {
            let columns = metadata.columns.unwrap_or(1);
            unpredict_png(&decompressed, columns, p, xref_offset)?
        }
        Some(other) => {
            return Err(ParseError::UnsupportedPredictor {
                offset: xref_offset,
                value: other,
            });
        }
    };

    let entries = decode_entries(&unpredicted, metadata.w, &metadata.index, xref_offset)?;
    let entry_count = entries.len();

    Ok(XrefSectionResult {
        entries,
        trailer,
        section_info: XrefSectionInfo {
            offset: xref_offset,
            entry_count,
        },
    })
}

/// `xref_offset` 위치의 간접 객체가 `/Type /XRef` 스트림인지 검사한다.
///
/// `trailer.rs::is_xref_stream`(빠른 휴리스틱)과 달리 실제 파싱을 수행해
/// stream 여부와 `/Type /XRef`를 모두 확인한다. ISO 32000 §7.5.8.
pub(crate) fn is_xref_stream(data: &[u8], xref_offset: u64) -> bool {
    let offset = xref_offset as usize;
    if offset >= data.len() {
        return false;
    }
    let Ok((indirect, _)) = parse_indirect_object(data, offset) else {
        return false;
    };
    let PdfObject::Stream(stream) = &indirect.object else {
        return false;
    };
    matches!(
        stream.dict.get(b"Type"),
        Some(PdfObject::Name(n)) if n == b"XRef"
    )
}

/// xref 스트림 간접 객체를 파싱해 `(IndirectObject, PdfStream, body_offset)` 튜플로 반환한다.
///
/// `body_offset`은 스트림 데이터가 시작하는 파일 내 절대 오프셋이다(에러 보고용).
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — stream이 아니거나 `/Type /XRef` 불일치
///
/// ISO 32000 §7.5.8
pub(crate) fn parse_xref_stream_dict(
    data: &[u8],
    xref_offset: u64,
) -> Result<(IndirectObject, PdfStream, u64), ParseError> {
    let offset = xref_offset as usize;
    let (indirect, _consumed) =
        parse_indirect_object(data, offset).map_err(|e| ParseError::MalformedXrefStream {
            offset: xref_offset,
            reason: format!("간접 객체 파싱 실패: {e}"),
        })?;

    let stream = match &indirect.object {
        PdfObject::Stream(s) => s.clone(),
        _ => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: "xref 스트림 객체가 stream이 아님".to_string(),
            });
        }
    };

    // /Type == /XRef 확인 (ISO 32000 §7.5.8.2)
    match stream.dict.get(b"Type") {
        Some(PdfObject::Name(n)) if n == b"XRef" => {}
        Some(other) => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: format!("/Type가 /XRef가 아님: {other:?}"),
            });
        }
        None => {
            return Err(ParseError::MalformedXrefStream {
                offset: xref_offset,
                reason: "/Type 키 누락".to_string(),
            });
        }
    }

    Ok((indirect, stream, xref_offset))
}

/// xref 스트림 딕셔너리에서 메타데이터를 추출한다.
///
/// `/W`, `/Index`, `/Filter`, `/DecodeParms`, `/Root`, `/Prev` 등을 파싱한다.
/// ISO 32000 §7.5.8.2 Table 17
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — `/Size` 누락·비정수, `/DecodeParms` 형식 오류
/// - [`ParseError::XrefStreamInvalidW`] — `/W` 없거나 3개 정수 아님
/// - [`ParseError::XrefStreamInvalidIndex`] — `/Index` 홀수 원소 또는 비정수
/// - [`ParseError::InvalidXrefStreamFilter`] — FlateDecode 외 필터
pub(crate) fn extract_xref_stream_metadata(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<XrefStreamMetadata, ParseError> {
    // /Size — 필수
    let size = match dict.get(b"Size") {
        None => {
            return Err(ParseError::MalformedXrefStream {
                offset,
                reason: "/Size 키 누락".to_string(),
            });
        }
        Some(obj) => {
            obj.as_u64()
                .map(|n| n as u32)
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/Size 값이 비음수 정수가 아님: {obj:?}"),
                })?
        }
    };

    // /W — 필수, [W1 W2 W3] 3개 비음수 정수
    let w = extract_w(dict, offset)?;

    // /Index — 선택, 기본값 [(0, size)]
    let index = extract_index(dict, size, offset)?;

    // /Filter — 선택
    let filter = extract_filter(dict, offset)?;

    // /DecodeParms — 선택
    let (predictor, columns) = extract_decode_parms(dict, offset)?;

    // /Prev — 선택
    let prev = match dict.get(b"Prev") {
        None => None,
        Some(obj) => Some(
            obj.as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: "/Prev 값이 정수가 아님".to_string(),
                })?,
        ),
    };

    // /Root — 선택 (PdfTrailer 생성 시 필수 검사)
    let root = match dict.get(b"Root") {
        Some(PdfObject::Reference(id)) => Some(*id),
        _ => None,
    };

    // /Info — 선택
    let info = match dict.get(b"Info") {
        Some(PdfObject::Reference(id)) => Some(*id),
        _ => None,
    };

    Ok(XrefStreamMetadata {
        size,
        w,
        index,
        filter,
        predictor,
        columns,
        prev,
        root,
        info,
    })
}

/// xref 스트림 메타데이터에서 `PdfTrailer`를 구성한다.
///
/// `/Root`이 없으면 [`ParseError::MissingRequiredKey`]를 반환한다.
pub(crate) fn build_trailer_from_xref_stream_metadata(
    meta: &XrefStreamMetadata,
) -> Result<PdfTrailer, ParseError> {
    let root = meta
        .root
        .ok_or(ParseError::MissingRequiredKey { key: "Root" })?;

    Ok(PdfTrailer {
        size: meta.size,
        root,
        info: meta.info,
        prev: meta.prev,
    })
}

/// `/W [W1 W2 W3]` 배열을 추출한다.
fn extract_w(dict: &rpdf_core::types::PdfDict, offset: u64) -> Result<[usize; 3], ParseError> {
    let arr = match dict.get(b"W") {
        None => {
            return Err(ParseError::XrefStreamInvalidW {
                offset,
                reason: "/W 키 누락".to_string(),
            });
        }
        Some(PdfObject::Array(a)) => a,
        Some(other) => {
            return Err(ParseError::XrefStreamInvalidW {
                offset,
                reason: format!("/W가 배열이 아님: {other:?}"),
            });
        }
    };

    if arr.len() != 3 {
        return Err(ParseError::XrefStreamInvalidW {
            offset,
            reason: format!("/W 원소 수가 3이 아님: {}", arr.len()),
        });
    }

    let mut w = [0usize; 3];
    for (i, obj) in arr.iter().enumerate() {
        w[i] = obj.as_u64().ok_or_else(|| ParseError::XrefStreamInvalidW {
            offset,
            reason: format!("/W[{i}]가 비음수 정수가 아님: {obj:?}"),
        })? as usize;
    }

    // u64는 8바이트 — W[i] > 8이면 silent wrap-around 발생 가능 (악성 PDF 방어)
    if w[0] > 8 || w[1] > 8 || w[2] > 8 {
        return Err(ParseError::XrefStreamInvalidW {
            offset,
            reason: format!("W 필드 값이 8을 초과함: {:?}", w),
        });
    }

    Ok(w)
}

/// `/Index` 배열을 추출한다. 없으면 `[(0, size)]` 기본값을 반환한다.
fn extract_index(
    dict: &rpdf_core::types::PdfDict,
    size: u32,
    offset: u64,
) -> Result<Vec<(u32, u32)>, ParseError> {
    let arr = match dict.get(b"Index") {
        None => return Ok(vec![(0, size)]),
        Some(PdfObject::Array(a)) => a,
        Some(other) => {
            return Err(ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index가 배열이 아님: {other:?}"),
            });
        }
    };

    if arr.len() % 2 != 0 {
        return Err(ParseError::XrefStreamInvalidIndex {
            offset,
            reason: format!("/Index 원소 수가 홀수: {}", arr.len()),
        });
    }

    let mut pairs = Vec::with_capacity(arr.len() / 2);
    for chunk in arr.chunks(2) {
        let first = chunk[0]
            .as_u64()
            .ok_or_else(|| ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index first 값이 비음수 정수가 아님: {:?}", chunk[0]),
            })? as u32;
        let count = chunk[1]
            .as_u64()
            .ok_or_else(|| ParseError::XrefStreamInvalidIndex {
                offset,
                reason: format!("/Index count 값이 비음수 정수가 아님: {:?}", chunk[1]),
            })? as u32;
        pairs.push((first, count));
    }
    Ok(pairs)
}

/// `/Filter` 를 추출한다.
fn extract_filter(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<Option<XrefStreamFilter>, ParseError> {
    match dict.get(b"Filter") {
        None => Ok(None),
        Some(PdfObject::Name(n)) => {
            if n == b"FlateDecode" {
                Ok(Some(XrefStreamFilter::FlateDecode))
            } else {
                Err(ParseError::InvalidXrefStreamFilter {
                    offset,
                    filter: String::from_utf8_lossy(n).into_owned(),
                })
            }
        }
        Some(PdfObject::Array(arr)) => {
            // [/FlateDecode] 단일 원소 배열 허용 (ISO 32000 §7.3.8.2)
            if arr.len() == 1 {
                match &arr[0] {
                    PdfObject::Name(n) if n == b"FlateDecode" => {
                        Ok(Some(XrefStreamFilter::FlateDecode))
                    }
                    PdfObject::Name(n) => Err(ParseError::InvalidXrefStreamFilter {
                        offset,
                        filter: String::from_utf8_lossy(n).into_owned(),
                    }),
                    other => Err(ParseError::InvalidXrefStreamFilter {
                        offset,
                        filter: format!("{other:?}"),
                    }),
                }
            } else {
                Err(ParseError::InvalidXrefStreamFilter {
                    offset,
                    filter: "다중 필터 체인 미지원".to_string(),
                })
            }
        }
        Some(other) => Err(ParseError::InvalidXrefStreamFilter {
            offset,
            filter: format!("{other:?}"),
        }),
    }
}

/// `/DecodeParms`에서 `(predictor, columns)` 를 추출한다.
fn extract_decode_parms(
    dict: &rpdf_core::types::PdfDict,
    offset: u64,
) -> Result<(Option<u8>, Option<usize>), ParseError> {
    let parms_obj = match dict.get(b"DecodeParms") {
        None => return Ok((None, None)),
        Some(obj) => obj,
    };

    let parms_dict = match parms_obj {
        PdfObject::Dictionary(d) => d,
        other => {
            return Err(ParseError::MalformedXrefStream {
                offset,
                reason: format!("/DecodeParms가 딕셔너리가 아님: {other:?}"),
            });
        }
    };

    let predictor = match parms_dict.get(b"Predictor") {
        None => None,
        Some(obj) => {
            let n = obj
                .as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/DecodeParms /Predictor가 정수가 아님: {obj:?}"),
                })? as u8;
            Some(n)
        }
    };

    let columns = match parms_dict.get(b"Columns") {
        None => None,
        Some(obj) => {
            let n = obj
                .as_u64()
                .ok_or_else(|| ParseError::MalformedXrefStream {
                    offset,
                    reason: format!("/DecodeParms /Columns가 정수가 아님: {obj:?}"),
                })? as usize;
            Some(n)
        }
    };

    Ok((predictor, columns))
}

/// FlateDecode 압축을 해제한다 (ISO 32000 §7.4.4).
///
/// PDF FlateDecode는 RFC 1950 zlib 형식(2바이트 헤더 + deflate + Adler-32 checksum).
/// raw deflate(RFC 1951)가 아님에 주의.
///
/// # Errors
///
/// - [`ParseError::XrefStreamDecompressError`] — zlib 헤더 오류, 손상된 스트림 등.
pub(crate) fn decompress_flate(compressed: &[u8], offset: u64) -> Result<Vec<u8>, ParseError> {
    let mut decoder = ZlibDecoder::new(compressed);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| ParseError::XrefStreamDecompressError {
            offset,
            reason: e.to_string(),
        })?;
    Ok(out)
}

/// PNG 예측 필터를 제거해 원본 xref 엔트리 데이터를 복원한다 (ISO 32000 §7.4.4.4).
///
/// 각 행은 `1바이트 predictor tag + columns 바이트 데이터` 형식.
/// - Predictor 10 (None): 원본 그대로
/// - Predictor 11 (Sub): 좌측 픽셀 차분
/// - Predictor 12 (Up): 위 행 픽셀 차분 (xref 스트림에서 가장 흔함)
/// - Predictor 13 (Average): (좌 + 위) / 2 차분
/// - Predictor 14 (Paeth): RFC 2083 §6.6 Paeth 차분
/// - Predictor 15 (Optimum): 행마다 첫 바이트가 실제 predictor
///
/// # Errors
///
/// - [`ParseError::MalformedXrefStream`] — 데이터 길이가 `columns + 1` 배수 아님
/// - [`ParseError::UnsupportedPredictor`] — tag가 10–14 범위 밖
pub(crate) fn unpredict_png(
    data: &[u8],
    columns: usize,
    predictor: u8,
    offset: u64,
) -> Result<Vec<u8>, ParseError> {
    let row_len = columns + 1;
    if !data.len().is_multiple_of(row_len) {
        return Err(ParseError::MalformedXrefStream {
            offset,
            reason: format!(
                "PNG 필터 행 길이 불일치: 데이터 {}바이트, 행 크기 {} (columns={})",
                data.len(),
                row_len,
                columns
            ),
        });
    }

    let num_rows = data.len() / row_len;
    let mut out = Vec::with_capacity(columns * num_rows);
    let mut prev_row = vec![0u8; columns];

    for row_idx in 0..num_rows {
        let row = &data[row_idx * row_len..(row_idx + 1) * row_len];
        let tag = if predictor == 15 { row[0] } else { predictor };
        let pixels = &row[1..];

        let mut current_row = vec![0u8; columns];
        for i in 0..columns {
            let left = if i > 0 { current_row[i - 1] } else { 0 };
            let up = prev_row[i];
            let upper_left = if i > 0 { prev_row[i - 1] } else { 0 };

            current_row[i] = match tag {
                10 => pixels[i],
                11 => pixels[i].wrapping_add(left),
                12 => pixels[i].wrapping_add(up),
                13 => pixels[i].wrapping_add(((left as u16 + up as u16) / 2) as u8),
                14 => pixels[i].wrapping_add(paeth_predictor(left, up, upper_left)),
                _ => {
                    return Err(ParseError::UnsupportedPredictor { offset, value: tag });
                }
            };
        }

        out.extend_from_slice(&current_row);
        prev_row = current_row;
    }

    Ok(out)
}

/// RFC 2083 §6.6 Paeth predictor.
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

/// 압축 해제·언필터링된 바이트를 `/W`와 `/Index`에 따라 xref 엔트리로 변환한다.
///
/// ISO 32000 §7.5.8.3 — 각 엔트리는 `W[0]+W[1]+W[2]` 바이트, big-endian 순서.
/// - Type 0: Free — `XrefEntry::Free { next_free_obj_num, generation }`
/// - Type 1: InUse — `XrefEntry::InUse { offset, generation }`
/// - Type 2: Compressed — `XrefEntry::Compressed { obj_stm_num, index }`
/// - Type ≥ 3: 스펙상 application-defined, 무시(엔트리 스킵)
///
/// W[0]=0이면 type 필드 없음 → default Type=1 적용. W[i]=0이면 해당 필드=0.
///
/// # Errors
///
/// - [`ParseError::XrefStreamInvalidW`] — `W[0]+W[1]+W[2] == 0`
/// - [`ParseError::XrefStreamWFieldMismatch`] — 데이터 길이가 row 크기 배수 아님
pub(crate) fn decode_entries(
    data: &[u8],
    w: [usize; 3],
    index: &[(u32, u32)],
    offset: u64,
) -> Result<Vec<(u32, XrefEntry)>, ParseError> {
    let row_size = w[0] + w[1] + w[2];
    if row_size == 0 {
        return Err(ParseError::XrefStreamInvalidW {
            offset,
            reason: "W 합이 0 — 엔트리 행 크기를 결정할 수 없음".to_string(),
        });
    }

    let total_count: usize = index.iter().map(|(_, count)| *count as usize).sum();
    let expected_len = row_size * total_count;
    if data.len() != expected_len {
        return Err(ParseError::XrefStreamWFieldMismatch {
            offset,
            w_total: row_size,
            data_len: data.len(),
        });
    }

    let mut result = Vec::with_capacity(total_count);
    let mut data_pos = 0usize;

    for &(start_obj, count) in index {
        for i in 0..count {
            let row = &data[data_pos..data_pos + row_size];
            data_pos += row_size;

            let obj_num = start_obj + i;

            // Type 필드: W[0]=0이면 default=1
            let type_val = if w[0] == 0 {
                1u8
            } else {
                read_be_u64(&row[..w[0]]) as u8
            };

            // val1: W[1]=0이면 0
            let val1 = if w[1] == 0 {
                0u64
            } else {
                read_be_u64(&row[w[0]..w[0] + w[1]])
            };

            // val2: W[2]=0이면 0
            let val2 = if w[2] == 0 {
                0u64
            } else {
                read_be_u64(&row[w[0] + w[1]..w[0] + w[1] + w[2]])
            };

            let entry = match type_val {
                0 => XrefEntry::Free {
                    next_free_obj_num: val1 as u32,
                    generation: val2 as u16,
                },
                1 => XrefEntry::InUse {
                    offset: val1,
                    generation: val2 as u16,
                },
                2 => XrefEntry::Compressed {
                    obj_stm_num: val1 as u32,
                    index: val2 as u32,
                },
                // PDF §7.5.8.3: 알 수 없는 type은 무시(스킵)
                _ => continue,
            };

            result.push((obj_num, entry));
        }
    }

    Ok(result)
}

/// `data`를 big-endian 순서로 읽어 `u64`로 반환한다.
///
/// W[i]가 8을 초과하면 상위 바이트는 버려진다(u64 범위 내 최하위 8바이트 유효).
fn read_be_u64(data: &[u8]) -> u64 {
    data.iter().fold(0u64, |acc, &b| (acc << 8) | b as u64)
}

/// xref 스트림 메타데이터 — `extract_xref_stream_metadata` 반환값.
///
/// ISO 32000 §7.5.8.2 Table 17
#[derive(Debug)]
pub(crate) struct XrefStreamMetadata {
    /// `/Size` — xref 엔트리 총 개수.
    pub size: u32,
    /// `/W [W1 W2 W3]` — 엔트리 필드별 바이트 너비.
    pub w: [usize; 3],
    /// `/Index` 서브섹션 쌍. 없으면 `[(0, size)]` 기본값 적용.
    pub index: Vec<(u32, u32)>,
    /// `/Filter` — None이면 비압축.
    pub filter: Option<XrefStreamFilter>,
    /// `/DecodeParms /Predictor` — None이면 1(기본값).
    pub predictor: Option<u8>,
    /// `/DecodeParms /Columns` — None이면 C 단계에서 W1+W2+W3 사용.
    pub columns: Option<usize>,
    /// `/Prev` — 이전 xref 위치.
    pub prev: Option<u64>,
    /// `/Root` — 문서 카탈로그 간접 참조. PdfTrailer 생성 시 필수.
    pub root: Option<ObjectId>,
    /// `/Info` — 문서 정보 간접 참조.
    pub info: Option<ObjectId>,
}

/// xref 스트림 압축 필터 종류.
///
/// ISO 32000 §7.4 — FlateDecode만 지원. 다른 필터는 `InvalidXrefStreamFilter`.
#[derive(Debug)]
pub(crate) enum XrefStreamFilter {
    /// `/FlateDecode` — zlib/deflate 압축.
    FlateDecode,
}

// ── 단위 테스트 ─────────────────────────────────────────────────────────────────
// pub(crate) 함수 테스트는 CLAUDE.md 규칙에 따라 인라인 배치.
// tests/ 폴더는 크레이트 외부이므로 pub(crate) 접근 불가 → 인라인이 유일한 방법.
// parse_xref_stream이 pub으로 노출되면 통합 테스트 파일 추가.

#[cfg(test)]
mod internal_tests {
    use super::*;
    use rpdf_core::types::{ObjectId, PdfDict, PdfObject};

    // ── 합성 데이터 헬퍼 ──────────────────────────────────────────────────────

    /// `/Type /XRef` 스트림 간접 객체 바이트를 생성한다.
    /// `dict_extra`는 기본 키(/Type, /Size, /W, /Root, /Length) 이외의 추가 내용.
    fn make_xref_stream_object(dict_extra: &str, body: &[u8]) -> Vec<u8> {
        let length = body.len();
        let extra = if dict_extra.is_empty() {
            String::new()
        } else {
            format!(" {dict_extra}")
        };
        let dict =
            format!("<< /Type /XRef /Size 10 /W [1 3 1] /Root 1 0 R /Length {length}{extra} >>");
        let mut buf = Vec::new();
        buf.extend_from_slice(format!("5 0 obj\n{dict}\nstream\n").as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        buf
    }

    /// `PdfDict`를 직접 구성하는 헬퍼.
    fn make_dict(entries: Vec<(&'static [u8], PdfObject)>) -> PdfDict {
        PdfDict(entries.into_iter().map(|(k, v)| (k.to_vec(), v)).collect())
    }

    fn name(s: &str) -> PdfObject {
        PdfObject::Name(s.as_bytes().to_vec())
    }

    fn int(n: i64) -> PdfObject {
        PdfObject::Integer(n)
    }

    fn arr(items: Vec<PdfObject>) -> PdfObject {
        PdfObject::Array(items)
    }

    fn ref_obj(num: u32) -> PdfObject {
        PdfObject::Reference(ObjectId {
            number: num,
            generation: 0,
        })
    }

    // ── is_xref_stream ────────────────────────────────────────────────────────

    #[test]
    fn is_xref_stream_with_xref_type_returns_true() {
        let data = make_xref_stream_object("", b"dummy");
        assert!(is_xref_stream(&data, 0));
    }

    #[test]
    fn is_xref_stream_with_catalog_type_returns_false() {
        let body = b"dummy";
        let dict = format!(
            "<< /Type /Catalog /Size 10 /W [1 3 1] /Root 1 0 R /Length {} >>",
            body.len()
        );
        let mut data = Vec::new();
        data.extend_from_slice(format!("5 0 obj\n{dict}\nstream\n").as_bytes());
        data.extend_from_slice(body);
        data.extend_from_slice(b"\nendstream\nendobj\n");
        assert!(!is_xref_stream(&data, 0));
    }

    #[test]
    fn is_xref_stream_non_stream_object_returns_false() {
        assert!(!is_xref_stream(b"5 0 obj\n42\nendobj\n", 0));
    }

    // ── parse_xref_stream_dict ────────────────────────────────────────────────

    #[test]
    fn parse_xref_stream_dict_valid_returns_ok() {
        let data = make_xref_stream_object("", b"rawbytes");
        let (indirect, stream, _body_offset) = parse_xref_stream_dict(&data, 0).unwrap();
        assert_eq!(indirect.id.number, 5);
        assert_eq!(stream.data, b"rawbytes");
        assert_eq!(
            stream.dict.get(b"Type"),
            Some(&PdfObject::Name(b"XRef".to_vec()))
        );
    }

    #[test]
    fn parse_xref_stream_dict_non_stream_returns_malformed() {
        let data = b"5 0 obj\n42\nendobj\n";
        let err = parse_xref_stream_dict(data, 0).unwrap_err();
        assert!(matches!(err, ParseError::MalformedXrefStream { .. }));
    }

    // ── extract_xref_stream_metadata ─────────────────────────────────────────

    #[test]
    fn extract_metadata_minimal_size_and_w_only() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert_eq!(meta.size, 10);
        assert_eq!(meta.w, [1, 3, 1]);
        // /Index 없으면 기본값 [(0, size)]
        assert_eq!(meta.index, vec![(0, 10)]);
        assert!(meta.filter.is_none());
        assert!(meta.predictor.is_none());
        assert!(meta.columns.is_none());
    }

    #[test]
    fn extract_metadata_with_explicit_index() {
        let dict = make_dict(vec![
            (b"Size", int(20)),
            (b"W", arr(vec![int(1), int(4), int(2)])),
            (b"Index", arr(vec![int(5), int(3), int(15), int(5)])),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert_eq!(meta.index, vec![(5, 3), (15, 5)]);
    }

    #[test]
    fn extract_metadata_with_flate_and_decode_parms() {
        let decode_parms = PdfObject::Dictionary(make_dict(vec![
            (b"Predictor", int(12)),
            (b"Columns", int(5)),
        ]));
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Filter", name("FlateDecode")),
            (b"DecodeParms", decode_parms),
            (b"Root", ref_obj(1)),
        ]);
        let meta = extract_xref_stream_metadata(&dict, 0).unwrap();
        assert!(matches!(meta.filter, Some(XrefStreamFilter::FlateDecode)));
        assert_eq!(meta.predictor, Some(12));
        assert_eq!(meta.columns, Some(5));
    }

    #[test]
    fn extract_metadata_invalid_filter_returns_error() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3), int(1)])),
            (b"Filter", name("LZWDecode")),
            (b"Root", ref_obj(1)),
        ]);
        let err = extract_xref_stream_metadata(&dict, 0).unwrap_err();
        assert!(
            matches!(&err, ParseError::InvalidXrefStreamFilter { filter, .. } if filter == "LZWDecode")
        );
    }

    #[test]
    fn extract_metadata_invalid_w_too_few_elements_returns_error() {
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(3)])), // 2개 원소 (3개 필요)
            (b"Root", ref_obj(1)),
        ]);
        let err = extract_xref_stream_metadata(&dict, 0).unwrap_err();
        assert!(matches!(err, ParseError::XrefStreamInvalidW { .. }));
    }

    #[test]
    fn decode_entries_rejects_w_field_exceeding_8_bytes() {
        // W[1]=100은 u64 범위(8바이트)를 초과 → silent wrap-around 방지를 위해 거부
        let dict = make_dict(vec![
            (b"Size", int(10)),
            (b"W", arr(vec![int(1), int(100), int(2)])),
            (b"Root", ref_obj(1)),
        ]);
        let err = extract_xref_stream_metadata(&dict, 0).unwrap_err();
        assert!(matches!(err, ParseError::XrefStreamInvalidW { .. }));
    }

    // ── decompress_flate / unpredict_png 헬퍼 ─────────────────────────────────

    /// `plain` 바이트를 zlib(RFC 1950) 형식으로 압축해 반환한다.
    fn make_zlib_data(plain: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(plain).unwrap();
        enc.finish().unwrap()
    }

    // ── decompress_flate ─────────────────────────────────────────────────────

    #[test]
    fn decompress_flate_valid_zlib_returns_original() {
        let plain = b"hello xref stream";
        let compressed = make_zlib_data(plain);
        let result = decompress_flate(&compressed, 0).unwrap();
        assert_eq!(result, plain);
    }

    #[test]
    fn decompress_flate_invalid_data_returns_error() {
        let garbage = b"not zlib data at all";
        let err = decompress_flate(garbage, 42).unwrap_err();
        assert!(matches!(
            err,
            ParseError::XrefStreamDecompressError { offset: 42, .. }
        ));
    }

    #[test]
    fn decompress_flate_empty_input_returns_empty_vec() {
        // flate2 ZlibDecoder는 빈 입력을 Ok([])으로 처리한다 (에러 없음).
        let result = decompress_flate(&[], 0).unwrap();
        assert!(result.is_empty());
    }

    // ── unpredict_png ────────────────────────────────────────────────────────

    #[test]
    fn unpredict_png_predictor10_none_returns_original() {
        // predictor 10: 각 행 tag=10, 데이터 그대로
        let columns = 3usize;
        // 행 1: [10, 1, 2, 3], 행 2: [10, 4, 5, 6]
        let data = [10u8, 1, 2, 3, 10, 4, 5, 6];
        let result = unpredict_png(&data, columns, 10, 0).unwrap();
        assert_eq!(result, [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn unpredict_png_predictor12_up_applies_delta_from_prev_row() {
        // predictor 12 (Up): current[i] += prev[i]
        let columns = 3usize;
        // 행 1: prev=0 → delta [10, 20, 30] → result [10, 20, 30]
        // 행 2: delta [1, 2, 3], prev=[10,20,30] → result [11, 22, 33]
        let data = [12u8, 10, 20, 30, 12, 1, 2, 3];
        let result = unpredict_png(&data, columns, 12, 0).unwrap();
        assert_eq!(result, [10, 20, 30, 11, 22, 33]);
    }

    #[test]
    fn unpredict_png_predictor15_optimum_uses_per_row_tag() {
        // predictor 15: 행마다 첫 바이트가 실제 predictor
        let columns = 2usize;
        // 행 1: tag=10(None), [5, 7] → [5, 7]
        // 행 2: tag=12(Up),   [1, 2], prev=[5,7] → [6, 9]
        let data = [10u8, 5, 7, 12, 1, 2];
        let result = unpredict_png(&data, columns, 15, 0).unwrap();
        assert_eq!(result, [5, 7, 6, 9]);
    }

    #[test]
    fn unpredict_png_row_length_mismatch_returns_malformed() {
        // columns=3 → row_len=4, 데이터 7바이트는 4의 배수 아님
        let data = [10u8, 1, 2, 3, 10, 4, 5]; // 7바이트
        let err = unpredict_png(&data, 3, 10, 99).unwrap_err();
        assert!(matches!(
            err,
            ParseError::MalformedXrefStream { offset: 99, .. }
        ));
    }

    #[test]
    fn unpredict_png_unknown_tag_returns_unsupported_predictor() {
        // tag=16은 알 수 없는 값 → UnsupportedPredictor
        let data = [16u8, 1, 2, 3]; // columns=3
        let err = unpredict_png(&data, 3, 15, 0).unwrap_err(); // predictor=15 → 행 tag 우선
        assert!(matches!(
            err,
            ParseError::UnsupportedPredictor { value: 16, .. }
        ));
    }

    // ── decode_entries 헬퍼 ───────────────────────────────────────────────────

    /// raw xref 스트림 엔트리 바이트를 big-endian으로 빌드한다.
    /// `fields`: [(field_bytes, value)] — 각 필드를 주어진 너비로 big-endian 인코딩.
    fn make_entry_row(fields: &[(usize, u64)]) -> Vec<u8> {
        let mut row = Vec::new();
        for &(width, val) in fields {
            for i in (0..width).rev() {
                row.push(((val >> (i * 8)) & 0xff) as u8);
            }
        }
        row
    }

    // ── decode_entries ────────────────────────────────────────────────────────

    #[test]
    fn decode_entries_type0_free_entry() {
        // W = [1, 4, 2]: type(1) + next_free(4) + gen(2)
        // type=0, next_free=5, gen=0
        let w = [1, 4, 2];
        let row = make_entry_row(&[(1, 0), (4, 5), (2, 0)]);
        let index = vec![(0u32, 1u32)];
        let entries = decode_entries(&row, w, &index, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            (
                0,
                XrefEntry::Free {
                    next_free_obj_num: 5,
                    generation: 0
                }
            )
        );
    }

    #[test]
    fn decode_entries_type1_inuse_entry() {
        // W = [1, 4, 2]: type=1, offset=1024, gen=3
        let w = [1, 4, 2];
        let row = make_entry_row(&[(1, 1), (4, 1024), (2, 3)]);
        let index = vec![(7u32, 1u32)]; // 객체 번호 7
        let entries = decode_entries(&row, w, &index, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            (
                7,
                XrefEntry::InUse {
                    offset: 1024,
                    generation: 3
                }
            )
        );
    }

    #[test]
    fn decode_entries_type2_compressed_entry() {
        // W = [1, 4, 2]: type=2, obj_stm_num=10, index=2
        let w = [1, 4, 2];
        let row = make_entry_row(&[(1, 2), (4, 10), (2, 2)]);
        let index = vec![(3u32, 1u32)]; // 객체 번호 3
        let entries = decode_entries(&row, w, &index, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            (
                3,
                XrefEntry::Compressed {
                    obj_stm_num: 10,
                    index: 2
                }
            )
        );
    }

    #[test]
    fn decode_entries_w0_zero_defaults_to_type1() {
        // W = [0, 4, 2]: type 필드 없음 → default Type=1
        // val1=2048(offset), val2=0(gen)
        let w = [0, 4, 2];
        let row = make_entry_row(&[(4, 2048), (2, 0)]);
        let index = vec![(1u32, 1u32)];
        let entries = decode_entries(&row, w, &index, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            (
                1,
                XrefEntry::InUse {
                    offset: 2048,
                    generation: 0
                }
            )
        );
    }

    #[test]
    fn decode_entries_data_length_mismatch_returns_error() {
        // W = [1, 4, 2] → row_size=7, total=2 → expected=14, 실제 13
        let w = [1, 4, 2];
        let mut data = make_entry_row(&[(1, 1), (4, 100), (2, 0)]);
        data.extend_from_slice(&make_entry_row(&[(1, 1), (4, 200)])); // 부족 (6바이트)
        let index = vec![(0u32, 2u32)];
        let err = decode_entries(&data, w, &index, 99).unwrap_err();
        assert!(matches!(
            err,
            ParseError::XrefStreamWFieldMismatch { offset: 99, .. }
        ));
    }

    #[test]
    fn decode_entries_multi_subsection_and_unknown_type_skipped() {
        // W = [1, 4, 2], index = [(0, 2), (10, 1)]
        // 엔트리 0: type=1, offset=100
        // 엔트리 1: type=99 (알 수 없음 → 스킵)
        // 엔트리 10: type=0, next_free=0
        let w = [1, 4, 2];
        let mut data = Vec::new();
        data.extend_from_slice(&make_entry_row(&[(1, 1), (4, 100), (2, 0)]));
        data.extend_from_slice(&make_entry_row(&[(1, 99), (4, 0), (2, 0)])); // 스킵
        data.extend_from_slice(&make_entry_row(&[(1, 0), (4, 0), (2, 0)]));
        let index = vec![(0u32, 2u32), (10u32, 1u32)];
        let entries = decode_entries(&data, w, &index, 0).unwrap();
        // type=99는 스킵됨 → 2개 (obj 0 InUse, obj 10 Free)
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0],
            (
                0,
                XrefEntry::InUse {
                    offset: 100,
                    generation: 0
                }
            )
        );
        assert_eq!(
            entries[1],
            (
                10,
                XrefEntry::Free {
                    next_free_obj_num: 0,
                    generation: 0
                }
            )
        );
    }

    // ── parse_xref_stream 통합 ────────────────────────────────────────────────

    /// 비압축(filter 없음) xref 스트림 객체 바이트를 생성한다.
    /// `entries_data`: 원시 엔트리 바이트 (W×count 바이트)
    fn make_raw_xref_stream(entries_data: &[u8], w: [usize; 3], count: u32) -> Vec<u8> {
        let length = entries_data.len();
        let dict = format!(
            "<< /Type /XRef /Size {count} /W [{} {} {}] /Root 1 0 R /Length {length} >>",
            w[0], w[1], w[2]
        );
        let mut buf = Vec::new();
        buf.extend_from_slice(format!("1 0 obj\n{dict}\nstream\n").as_bytes());
        buf.extend_from_slice(entries_data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        buf
    }

    #[test]
    fn parse_xref_stream_no_filter_type1_entries() {
        // 비압축 xref 스트림, W=[1,4,2], 2개 엔트리
        let w = [1usize, 4, 2];
        let mut entries_data = Vec::new();
        entries_data.extend_from_slice(&make_entry_row(&[(1, 1), (4, 0), (2, 0)])); // obj0 free
        entries_data.extend_from_slice(&make_entry_row(&[(1, 1), (4, 512), (2, 0)])); // obj1 inuse

        let data = make_raw_xref_stream(&entries_data, w, 2);
        let result = parse_xref_stream(&data, 0).unwrap();

        assert_eq!(result.section_info.entry_count, 2);
        assert!(result.entries.iter().any(|(n, e)| {
            *n == 1
                && matches!(
                    e,
                    XrefEntry::InUse {
                        offset: 512,
                        generation: 0
                    }
                )
        }));
    }

    #[test]
    fn parse_xref_stream_flate_compressed_entries() {
        // FlateDecode 압축 xref 스트림, W=[1,4,2], 1개 엔트리
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        let raw_entry = make_entry_row(&[(1, 1), (4, 1024), (2, 0)]); // obj0 InUse offset=1024

        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&raw_entry).unwrap();
        let compressed = enc.finish().unwrap();

        let length = compressed.len();
        let dict = format!(
            "<< /Type /XRef /Size 1 /W [1 4 2] /Root 1 0 R /Filter /FlateDecode /Length {length} >>"
        );
        let mut data = Vec::new();
        data.extend_from_slice(format!("1 0 obj\n{dict}\nstream\n").as_bytes());
        data.extend_from_slice(&compressed);
        data.extend_from_slice(b"\nendstream\nendobj\n");

        let result = parse_xref_stream(&data, 0).unwrap();
        assert_eq!(result.section_info.entry_count, 1);
        assert_eq!(
            result.entries[0],
            (
                0,
                XrefEntry::InUse {
                    offset: 1024,
                    generation: 0
                }
            )
        );
    }

    // ── proptest: panic-freedom 검증 ─────────────────────────────────────────
    // pub(crate) 함수는 tests/ 크레이트 외부에서 접근 불가이므로 인라인 배치.

    use proptest::prelude::*;

    proptest! {
        /// 임의 바이트 입력에서 parse_xref_stream이 패닉을 일으키지 않는다.
        #[test]
        fn arbitrary_input_never_panics_parse_xref_stream(
            data in proptest::collection::vec(any::<u8>(), 0..4096)
        ) {
            let _ = parse_xref_stream(&data, 0);
        }

        /// 임의 바이트 입력에서 decompress_flate가 패닉을 일으키지 않는다.
        #[test]
        fn arbitrary_input_never_panics_decompress_flate(
            data in proptest::collection::vec(any::<u8>(), 0..4096)
        ) {
            let _ = decompress_flate(&data, 0);
        }

        /// 임의 데이터·W·Index 조합에서 decode_entries가 패닉을 일으키지 않는다.
        /// W[i]는 0–8 범위로 제한 (W 필드 너비 검증 통과 범위).
        #[test]
        fn arbitrary_input_never_panics_decode_entries(
            data in proptest::collection::vec(any::<u8>(), 0..4096),
            w0 in 0usize..=8usize,
            w1 in 0usize..=8usize,
            w2 in 0usize..=8usize,
            start_obj: u32,
            count in 0u32..1000u32,
        ) {
            let w = [w0, w1, w2];
            let index = vec![(start_obj, count)];
            let _ = decode_entries(&data, w, &index, 0);
        }

        /// 임의 데이터·columns·predictor 조합에서 unpredict_png가 패닉을 일으키지 않는다.
        #[test]
        fn arbitrary_input_never_panics_unpredict_png(
            data in proptest::collection::vec(any::<u8>(), 0..4096),
            columns in 1usize..1000usize,
            predictor in 10u8..=15u8,
        ) {
            let _ = unpredict_png(&data, columns, predictor, 0);
        }
    }
}
