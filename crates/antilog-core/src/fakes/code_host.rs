use std::collections::HashSet;

use antilog_types::OrganizationId;
use antilog_types::judge::{PullRequestState, TrialId};

use super::{FakeStore, locked};
use crate::store::{CodeHostStore, PullRequestRow, StoreError};

#[derive(Default)]
pub(super) struct State {
    rows: Vec<PullRequestRow>,
    deliveries: HashSet<(OrganizationId, String)>,
}

impl CodeHostStore for FakeStore {
    async fn put_pull_requests(
        &self,
        organization: OrganizationId,
        rows: &[PullRequestRow],
    ) -> Result<(), StoreError> {
        let mut state = locked(&self.code_host);
        let taken = |row: &PullRequestRow, state: &State| {
            state.rows.iter().any(|have| {
                have.organization == organization
                    && have.repo == row.repo
                    && have.number == row.number
            })
        };
        if rows
            .iter()
            .any(|row| row.organization != organization || taken(row, &state))
        {
            return Err(StoreError::Conflict);
        }
        state.rows.extend_from_slice(rows);
        Ok(())
    }

    async fn pull_request(
        &self,
        repo: &str,
        number: u64,
    ) -> Result<Option<PullRequestRow>, StoreError> {
        let state = locked(&self.code_host);
        let mut found = state
            .rows
            .iter()
            .filter(|row| row.repo == repo && row.number == number);
        let first = found.next().cloned();
        // Two organizations that share one pull request: none of them owns the event.
        Ok(first.filter(|first| found.all(|other| other.organization == first.organization)))
    }

    async fn pull_requests_of(
        &self,
        organization: OrganizationId,
        trial: TrialId,
    ) -> Result<Vec<PullRequestRow>, StoreError> {
        Ok(locked(&self.code_host)
            .rows
            .iter()
            .filter(|row| row.organization == organization && row.trial == trial)
            .cloned()
            .collect())
    }

    async fn set_pull_request_state(
        &self,
        organization: OrganizationId,
        repo: &str,
        number: u64,
        state: PullRequestState,
    ) -> Result<(), StoreError> {
        let mut held = locked(&self.code_host);
        let row = held
            .rows
            .iter_mut()
            .find(|row| {
                row.organization == organization && row.repo == repo && row.number == number
            })
            .ok_or(StoreError::NotFound)?;
        row.state = state;
        Ok(())
    }

    async fn delivery_seen(
        &self,
        organization: OrganizationId,
        delivery: &str,
    ) -> Result<bool, StoreError> {
        Ok(locked(&self.code_host)
            .deliveries
            .contains(&(organization, delivery.to_owned())))
    }

    async fn put_delivery(
        &self,
        organization: OrganizationId,
        delivery: &str,
        _at: i64,
    ) -> Result<(), StoreError> {
        locked(&self.code_host)
            .deliveries
            .insert((organization, delivery.to_owned()));
        Ok(())
    }
}
