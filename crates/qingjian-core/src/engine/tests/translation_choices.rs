//! 英译中选择只在确认后进入既有 Learner，并与其他模式隔离。

use super::CountingLearner;
use super::translation::translating;
use crate::{Candidate, Engine};
use std::collections::HashMap;

fn engine() -> Engine {
    translating().with_learner(Box::new(CountingLearner(HashMap::new())))
}

fn items(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

fn second(engine: &mut Engine) -> Candidate {
    engine.set_input("set");
    engine.query().unwrap().candidates.items[1].clone()
}

#[test]
fn translation_choice_requires_confirmation_and_one_choice_promotes_the_sense() {
    let mut engine = engine();
    assert_eq!(items(&mut engine, "set"), ["设置", "放置", "set"]);
    let selected = second(&mut engine);
    assert_eq!(engine.commit(&selected), "放置");
    let choice = engine.take_translation_choice().unwrap();
    assert_eq!(engine.learner().choice_weight("en-zh:set", "放置"), 0);
    assert_eq!(items(&mut engine, "set"), ["设置", "放置", "set"]);
    assert!(engine.confirm_translation_choice(choice));
    assert_eq!(items(&mut engine, "SET"), ["放置", "设置", "SET"]);
    assert_eq!(engine.learner().choice_weight("en-zh:set", "放置"), 1);
    assert_eq!(engine.learner().choice_weight("set", "放置"), 0);
    assert_eq!(engine.learner().weight("放置"), 0);
    assert_eq!(engine.learner().weight("set"), 0);
    assert!(engine.learner().user_english().is_none());
    assert!(engine.learner().user_words().is_none());
}

#[test]
fn translation_choices_ignore_other_keys_and_do_not_inject_missing_senses() {
    let mut engine = engine();
    engine.learner_mut().record_choice("set", "", "放置");
    engine
        .learner_mut()
        .record_choice("en-zh:set", "", "并非译义");
    engine
        .learner_mut()
        .record_choice("en-zh:studio", "", "放置");
    assert_eq!(items(&mut engine, "set"), ["设置", "放置", "set"]);
    assert_eq!(items(&mut engine, "studio"), ["工作室", "studio"]);
    assert_eq!(items(&mut engine, "setz"), ["setz"]);
    assert_eq!(
        items(&mut engine, "\"set a record\""),
        ["创造纪录", "\"set a record\""]
    );
}

#[test]
fn translation_choices_do_not_learn_cancel_raw_or_muted_selections() {
    let mut engine = engine();
    let selected = second(&mut engine);
    engine.clear();
    assert!(engine.take_translation_choice().is_none());
    engine.set_input("set");
    assert_eq!(engine.take_raw(), "set");
    assert!(engine.take_translation_choice().is_none());
    engine.set_learning(false);
    engine.set_input("set");
    assert_eq!(engine.commit(&selected), "放置");
    assert!(engine.take_translation_choice().is_none());
    engine.set_learning(true);
    engine.set_private(true);
    engine.set_input("set");
    assert_eq!(engine.commit(&selected), "放置");
    assert!(engine.take_translation_choice().is_none());
    engine.set_private(false);
    engine.set_input("set");
    engine.commit(&selected);
    let pending = engine.take_translation_choice().unwrap();
    engine.set_learning(false);
    assert!(!engine.confirm_translation_choice(pending));
    assert_eq!(engine.learner().choice_weight("en-zh:set", "放置"), 0);
}

#[test]
fn translation_choices_reject_stale_or_forged_candidates() {
    let mut engine = engine();
    let mut selected = second(&mut engine);
    selected.text = "银行".into();
    assert!(engine.commit(&selected).is_empty());
    assert_eq!(engine.composition().text(), "set");
    assert!(engine.take_translation_choice().is_none());
    selected.text = "放置".into();
    engine.set_input("studio");
    assert!(engine.commit(&selected).is_empty());
    assert_eq!(engine.composition().text(), "studio");
    engine.set_english_mode(false);
    engine.set_input("kaifa");
    assert!(engine.commit(&selected).is_empty());
    assert_eq!(engine.composition().text(), "kaifa");
}
