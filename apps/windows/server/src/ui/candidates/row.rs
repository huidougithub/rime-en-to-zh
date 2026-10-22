//! 候选窗口的一行：[`Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），与 macOS 端 `candidates/row.rs` 一致。
//! GDI 画法也用同一个类型。

use qingjian_core::{Candidate, CandidateKind};
use qingjian_render::{Row, Tone};

/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 打开且候选带码时，码用方括号括起来紧跟在候选词后面（`鹤[rbm]`），不进 annotation。
/// `show_pos` 是 `[general] translation_pos`：关掉时译词保留、词性段（v. / n. / adj.）不画。
pub(crate) fn from_candidate(
    position: usize,
    candidate: &Candidate,
    show_code: bool,
    show_pos: bool,
) -> Row {
    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
    if let Some(reading) = &candidate.reading {
        annotation.push((reading.clone(), Tone::Gloss));
    }
    if let Some(translation) = &candidate.translation {
        for (i, sense) in translation.senses().iter().enumerate() {
            if i > 0 || !annotation.is_empty() {
                annotation.push((" · ".to_owned(), Tone::Separator));
            }
            if show_pos && let Some(pos) = sense.part_of_speech {
                annotation.push((format!("{pos} "), Tone::Pos));
            }
            let tone = if sense.fresh {
                Tone::Fresh
            } else {
                Tone::Gloss
            };
            for segment in sense.furigana() {
                annotation.push((segment.text, tone));
                if let Some(reading) = segment.reading {
                    annotation.push((format!("({reading})"), Tone::Faint));
                }
            }
        }
    }
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        code,
        annotation,
        cloud: candidate.kind == CandidateKind::Cloud,
    }
}

#[cfg(test)]
mod tests {
    use qingjian_core::{Language, PartOfSpeech, Sense, Translation};

    use super::*;

    fn candidate_with_gloss() -> Candidate {
        Candidate {
            text: "开发".to_owned(),
            kind: CandidateKind::Chinese,
            syllables: vec!["kai".to_owned(), "fa".to_owned()],
            reading: None,
            translation: Some(Translation::new(
                Language::English,
                vec![Sense {
                    part_of_speech: Some(PartOfSpeech::Verb),
                    text: "develop".to_owned(),
                    reading: None,
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    #[test]
    fn part_of_speech_follows_the_frame_switch() {
        let on = from_candidate(0, &candidate_with_gloss(), false, true);
        let joined: String = on
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");

        // 关掉后词性段消失，译词与分隔结构原样保留
        let off = from_candidate(0, &candidate_with_gloss(), false, false);
        let joined: String = off
            .annotation
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(!joined.contains("v. "), "{joined}");
        assert!(joined.contains("develop"), "{joined}");
        assert_eq!(on.annotation.len() - 1, off.annotation.len());
    }

    #[test]
    fn a_gloss_without_part_of_speech_is_unchanged_by_the_switch() {
        let mut candidate = candidate_with_gloss();
        candidate.translation.as_mut().unwrap().senses_mut()[0].part_of_speech = None;
        for show_pos in [true, false] {
            let row = from_candidate(0, &candidate, false, show_pos);
            let joined: String = row
                .annotation
                .iter()
                .map(|(text, _)| text.as_str())
                .collect();
            assert_eq!(joined, "develop");
        }
    }

    #[test]
    fn translated_real_senses_show_source_once_and_follow_the_pos_switch() {
        use qingjian_core::Engine;
        use qingjian_dictionary::Dictionary;
        use qingjian_translate::Glossary;

        let glossary = Glossary::parse(
            Language::Chinese,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../assets/glossary/glossary-zh.tsv"
            )),
        )
        .unwrap();
        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap())
            .with_english_translator(Box::new(glossary));
        engine.set_english_mode(true);
        engine.set_english_to_chinese(true);
        for (input, expected) in [
            ("tear", vec![("撕", "v."), ("眼泪", "n.")]),
            ("may", vec![("可能", "v."), ("可以", "v."), ("五月", "n.")]),
            ("bank", vec![("银行", "n."), ("河岸", "n.")]),
            ("\"look up\"", vec![("查阅", "v."), ("好转", "v.")]),
            ("\"ice cream\"", vec![("冰淇淋", "n.")]),
        ] {
            engine.set_input(input);
            let mut list = engine.query().unwrap().candidates;
            engine.annotate(&mut list);
            assert_eq!(list.items.len(), expected.len() + 1, "{input}");
            for (candidate, (text, pos)) in list.items.iter().zip(expected) {
                let wire = serde_json::to_vec(candidate).unwrap();
                let candidate: Candidate = serde_json::from_slice(&wire).unwrap();
                for show_pos in [true, false] {
                    let row = from_candidate(0, &candidate, false, show_pos);
                    assert_eq!(row.text, text);
                    let annotation: String =
                        row.annotation.iter().map(|(s, _)| s.as_str()).collect();
                    let source = input.trim_matches('"');
                    let expected = if show_pos {
                        format!("{pos} {source}")
                    } else {
                        source.to_owned()
                    };
                    assert_eq!(annotation, expected, "{input}: {show_pos}");
                }
            }
        }
    }
}
