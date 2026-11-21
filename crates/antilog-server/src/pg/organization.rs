use antilog_core::store::{Membership, OrganizationStore, SignInRecord, StoreError, TokenRecord};
use antilog_types::{
    AccessToken, AuditEntry, Identity, Member, MemberId, MemberKind, Organization, OrganizationId,
    Role, SignInId, SignInSession, TokenId,
};
use uuid::Uuid;

use super::{PgStore, audit, corrupt, map};

struct MemberRow {
    id: Uuid,
    organization_id: Uuid,
    kind: String,
    name: String,
    provider: Option<String>,
    subject: Option<String>,
    role: String,
    created_at: i64,
}

impl TryFrom<MemberRow> for Member {
    type Error = StoreError;

    fn try_from(row: MemberRow) -> Result<Self, StoreError> {
        Ok(Member {
            id: MemberId::from_uuid(row.id),
            organization_id: OrganizationId::from_uuid(row.organization_id),
            kind: MemberKind::parse(&row.kind).ok_or_else(|| corrupt("member kind", &row.kind))?,
            name: row.name,
            identity: row
                .provider
                .zip(row.subject)
                .map(|(provider, subject)| Identity { provider, subject }),
            role: Role::parse(&row.role).ok_or_else(|| corrupt("role", &row.role))?,
            created_at: row.created_at,
        })
    }
}

struct TokenRow {
    id: Uuid,
    organization_id: Uuid,
    member_id: Uuid,
    name: String,
    hash: Vec<u8>,
    created_at: i64,
    revoked_at: Option<i64>,
}

impl TryFrom<TokenRow> for TokenRecord {
    type Error = StoreError;

    fn try_from(row: TokenRow) -> Result<Self, StoreError> {
        Ok(TokenRecord {
            hash: row
                .hash
                .try_into()
                .map_err(|_| corrupt("token hash", "of the wrong length"))?,
            token: AccessToken {
                id: TokenId::from_uuid(row.id),
                organization_id: OrganizationId::from_uuid(row.organization_id),
                member_id: MemberId::from_uuid(row.member_id),
                name: row.name,
                created_at: row.created_at,
                revoked_at: row.revoked_at,
            },
        })
    }
}

impl OrganizationStore for PgStore {
    async fn create_organization(&self, organization: &Organization) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization.id).await?;
        sqlx::query!(
            "insert into organizations (id, name, created_at, monthly_cap_cents, default_budget_cents, max_running_sessions) values ($1, $2, to_timestamp($3::bigint / 1000.0), $4, $5, $6)",
            organization.id.as_uuid(),
            organization.name,
            organization.created_at,
            organization.monthly_cap_cents,
            organization.default_budget_cents,
            i32::try_from(organization.max_running_sessions).unwrap_or(i32::MAX),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        tx.commit().await.map_err(map)
    }

    async fn organization(&self, id: OrganizationId) -> Result<Option<Organization>, StoreError> {
        let mut tx = self.scoped(id).await?;
        let row = sqlx::query!(
            "select id, name, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", monthly_cap_cents, default_budget_cents, max_running_sessions from organizations where id = $1",
            id.as_uuid()
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?;
        Ok(row.map(|row| Organization {
            id: OrganizationId::from_uuid(row.id),
            name: row.name,
            created_at: row.created_at,
            monthly_cap_cents: row.monthly_cap_cents,
            default_budget_cents: row.default_budget_cents,
            max_running_sessions: u32::try_from(row.max_running_sessions).unwrap_or(1),
        }))
    }

    async fn add_member(&self, member: &Member, entry: &AuditEntry) -> Result<(), StoreError> {
        let mut tx = self.scoped(member.organization_id).await?;
        sqlx::query!(
            "insert into members (id, organization_id, kind, name, provider, subject, role, created_at) values ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8::bigint / 1000.0))",
            member.id.as_uuid(),
            member.organization_id.as_uuid(),
            member.kind.as_str(),
            member.name,
            member.identity.as_ref().map(|identity| identity.provider.as_str()),
            member.identity.as_ref().map(|identity| identity.subject.as_str()),
            member.role.as_str(),
            member.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn member(
        &self,
        organization: OrganizationId,
        id: MemberId,
    ) -> Result<Option<Member>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            MemberRow,
            "select id, organization_id, kind, name, provider, subject, role, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from members where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Member::try_from)
        .transpose()
    }

    async fn member_by_identity(
        &self,
        organization: OrganizationId,
        identity: &Identity,
    ) -> Result<Option<Member>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            MemberRow,
            "select id, organization_id, kind, name, provider, subject, role, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from members where organization_id = $1 and provider = $2 and subject = $3",
            organization.as_uuid(),
            identity.provider,
            identity.subject,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(Member::try_from)
        .transpose()
    }

    async fn memberships(&self, identity: &Identity) -> Result<Vec<Membership>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map)?;
        for (setting, value) in [
            ("antilog.identity_provider", &identity.provider),
            ("antilog.identity_subject", &identity.subject),
        ] {
            sqlx::query("select set_config($1, $2, true)")
                .bind(setting)
                .bind(value)
                .execute(&mut *tx)
                .await
                .map_err(map)?;
        }
        let rows = sqlx::query_as!(
            MemberRow,
            "select id, organization_id, kind, name, provider, subject, role, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from members where provider = $1 and subject = $2 order by created_at, id",
            identity.provider,
            identity.subject,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?;
        let mut found = Vec::with_capacity(rows.len());
        for row in rows {
            let organization = sqlx::query!(
                "select id, name, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", monthly_cap_cents, default_budget_cents, max_running_sessions from organizations where id = $1",
                row.organization_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map)?;
            found.push(Membership {
                member: Member::try_from(row)?,
                organization: Organization {
                    id: OrganizationId::from_uuid(organization.id),
                    name: organization.name,
                    created_at: organization.created_at,
                    monthly_cap_cents: organization.monthly_cap_cents,
                    default_budget_cents: organization.default_budget_cents,
                    max_running_sessions: u32::try_from(organization.max_running_sessions)
                        .unwrap_or(1),
                },
            });
        }
        tx.commit().await.map_err(map)?;
        Ok(found)
    }

    async fn members(&self, organization: OrganizationId) -> Result<Vec<Member>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            MemberRow,
            "select id, organization_id, kind, name, provider, subject, role, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\" from members where organization_id = $1 order by created_at, id",
            organization.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(Member::try_from)
        .collect()
    }

    async fn set_member_role(
        &self,
        organization: OrganizationId,
        member: MemberId,
        role: Role,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update members set role = $3 where organization_id = $1 and id = $2",
            organization.as_uuid(),
            member.as_uuid(),
            role.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn create_access_token(
        &self,
        record: &TokenRecord,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(record.token.organization_id).await?;
        sqlx::query!(
            "insert into access_tokens (id, organization_id, member_id, name, hash, created_at) values ($1, $2, $3, $4, $5, to_timestamp($6::bigint / 1000.0))",
            record.token.id.as_uuid(),
            record.token.organization_id.as_uuid(),
            record.token.member_id.as_uuid(),
            record.token.name,
            record.hash.as_slice(),
            record.token.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn access_token(&self, id: TokenId) -> Result<Option<TokenRecord>, StoreError> {
        let mut tx = self.found(id.as_uuid()).await?;
        sqlx::query_as!(
            TokenRow,
            "select id, organization_id, member_id, name, hash, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from revoked_at) * 1000)::bigint as \"revoked_at\" from access_tokens where id = $1",
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(TokenRecord::try_from)
        .transpose()
    }

    async fn access_tokens(
        &self,
        organization: OrganizationId,
        member: MemberId,
    ) -> Result<Vec<AccessToken>, StoreError> {
        let mut tx = self.scoped(organization).await?;
        sqlx::query_as!(
            TokenRow,
            "select id, organization_id, member_id, name, hash, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from revoked_at) * 1000)::bigint as \"revoked_at\" from access_tokens where organization_id = $1 and member_id = $2 order by created_at, id",
            organization.as_uuid(),
            member.as_uuid(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map)?
        .into_iter()
        .map(|row| TokenRecord::try_from(row).map(|record| record.token))
        .collect()
    }

    async fn revoke_access_token(
        &self,
        organization: OrganizationId,
        id: TokenId,
        at: i64,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update access_tokens set revoked_at = coalesce(revoked_at, to_timestamp($3::bigint / 1000.0)) where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn create_sign_in_session(
        &self,
        record: &SignInRecord,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let session = &record.session;
        let mut tx = self.scoped(session.organization_id).await?;
        sqlx::query!(
            "insert into sign_in_sessions (id, organization_id, member_id, hash, created_at, expires_at) values ($1, $2, $3, $4, to_timestamp($5::bigint / 1000.0), to_timestamp($6::bigint / 1000.0))",
            session.id.as_uuid(),
            session.organization_id.as_uuid(),
            session.member_id.as_uuid(),
            record.hash.as_slice(),
            session.created_at,
            session.expires_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?;
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }

    async fn sign_in_session(&self, id: SignInId) -> Result<Option<SignInRecord>, StoreError> {
        let mut tx = self.found(id.as_uuid()).await?;
        sqlx::query!(
            "select id, organization_id, member_id, hash, (extract(epoch from created_at) * 1000)::bigint as \"created_at!\", (extract(epoch from expires_at) * 1000)::bigint as \"expires_at!\", (extract(epoch from ended_at) * 1000)::bigint as \"ended_at\" from sign_in_sessions where id = $1",
            id.as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map)?
        .map(|row| {
            Ok(SignInRecord {
                hash: row
                    .hash
                    .try_into()
                    .map_err(|_| corrupt("sign-in hash", "of the wrong length"))?,
                session: SignInSession {
                    id: SignInId::from_uuid(row.id),
                    organization_id: OrganizationId::from_uuid(row.organization_id),
                    member_id: MemberId::from_uuid(row.member_id),
                    created_at: row.created_at,
                    expires_at: row.expires_at,
                    ended_at: row.ended_at,
                },
            })
        })
        .transpose()
    }

    async fn end_sign_in_session(
        &self,
        organization: OrganizationId,
        id: SignInId,
        at: i64,
        entry: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut tx = self.scoped(organization).await?;
        let changed = sqlx::query!(
            "update sign_in_sessions set ended_at = coalesce(ended_at, to_timestamp($3::bigint / 1000.0)) where organization_id = $1 and id = $2",
            organization.as_uuid(),
            id.as_uuid(),
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound);
        }
        audit(&mut tx, entry).await?;
        tx.commit().await.map_err(map)
    }
}
