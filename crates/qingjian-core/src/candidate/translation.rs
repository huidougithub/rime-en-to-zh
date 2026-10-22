use serde::{Deserialize, Serialize};

use super::language::Language;
use super::sense::Sense;

/// 候选词在单一学习语言下的翻译。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Translation {
    /// 译文所属的学习语言。
    pub language: Language,

    /// 释义，保留来源顺序和全部义项；外部数据容量由读写层校验。
    senses: Vec<Sense>,
}

impl Translation {
    pub fn new(language: Language, senses: Vec<Sense>) -> Self {
        Self { language, senses }
    }

    pub fn senses(&self) -> &[Sense] {
        &self.senses
    }

    pub fn senses_mut(&mut self) -> &mut [Sense] {
        &mut self.senses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::PartOfSpeech;

    #[test]
    fn keeps_all_senses_in_order() {
        for count in [0, 1, 2, 3, 129] {
            let senses: Vec<Sense> = (0..count)
                .map(|i| Sense {
                    part_of_speech: Some(if i % 2 == 0 {
                        PartOfSpeech::Noun
                    } else {
                        PartOfSpeech::Verb
                    }),
                    text: format!("meaning-{i}"),
                    reading: Some(format!("reading-{i}")),
                    fresh: i % 2 == 0,
                })
                .collect();
            let translation = Translation::new(Language::English, senses.clone());
            assert_eq!(translation.senses(), senses, "count={count}");
        }
    }
}
