use std::collections::HashMap;

use antilog_types::{
    ContextMark, ContextPart, ContextUse, OrganizationId, SessionContext, SessionId,
};

use super::{FakeStore, locked};
use crate::store::{ContextStore, StoreError};

#[derive(Default)]
pub(super) struct State {
    rows: HashMap<SessionId, (OrganizationId, SessionContext)>,
}

impl FakeStore {
    fn context_row(
        &self,
        organization: OrganizationId,
        session: SessionId,
        change: impl FnOnce(&mut SessionContext),
    ) -> Result<(), StoreError> {
        locked(&self.ledger)
            .session(organization, session)
            .ok_or(StoreError::NotFound)?;
        let mut state = locked(&self.context);
        let row = state
            .rows
            .entry(session)
            .or_insert_with(|| (organization, SessionContext::default()));
        change(&mut row.1);
        Ok(())
    }
}

impl ContextStore for FakeStore {
    async fn put_context_use(
        &self,
        organization: OrganizationId,
        session: SessionId,
        context: ContextUse,
        at: i64,
    ) -> Result<(), StoreError> {
        self.context_row(organization, session, |row| {
            row.used = Some(context.used);
            // A breakdown larger than the window now holds is from before a compaction.
            if row.parts.iter().map(|part| part.tokens).sum::<u64>() > context.used {
                row.parts.clear();
            }
            if context.window.is_some() {
                row.window = context.window;
            }
            row.reported_at = Some(at);
        })
    }

    async fn put_context_parts(
        &self,
        organization: OrganizationId,
        session: SessionId,
        parts: &[ContextPart],
        at: i64,
    ) -> Result<(), StoreError> {
        self.context_row(organization, session, |row| {
            row.parts = parts.to_vec();
            row.reported_at = Some(at);
        })
    }

    async fn context(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<SessionContext, StoreError> {
        Ok(locked(&self.context)
            .rows
            .get(&session)
            .filter(|row| row.0 == organization)
            .map(|row| row.1.clone())
            .unwrap_or_default())
    }

    async fn context_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<ContextMark>, StoreError> {
        let mut marks: Vec<ContextMark> = locked(&self.context)
            .rows
            .iter()
            .filter(|(_, row)| row.0 == organization)
            .filter_map(|(session, row)| {
                row.1
                    .reported_at
                    .filter(|at| *at >= since)
                    .map(|reported_at| ContextMark {
                        session: *session,
                        reported_at,
                    })
            })
            .collect();
        marks.sort_by_key(|mark| mark.reported_at);
        marks.truncate(limit as usize);
        Ok(marks)
    }
}
