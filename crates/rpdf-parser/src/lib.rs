//! rpdf-parser: PDF 파일 파싱 로직

mod eof;
mod error;
mod header;
mod object_stream;
mod objects;
mod startxref;
mod trailer;
mod xref;
mod xref_stream;

pub use eof::find_eof;
pub use error::ParseError;
pub use header::{PdfHeader, parse_header};
pub use object_stream::ParsedObjectStream;
pub use objects::{parse_indirect_object, parse_object};
pub use startxref::parse_startxref;
pub use trailer::{ParsedTrailer, PdfTrailer, parse_trailer};
pub use xref::{ParsedXref, XrefSectionInfo, parse_xref};
