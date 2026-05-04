use anyhow::{Result, bail};
use rpdf_parser::load_document;
use serde::Serialize;

/// `rpdf dump-pages` JSON 출력 구조.
#[derive(Serialize)]
struct DumpPagesOutput {
    page_count: usize,
    filtered_page: Option<usize>,
    pages: Vec<PageInfoOutput>,
}

#[derive(Serialize)]
struct PageInfoOutput {
    index: usize,
    media_box: Option<[f64; 4]>,
    crop_box: Option<[f64; 4]>,
    rotation: i32,
    op_count: usize,
}

pub fn run(data: &[u8], page: Option<usize>, json: bool) -> Result<()> {
    let doc = load_document(data)?;
    let total = doc.page_count();

    if let Some(p) = page
        && p >= total
    {
        bail!("page {p} not found (total: {total}, valid: 0..{total})");
    }

    let pages: Vec<PageInfoOutput> = doc
        .pages()
        .iter()
        .filter(|p_obj| page.is_none_or(|idx| p_obj.index == idx))
        .map(|p_obj| PageInfoOutput {
            index: p_obj.index,
            media_box: p_obj.media_box(),
            crop_box: p_obj.crop_box(),
            rotation: p_obj.rotation(),
            op_count: p_obj.content().len(),
        })
        .collect();

    let output = DumpPagesOutput {
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

fn print_human(output: &DumpPagesOutput) {
    for p in &output.pages {
        println!("Page {}:", p.index);
        match p.media_box {
            Some(b) => println!("  MediaBox: [{}, {}, {}, {}]", b[0], b[1], b[2], b[3]),
            None => println!("  MediaBox: none"),
        }
        match p.crop_box {
            Some(b) => println!("  CropBox:  [{}, {}, {}, {}]", b[0], b[1], b[2], b[3]),
            None => println!("  CropBox:  none"),
        }
        println!("  Rotation: {}", p.rotation);
        println!("  Ops:      {}", p.op_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(
        total: usize,
        pages: Vec<PageInfoOutput>,
        filtered: Option<usize>,
    ) -> DumpPagesOutput {
        DumpPagesOutput {
            page_count: total,
            filtered_page: filtered,
            pages,
        }
    }

    fn page(index: usize, op_count: usize) -> PageInfoOutput {
        PageInfoOutput {
            index,
            media_box: Some([0.0, 0.0, 612.0, 792.0]),
            crop_box: None,
            rotation: 0,
            op_count,
        }
    }

    // CD-1: JSON 직렬화 — page_count + filtered_page + pages 배열
    #[test]
    fn cd1_serializes_page_count_and_filtered() {
        let output = make_output(5, vec![page(0, 100)], Some(0));
        let json = serde_json::to_string(&output).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["page_count"], 5);
        assert_eq!(v["filtered_page"], 0);
        assert_eq!(v["pages"].as_array().unwrap().len(), 1);
    }

    // CD-2: filtered_page 없을 때 null
    #[test]
    fn cd2_filtered_page_null_when_not_set() {
        let output = make_output(3, vec![page(0, 10), page(1, 20), page(2, 30)], None);
        let json = serde_json::to_string(&output).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["filtered_page"].is_null());
        assert_eq!(v["pages"].as_array().unwrap().len(), 3);
    }

    // CD-3: op_count 필드 존재 확인
    #[test]
    fn cd3_op_count_in_page_output() {
        let output = make_output(1, vec![page(0, 247)], None);
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();
        assert_eq!(v["pages"][0]["op_count"], 247);
    }
}
