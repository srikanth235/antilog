//! Sign-in: a provider proves who someone is, a member of the organization gets a session.

use std::future::Future;

use antilog_types::{
    AuditEntry, AuditOutcome, Identity, Member, Organization, OrganizationId, Role, SignInSession,
};

use crate::organization::{Principal, Refusal};
use crate::store::Store;
use crate::token::{self, IssuedSession, SecretKind};

/// Random values bound to one sign-in attempt, kept in a cookie between start and callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub state: String,
    pub nonce: String,
}

impl Challenge {
    pub fn new() -> Result<Self, token::RandomUnavailable> {
        let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(Self {
            state: hex(token::entropy()?),
            nonce: hex(token::entropy()?),
        })
    }

    pub fn cookie(&self) -> String {
        format!("{}.{}", self.state, self.nonce)
    }

    pub fn from_cookie(value: &str) -> Option<Self> {
        let (state, nonce) = value.split_once('.')?;
        Some(Self {
            state: state.to_owned(),
            nonce: nonce.to_owned(),
        })
    }
}

/// Who the provider says someone is. Never an email: the provider's stable subject is the identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proven {
    pub identity: Identity,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    #[error("the provider refused the sign-in")]
    Refused,
    #[error("the provider is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait SignInProvider: Send + Sync {
    fn name(&self) -> &str;

    /// Where to send the person; the provider's endpoints are configuration.
    fn authorize_url(
        &self,
        challenge: &Challenge,
    ) -> impl Future<Output = Result<String, SignInError>> + Send;

    /// Exchanges the code the provider returned for a proven identity.
    fn complete(
        &self,
        code: &str,
        challenge: &Challenge,
    ) -> impl Future<Output = Result<Proven, SignInError>> + Send;
}

/// For agents and tests only, and never configured on a server people sign in to:
/// the code `fake:<subject>` proves that subject.
pub struct FakeSignIn;

impl SignInProvider for FakeSignIn {
    fn name(&self) -> &str {
        "fake"
    }

    async fn authorize_url(&self, challenge: &Challenge) -> Result<String, SignInError> {
        Ok(format!("fake://sign-in?state={}", challenge.state))
    }

    async fn complete(&self, code: &str, _: &Challenge) -> Result<Proven, SignInError> {
        let subject = code
            .strip_prefix("fake:")
            .filter(|subject| !subject.is_empty() && subject.len() <= 200)
            .ok_or(SignInError::Refused)?;
        Ok(Proven {
            identity: Identity {
                provider: "fake".to_owned(),
                subject: subject.to_owned(),
            },
            name: subject.to_owned(),
        })
    }
}

/// A signed-in member: their session's secret is shown once.
#[derive(Debug)]
pub struct SignedIn {
    pub member: Member,
    pub issued: IssuedSession,
}

fn audit(
    organization: OrganizationId,
    member: Option<&Member>,
    action: &str,
    target: impl ToString,
    outcome: AuditOutcome,
    now: i64,
) -> AuditEntry {
    AuditEntry::new(
        organization,
        member.map(|member| member.id),
        action,
        target,
        outcome,
        now,
    )
}

/// Signs a proven identity in to an organization they are a member of. A stranger is refused and
/// audited. With no organization named, the identity's own memberships are looked up and the oldest
/// signs in; `me` lists the rest. A stranger to every organization has none to audit it in.
pub async fn sign_in<S: Store>(
    store: &S,
    now: i64,
    organization: Option<OrganizationId>,
    proven: &Proven,
) -> Result<SignedIn, Refusal> {
    let organization = match organization {
        Some(organization) => organization,
        None => store
            .memberships(&proven.identity)
            .await?
            .first()
            .map(|found| found.organization.id)
            .ok_or(Refusal::Denied)?,
    };
    store
        .organization(organization)
        .await?
        .ok_or(Refusal::NotFound)?;
    let Some(member) = store
        .member_by_identity(organization, &proven.identity)
        .await?
    else {
        store
            .append_audit(&audit(
                organization,
                None,
                "sign_in",
                &proven.identity.provider,
                AuditOutcome::Denied,
                now,
            ))
            .await?;
        return Err(Refusal::Denied);
    };
    let issued = token::issue_session(organization, member.id, now)?;
    store
        .create_sign_in_session(
            &issued.record,
            &audit(
                organization,
                Some(&member),
                "sign_in",
                issued.record.session.id,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(SignedIn { member, issued })
}

/// A new organization with its first owner, signed in. The caller decides who may do this.
pub async fn create_organization<S: Store>(
    store: &S,
    now: i64,
    name: &str,
    proven: &Proven,
) -> Result<(Organization, SignedIn), Refusal> {
    let length = name.trim().chars().count();
    if !(1..=200).contains(&length) {
        return Err(Refusal::Invalid(
            "an organization's name is 1 to 200 characters",
        ));
    }
    let organization = Organization::new(name.trim(), now);
    store.create_organization(&organization).await?;
    let owner = Member::person(
        organization.id,
        proven.name.trim().chars().take(200).collect::<String>(),
        proven.identity.clone(),
        Role::Owner,
        now,
    );
    store
        .add_member(
            &owner,
            &audit(
                organization.id,
                Some(&owner),
                "organization.create",
                organization.id,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    let signed = sign_in(store, now, Some(organization.id), proven).await?;
    Ok((organization, signed))
}

pub async fn sign_out<S: Store>(
    store: &S,
    session: &SignInSession,
    now: i64,
) -> Result<(), Refusal> {
    store
        .end_sign_in_session(
            session.organization_id,
            session.id,
            now,
            &AuditEntry::new(
                session.organization_id,
                Some(session.member_id),
                "sign_out",
                session.id,
                AuditOutcome::Done,
                now,
            ),
        )
        .await?;
    Ok(())
}

/// How a request proved who it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    AccessToken,
    SignIn(SignInSession),
}

/// Resolves a presented secret (access token or session) to a member that still exists.
pub async fn authenticate<S: Store>(
    store: &S,
    secret: &str,
    now: i64,
) -> Result<Option<(Principal, Credential)>, Refusal> {
    let found = match token::kind(secret) {
        Some(SecretKind::AccessToken) => token::authenticate(store, secret).await?.map(|token| {
            (
                Principal {
                    organization: token.organization_id,
                    member: token.member_id,
                },
                Credential::AccessToken,
            )
        }),
        Some(SecretKind::SignIn) => {
            token::authenticate_session(store, secret, now)
                .await?
                .map(|session| {
                    (
                        Principal {
                            organization: session.organization_id,
                            member: session.member_id,
                        },
                        Credential::SignIn(session),
                    )
                })
        }
        None => None,
    };
    Ok(found)
}
