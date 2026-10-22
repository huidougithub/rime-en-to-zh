//! 中文主候选复用原生上屏，选词偏好复用 Learner 的隔离输入键。

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::input_log::InputSource;
use crate::engine::{Engine, Learner, TranslationChoice};

impl Engine {
    pub(super) fn commit_english_translation(&mut self, candidate: &Candidate) -> Option<String> {
        if candidate.kind != CandidateKind::Translation {
            return None;
        }
        // 无效或已过期的译词不能落到普通候选提交路径。
        let Some(key) = self.translation_choice_key(candidate) else {
            return Some(String::new());
        };
        let canonical = self
            .traditional
            .restore(&candidate.text)
            .unwrap_or_else(|| candidate.text.clone());
        let selected = Candidate {
            text: candidate.text.clone(),
            kind: CandidateKind::Translation,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        };
        let lead = self.auto_space_before(&selected.text);
        let text = self.commit_with(&selected, InputSource::Translation, None);
        let text = if lead { format!(" {text}") } else { text };
        let text = self.take_held() + &text;
        if !text.is_empty() && !self.learner.muted() {
            self.translation_choice = Some(TranslationChoice {
                key,
                text: canonical,
            });
        }
        Some(text)
    }

    fn translation_choice_key(&self, candidate: &Candidate) -> Option<String> {
        if !self.english_translation_mode() {
            return None;
        }
        let annotation = candidate.translation.as_ref()?;
        let source = &annotation.senses().first()?.text;
        let key = TranslationChoice::key(source)?;
        if TranslationChoice::key(self.composition.scope()).as_ref() != Some(&key) {
            return None;
        }
        let canonical = self
            .traditional
            .restore(&candidate.text)
            .unwrap_or_else(|| candidate.text.clone());
        self.english_translator
            .translate(&source.to_ascii_lowercase())?
            .senses()
            .iter()
            .any(|sense| sense.text == canonical)
            .then_some(key)
    }

    /// 取走本次中文译词选择。壳须在真实写入成功后交回，丢弃即不学习。
    pub fn take_translation_choice(&mut self) -> Option<TranslationChoice> {
        self.translation_choice.take()
    }

    /// 平台确认写入成功；只记隔离键，不增加全局词频、英文词或拼音用户词。
    /// 返回是否接受了记录；落盘仍通过现有 Learner::flush。
    pub fn confirm_translation_choice(&mut self, choice: TranslationChoice) -> bool {
        if self.learner.muted() {
            return false;
        }
        let Some(source) = choice.key.strip_prefix("en-zh:") else {
            return false;
        };
        if !self
            .english_translator
            .translate(source)
            .is_some_and(|translation| {
                translation
                    .senses()
                    .iter()
                    .any(|sense| sense.text == choice.text)
            })
        {
            return false;
        }
        self.learner.record_choice(&choice.key, "", &choice.text);
        true
    }
}
