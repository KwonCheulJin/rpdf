//! PDF content stream 파서 — 연산자 시퀀스 토큰화 + 분류.
//!
//! **Checkpoint A**: 모듈 뼈대 (stub).
//! **Checkpoint B**: 토큰화 (피연산자 + 키워드 분리).
//! **Checkpoint C**: 연산자 분류 (keyword_to_operator).
//! **Checkpoint D-1**: 인라인 이미지 + q/Q 검증.
//!
//! **범위 외 (의미 해석 X)**:
//! - 폰트 매핑 / 텍스트 렌더링 (Task #8, v0.2)
//! - 좌표 변환 누적 (v0.2)
//! - 색상 모델 해석 (v0.2)
use crate::error::ParseError;
use rpdf_core::types::ContentStreamOperation;

/// `data` 전체를 content stream으로 파싱한다.
///
/// 피연산자는 `parse_object`로 파싱된다. 연산자 키워드는 ASCII 이름
/// 토큰으로 인식된 후 `ContentStreamOperator`로 분류된다.
///
/// q/Q 깊이 검증:
/// - `Q` 만났을 때 depth == 0 → `UnbalancedGraphicsState { offset, depth: -1 }` 즉시 에러
/// - 파싱 완료 후 depth > 0 → `UnbalancedGraphicsState { offset: data.len(), depth }` 에러
///
/// 알 수 없는 키워드는 `Unknown(bytes)` 변형으로 보존 (에러 아님).
///
/// ISO 32000-1 §7.8.2
pub fn parse_content_stream(_data: &[u8]) -> Result<Vec<ContentStreamOperation>, ParseError> {
    // Checkpoint A stub — Checkpoint B에서 구현
    Ok(vec![])
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn stub_returns_empty_for_nonempty_input() {
        let result = parse_content_stream(b"BT ET").unwrap();
        assert!(result.is_empty());
    }
}
