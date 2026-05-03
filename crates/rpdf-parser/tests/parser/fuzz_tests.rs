use proptest::prelude::*;
use rpdf_parser::{
    find_eof, parse_header, parse_indirect_object, parse_object, parse_startxref, parse_trailer,
    parse_xref,
};

proptest! {
    /// 임의 바이트 입력에 대해 4개 파서 함수가 패닉을 일으키지 않는다.
    /// panic 이 발견되면 해당 케이스를 단위 테스트로 추가한 뒤 수정한다.
    #[test]
    fn arbitrary_input_never_panics(data in proptest::collection::vec(any::<u8>(), 0..65536)) {
        let eof = data.len().saturating_sub(6);
        let _ = find_eof(&data);
        let _ = parse_header(&data);
        let _ = parse_startxref(&data, eof);
        let _ = parse_trailer(&data, eof);
    }

    /// 임의 바이트 입력에 대해 parse_xref가 패닉을 일으키지 않는다.
    /// panic 이 발견되면 해당 케이스를 단위 테스트로 추가한 뒤 수정한다.
    #[test]
    fn arbitrary_input_never_panics_parse_xref(data in proptest::collection::vec(any::<u8>(), 0..65536)) {
        let _ = parse_xref(&data, 0);
    }

    /// 임의 바이트 입력에 대해 parse_object가 패닉을 일으키지 않는다.
    #[test]
    fn arbitrary_input_never_panics_parse_object(data in proptest::collection::vec(any::<u8>(), 0..65536)) {
        let _ = parse_object(&data, 0);
    }

    /// 임의 바이트 입력에 대해 parse_indirect_object가 패닉을 일으키지 않는다.
    #[test]
    fn arbitrary_input_never_panics_parse_indirect_object(data in proptest::collection::vec(any::<u8>(), 0..65536)) {
        let _ = parse_indirect_object(&data, 0);
    }

    /// 임의 오프셋과 데이터 조합에서 parse_object가 패닉을 일으키지 않는다.
    #[test]
    fn arbitrary_offset_never_panics(
        data in proptest::collection::vec(any::<u8>(), 1..65536),
        offset in 0usize..65536,
    ) {
        let safe_offset = offset % data.len();
        let _ = parse_object(&data, safe_offset);
    }
}
