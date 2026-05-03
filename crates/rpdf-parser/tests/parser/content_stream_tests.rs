use rpdf_parser::parse_content_stream;

#[test]
fn empty_input_returns_empty_vec() {
    let result = parse_content_stream(b"").unwrap();
    assert!(result.is_empty());
}
