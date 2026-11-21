#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the test"
)]

use antilog_types::{
    AuditEntry, AuditOutcome, Identity, Member, Organization, Role, Session, SessionParams,
};

use crate::store::Store;

pub const T0: i64 = 1_800_000_000_000;

pub struct Seed {
    pub organization: Organization,
    pub owner: Member,
}

/// A session of this seed's organization, started by its owner.
pub fn session(seed: &Seed, agent: &str, at: i64) -> Session {
    Session::new(
        seed.organization.id,
        seed.owner.id,
        SessionParams {
            title: "cut p95".into(),
            agent: agent.into(),
            model: "opencode/test".into(),
            ..SessionParams::default()
        },
        at,
    )
}

/// An organization with an owner.
pub async fn organization<S: Store>(store: &S) -> Seed {
    let organization = Organization::new(format!("acme-{}", uuid::Uuid::now_v7()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("an organization is created");
    let owner = Member::person(
        organization.id,
        "Grace",
        Identity {
            provider: "fake".into(),
            subject: uuid::Uuid::now_v7().to_string(),
        },
        Role::Owner,
        T0,
    );
    let log = |action: &str, target: String| {
        AuditEntry::new(
            organization.id,
            Some(owner.id),
            action,
            target,
            AuditOutcome::Done,
            T0,
        )
    };
    store
        .add_member(&owner, &log("member.add", owner.id.to_string()))
        .await
        .expect("an owner is added");
    Seed {
        organization,
        owner,
    }
}

/// What every `CardStore` must do: apply a card once.
pub async fn card_suite<S: Store>(store: &S) {
    use antilog_types::{Approval, ApprovalId, ApprovalStatus, CardKind};

    let seed = organization(store).await;
    let org = seed.organization.id;
    let log = |action: &str| {
        AuditEntry::new(
            org,
            Some(seed.owner.id),
            action,
            "x",
            AuditOutcome::Done,
            T0,
        )
    };
    let session = session(&seed, "claude", T0);
    store
        .create_session(&session, &log("session.start"))
        .await
        .expect("a session is created");
    let approval = Approval {
        id: ApprovalId::new(),
        organization_id: org,
        session_id: session.id,
        card: CardKind::AddCredential,
        summary: "Add a credential".into(),
        payload: None,
        cost_cents: None,
        status: ApprovalStatus::Pending,
        decided_by: None,
        created_at: T0,
        decided_at: None,
    };
    store
        .request_approval(&approval, &log("tool.request_credential"))
        .await
        .expect("requested");
    assert!(store.mark_applied(org, approval.id).await.expect("marked"));
    assert!(!store.mark_applied(org, approval.id).await.expect("marked"));
}

/// What every `RunnerStore` must do: lease sessions to a runner, hand each
/// command over once, send an unfinished one again after a requeue, and keep exports.
pub async fn runner_suite<S: Store>(store: &S) {
    use antilog_types::{Command, MachineState, Seed, SessionId, SessionStatus};
    use uuid::Uuid;

    use crate::store::{SessionChange, StoreError};

    let mut sessions = Vec::new();
    // A runner serves one organization, so both sessions are in the same one.
    let seed = organization(store).await;
    for _ in 0..2 {
        let org = seed.organization.id;
        let session = session(&seed, "opencode", T0);
        let log = AuditEntry::new(
            org,
            Some(seed.owner.id),
            "session.start",
            session.id,
            AuditOutcome::Done,
            T0,
        );
        store
            .create_session(&session, &log)
            .await
            .expect("a session is created");
        let seed_of = Seed {
            repos: vec!["acme/api".into()],
            environment: None,
            environment_version: None,
        };
        store
            .enqueue_machine(org, session.id, &seed_of, T0)
            .await
            .expect("queued");
        assert!(matches!(
            store.enqueue_machine(org, session.id, &seed_of, T0).await,
            Err(StoreError::Conflict)
        ));
        sessions.push((org, session));
    }
    let mut both = sessions.into_iter();
    let (org, first) = both.next().expect("two sessions");
    let (_, second) = both.next().expect("two sessions");
    let (org, first, second) = (&org, &first, &second);
    let queued = store
        .machine(first.id)
        .await
        .expect("read")
        .expect("a machine");
    assert_eq!(queued.state, MachineState::Queued);
    assert_eq!(queued.seed.repos, vec!["acme/api".to_owned()]);
    assert_eq!(queued.agent, "opencode");
    assert_eq!(
        store
            .machine(antilog_types::SessionId::new())
            .await
            .expect("read"),
        None
    );

    // One runner takes work up to its limit, once each.
    let claimed = store.claim("r1", 1, 1_000, 61_000).await.expect("claim");
    assert_eq!(claimed.len(), 1);
    let mine = claimed.first().expect("one");
    assert_eq!(mine.state, MachineState::Claimed);
    assert_eq!(mine.holder.as_deref(), Some("r1"));
    let rest = store.claim("r1", 5, 1_000, 61_000).await.expect("claim");
    assert_eq!(rest.len(), 1);
    assert_ne!(rest.first().expect("one").session, mine.session);
    assert!(
        store
            .claim("r2", 5, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );

    // A lease is renewed by its holder and by nobody else.
    assert_eq!(
        store
            .renew("r1", 30_000, 90_000)
            .await
            .expect("renew")
            .len(),
        2
    );
    assert!(
        store
            .renew("r2", 30_000, 90_000)
            .await
            .expect("renew")
            .is_empty()
    );

    // Commands go to the holder once, oldest first.
    let message = Command::Message {
        text: "hello".into(),
    };
    let one = store
        .push_command(*org, first.id, &message, T0)
        .await
        .expect("push");
    let two = store
        .push_command(*org, first.id, &Command::Abort, T0)
        .await
        .expect("push");
    assert!(store.take_commands("r2").await.expect("take").is_empty());
    let taken = store.take_commands("r1").await.expect("take");
    assert_eq!(
        taken.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![one, two]
    );
    assert_eq!(taken.first().expect("one").command, message);
    assert!(store.take_commands("r1").await.expect("take").is_empty());
    assert!(matches!(
        store
            .push_command(*org, SessionId::new(), &Command::Abort, T0)
            .await,
        Err(StoreError::NotFound)
    ));

    // A finished command is not sent again after a requeue; an unfinished one is.
    store.finish_commands("r1", &[one]).await.expect("finish");
    store.finish_commands("r2", &[two]).await.expect("finish");
    assert_eq!(
        store
            .claim("r2", 5, 200_000, 260_000)
            .await
            .expect("claim")
            .len(),
        2
    );
    let again = store.take_commands("r2").await.expect("take");
    let ids: Vec<Uuid> = again
        .iter()
        .filter(|e| e.session == first.id)
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, vec![two]);
    assert!(
        !store
            .set_machine_state(first.id, "r1", MachineState::Idle, 200_000)
            .await
            .expect("state"),
        "the lapsed holder no longer holds it"
    );

    // An approved repo joins the seed once.
    assert!(
        store
            .add_seed_repo(first.id, "acme/lib")
            .await
            .expect("add")
    );
    assert!(
        !store
            .add_seed_repo(first.id, "acme/lib")
            .await
            .expect("again")
    );
    let seeded = store.machine(first.id).await.expect("read").expect("row");
    assert_eq!(
        seeded.seed.repos.last().map(String::as_str),
        Some("acme/lib")
    );

    // Idle, then let go with an export; a later message queues it again to resume.
    assert!(
        store
            .set_machine_state(first.id, "r2", MachineState::Idle, 200_000)
            .await
            .expect("state")
    );
    assert!(!store.wake_machine(first.id, 200_000).await.expect("wake"));
    store
        .put_export(*org, first.id, "{\"export\":1}", 200_000)
        .await
        .expect("export");
    assert!(
        store
            .set_machine_state(first.id, "r2", MachineState::Released, 200_000)
            .await
            .expect("state")
    );
    assert!(
        store
            .machine(first.id)
            .await
            .expect("read")
            .expect("row")
            .holder
            .is_none()
    );
    assert!(
        store
            .take_commands("r2")
            .await
            .expect("take")
            .iter()
            .all(|e| e.session != first.id)
    );
    assert!(store.wake_machine(first.id, 210_000).await.expect("wake"));
    let woken = store.machine(first.id).await.expect("read").expect("row");
    assert_eq!((woken.state, woken.resume), (MachineState::Queued, true));
    assert_eq!(
        store.export(first.id).await.expect("read").as_deref(),
        Some("{\"export\":1}")
    );
    assert_eq!(store.export(second.id).await.expect("read"), None);

    // A session that has ended is not claimed again.
    let ended = SessionChange {
        status: Some(SessionStatus::Done),
        note: None,
    };
    store
        .update_session(*org, first.id, &ended, 220_000, None)
        .await
        .expect("done");
    assert!(
        store
            .claim("r3", 5, 220_000, 280_000)
            .await
            .expect("claim")
            .is_empty()
    );
}

/// A runner locks to the organization of the first session it claims, and claims only that
/// organization's sessions until it exits (`unlock_runner`), so customers never share a host.
pub async fn runner_lock_suite<S: Store>(store: &S) {
    use antilog_types::{MachineState, Seed};

    let mut queued = Vec::new();
    for _ in 0..2 {
        let seed = organization(store).await;
        let org = seed.organization.id;
        for _ in 0..2 {
            let session = session(&seed, "opencode", T0);
            let log = AuditEntry::new(
                org,
                Some(seed.owner.id),
                "session.start",
                session.id,
                AuditOutcome::Done,
                T0,
            );
            store
                .create_session(&session, &log)
                .await
                .expect("a session is created");
            let seed_of = Seed {
                repos: Vec::new(),
                environment: None,
                environment_version: None,
            };
            store
                .enqueue_machine(org, session.id, &seed_of, T0)
                .await
                .expect("queued");
            queued.push((org, session.id));
        }
    }
    let at = |n: usize| *queued.get(n).expect("four sessions");
    let (org_a, org_b) = (at(0).0, at(2).0);
    assert_ne!(org_a, org_b);

    // A runner with no room takes nothing and locks to nothing.
    assert!(
        store
            .claim("busy", 0, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );

    // The first claim locks r1 to that session's organization; it never takes the other's.
    let first = store.claim("r1", 1, 1_000, 61_000).await.expect("claim");
    assert_eq!(first.len(), 1);
    let locked_to = first.first().expect("one").organization;
    let (other, other_org) = if locked_to == org_a {
        (at(2).1, org_b)
    } else {
        (at(0).1, org_a)
    };
    let more = store.claim("r1", 10, 1_000, 61_000).await.expect("claim");
    assert_eq!(more.len(), 1, "the rest of its own organization's sessions");
    assert!(more.iter().all(|m| m.organization == locked_to));
    assert!(
        store
            .claim("r1", 10, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );
    let b = store.machine(other).await.expect("read").expect("row");
    assert_eq!((b.state, b.organization), (MachineState::Queued, other_org));

    // Another runner takes the other organization, and only that.
    let theirs = store.claim("r2", 10, 1_000, 61_000).await.expect("claim");
    assert_eq!(theirs.len(), 2);
    assert!(theirs.iter().all(|m| m.organization == other_org));

    // Everything is let go and queued again, both organizations' sessions.
    for (holder, held) in [
        ("r1", first.iter().chain(more.iter()).collect::<Vec<_>>()),
        ("r2", theirs.iter().collect()),
    ] {
        for machine in held {
            assert!(
                store
                    .set_machine_state(machine.session, holder, MachineState::Released, 2_000)
                    .await
                    .expect("state")
            );
            assert!(
                store
                    .wake_machine(machine.session, 2_000)
                    .await
                    .expect("wake")
            );
        }
    }

    // The lock outlives the leases: r2 holds nothing, but the other organization's sessions are
    // still not its to take. r1 takes one of its own.
    let again = store.claim("r1", 1, 3_000, 63_000).await.expect("claim");
    assert_eq!(again.len(), 1);
    assert_eq!(again.first().expect("one").organization, locked_to);
    let b_again = store.claim("r2", 10, 3_000, 63_000).await.expect("claim");
    assert_eq!(b_again.len(), 2);
    assert!(b_again.iter().all(|m| m.organization == other_org));

    // A runner that still holds a session is not unlocked by saying it is leaving.
    store.unlock_runner("r1").await.expect("unlock");
    let rest = store.claim("r1", 10, 3_100, 63_100).await.expect("claim");
    assert!(rest.iter().all(|m| m.organization == locked_to));

    // Once it holds nothing, leaving lets the lock go, and it locks afresh to whoever is next.
    for machine in again.iter().chain(rest.iter()) {
        store
            .set_machine_state(machine.session, "r1", MachineState::Done, 4_000)
            .await
            .expect("state");
    }
    // The other organization's sessions wait again.
    for machine in &b_again {
        store
            .set_machine_state(machine.session, "r2", MachineState::Released, 4_050)
            .await
            .expect("state");
        assert!(
            store
                .wake_machine(machine.session, 4_050)
                .await
                .expect("wake")
        );
    }
    assert!(
        store
            .claim("r1", 10, 4_100, 64_100)
            .await
            .expect("claim")
            .is_empty(),
        "r1 is still locked to the first organization, which has nothing queued"
    );
    store.unlock_runner("r1").await.expect("unlock");
    let afresh = store.claim("r1", 10, 4_200, 64_200).await.expect("claim");
    assert_eq!(afresh.len(), 2);
    assert!(afresh.iter().all(|m| m.organization == other_org));
}

/// A queued session for `seed`'s organization.
async fn queue_session<S: Store>(store: &S, seed: &Seed, at: i64) -> antilog_types::SessionId {
    let org = seed.organization.id;
    let session = session(seed, "opencode", at);
    let log = AuditEntry::new(
        org,
        Some(seed.owner.id),
        "session.start",
        session.id,
        AuditOutcome::Done,
        at,
    );
    store
        .create_session(&session, &log)
        .await
        .expect("a session is created");
    let machine_seed = antilog_types::Seed {
        repos: Vec::new(),
        environment: None,
        environment_version: None,
    };
    store
        .enqueue_machine(org, session.id, &machine_seed, at)
        .await
        .expect("queued");
    session.id
}

/// The limits on the session machine: a lock whose runner stopped polling lapses after ten
/// minutes, an organization is handed no claim beyond its cap, and a machine that has had its
/// time is flagged once.
pub async fn runner_limits_suite<S: Store>(store: &S) {
    use antilog_types::MachineState;

    const MINUTE: i64 = 60_000;
    let lapse = crate::store::RUNNER_LOCK_LAPSE_MS;

    // A lock lapses: r1 locks to the first organization, and a poll keeps it there.
    let a = organization(store).await;
    let b = organization(store).await;
    let in_a = queue_session(store, &a, T0).await;
    let first = store.claim("r1", 1, 1_000, 61_000).await.expect("claim");
    assert_eq!(first.first().map(|m| m.session), Some(in_a));
    let in_b = queue_session(store, &b, T0).await;
    let beat = 1_000 + 9 * MINUTE;
    assert!(
        store
            .claim("r1", 0, beat, beat + MINUTE)
            .await
            .expect("claim")
            .is_empty()
    );
    let later = beat + 9 * MINUTE;
    let again = store
        .claim("r1", 5, later, later + MINUTE)
        .await
        .expect("claim");
    assert_eq!(
        again.iter().map(|m| m.session).collect::<Vec<_>>(),
        vec![in_a],
        "eighteen minutes after the first claim but nine after its last poll: still locked"
    );
    store
        .set_machine_state(in_a, "r1", MachineState::Done, later)
        .await
        .expect("state");
    // Silent for more than ten minutes: the lock is gone and the name is free for anyone.
    let now = later + lapse + 1;
    let taken = store
        .claim("r1", 5, now, now + MINUTE)
        .await
        .expect("claim");
    assert_eq!(
        taken.iter().map(|m| m.session).collect::<Vec<_>>(),
        vec![in_b],
        "the lapsed runner's name now takes the other organization's session"
    );
    store
        .set_machine_state(in_b, "r1", MachineState::Done, now)
        .await
        .expect("state");

    // A cap: an organization of two running sessions is handed no third while two run.
    let c = organization(store).await;
    let cap_audit = AuditEntry::new(
        c.organization.id,
        Some(c.owner.id),
        "organization.set_limits",
        c.organization.id,
        AuditOutcome::Done,
        T0,
    );
    store
        .set_organization_limits(c.organization.id, None, None, Some(2), &cap_audit)
        .await
        .expect("limits");
    let read = store
        .organization(c.organization.id)
        .await
        .expect("read")
        .expect("organization");
    assert_eq!(read.max_running_sessions, 2);
    let mut queued = Vec::new();
    for _ in 0..3 {
        queued.push(queue_session(store, &c, T0).await);
    }
    let at = now + 5 * MINUTE;
    let got = store.claim("r9", 10, at, at + MINUTE).await.expect("claim");
    assert_eq!(got.len(), 2, "two places, three queued");
    assert!(
        store
            .claim("r8", 10, at, at + MINUTE)
            .await
            .expect("claim")
            .is_empty(),
        "no runner is handed the third while two run"
    );
    let waiting = store.claim("r9", 10, at, at + MINUTE).await.expect("claim");
    assert!(waiting.is_empty(), "the same runner too");
    let released = got.first().expect("two");
    store
        .set_machine_state(released.session, "r9", MachineState::Released, at)
        .await
        .expect("state");
    let third = store.claim("r9", 10, at, at + MINUTE).await.expect("claim");
    assert_eq!(
        third.len(),
        1,
        "a place opened and the queued session took it"
    );
    assert!(queued.contains(&third.first().expect("one").session));

    // Machine time: flagged once, only after the cutoff, and only for this holder.
    assert!(
        store
            .mark_overdue("r9", at - 1)
            .await
            .expect("overdue")
            .is_empty(),
        "claimed at `at`, not before"
    );
    let over = store.mark_overdue("r9", at + 1).await.expect("overdue");
    assert_eq!(over.len(), 2);
    assert!(
        store
            .mark_overdue("r9", at + 1)
            .await
            .expect("overdue")
            .is_empty(),
        "the release is sent once"
    );
    assert!(
        store
            .mark_overdue("r8", at + 1)
            .await
            .expect("overdue")
            .is_empty()
    );
}
