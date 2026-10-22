//! 真实英汉表的语义样例，以及 Windows 按键到上屏的闭环回归。

use qingjian_core::{CandidateKind, Engine, PartOfSpeech, Translator};
use qingjian_translate::Glossary;

use crate::support::{
    AssemblySpec, ENGLISH, KeyEvent, KeyModifiers, KeyOutcome, Language, PathBuf, Router,
    RouterConfig, SESSION, assembly, candidate_texts, function_key, open_session, preedit, press,
    type_english, type_letters,
};

fn engine() -> Engine {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    assembly::assemble(&AssemblySpec {
        english_glossary: Some(root.join("assets/glossary/glossary-zh.tsv")),
        english: Some(root.join("assets/sample/english.tsv")),
        glossary: Some((
            Language::English,
            root.join("assets/sample/glossary-en.tsv"),
        )),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .unwrap()
}

fn router() -> Router {
    let mut router = Router::new(
        engine(),
        RouterConfig {
            english_to_chinese: true,
            ..RouterConfig::default()
        },
    );
    open_session(&mut router, SESSION, Some("notepad.exe".to_owned()));
    router
}

#[test]
fn translation_fixed_semantic_cases_use_the_real_glossary() {
    let mut engine = engine();
    engine.set_english_mode(true);
    engine.set_english_to_chinese(true);
    let fixture = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tests/english_to_chinese/cases.tsv"
    ));
    for line in fixture
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let (input, expected, forbidden) = (fields[0], fields[1], fields[2]);
        engine.set_input(input);
        let mut list = engine.query().unwrap().candidates;
        engine.annotate(&mut list);
        let actual: Vec<_> = list
            .items
            .iter()
            .filter(|c| c.kind == CandidateKind::Translation)
            .map(|c| c.text.as_str())
            .collect();
        let expected: Vec<_> = expected.split('|').filter(|s| !s.is_empty()).collect();
        assert_eq!(actual, expected, "{input}: {}", fields[3]);
        for bad in forbidden.split('|').filter(|s| !s.is_empty()) {
            assert!(
                actual.iter().all(|text| !text.contains(bad)),
                "{input}: {bad}"
            );
        }
        assert_eq!(list.items.last().unwrap().text, input);
        assert!(list.items.last().unwrap().translation.is_none());
        let first = list.items[0].clone();
        assert_eq!(engine.commit(&first), first.text, "{input}");
        assert!(engine.composition().is_empty(), "{input}");
    }
}

#[test]
fn translation_space_commits_chinese_without_trailing_space_and_enter_keeps_raw() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "studio");
    assert_eq!(candidate_texts(&frame), ["工作室", "studio"]);
    let wire = serde_json::to_vec(&frame).unwrap();
    let decoded: qingjian_platform::protocol::Frame = serde_json::from_slice(&wire).unwrap();
    assert_eq!(decoded.candidates.items[0].kind, CandidateKind::Translation);
    assert_eq!(decoded.candidates.items[0].text, "工作室");
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("工作室"))
    );
    assert!(after.is_empty());
    type_english(&mut router, "STUDIO");
    let (_, commit, after) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("STUDIO"));
    assert!(after.is_empty());
    let (_, _, frame) = type_english(&mut router, "bank");
    assert_eq!(candidate_texts(&frame), ["银行", "河岸", "bank"]);
    press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("河岸"));
}

#[test]
fn translation_quoted_phrase_keeps_internal_spaces_and_the_qualifier() {
    let mut router = router();
    let shifted = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    for input in ["\"set a record\"", "\"good morning\""] {
        for c in input.chars() {
            let key = match c {
                '"' => KeyEvent::new(0xDE, Some(c), shifted),
                ' ' => KeyEvent::new(0x20, Some(c), ENGLISH),
                _ => KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), ENGLISH),
            };
            let (_, commit, _) = press(&mut router, key);
            assert_eq!(commit, None, "phrase must stay in composition: {input}");
        }
        let expected = if input.contains("record") {
            "创造纪录"
        } else {
            "早上好"
        };
        let (_, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
        assert_eq!(commit.as_deref(), Some(expected));
        assert!(after.is_empty());
    }
}

#[test]
fn translation_unknown_stays_literal_and_chinese_pinyin_still_works() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "bankk");
    assert_eq!(preedit(&frame), "bankk");
    assert_eq!(candidate_texts(&frame), ["bankk"]);
    assert!(frame.candidates.items[0].translation.is_none());
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("bankk"));
    type_english(&mut router, "bankk");
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0x31, Some('1'), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "bankk1");
    assert_eq!(candidate_texts(&frame), ["bankk1"]);
    let (_, commit, after) = press(&mut router, function_key(0x0D));
    assert_eq!(commit.as_deref(), Some("bankk1"));
    assert!(after.is_empty());
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert!(candidate_texts(&frame).contains(&"你好"));
}

#[test]
fn real_english_glossary_round_trip_keeps_all_senses_and_corrected_pos() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets/glossary/glossary-zh.tsv"
    ));
    let glossary = Glossary::parse(Language::Chinese, source).unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "qingjian-english-senses-{}-{nonce}.qj",
        std::process::id()
    ));
    glossary.write_qj(&path, &Default::default()).unwrap();
    let mapped = Glossary::from_path(Language::Chinese, &path).unwrap();
    assert_eq!(mapped.len(), glossary.len());
    for line in source
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let word = line.split('\t').next().unwrap();
        assert_eq!(mapped.translate(word), glossary.translate(word), "{word}");
    }
    for (word, texts, positions) in [
        (
            "tear",
            vec!["撕", "眼泪"],
            vec![PartOfSpeech::Verb, PartOfSpeech::Noun],
        ),
        (
            "may",
            vec!["可能", "可以", "五月"],
            vec![PartOfSpeech::Verb, PartOfSpeech::Verb, PartOfSpeech::Noun],
        ),
        (
            "look up",
            vec!["查阅", "好转"],
            vec![PartOfSpeech::Verb, PartOfSpeech::Verb],
        ),
        ("ice cream", vec!["冰淇淋"], vec![PartOfSpeech::Noun]),
    ] {
        let translation = mapped.translate(word).unwrap();
        assert_eq!(
            translation
                .senses()
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            texts
        );
        assert_eq!(
            translation
                .senses()
                .iter()
                .map(|s| s.part_of_speech.unwrap())
                .collect::<Vec<_>>(),
            positions
        );
    }
    drop(mapped);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn may_three_senses_are_full_candidates_over_existing_pages() {
    let mut router = Router::new(
        engine(),
        RouterConfig {
            english_to_chinese: true,
            page_size: 1,
            ..RouterConfig::default()
        },
    );
    open_session(&mut router, SESSION, Some("notepad.exe".to_owned()));
    let (_, _, mut frame) = type_english(&mut router, "may");
    assert_eq!(frame.page_count, 4);
    for (page, text) in ["可能", "可以", "五月", "may"].iter().enumerate() {
        assert_eq!(frame.page, page);
        assert_eq!(candidate_texts(&frame), [*text]);
        if page < 3 {
            let sense = &frame.candidates.items[0]
                .translation
                .as_ref()
                .unwrap()
                .senses()[0];
            assert_eq!(sense.text, "may");
            assert_eq!(
                sense.part_of_speech,
                Some(if page == 2 {
                    PartOfSpeech::Noun
                } else {
                    PartOfSpeech::Verb
                })
            );
            let (_, commit, next) = press(&mut router, KeyEvent::new(0x22, None, ENGLISH));
            assert!(commit.is_none());
            frame = next;
        }
    }
    let (_, _, third) = press(&mut router, KeyEvent::new(0x21, None, ENGLISH));
    assert_eq!(candidate_texts(&third), ["五月"]);
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("五月"));
}

#[test]
fn translation_multisense_source_qj_query_is_lossless_and_deduplicated() {
    for count in [1, 2, 3, 17, 129] {
        let mut fields: Vec<_> = (0..count)
            .map(|i| format!("{} 义项{i}", if i % 2 == 0 { "n." } else { "v." }))
            .collect();
        fields.push("v. 义项0".to_owned());
        let glossary =
            Glossary::parse(Language::Chinese, &format!("poly\t{}\n", fields.join("\t"))).unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "qingjian-chain-{count}-{}-{nonce}.qj",
            std::process::id()
        ));
        glossary.write_qj(&path, &Default::default()).unwrap();
        let mapped = Glossary::from_path(Language::Chinese, &path).unwrap();
        let mut engine =
            Engine::new(qingjian_dictionary::Dictionary::parse("字\tzi\t1\n").unwrap())
                .with_english_translator(Box::new(mapped));
        engine.set_english_mode(true);
        engine.set_english_to_chinese(true);
        engine.set_input("poly");
        let mut list = engine.query().unwrap().candidates;
        engine.annotate(&mut list);
        assert_eq!(list.items.len(), count + 1);
        for (i, candidate) in list.items[..count].iter().enumerate() {
            assert_eq!(candidate.text, format!("义项{i}"));
            assert_eq!(candidate.kind, CandidateKind::Translation);
            let annotation = candidate.translation.as_ref().unwrap();
            assert_eq!(annotation.senses()[0].text, "poly");
            assert_eq!(
                annotation.senses()[0].part_of_speech,
                Some(if i % 2 == 0 {
                    PartOfSpeech::Noun
                } else {
                    PartOfSpeech::Verb
                })
            );
        }
        assert_eq!(list.items[count].text, "poly");
        drop(engine);
        std::fs::remove_file(path).unwrap();
    }
}
