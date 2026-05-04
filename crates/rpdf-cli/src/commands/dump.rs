use anyhow::{Result, bail};
use rpdf_core::types::{ContentStreamOperation, PdfObject};
use rpdf_parser::load_document;
use serde::Serialize;
use serde_json::Value as JsonValue;

/// `rpdf dump` JSON 출력 구조.
#[derive(Serialize)]
struct DumpOutput {
    page_count: usize,
    filtered_page: Option<usize>,
    pages: Vec<PageDumpOutput>,
}

#[derive(Serialize)]
struct PageDumpOutput {
    index: usize,
    op_count: usize,
    ops: Vec<OpOutput>,
}

#[derive(Serialize)]
struct OpOutput {
    op: String,
    operands: Vec<JsonValue>,
}

pub fn run(data: &[u8], page: Option<usize>, json: bool) -> Result<()> {
    let doc = load_document(data)?;
    let total = doc.page_count();

    if let Some(p) = page
        && p >= total
    {
        bail!("page {p} not found (total: {total}, valid: 0..{total})");
    }

    let pages: Vec<PageDumpOutput> = doc
        .pages()
        .iter()
        .filter(|p_obj| page.is_none_or(|idx| p_obj.index == idx))
        .map(|p_obj| {
            let ops: Vec<OpOutput> = p_obj.content().iter().map(op_to_output).collect();
            PageDumpOutput {
                index: p_obj.index,
                op_count: ops.len(),
                ops,
            }
        })
        .collect();

    let output = DumpOutput {
        page_count: total,
        filtered_page: page,
        pages,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(())
}

fn op_to_output(op: &ContentStreamOperation) -> OpOutput {
    OpOutput {
        op: op.operator.display_name(),
        operands: op.operands.iter().map(operand_to_json_value).collect(),
    }
}

/// PdfObject를 JSON value로 변환.
///
/// v0.1 한정:
/// - Boolean, Integer, Real, Name, String: 해당 JSON 타입
/// - Array: 재귀 처리
/// - Dict, Stream, Reference: `"<complex>"` 문자열로 대체
///
/// 더 정밀한 객체 덤프는 v0.2 이후 별도 명령(rpdf inspect 등)으로 분리 예정.
fn operand_to_json_value(obj: &PdfObject) -> JsonValue {
    match obj {
        PdfObject::Boolean(b) => JsonValue::Bool(*b),
        PdfObject::Integer(n) => JsonValue::Number((*n).into()),
        PdfObject::Real(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::String("<nan>".into())),
        PdfObject::Name(bytes) => JsonValue::String(format!("/{}", String::from_utf8_lossy(bytes))),
        PdfObject::LiteralString(bytes) | PdfObject::HexString(bytes) => {
            if let Ok(s) = std::str::from_utf8(bytes) {
                JsonValue::String(s.to_owned())
            } else {
                JsonValue::String(format!("<binary:{}>", bytes.len()))
            }
        }
        PdfObject::Array(arr) => JsonValue::Array(arr.iter().map(operand_to_json_value).collect()),
        PdfObject::Dictionary(_) | PdfObject::Stream(_) | PdfObject::Reference(_) => {
            JsonValue::String("<complex>".into())
        }
        PdfObject::Null => JsonValue::Null,
    }
}

fn print_human(output: &DumpOutput) {
    for page in &output.pages {
        println!("=== Page {} ({} ops) ===", page.index, page.op_count);
        for op in &page.ops {
            println!("  {}", format_op_line(op));
        }
    }
}

fn format_op_line(op: &OpOutput) -> String {
    if op.operands.is_empty() {
        op.op.clone()
    } else {
        let operands: Vec<String> = op.operands.iter().map(format_operand).collect();
        format!("{} {}", operands.join(" "), op.op)
    }
}

fn format_operand(val: &JsonValue) -> String {
    match val {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        JsonValue::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_operand).collect();
            format!("[{}]", items.join(" "))
        }
        JsonValue::Object(_) => "<complex>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CE-1: operand_to_json_value — Integer, Real, Name, String
    #[test]
    fn ce1_scalar_types() {
        assert_eq!(
            operand_to_json_value(&PdfObject::Integer(42)),
            JsonValue::Number(42.into())
        );
        assert_eq!(
            operand_to_json_value(&PdfObject::Real(1.5)),
            JsonValue::Number(serde_json::Number::from_f64(1.5).unwrap())
        );
        assert_eq!(
            operand_to_json_value(&PdfObject::Name(b"F1".to_vec())),
            JsonValue::String("/F1".into())
        );
        assert_eq!(
            operand_to_json_value(&PdfObject::LiteralString(b"Hello".to_vec())),
            JsonValue::String("Hello".into())
        );
        assert_eq!(
            operand_to_json_value(&PdfObject::HexString(b"Hello".to_vec())),
            JsonValue::String("Hello".into())
        );
    }

    // CE-2: operand_to_json_value — Array 재귀
    #[test]
    fn ce2_array_recursive() {
        let arr = PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(2)]);
        let result = operand_to_json_value(&arr);
        assert_eq!(
            result,
            JsonValue::Array(vec![
                JsonValue::Number(1.into()),
                JsonValue::Number(2.into()),
            ])
        );
    }

    // CE-3: operand_to_json_value — Dict/Stream/Reference → "<complex>"
    #[test]
    fn ce3_complex_types_become_placeholder() {
        use rpdf_core::types::ObjectId;
        assert_eq!(
            operand_to_json_value(&PdfObject::Dictionary(Default::default())),
            JsonValue::String("<complex>".into())
        );
        assert_eq!(
            operand_to_json_value(&PdfObject::Reference(ObjectId::new(1, 0))),
            JsonValue::String("<complex>".into())
        );
    }

    // CE-4: format_op_line — 피연산자 + 연산자 순 출력
    #[test]
    fn ce4_format_op_line_with_operands() {
        let op = OpOutput {
            op: "Tj".into(),
            operands: vec![JsonValue::String("Hello".into())],
        };
        assert_eq!(format_op_line(&op), "Hello Tj");
    }

    // CE-5: format_op_line — 피연산자 없을 때 연산자만
    #[test]
    fn ce5_format_op_line_no_operands() {
        let op = OpOutput {
            op: "BT".into(),
            operands: vec![],
        };
        assert_eq!(format_op_line(&op), "BT");
    }
}
