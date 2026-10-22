//! 新提交回执兼容旧帧，并且不传输入文本。

use qingjian_platform::protocol::{ClientMessage, Frame, KeyOutcome, ServerMessage, SessionId};

#[test]
fn old_server_frames_default_to_no_commit_receipt() {
    for message in [
        ServerMessage::KeyResult {
            session: SessionId(1),
            outcome: KeyOutcome::Consumed,
            frame: Frame::default(),
            commit: Some("公开样例".into()),
            commit_receipt: Some(42),
        },
        ServerMessage::Update {
            session: SessionId(1),
            frame: Frame::default(),
            commit: Some("公开样例".into()),
            commit_receipt: Some(43),
        },
    ] {
        let mut value = serde_json::to_value(message).unwrap();
        let fields = value
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()
            .as_object_mut()
            .unwrap();
        fields.remove("commit_receipt");
        match serde_json::from_value::<ServerMessage>(value).unwrap() {
            ServerMessage::KeyResult { commit_receipt, .. }
            | ServerMessage::Update { commit_receipt, .. } => {
                assert_eq!(commit_receipt, None);
            }
            _ => panic!("wrong response kind"),
        }
    }
}

#[test]
fn receipt_wire_contains_only_session_identifier_and_result() {
    let message = ClientMessage::CommitApplied {
        session: SessionId(2),
        receipt: 17,
        applied: false,
    };
    let value = serde_json::to_value(&message).unwrap();
    let fields = value["CommitApplied"].as_object().unwrap();
    assert_eq!(fields.len(), 3);
    assert!(
        fields.contains_key("session")
            && fields.contains_key("receipt")
            && fields.contains_key("applied")
    );
    assert_eq!(
        serde_json::from_value::<ClientMessage>(value).unwrap(),
        message
    );
}
