use anyhow::Result;
use rpdf_core::types::DocumentMetadata;
use rpdf_parser::load_document;
use serde::Serialize;

/// `rpdf info` JSON 출력 구조.
#[derive(Serialize)]
struct InfoOutput {
    page_count: usize,
    metadata: Option<MetadataOutput>,
}

#[derive(Serialize)]
struct MetadataOutput {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    modification_date: Option<String>,
}

pub fn run(data: &[u8], json: bool) -> Result<()> {
    let doc = load_document(data)?;
    let output = InfoOutput {
        page_count: doc.page_count(),
        metadata: doc.metadata().map(meta_to_output),
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(())
}

fn meta_to_output(meta: &DocumentMetadata) -> MetadataOutput {
    MetadataOutput {
        title: decode_bytes(meta.title.as_deref()),
        author: decode_bytes(meta.author.as_deref()),
        subject: decode_bytes(meta.subject.as_deref()),
        creator: decode_bytes(meta.creator.as_deref()),
        producer: decode_bytes(meta.producer.as_deref()),
        creation_date: decode_bytes(meta.creation_date.as_deref()),
        modification_date: decode_bytes(meta.modification_date.as_deref()),
    }
}

/// PDF 문자열 바이트를 UTF-8로 변환 시도. 실패 시 lossy 변환.
fn decode_bytes(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(|b| String::from_utf8_lossy(b).into_owned())
}

fn print_human(output: &InfoOutput) {
    println!("Pages:    {}", output.page_count);
    if let Some(meta) = &output.metadata {
        print_field("Title", meta.title.as_deref());
        print_field("Author", meta.author.as_deref());
        print_field("Subject", meta.subject.as_deref());
        print_field("Creator", meta.creator.as_deref());
        print_field("Producer", meta.producer.as_deref());
        print_field("Created", meta.creation_date.as_deref());
        print_field("Modified", meta.modification_date.as_deref());
    }
}

fn print_field(label: &str, value: Option<&str>) {
    if let Some(v) = value {
        println!("{label:<10}{v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(title: Option<&str>, producer: Option<&str>) -> MetadataOutput {
        MetadataOutput {
            title: title.map(str::to_owned),
            author: None,
            subject: None,
            creator: None,
            producer: producer.map(str::to_owned),
            creation_date: None,
            modification_date: None,
        }
    }

    // CI-1: title 있을 때 출력 구조 확인
    #[test]
    fn ci1_info_output_with_title_serializes() {
        let output = InfoOutput {
            page_count: 3,
            metadata: Some(make_meta(Some("Test Doc"), Some("Adobe"))),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"page_count\":3"));
        assert!(json.contains("\"title\":\"Test Doc\""));
        assert!(json.contains("\"producer\":\"Adobe\""));
    }

    // CI-2: 모든 필드 None일 때
    #[test]
    fn ci2_info_output_all_none_serializes() {
        let output = InfoOutput {
            page_count: 1,
            metadata: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"metadata\":null"));
    }

    // CI-3: page_count 최상위 포함 확인
    #[test]
    fn ci3_page_count_at_top_level() {
        let output = InfoOutput {
            page_count: 5,
            metadata: Some(make_meta(None, None)),
        };
        let parsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();
        assert_eq!(parsed["page_count"], 5);
        assert!(parsed.get("metadata").is_some());
    }
}
