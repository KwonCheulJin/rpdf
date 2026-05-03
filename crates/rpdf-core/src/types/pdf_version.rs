/// PDF 버전을 나타내는 열거형.
///
/// PDF 스펙에 정의된 버전은 named variant로, 미지/미래 버전은 `Other`로 처리한다.
/// 알 수 없는 버전을 에러로 처리하지 않기 위해 `Other { major, minor }`를 둔다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfVersion {
    V1_0,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V2_0,
    /// 스펙에 정의되지 않은 버전.
    Other {
        major: u8,
        minor: u8,
    },
}

impl PdfVersion {
    pub fn from_bytes(major: u8, minor: u8) -> Self {
        match (major, minor) {
            (1, 0) => Self::V1_0,
            (1, 1) => Self::V1_1,
            (1, 2) => Self::V1_2,
            (1, 3) => Self::V1_3,
            (1, 4) => Self::V1_4,
            (1, 5) => Self::V1_5,
            (1, 6) => Self::V1_6,
            (1, 7) => Self::V1_7,
            (2, 0) => Self::V2_0,
            _ => Self::Other { major, minor },
        }
    }

    pub fn major(&self) -> u8 {
        match self {
            Self::V1_0
            | Self::V1_1
            | Self::V1_2
            | Self::V1_3
            | Self::V1_4
            | Self::V1_5
            | Self::V1_6
            | Self::V1_7 => 1,
            Self::V2_0 => 2,
            Self::Other { major, .. } => *major,
        }
    }

    pub fn minor(&self) -> u8 {
        match self {
            Self::V1_0 => 0,
            Self::V1_1 => 1,
            Self::V1_2 => 2,
            Self::V1_3 => 3,
            Self::V1_4 => 4,
            Self::V1_5 => 5,
            Self::V1_6 => 6,
            Self::V1_7 => 7,
            Self::V2_0 => 0,
            Self::Other { minor, .. } => *minor,
        }
    }
}

impl std::fmt::Display for PdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major(), self.minor())
    }
}
