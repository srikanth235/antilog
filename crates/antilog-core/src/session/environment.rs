//! Moving a running session to another version of its environment: what an agent's approved
//! change or a person's pick does. The network follows at once (the proxy reads it from the
//! session's machine row); variables and the setup script are sent to the machine when a runner
//! holds it, and otherwise apply at its next start, which reads the same row.

use antilog_types::{
    Approval, CardPayload, Command, Environment, EventKind, MemberId, OrganizationId, Session,
    SessionId,
};

use super::settle::attach_named;
use super::{SessionError, active, audit, now_ms, record, tell};
use crate::deps::Deps;
use crate::environments::{self, Unavailable};
use crate::organization::Refusal;
use crate::store::{EnvironmentStore, RunnerStore, SessionStore, StoreError};

/// What the move came to, in words for the agent and the conversation.
fn applies(before: &Environment, after: &Environment, running: bool) -> String {
    let mut said = Vec::new();
    let network = (
        before.network,
        &before.allowed_hosts,
        before.allow_package_managers,
    ) != (
        after.network,
        &after.allowed_hosts,
        after.allow_package_managers,
    );
    if network {
        said.push("The network change applies now.".to_owned());
    }
    if before.variables != after.variables {
        said.push(if running {
            "The variables are set for commands run from now on; a process that is already running keeps the old ones until the machine next starts.".to_owned()
        } else {
            "The variables are set when the machine next starts.".to_owned()
        });
    }
    if before.setup_script != after.setup_script && !after.setup_script.trim().is_empty() {
        said.push(if running {
            "The setup script is running now in the machine; if it fails you get a note and the session carries on.".to_owned()
        } else {
            "The setup script runs when the machine next starts.".to_owned()
        });
    }
    if said.is_empty() {
        "Nothing else applies.".to_owned()
    } else {
        said.join(" ")
    }
}

/// Points the session at `target`, adds the repos it brings and, when a runner holds the machine,
/// sends it the version. Returns what applies now and what at the next start.
pub(super) async fn switch<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    actor: Option<MemberId>,
    session: &Session,
    before: &Environment,
    target: &Environment,
) -> Result<String, SessionError> {
    let (id, version) = if target.built_in {
        (None, None)
    } else {
        (Some(target.id.as_str()), Some(target.version))
    };
    deps.store()
        .set_session_environment(
            organization,
            session.id,
            id,
            version,
            &audit(
                deps,
                organization,
                actor,
                "session.set_environment",
                session.id,
            ),
        )
        .await?;
    let running = deps
        .store()
        .machine(session.id)
        .await?
        .is_some_and(|machine| machine.state.is_held());
    let mut said = vec![applies(before, target, running)];
    for repo in &target.repos {
        if !session.repos.contains(repo) {
            said.push(attach_named(deps, organization, session.id, repo).await);
        }
    }
    if running {
        tell(
            deps,
            organization,
            session.id,
            &Command::ApplyEnvironment {
                environment: Box::new(target.spec()),
            },
        )
        .await?;
    }
    Ok(said.join(" "))
}

/// An approved Change environment card: the change goes onto the newest version of the session's
/// environment, which becomes the next version (the built-in default cannot change, so the
/// session gets an environment of its own), and the session moves to it.
pub(super) async fn change_environment<D: Deps>(deps: &D, approval: &Approval) -> String {
    let Some(CardPayload::ChangeEnvironment(proposal)) = &approval.payload else {
        return "Done.".to_owned();
    };
    let org = approval.organization_id;
    let result = async {
        let store = deps.store();
        let session = store
            .session(org, approval.session_id)
            .await?
            .ok_or(SessionError::NotFound)?;
        let before = environments::of_session(store, org, &session)
            .await
            .map_err(refused)?;
        let newest = environments::newest_of_session(store, org, &session)
            .await
            .map_err(refused)?;
        let name = if newest.built_in {
            crate::tools::own_name(session.id)
        } else {
            newest.name.clone()
        };
        let new = environments::changed(&newest, &proposal.change, &name).map_err(refused)?;
        let made = environments::save_for_session(
            store,
            org,
            approval.decided_by,
            now_ms(deps),
            Some(&newest),
            new,
        )
        .await
        .map_err(refused)?;
        let said = switch(deps, org, approval.decided_by, &session, &before, &made).await?;
        Ok::<_, SessionError>((made, newest, said))
    }
    .await;
    match result {
        Ok((made, newest, said)) => {
            let shared = if newest.built_in {
                format!(
                    "It is a new environment, {}, for this session alone until someone else picks it.",
                    made.name
                )
            } else {
                format!(
                    "It is now version {} of {}; other sessions on that environment get it when they start.",
                    made.version, made.name
                )
            };
            format!("{shared} {said}")
        }
        Err(error) => format!("The change could not be applied: {error}. Propose it again."),
    }
}

fn refused(error: Refusal) -> SessionError {
    match error {
        Refusal::Invalid(message) => SessionError::Invalid(message),
        Refusal::NotFound => SessionError::NotFound,
        _ => SessionError::Store(StoreError::Unavailable("environment".into())),
    }
}

/// A person moves the session to the newest version of an environment (`default` for the built-in
/// one). The agent is told, as it is of a decided card.
pub async fn move_to_environment<D: Deps>(
    deps: &D,
    organization: OrganizationId,
    id: SessionId,
    actor: Option<MemberId>,
    environment: &str,
) -> Result<Session, SessionError> {
    let session = active(deps, organization, id).await?;
    let store = deps.store();
    let target = if environment == environments::DEFAULT {
        Environment::built_in()
    } else {
        let found = store
            .environments(organization)
            .await?
            .into_iter()
            .find(|row| row.id == environment)
            .ok_or(Unavailable::Unknown)?;
        if found.archived {
            return Err(Unavailable::Archived.into());
        }
        found
    };
    let before = environments::of_session(store, organization, &session)
        .await
        .map_err(refused)?;
    if before.id == target.id && before.version == target.version {
        return Ok(session);
    }
    let said = switch(deps, organization, actor, &session, &before, &target).await?;
    let text = format!(
        "A person moved this session to the environment {} (version {}). {said}",
        target.name, target.version
    );
    record(
        deps,
        &session,
        format!("environment:{}", uuid::Uuid::now_v7()),
        EventKind::Agent,
        None,
        &text,
    )
    .await?;
    store.wake_machine(id, now_ms(deps)).await?;
    tell(deps, organization, id, &Command::Message { text }).await?;
    store
        .session(organization, id)
        .await?
        .ok_or(SessionError::NotFound)
}
