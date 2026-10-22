//! 一次选择、原有文件持久化、词间与模式隔离。

use super::{acknowledge, choose_second, count, fresh_dir, router, show};
use crate::support::{ClientMessage, ENGLISH, KeyEvent, SESSION, ServerMessage};
use qingjian_render::HitTarget;

#[test]
fn translation_choice_once_promotes_real_bank_sense_and_survives_restart() {
    let dir = fresh_dir("restart");
    let mut first = router(&dir);
    assert_eq!(show(&mut first, "bank"), ["银行", "河岸", "bank"]);
    let (text, receipt) = choose_second(&mut first, "bank");
    assert_eq!(text, "河岸");
    assert_eq!(count(&mut first, "bank", "河岸"), 0);
    acknowledge(&mut first, receipt.unwrap(), true);
    assert_eq!(count(&mut first, "bank", "河岸"), 1);
    assert_eq!(show(&mut first, "bank"), ["河岸", "银行", "bank"]);
    assert_eq!(show(&mut first, "BANK"), ["河岸", "银行", "BANK"]);
    assert_eq!(show(&mut first, "right"), ["正确的", "右边的", "right"]);
    assert_eq!(first.engine_mut().learner().weight("bank"), 0);
    assert_eq!(first.engine_mut().learner().weight("河岸"), 0);
    assert_eq!(
        first.engine_mut().learner().choice_weight("bank", "河岸"),
        0
    );
    let rows = std::fs::read_to_string(dir.join("user-choices.tsv")).unwrap();
    assert!(rows.lines().any(|line| line == "en-zh:bank\t河岸\t1"));
    assert!(!rows.lines().any(|line| line.starts_with("bank\t")));
    drop(first);
    let mut restarted = router(&dir);
    assert_eq!(show(&mut restarted, "bank"), ["河岸", "银行", "bank"]);
    assert_eq!(count(&mut restarted, "bank", "河岸"), 1);
    assert_eq!(show(&mut restarted, "set"), ["设置", "放置", "set"]);
}

#[test]
fn translation_choice_keeps_pinyin_and_plain_english_learning_separate() {
    let mut router = router(&fresh_dir("isolation"));
    let (_, receipt) = choose_second(&mut router, "bank");
    acknowledge(&mut router, receipt.unwrap(), true);
    let engine = router.engine_mut();
    engine.set_english_mode(false);
    engine.set_input("nihao");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "你好");
    assert_eq!(engine.learner().weight("河岸"), 0);
    assert!(engine.learner().user_words().is_none());
    assert!(engine.learner().user_english().is_none());
    engine.set_english_to_chinese(false);
    engine.set_english_mode(true);
    engine.set_input("hello");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "hello");
    assert_eq!(engine.learner().weight("bank"), 0);
}

#[test]
fn translation_choice_click_waits_for_actual_document_receipt() {
    let mut router = router(&fresh_dir("click"));
    show(&mut router, "bank");
    router.handle_click(HitTarget::Candidate(1));
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    let Some(ServerMessage::Update {
        commit: Some(text),
        commit_receipt: Some(receipt),
        ..
    }) = router.handle(ClientMessage::Poll { session: SESSION })
    else {
        panic!("missing click commit");
    };
    assert_eq!(text, "河岸");
    acknowledge(&mut router, receipt, true);
    assert_eq!(count(&mut router, "bank", "河岸"), 1);
    assert_eq!(show(&mut router, "bank"), ["河岸", "银行", "bank"]);
    // 高亮首项空格再次成功：累计次数为二，仍沿用原生计数。
    let Some(ServerMessage::KeyResult {
        commit_receipt: Some(receipt),
        ..
    }) = router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x20, Some(' '), ENGLISH),
    })
    else {
        panic!("missing keyboard receipt");
    };
    acknowledge(&mut router, receipt, true);
    assert_eq!(count(&mut router, "bank", "河岸"), 2);
}

#[test]
fn translation_choice_stale_file_rows_cannot_create_or_misattribute_senses() {
    let dir = fresh_dir("stale");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("user-choices.tsv"),
        "en-zh:bank\t学习\t9999\nen-zh:bankk\t河岸\t9999\n",
    )
    .unwrap();
    let mut router = router(&dir);
    assert_eq!(show(&mut router, "bank"), ["银行", "河岸", "bank"]);
    assert_eq!(show(&mut router, "bankk"), ["bankk"]);
    assert_eq!(show(&mut router, "studio"), ["工作室", "studio"]);
}

#[test]
fn may_three_senses_keep_one_choice_persistence_and_failed_commits_do_not_count() {
    let dir = fresh_dir("may-three");
    let mut first = router(&dir);
    assert_eq!(show(&mut first, "may"), ["可能", "可以", "五月", "may"]);
    let (text, receipt) = choose_second(&mut first, "may");
    assert_eq!(text, "可以");
    acknowledge(&mut first, receipt.unwrap(), false);
    assert_eq!(count(&mut first, "may", "可以"), 0);
    assert_eq!(show(&mut first, "may"), ["可能", "可以", "五月", "may"]);
    let (_, receipt) = choose_second(&mut first, "may");
    acknowledge(&mut first, receipt.unwrap(), true);
    assert_eq!(show(&mut first, "may"), ["可以", "可能", "五月", "may"]);
    drop(first);
    let mut restarted = router(&dir);
    assert_eq!(show(&mut restarted, "may"), ["可以", "可能", "五月", "may"]);
    assert_eq!(count(&mut restarted, "may", "可以"), 1);
    crate::support::press(&mut restarted, KeyEvent::new(0x0D, None, ENGLISH));
    assert_eq!(count(&mut restarted, "may", "可以"), 1);
}
