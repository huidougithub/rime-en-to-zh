use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 文件头魔数。
pub const MAGIC: [u8; 8] = *b"QINGJIAN";

/// 基础格式版本；一义、两义释义表和其他数据仍沿用此版本。
pub const FORMAT_VERSION: u16 = 1;

/// 多义释义表的语义版本：布局不变，旧读器必须拒绝，避免再截断为两义。
pub const GLOSSARY_MULTISENSE_VERSION: u16 = 2;

/// 文件头，32 字节，紧跟着 `section_count` 条 [`crate::SectionEntry`]。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Header {
    /// [`MAGIC`]。
    pub magic: [u8; 8],

    /// [`FORMAT_VERSION`]，或多义释义表的 [`GLOSSARY_MULTISENSE_VERSION`]。
    pub version: u16,

    /// 数据种类，见 [`crate::Kind`]。
    pub kind: u16,

    /// 分节数。
    pub section_count: u32,

    /// 留给以后（校验和、标志位），现在全零。
    pub reserved: [u8; 16],
}

impl Header {
    pub const SIZE: usize = size_of::<Self>();
}
