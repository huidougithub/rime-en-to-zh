//! Windows 文档提交回执：只把确认成功的英译中选择交回 Core。

use crate::dispatch::Router;
use qingjian_core::TranslationChoice;
use qingjian_platform::protocol::{COMMIT_RECEIPT_SINCE, SessionId};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 队列有界；旧 DLL、丢失、重复、跨会话或失败回执均不学习。
pub(super) struct CommitReceipts {
    next: u64,

    staged: Vec<(SessionId, TranslationChoice)>,

    pending: BTreeMap<u64, (SessionId, Vec<TranslationChoice>)>,
}

impl Default for CommitReceipts {
    fn default() -> Self {
        Self {
            // 进程重启后不复用刚发过的编号，避免旧异步回调误确认新提交。
            next: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            staged: Vec::new(),
            pending: BTreeMap::new(),
        }
    }
}

impl CommitReceipts {
    pub(super) fn discard(&mut self, session: SessionId) {
        self.staged.retain(|(owner, _)| *owner != session);
        self.pending.retain(|_, (owner, _)| *owner != session);
    }
}

impl Router {
    pub(super) fn capture_translation_choice(&mut self, session: SessionId) {
        if let Some(choice) = self.engine.take_translation_choice() {
            if self.receipts.staged.len() >= 64 {
                self.receipts.staged.remove(0);
            }
            self.receipts.staged.push((session, choice));
        }
    }

    pub(super) fn issue_commit_receipt(
        &mut self,
        session: SessionId,
        has_commit: bool,
    ) -> Option<u64> {
        let mut choices = Vec::new();
        // 取出暂存记录，避免克隆 Core 生成的选择。
        let staged = std::mem::take(&mut self.receipts.staged);
        for (owner, choice) in staged {
            if owner == session {
                choices.push(choice);
            } else {
                self.receipts.staged.push((owner, choice));
            }
        }
        let supported = self
            .sessions
            .get(&session)
            .is_some_and(|info| info.protocol >= COMMIT_RECEIPT_SINCE && !info.private);
        if !has_commit || !supported || choices.is_empty() {
            return None;
        }
        self.receipts.next = self.receipts.next.wrapping_add(1);
        let receipt = self.receipts.next;
        if self.receipts.pending.len() >= 64 {
            self.receipts.pending.pop_first();
        }
        self.receipts.pending.insert(receipt, (session, choices));
        Some(receipt)
    }

    pub(super) fn confirm_commit(&mut self, session: SessionId, receipt: u64, applied: bool) {
        if self
            .receipts
            .pending
            .get(&receipt)
            .is_none_or(|(owner, _)| *owner != session)
        {
            return;
        }
        let (_, choices) = self.receipts.pending.remove(&receipt).unwrap();
        if !applied || self.sessions.get(&session).is_none_or(|info| info.private) {
            return;
        }
        let mut recorded = false;
        for choice in choices {
            recorded |= self.engine.confirm_translation_choice(choice);
        }
        // 回执在 Server 工人线程单向处理；沿用原子写回，成功选择在重启前可持久化。
        if recorded {
            self.engine.learner_mut().flush();
        }
    }
}
