use rpdf_parser::load_document;
use std::path::PathBuf;

fn main() {
    let files = [
        "fw4-2024.pdf",
        "irs-f1040.pdf",
        "pdfjs-basicapi.pdf",
        "pdfjs-tracemonkey.pdf",
        "pdfjs-annotation-border.pdf",
    ];

    for name in &files {
        let path = PathBuf::from("examples").join(name);
        match std::fs::read(&path) {
            Ok(data) => match load_document(&data) {
                Ok(doc) => println!("{}: {} pages", name, doc.page_count()),
                Err(e) => println!("{}: ERROR — {}", name, e),
            },
            Err(e) => println!("{}: READ ERROR — {}", name, e),
        }
    }
}
