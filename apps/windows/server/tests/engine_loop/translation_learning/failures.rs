//! 失败、缺失、重复、跨会话回执与原文/取消路径不增加选择。

use super::{acknowledge, choose_second, count, fresh_dir, router, show};
use crate::support::{ClientMessage, ENGLISH, KeyEvent, SESSION, ServerMessage, SessionId, press};

#[test]
fn translation_choice_rejects_failure_duplicate_and_cross_session_receipts() {
    let mut router = router(&fresh_dir("receipts"));
    let (_, receipt) = choose_second(&mut router, "bank");
    let receipt = receipt.unwrap();
    router.handle(ClientMessage::CommitApplied {
        session: SessionId(77),
        receipt,
        applied: true,
    });
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    acknowledge(&mut router, receipt, false);
    acknowledge(&mut router, receipt, true);
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    assert_eq!(show(&mut router, "bank"), ["银行", "河岸", "bank"]);
    let (_, receipt) = choose_second(&mut router, "bank");
    let receipt = receipt.unwrap();
    acknowledge(&mut router, receipt, true);
    acknowledge(&mut router, receipt, true);
    assert_eq!(count(&mut router, "bank", "河岸"), 1);
}

#[test]
fn translation_choice_no_receipt_cancel_and_enter_never_train() {
    let mut router = router(&fresh_dir("cancel"));
    let (_, _lost_receipt) = choose_second(&mut router, "bank");
    assert_eq!(show(&mut router, "bank"), ["银行", "河岸", "bank"]);
    press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    press(&mut router, KeyEvent::new(0x1B, None, ENGLISH));
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    show(&mut router, "bank");
    press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    let Some(ServerMessage::KeyResult {
        commit: Some(text),
        commit_receipt,
        ..
    }) = router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x0D, None, ENGLISH),
    })
    else {
        panic!("missing raw commit");
    };
    assert_eq!(text, "bank");
    assert_eq!(commit_receipt, None);
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    show(&mut router, "bankk");
    let Some(ServerMessage::KeyResult { commit_receipt, .. }) = router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x20, Some(' '), ENGLISH),
    }) else {
        panic!("missing literal commit");
    };
    assert_eq!(commit_receipt, None);
    assert_eq!(count(&mut router, "bankk", "河岸"), 0);
}

#[test]
fn translation_choice_closed_session_or_old_dll_cannot_train() {
    let mut router = router(&fresh_dir("old-client"));
    let (_, receipt) = choose_second(&mut router, "bank");
    router.handle(ClientMessage::CloseSession { session: SESSION });
    acknowledge(&mut router, receipt.unwrap(), true);
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: Some("notepad.exe".into()),
        protocol: 10,
    });
    let (text, receipt) = choose_second(&mut router, "bank");
    assert_eq!(text, "河岸");
    assert_eq!(receipt, None);
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
}

#[test]
fn translation_choice_muted_or_private_commits_do_not_train() {
    let mut router = router(&fresh_dir("privacy"));
    router.engine_mut().set_learning(false);
    let (_, receipt) = choose_second(&mut router, "bank");
    assert_eq!(receipt, None);
    router.engine_mut().set_learning(true);
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    let (_, receipt) = choose_second(&mut router, "bank");
    assert_eq!(receipt, None);
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: false,
    });
    let (_, receipt) = choose_second(&mut router, "bank");
    router.handle(ClientMessage::Privacy {
        session: SESSION,
        private: true,
    });
    acknowledge(&mut router, receipt.unwrap(), true);
    assert_eq!(count(&mut router, "bank", "河岸"), 0);
}
