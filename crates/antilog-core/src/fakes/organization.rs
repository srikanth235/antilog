use antilog_types::{
    AccessToken, AuditEntry, Identity, Member, MemberId, Organization, OrganizationId, Role,
    SignInId, TokenId,
};

use super::{FakeStore, ledger::not_found, locked};
use crate::store::{Membership, OrganizationStore, SignInRecord, StoreError, TokenRecord};

impl OrganizationStore for FakeStore {
    async fn create_organization(&self, organization: &Organization) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let taken = ledger
            .organizations
            .iter()
            .any(|row| row.id == organization.id || row.name == organization.name);
        if taken {
            return Err(StoreError::Conflict);
        }
        ledger.organizations.push(organization.clone());
        Ok(())
    }

    async fn organization(&self, id: OrganizationId) -> Result<Option<Organization>, StoreError> {
        Ok(locked(&self.ledger).organization(id).cloned())
    }

    async fn add_member(&self, member: &Member, audit: &AuditEntry) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.organization(member.organization_id))?;
        let taken = ledger.members.iter().any(|row| {
            row.id == member.id
                || (row.organization_id == member.organization_id
                    && member.identity.is_some()
                    && row.identity == member.identity)
        });
        if taken {
            return Err(StoreError::Conflict);
        }
        ledger.members.push(member.clone());
        if let Err(error) = ledger.keeps_owners(member.organization_id) {
            ledger.members.pop();
            return Err(error);
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn member(
        &self,
        organization: OrganizationId,
        id: MemberId,
    ) -> Result<Option<Member>, StoreError> {
        Ok(locked(&self.ledger).member(organization, id).cloned())
    }

    async fn member_by_identity(
        &self,
        organization: OrganizationId,
        identity: &Identity,
    ) -> Result<Option<Member>, StoreError> {
        Ok(locked(&self.ledger)
            .members
            .iter()
            .find(|row| {
                row.organization_id == organization && row.identity.as_ref() == Some(identity)
            })
            .cloned())
    }

    async fn memberships(&self, identity: &Identity) -> Result<Vec<Membership>, StoreError> {
        let ledger = locked(&self.ledger);
        let mut found: Vec<Membership> = ledger
            .members
            .iter()
            .filter(|row| row.identity.as_ref() == Some(identity))
            .filter_map(|member| {
                Some(Membership {
                    member: member.clone(),
                    organization: ledger.organization(member.organization_id)?.clone(),
                })
            })
            .collect();
        found.sort_by_key(|found| (found.member.created_at, found.member.id.as_uuid()));
        Ok(found)
    }

    async fn members(&self, organization: OrganizationId) -> Result<Vec<Member>, StoreError> {
        Ok(locked(&self.ledger)
            .members
            .iter()
            .filter(|row| row.organization_id == organization)
            .cloned()
            .collect())
    }

    async fn set_member_role(
        &self,
        organization: OrganizationId,
        member: MemberId,
        role: Role,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let row = not_found(
            ledger
                .members
                .iter_mut()
                .find(|row| row.organization_id == organization && row.id == member),
        )?;
        let before = std::mem::replace(&mut row.role, role);
        if let Err(error) = ledger.keeps_owners(organization) {
            if let Some(row) = ledger.members.iter_mut().find(|row| row.id == member) {
                row.role = before;
            }
            return Err(error);
        }
        ledger.audit.push(audit.clone());
        Ok(())
    }







    async fn create_access_token(
        &self,
        record: &TokenRecord,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(record.token.organization_id, record.token.member_id))?;
        let taken = ledger
            .tokens
            .iter()
            .any(|row| row.token.id == record.token.id || row.hash == record.hash);
        if taken {
            return Err(StoreError::Conflict);
        }
        ledger.tokens.push(record.clone());
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn access_token(&self, id: TokenId) -> Result<Option<TokenRecord>, StoreError> {
        Ok(locked(&self.ledger)
            .tokens
            .iter()
            .find(|row| row.token.id == id)
            .cloned())
    }

    async fn access_tokens(
        &self,
        organization: OrganizationId,
        member: MemberId,
    ) -> Result<Vec<AccessToken>, StoreError> {
        Ok(locked(&self.ledger)
            .tokens
            .iter()
            .filter(|row| {
                row.token.organization_id == organization && row.token.member_id == member
            })
            .map(|row| row.token.clone())
            .collect())
    }

    async fn revoke_access_token(
        &self,
        organization: OrganizationId,
        id: TokenId,
        at: i64,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let row = not_found(
            ledger
                .tokens
                .iter_mut()
                .find(|row| row.token.organization_id == organization && row.token.id == id),
        )?;
        row.token.revoked_at.get_or_insert(at);
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn create_sign_in_session(
        &self,
        record: &SignInRecord,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        not_found(ledger.member(record.session.organization_id, record.session.member_id))?;
        let taken = ledger
            .sign_ins
            .iter()
            .any(|row| row.session.id == record.session.id || row.hash == record.hash);
        if taken {
            return Err(StoreError::Conflict);
        }
        ledger.sign_ins.push(record.clone());
        ledger.audit.push(audit.clone());
        Ok(())
    }

    async fn sign_in_session(&self, id: SignInId) -> Result<Option<SignInRecord>, StoreError> {
        Ok(locked(&self.ledger)
            .sign_ins
            .iter()
            .find(|row| row.session.id == id)
            .cloned())
    }

    async fn end_sign_in_session(
        &self,
        organization: OrganizationId,
        id: SignInId,
        at: i64,
        audit: &AuditEntry,
    ) -> Result<(), StoreError> {
        let mut ledger = locked(&self.ledger);
        let row = not_found(
            ledger
                .sign_ins
                .iter_mut()
                .find(|row| row.session.organization_id == organization && row.session.id == id),
        )?;
        row.session.ended_at.get_or_insert(at);
        ledger.audit.push(audit.clone());
        Ok(())
    }
}
