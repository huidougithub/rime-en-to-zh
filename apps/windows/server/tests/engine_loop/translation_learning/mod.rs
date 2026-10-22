//! 真实英汉表 + 原生用户选择表 + Windows 提交回执的离线回归。

mod choices;
mod failures;

use crate::support::{
    AppsConfig, AssemblySpec, ClientMessage, DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS, ENGLISH,
    KeyEvent, Router, RouterConfig, SESSION, ServerMessage, assembly, candidate_texts,
    open_session, press, type_english,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fresh_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qingjian-translation-choice-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn router(dir: &Path) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let engine = assembly::assemble(&AssemblySpec {
        english_glossary: Some(root.join("assets/glossary/glossary-zh.tsv")),
        english: Some(root.join("assets/sample/english.tsv")),
        user_dir: Some(dir.to_owned()),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .unwrap();
    let mut router = Router::new(
        engine,
        RouterConfig {
            english_to_chinese: true,
            apps: AppsConfig::with_english_candidates_on(DEFAULT_ENGLISH_CANDIDATES_ON_WINDOWS),
            ..RouterConfig::default()
        },
    );
    open_session(&mut router, SESSION, Some("notepad.exe".into()));
    router
}

fn show(router: &mut Router, input: &str) -> Vec<String> {
    press(router, KeyEvent::new(0x1B, None, ENGLISH));
    let (_, _, frame) = type_english(router, input);
    candidate_texts(&frame)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn choose_second(router: &mut Router, input: &str) -> (String, Option<u64>) {
    show(router, input);
    press(router, KeyEvent::new(0x28, None, ENGLISH));
    match router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x20, Some(' '), ENGLISH),
    }) {
        Some(ServerMessage::KeyResult {
            commit: Some(text),
            commit_receipt,
            ..
        }) => (text, commit_receipt),
        other => panic!("expected commit: {other:?}"),
    }
}

fn acknowledge(router: &mut Router, receipt: u64, applied: bool) {
    assert!(
        router
            .handle(ClientMessage::CommitApplied {
                session: SESSION,
                receipt,
                applied
            })
            .is_none()
    );
}

fn count(router: &mut Router, input: &str, text: &str) -> u32 {
    router
        .engine_mut()
        .learner()
        .choice_weight(&format!("en-zh:{input}"), text)
}
