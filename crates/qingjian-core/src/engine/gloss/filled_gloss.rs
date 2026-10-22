use crate::candidate::Translation;

/// 云端写好的一条释义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledGloss {
    /// 中文词。
    pub word: String,

    /// 学习语言的译词（保留返回的有效义项）。
    pub translation: Translation,
}
