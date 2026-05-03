//! rpdf-core: Rust PDF 편집기 코어 라이브러리

pub mod types;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!crate::version().is_empty());
    }
}
