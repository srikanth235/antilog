use antilog_core::store::{ChangesStore, StoreError};
use antilog_types::{
    ChangeMark, ChangesReport, CommitChanges, CommitInfo, FileChange, OrganizationId, RepoChanges,
    SessionId,
};

use super::{PgStore, corrupt, map};

fn json<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::Unavailable(Box::new(error)))
}

fn parse<T: serde::de::DeserializeOwned>(what: &str, text: &str) -> Result<T, StoreError> {
    serde_json::from_str(text).map_err(|_| corrupt(what, "not json"))
}

impl ChangesStore for PgStore {
    async fn put_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        report: &ChangesReport,
        at: i64,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_scalar!(
            "select id from sessions where id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .ok_or(StoreError::NotFound)?;
        sqlx::query!(
            "delete from session_changes where session_id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        sqlx::query!(
            "delete from session_commit_changes where session_id = $1 and organization_id = $2",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        for repo in &report.repos {
            sqlx::query!(
                "insert into session_changes (session_id, organization_id, repo, base_branch, head_branch, base_sha, head_sha, commits, files, reported_at) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, to_timestamp($10::bigint / 1000.0))",
                session.as_uuid(),
                organization.as_uuid(),
                repo.repo,
                repo.base_branch,
                repo.head_branch,
                repo.base_sha,
                repo.head_sha,
                json(&repo.commits)?,
                json(&repo.files)?,
                at,
            )
            .execute(&mut *tx)
            .await
            .map_err(map)?;
            for diff in &repo.commit_diffs {
                sqlx::query!(
                    "insert into session_commit_changes (session_id, organization_id, repo, sha, files) values ($1, $2, $3, $4, $5) on conflict do nothing",
                    session.as_uuid(),
                    organization.as_uuid(),
                    repo.repo,
                    diff.sha,
                    json(&diff.files)?,
                )
                .execute(&mut *tx)
                .await
                .map_err(map)?;
            }
        }
        tx.commit().await.map_err(map)
    }

    async fn changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
    ) -> Result<Vec<RepoChanges>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let rows = sqlx::query!(
            "select repo, base_branch, head_branch, base_sha, head_sha, commits, files, (extract(epoch from reported_at) * 1000)::bigint as \"reported_at!\" from session_changes where session_id = $1 and organization_id = $2 order by repo",
            session.as_uuid(),
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        rows.into_iter()
            .map(|row| {
                Ok(RepoChanges {
                    repo: row.repo,
                    base_branch: row.base_branch,
                    head_branch: row.head_branch,
                    base_sha: row.base_sha,
                    head_sha: row.head_sha,
                    commits: parse::<Vec<CommitInfo>>("commits", &row.commits)?,
                    files: parse::<Vec<FileChange>>("files", &row.files)?,
                    reported_at: row.reported_at,
                })
            })
            .collect()
    }

    async fn commit_changes(
        &self,
        organization: OrganizationId,
        session: SessionId,
        repo: Option<&str>,
        sha: &str,
    ) -> Result<Option<CommitChanges>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let found = sqlx::query_scalar!(
            "select files from session_commit_changes where session_id = $1 and organization_id = $2 and sha = $3 and ($4::text is null or repo = $4) order by repo limit 1",
            session.as_uuid(),
            organization.as_uuid(),
            sha,
            repo,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        found
            .map(|files| {
                Ok(CommitChanges {
                    sha: sha.to_owned(),
                    files: parse("files", &files)?,
                })
            })
            .transpose()
    }

    async fn changes_marks(
        &self,
        organization: OrganizationId,
        since: i64,
        limit: u32,
    ) -> Result<Vec<ChangeMark>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        let rows = sqlx::query!(
            "select session_id, (extract(epoch from max(reported_at)) * 1000)::bigint as \"reported_at!\" from session_changes where organization_id = $1 and reported_at >= to_timestamp($2::bigint / 1000.0) group by session_id order by 2, session_id limit $3",
            organization.as_uuid(),
            since,
            i64::from(limit),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        Ok(rows
            .into_iter()
            .map(|row| ChangeMark {
                session: SessionId::from_uuid(row.session_id),
                reported_at: row.reported_at,
            })
            .collect())
    }
}
