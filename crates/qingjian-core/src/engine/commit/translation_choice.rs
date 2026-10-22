//! 英译中选择的待确认记录；只有平台确认文档写入成功后才进入既有选择表。

/// 一次已生成上屏文本、尚未确认写入的中文译词选择。字段不对外开放，避免壳构造义项。
#[derive(Debug)]
pub struct TranslationChoice {
    pub(super) key: String,

    pub(super) text: String,
}

impl TranslationChoice {
    /// 模式前缀隔离拼音；大小写和完整短语共用选择键，不删除限定词。
    pub(in crate::engine) fn key(scope: &str) -> Option<String> {
        let input = if scope.starts_with('"') {
            scope.strip_prefix('"')?.strip_suffix('"')?
        } else {
            scope
        };
        (!input.is_empty() && !input.chars().any(char::is_control))
            .then(|| format!("en-zh:{}", input.to_ascii_lowercase()))
    }
}
