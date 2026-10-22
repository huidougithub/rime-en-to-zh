//! TSF 客户端传递编号和失败结果，不把原文塞进确认消息。

use qingjian_platform::protocol::{
    ClientMessage, Frame, InputSettings, KeyEvent, KeyOutcome, ServerMessage, SessionId,
    read_message, write_message,
};
use qingjian_tsf::client::{EngineClient, KeyReply};
use std::cell::RefCell;
use std::io::{self, Cursor, Read, Write};
use std::rc::Rc;

struct MemoryStream {
    input: Cursor<Vec<u8>>,

    output: Rc<RefCell<Vec<u8>>>,
}

impl Read for MemoryStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.input.read(bytes)
    }
}

impl Write for MemoryStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.output.borrow_mut().write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn client_carries_receipts_for_keyboard_and_click_and_reports_failure() {
    let session = SessionId(9);
    let mut input = Vec::new();
    for message in [
        ServerMessage::SessionOpened {
            session,
            input: InputSettings::default(),
        },
        ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: Some("河岸".into()),
            frame: Frame::default(),
            commit_receipt: Some(41),
        },
        ServerMessage::Update {
            session,
            commit: Some("放置".into()),
            frame: Frame::default(),
            commit_receipt: Some(42),
        },
    ] {
        write_message(&mut input, &message).unwrap();
    }
    let output = Rc::new(RefCell::new(Vec::new()));
    let stream = MemoryStream {
        input: Cursor::new(input),
        output: output.clone(),
    };
    let (mut client, _) = EngineClient::open(stream, session, None).unwrap();
    let KeyReply::Result(response) = client
        .key(KeyEvent::new(0x20, Some(' '), Default::default()))
        .unwrap()
    else {
        panic!("unexpected selection request");
    };
    assert_eq!(response.commit.as_deref(), Some("河岸"));
    assert_eq!(response.commit_receipt, Some(41));
    client.commit_applied(41, false).unwrap();
    let (_, text, receipt) = client.poll_with_receipt().unwrap();
    assert_eq!(text.as_deref(), Some("放置"));
    assert_eq!(receipt, Some(42));
    client.commit_applied(42, true).unwrap();
    let mut wire = Cursor::new(output.borrow().clone());
    let mut receipts = Vec::new();
    while let Some(message) = read_message::<_, ClientMessage>(&mut wire).unwrap() {
        if let ClientMessage::CommitApplied {
            session: owner,
            receipt,
            applied,
        } = message
        {
            assert_eq!(owner, session);
            receipts.push((receipt, applied));
        }
    }
    assert_eq!(receipts, [(41, false), (42, true)]);
}
