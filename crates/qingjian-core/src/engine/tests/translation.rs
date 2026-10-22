//! 英译中模式的精确匹配、原文退路与原生译文提交回归。

use crate::{CandidateKind, Engine, Language, PartOfSpeech, Sense, Translation, Translator};
use qingjian_dictionary::WordList;

use super::engine;

struct TestGlossary;

impl Translator for TestGlossary {
    fn language(&self) -> Language {
        Language::Chinese
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        if matches!(text, "many" | "manydup") {
            let count = if text == "manydup" { 130 } else { 129 };
            return Some(Translation::new(
                Language::Chinese,
                (0..count)
                    .map(|i| Sense {
                        part_of_speech: Some(if i % 2 == 0 {
                            PartOfSpeech::Noun
                        } else {
                            PartOfSpeech::Verb
                        }),
                        text: format!("义项{}", i % 129),
                        reading: None,
                        fresh: false,
                    })
                    .collect(),
            ));
        }
        let senses: &[(PartOfSpeech, &str)] = match text {
            "studio" => &[(PartOfSpeech::Noun, "工作室")],
            "thwart" => &[(PartOfSpeech::Verb, "挫败")],
            "set" => &[(PartOfSpeech::Verb, "设置"), (PartOfSpeech::Verb, "放置")],
            "set a record" => &[(PartOfSpeech::Verb, "创造纪录")],
            "good morning" => &[(PartOfSpeech::Interjection, "早上好")],
            "tear" => &[(PartOfSpeech::Verb, "撕"), (PartOfSpeech::Noun, "眼泪")],
            "duplicate" => &[(PartOfSpeech::Verb, "重复"), (PartOfSpeech::Noun, "重复")],
            _ => return None,
        };
        Some(Translation::new(
            Language::Chinese,
            senses
                .iter()
                .map(|(pos, text)| Sense {
                    part_of_speech: Some(*pos),
                    text: (*text).to_owned(),
                    reading: None,
                    fresh: false,
                })
                .collect(),
        ))
    }
}

pub(super) fn translating() -> Engine {
    let mut engine = engine().with_english_translator(Box::new(TestGlossary));
    engine.set_english_mode(true);
    engine.set_english_to_chinese(true);
    engine
}

#[test]
fn exact_senses_are_primary_and_each_commits_through_translation() {
    let mut engine = translating();
    engine.set_input("set");
    let mut list = engine.query().unwrap().candidates;
    assert_eq!(
        list.items
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        ["设置", "放置", "set"]
    );
    assert_eq!(list.items[1].kind, CandidateKind::Translation);
    engine.annotate(&mut list);
    assert_eq!(
        list.items[1].translation.as_ref().unwrap().senses()[0].text,
        "set"
    );
    assert!(list.items[2].translation.is_none());
    assert_eq!(engine.commit(&list.items[1]), "放置");
    assert!(engine.composition().is_empty());
}

#[test]
fn unknown_prefixes_and_typos_have_only_the_literal_candidate() {
    let mut engine = translating()
        .with_english(WordList::parse("studio\tstudio\t100\nstudious\tstudious\t50\n").unwrap());
    for input in ["studi", "studoi", "zqxjkvv", "a record", "\"good morning"] {
        engine.set_input(input);
        let mut list = engine.query().unwrap().candidates;
        engine.annotate(&mut list);
        assert_eq!(list.items.len(), 1, "{input}");
        assert_eq!(list.items[0].text, input);
        assert_eq!(list.items[0].kind, CandidateKind::English);
        assert!(list.items[0].translation.is_none());
        assert_eq!(engine.commit(&list.items[0]), input);
    }
}

#[test]
fn phrase_qualifiers_are_not_removed_and_case_is_preserved_in_the_source() {
    let mut engine = translating();
    for (input, expected, source) in [
        ("STUDIO", "工作室", "STUDIO"),
        ("Thwart", "挫败", "Thwart"),
        ("\"set a record\"", "创造纪录", "set a record"),
        ("\"Good Morning\"", "早上好", "Good Morning"),
        ("good morning", "早上好", "good morning"),
    ] {
        engine.set_input(input);
        let list = engine.query().unwrap().candidates;
        assert_eq!(list.items[0].text, expected);
        assert!(list.items[0].reading.is_none());
        assert_eq!(
            list.items[0].translation.as_ref().unwrap().senses()[0].text,
            source
        );
        assert_eq!(engine.commit(&list.items[0]), expected);
    }
    engine.set_input("set");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| !c.text.contains("创造"))
    );
}

#[test]
fn original_text_and_missing_glossary_remain_available() {
    let mut engine = translating();
    engine.set_input("STUDIO");
    assert_eq!(engine.take_raw(), "STUDIO");
    let mut missing = super::engine();
    missing.set_english_to_chinese(true);
    missing.set_english_mode(true);
    missing.set_input("studio");
    let list = missing.query().unwrap().candidates;
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].text, "studio");
    missing.clear();
    assert!(missing.query().unwrap().candidates.items.is_empty());
}

#[test]
fn pinyin_and_default_english_mode_keep_their_existing_candidates() {
    let mut engine = engine()
        .with_english(WordList::parse("studio\tstudio\t100\nstudious\tstudious\t50\n").unwrap());
    assert!(!engine.english_translation_mode());
    engine.set_input("kaifa");
    let before = engine.query().unwrap().candidates;
    engine.set_english_to_chinese(true);
    assert_eq!(engine.query().unwrap().candidates, before);
    engine.set_english_to_chinese(false);
    engine.set_english_mode(true);
    engine.set_input("studi");
    let list = engine.query().unwrap().candidates;
    assert_eq!(list.items[0].kind, CandidateKind::English);
    assert_eq!(list.items[0].text, "studio");
}

#[test]
fn translation_metadata_preserves_each_sense_and_annotation_is_idempotent() {
    let mut engine = translating();
    engine.set_input("Tear");
    let mut list = engine.query().unwrap().candidates;
    for (candidate, pos) in list
        .items
        .iter()
        .zip([PartOfSpeech::Verb, PartOfSpeech::Noun])
    {
        assert!(candidate.reading.is_none());
        let annotation = candidate.translation.as_ref().unwrap();
        assert_eq!(annotation.language, Language::English);
        assert_eq!(annotation.senses().len(), 1);
        assert_eq!(annotation.senses()[0].text, "Tear");
        assert_eq!(annotation.senses()[0].part_of_speech, Some(pos));
    }
    let before = list.clone();
    engine.annotate(&mut list);
    engine.annotate(&mut list);
    assert_eq!(list, before);
    assert_eq!(engine.commit(&list.items[1]), "眼泪");
}

#[test]
fn duplicate_translation_text_keeps_one_candidate_and_the_first_sense_metadata() {
    let mut engine = translating();
    engine.set_input("duplicate");
    let list = engine.query().unwrap().candidates;
    assert_eq!(list.items.len(), 2);
    assert_eq!(list.items[0].text, "重复");
    let annotation = list.items[0].translation.as_ref().unwrap();
    assert_eq!(
        annotation.senses()[0].part_of_speech,
        Some(PartOfSpeech::Verb)
    );
    assert_eq!(list.items[1].text, "duplicate");
    assert!(list.items[1].translation.is_none());
}

#[test]
fn many_senses_are_independent_candidates_in_source_order_with_stable_deduplication() {
    let mut engine = translating();
    for input in ["many", "manydup"] {
        engine.set_input(input);
        let mut list = engine.query().unwrap().candidates;
        engine.annotate(&mut list);
        assert_eq!(list.items.len(), 130);
        for (i, candidate) in list.items[..129].iter().enumerate() {
            assert_eq!(candidate.text, format!("义项{i}"));
            assert_eq!(candidate.kind, CandidateKind::Translation);
            assert!(candidate.reading.is_none());
            let sense = &candidate.translation.as_ref().unwrap().senses()[0];
            assert_eq!(sense.text, input);
            assert_eq!(
                sense.part_of_speech,
                Some(if i % 2 == 0 {
                    PartOfSpeech::Noun
                } else {
                    PartOfSpeech::Verb
                })
            );
        }
        assert_eq!(list.items[129].text, input);
        assert_eq!(engine.commit(&list.items[128]), "义项128");
    }
}
