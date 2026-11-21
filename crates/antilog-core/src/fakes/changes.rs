use antilog_types::{
    ChangeMark, ChangesReport, CommitChanges, OrganizationId, RepoChanges, RepoReport, SessionId,
};

use super::{FakeStore, locked};
use crate::store::{ChangesStore, StoreError};

#[derive(Default)]
pub(super) struct State {
    rows: Vec<(OrganizationId, SessionId, RepoReport, i64)>,
}

impl ChangesStore for FakeStore {
    async fn put_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        report: &ChangesReport,
        at: i64,
    ) -> Result<(), StoreError> {
        locked(&self.ledger)
            .session(organization, session)
            .ok_or(StoreError::NotFound)?;
        let mut state = locked(&self.changes);
        state.rows.retain(|row| row.1 != session);
        for repo in &report.repos {
            state.rows.push((organization, session, repo.clone(), at));
        }
        Ok(())
    }

    async fn changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Vec<RepoChanges>, StoreError> {
        Ok(locked(&self.changes)
            .rows
            .iter()
            .filter(|row| row.0 == organization && row.1 == session)
            .map(|(_, _, repo, at)| RepoChanges {
                repo: repo.repo.clone(),
                base_branch: repo.base_branch.clone(),
                head_branch: repo.head_branch.clone(),
                base_sha: repo.base_sha.clone(),
                head_sha: repo.head_sha.clone(),
                commits: repo.commits.clone(),
                files: repo.files.clone(),
                reported_at: *at,
            })
            .collect())
    }

    async fn commit_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        repo: Option<&str>,
        sha: &str,
    ) -> Result<Option<CommitChanges>, StoreError> {
        Ok(locked(&self.changes)
            .rows
            .iter()
            .filter(|row| row.0 == organization && row.1 == session)
            .filter(|row| repo.is_none_or(|name| row.2.repo == name))
            .flat_map(|row| row.2.commit_diffs.iter())
            .find(|diff| diff.sha == sha)
            .map(|diff| CommitChanges {
                sha: diff.sha.clone(),
                files: diff.files.clone(),
            }))
    }

    async fn changes_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<ChangeMark>, StoreError> {
        let mut marks: Vec<ChangeMark> = Vec::new();
        for row in &locked(&self.changes).rows {
            if row.0 == organization
                && row.3 >= since
                && !marks.iter().any(|mark| mark.session == row.1)
            {
                marks.push(ChangeMark {
                    session: row.1,
                    reported_at: row.3,
                });
            }
        }
        marks.sort_by_key(|mark| mark.reported_at);
        marks.truncate(limit as usize);
        Ok(marks)
    }
}
