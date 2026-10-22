//! 可选英译中输入：只查当前英文的完整键，不把补全或纠错词的释义冒充当前输入。

use crate::candidate::{Candidate, CandidateKind, Language, Sense, Translation};
use crate::engine::{Engine, Learner, TranslationChoice};
use std::cmp::Reverse;
use std::collections::HashSet;

impl Engine {
    pub(in crate::engine) fn query_english_translations(&self, scope: &str) -> Vec<Candidate> {
        if scope.is_empty() {
            return Vec::new();
        }
        // 引号给短语内的空格让路；引号未闭合时保留原文，不抢先翻译第一个词。
        let input = if scope.starts_with('"') {
            scope.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
        } else {
            Some(scope)
        };
        let mut items = Vec::new();
        if let Some(input) = input.filter(|s| !s.is_empty())
            && let Some(translation) = self
                .english_translator
                .translate(&input.to_ascii_lowercase())
        {
            let mut seen = HashSet::new();
            for sense in translation.senses() {
                if !sense.text.is_empty() && seen.insert(sense.text.as_str()) {
                    items.push(Candidate {
                        text: sense.text.clone(),
                        kind: CandidateKind::Translation,
                        syllables: Vec::new(),
                        reading: None,
                        translation: Some(Translation::new(
                            Language::English,
                            vec![Sense {
                                part_of_speech: sense.part_of_speech,
                                text: input.to_owned(),
                                reading: None,
                                fresh: false,
                            }],
                        )),
                        aux_code: None,
                    });
                }
            }
        }
        // 查不到时也始终有清晰的原文路径，Enter 仍交给既有 take_raw。
        if let Some(key) = TranslationChoice::key(scope) {
            items.sort_by_key(|candidate| {
                Reverse(self.learner.choice_weight(&key, &candidate.text))
            });
        }
        items.push(Candidate {
            text: scope.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        });
        items
    }
}
