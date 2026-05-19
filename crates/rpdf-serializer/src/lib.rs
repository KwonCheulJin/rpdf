//! rpdf-serializer: Document IR → PDF 바이트 직렬화 크레이트.
//!
//! lopdf를 백엔드로 사용해 편집된 Document를 유효한 PDF로 저장한다.
//! `rpdf-core`, `rpdf-parser`, `rpdf-edit`의 변경 없이 직렬화 관심사만 담당한다.

mod error;
mod serialize;
mod types;

pub use error::SerializeError;
pub use serialize::{load_document_tracked, serialize_document};
pub use types::PageSource;
