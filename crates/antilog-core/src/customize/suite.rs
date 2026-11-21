#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test support: a failed call fails the suite"
)]

//! One suite run against every store and deps: the fake here, Postgres in the server's tests.

use antilog_types::{
    ApprovalStatus, ConnectorHealth, Decision, NewConnector, NewPlugin, NewSkill, PartKind,
    PluginConnector, PluginPart, RemovePlugin, SetToolMode, SkillFile, ToolMode,
};
use serde_json::json;

use super::reach::{Outcome, Reach};
use super::{connectors, plugins, skills};
use crate::deps::Deps;
use crate::fakes::FakeConnectors;
use crate::organization::{Principal, Refusal};
use crate::sealed::Sealer;
use crate::secrets::Secret;
use crate::seed;
use crate::session::now_ms;
use crate::store::{ApprovalStore, CustomizeStore, SessionStore};
use crate::tools::Caller;

fn pack(name: &str, extra: &str) -> NewSkill {
    NewSkill {
        files: vec![
            SkillFile {
                path: "SKILL.md".into(),
                content: format!(
                    "---\nname: {name}\ndescription: Tunes {name}\n---\n# {name}\n{extra}"
                ),
            },
            SkillFile {
                path: "notes/more.md".into(),
                content: "More text.".into(),
            },
        ],
    }
}

pub async fn reach_suite<D: Deps>(deps: &D) {
    let store = deps.store();
    let seed = seed::organization(store).await;
    let organization = seed.organization.id;
    let owner = Principal {
        organization,
        member: seed.owner.id,
    };
    let now = now_ms(deps);

    // Skills: a pack is checked, each upload is the next version, versions never change.
    let first = skills::add(store, &owner, now, pack("postgres-tuning", "v1"))
        .await
        .expect("a skill is added");
    assert_eq!((first.name.as_str(), first.version), ("postgres-tuning", 1));
    let second = skills::add(store, &owner, now, pack("postgres-tuning", "v2"))
        .await
        .expect("the next version");
    assert_eq!(second.version, 2);
    let listed = skills::list(store, &owner).await.expect("listed");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].latest, listed[0].versions.clone()),
        (2, vec![1, 2])
    );
    let old = skills::version(store, &owner, "postgres-tuning", 1)
        .await
        .expect("v1");
    assert!(old.files.iter().any(|f| f.content.ends_with("v1")));
    for bad in [
        NewSkill { files: vec![] },
        NewSkill {
            files: vec![SkillFile {
                path: "readme.md".into(),
                content: "x".into(),
            }],
        },
        NewSkill {
            files: vec![SkillFile {
                path: "SKILL.md".into(),
                content: "no front matter".into(),
            }],
        },
        NewSkill {
            files: vec![SkillFile {
                path: "SKILL.md".into(),
                content: "---\nname: Bad Name\ndescription: d\n---\n".into(),
            }],
        },
        NewSkill {
            files: vec![
                SkillFile {
                    path: "SKILL.md".into(),
                    content: "---\nname: ok\ndescription: d\n---\n".into(),
                },
                SkillFile {
                    path: "../escape".into(),
                    content: "x".into(),
                },
            ],
        },
        NewSkill {
            files: vec![
                SkillFile {
                    path: "SKILL.md".into(),
                    content: "---\nname: ok\ndescription: d\n---\n".into(),
                },
                SkillFile {
                    path: "bin".into(),
                    content: "a\0b".into(),
                },
            ],
        },
        NewSkill {
            files: vec![
                SkillFile {
                    path: "SKILL.md".into(),
                    content: "---\nname: ok\ndescription: d\n---\n".into(),
                },
                SkillFile {
                    path: "big".into(),
                    content: "x".repeat(skills::MAX_FILE_BYTES + 1),
                },
            ],
        },
        NewSkill {
            files: vec![SkillFile {
                path: "SKILL.md".into(),
                content: format!(
                    "---\nname: ok\ndescription: d\n---\n-----BEGIN RSA {}-----",
                    "PRIVATE KEY"
                ),
            }],
        },
    ] {
        assert!(matches!(
            skills::add(store, &owner, now, bad).await,
            Err(Refusal::Invalid(_))
        ));
    }

    skills::add(store, &owner, now, pack("loose-skill", ""))
        .await
        .expect("another");
    skills::remove(store, &owner, now, "loose-skill")
        .await
        .expect("a skill goes with all its versions");

    // Connectors: sealed credential, tools that ask first.
    let client = FakeConnectors::default()
        .with_tool("query", "Run a read-only query")
        .with_tool("drop_table", "Dangerous")
        .requiring("s3cret");
    let sealer = Sealer::new(&Secret::new("ab".repeat(32))).expect("a key");
    let added = connectors::add(
        store,
        &client,
        Some(&sealer),
        &owner,
        now,
        NewConnector {
            name: "Analytics DB".into(),
            url: "https://mcp.example.com/mcp".into(),
            credential: Some("s3cret".into()),
        },
    )
    .await
    .expect("a connector is added");
    assert_eq!(added.health, ConnectorHealth::Connected);
    assert!(added.has_credential);
    assert!(
        added.tools.iter().all(|t| t.mode == ToolMode::Ask),
        "new tools ask first"
    );
    assert!(!format!("{added:?}").contains("s3cret"));
    let sealed = store
        .connector_credential(organization, added.id)
        .await
        .expect("read")
        .expect("kept");
    assert!(
        !String::from_utf8_lossy(&sealed).contains("s3cret"),
        "stored sealed"
    );
    assert_eq!(
        sealer
            .open(&sealed, &format!("connector:{}", added.id))
            .expect("opens")
            .expose(),
        "s3cret"
    );
    assert!(
        sealer.open(&sealed, "connector:other").is_err(),
        "bound to its row"
    );
    for url in [
        "http://mcp.example.com",
        "https://localhost/x",
        "https://10.0.0.5/",
        "https://intranet/",
        "ftp://x.com",
        "https://u@x.com/",
    ] {
        let refused = connectors::add(
            store,
            &client,
            Some(&sealer),
            &owner,
            now,
            NewConnector {
                name: "x".into(),
                url: url.into(),
                credential: None,
            },
        )
        .await;
        assert!(matches!(refused, Err(Refusal::Invalid(_))), "{url}");
    }
    assert!(matches!(
        connectors::add(
            store,
            &client,
            Some(&sealer),
            &owner,
            now,
            NewConnector {
                name: "analytics db".into(),
                url: "https://other.example.com".into(),
                credential: None
            }
        )
        .await,
        Err(Refusal::Conflict)
    ));
    connectors::set_tool_mode(
        store,
        &owner,
        now,
        added.id,
        "query",
        SetToolMode { mode: ToolMode::On },
    )
    .await
    .expect("on");
    connectors::set_tool_mode(
        store,
        &owner,
        now,
        added.id,
        "drop_table",
        SetToolMode {
            mode: ToolMode::Off,
        },
    )
    .await
    .expect("off");
    client.set_down(true);
    let again = connectors::recheck(store, &client, Some(&sealer), &owner, now, added.id)
        .await
        .expect("checked");
    assert_eq!(
        again.health,
        ConnectorHealth::Reconnect,
        "never healthy when it did not answer"
    );
    assert_eq!(again.tools.len(), 2, "its tools stay");
    client.set_down(false);
    let again = connectors::recheck(store, &client, Some(&sealer), &owner, now, added.id)
        .await
        .expect("checked");
    assert_eq!(again.health, ConnectorHealth::Connected);
    assert_eq!(
        again
            .tools
            .iter()
            .find(|t| t.name == "query")
            .map(|t| t.mode),
        Some(ToolMode::On)
    );

    // The session's endpoint: Antilog's tools, then the connector's, off ones left out.
    let session = seed::session(&seed, "opencode", now);
    store
        .create_session(
            &session,
            &antilog_types::AuditEntry::new(
                organization,
                Some(seed.owner.id),
                "session.start",
                session.id,
                antilog_types::AuditOutcome::Done,
                now,
            ),
        )
        .await
        .expect("a session");
    let scope = store
        .session_scope(session.id)
        .await
        .expect("scope")
        .expect("found");
    assert_eq!(
        (scope.organization, scope.member),
        (organization, seed.owner.id)
    );
    let reach = Reach {
        deps,
        client: &client,
        sealer: Some(&sealer),
    };
    let names: Vec<String> = reach
        .tools(&scope)
        .await
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert!(names.contains(&"attach_repo".to_owned()));
    assert!(names.contains(&"analytics-db__query".to_owned()));
    assert!(
        !names.contains(&"analytics-db__drop_table".to_owned()),
        "off is not offered"
    );
    let caller = |n: &str| Caller {
        organization,
        session: session.id,
        actor: seed.owner.id,
        call: n.into(),
    };
    let Outcome::Remote { text, refused } = reach
        .call(
            &scope,
            &caller("1"),
            "analytics-db__query",
            json!({"sql": "select 1"}),
        )
        .await
    else {
        panic!("a connector tool answers as a connector")
    };
    assert_eq!((text.as_str(), refused), ("query ran", false));
    let seen = client.calls();
    assert_eq!(
        seen.last().map(|c| c.bearer.clone()),
        Some(Some("s3cret".to_owned())),
        "the server holds the credential"
    );
    let Outcome::Remote { refused, .. } = reach
        .call(&scope, &caller("2"), "analytics-db__drop_table", json!({}))
        .await
    else {
        panic!("off is refused as a connector")
    };
    assert!(refused);
    assert_eq!(
        client.calls().len(),
        seen.len(),
        "an off tool is never forwarded"
    );
    let log = store.audit_log(organization, None, 200).await.expect("log");
    assert!(
        log.iter().any(
            |e| e.action == "connector.use" && e.outcome == antilog_types::AuditOutcome::Denied
        )
    );
    assert!(
        log.iter()
            .any(|e| e.action == "connector.use" && e.outcome == antilog_types::AuditOutcome::Done)
    );

    // Ask first: a card, then one call per approval, for exactly the input approved.
    connectors::set_tool_mode(
        store,
        &owner,
        now,
        added.id,
        "query",
        SetToolMode {
            mode: ToolMode::Ask,
        },
    )
    .await
    .expect("ask");
    let calls_before = client.calls().len();
    let asked = json!({"sql": "select 2"});
    let Outcome::Antilog(reply) = reach
        .call(&scope, &caller("3"), "analytics-db__query", asked.clone())
        .await
    else {
        panic!("a card is raised")
    };
    assert_eq!(reply.answer["status"], "waiting for a person");
    let Outcome::Antilog(_) = reach
        .call(&scope, &caller("4"), "analytics-db__query", asked.clone())
        .await
    else {
        panic!("still waiting")
    };
    let pending: Vec<_> = store
        .approvals(organization, session.id)
        .await
        .expect("cards")
        .into_iter()
        .filter(|c| c.status == ApprovalStatus::Pending)
        .collect();
    assert_eq!(pending.len(), 1, "no second card for the same call");
    assert_eq!(
        client.calls().len(),
        calls_before,
        "nothing is forwarded before approval"
    );
    crate::approval::decide(store, &owner, now, pending[0].id, Decision::Approve)
        .await
        .expect("approved");
    let Outcome::Antilog(_) = reach
        .call(
            &scope,
            &caller("5"),
            "analytics-db__query",
            json!({"sql": "select 3"}),
        )
        .await
    else {
        panic!("a different input needs its own card")
    };
    let Outcome::Remote { text, .. } = reach
        .call(&scope, &caller("6"), "analytics-db__query", asked.clone())
        .await
    else {
        panic!("the approved call goes through")
    };
    assert_eq!(text, "query ran");
    assert_eq!(client.calls().len(), calls_before + 1);
    let Outcome::Antilog(reply) = reach
        .call(&scope, &caller("7"), "analytics-db__query", asked)
        .await
    else {
        panic!("an approval pays for one call")
    };
    assert_eq!(reply.answer["status"], "waiting for a person");

    // Plugins: unpacked into governed parts; removal asks per part.
    let bundle = NewPlugin {
        name: "Postgres kit".into(),
        description: "Tuning pack".into(),
        version: "1.0.0".into(),
        skills: vec![pack("pg-explain", "")],
        connectors: vec![PluginConnector {
            name: "Kit DB".into(),
            url: "https://kit.example.com/mcp".into(),
        }],
    };
    let plugin = plugins::add(store, false, &owner, now, bundle)
        .await
        .expect("a plugin");
    assert_eq!(plugin.id, "postgres-kit");
    assert_eq!(plugin.parts.len(), 2);
    assert!(
        skills::list(store, &owner)
            .await
            .expect("l")
            .iter()
            .any(|s| s.name == "pg-explain" && s.from_plugin.as_deref() == Some("postgres-kit"))
    );
    let kit = connectors::list(store, &owner)
        .await
        .expect("l")
        .into_iter()
        .find(|c| c.slug == "kit-db")
        .expect("kit db");
    assert_eq!(
        (kit.health, kit.has_credential),
        (ConnectorHealth::NotChecked, false)
    );
    plugins::remove(
        store,
        &owner,
        now,
        "postgres-kit",
        RemovePlugin {
            keep: vec![PluginPart {
                kind: PartKind::Connector,
                name: "kit-db".into(),
            }],
        },
    )
    .await
    .expect("removed, keeping the connector");
    assert!(plugins::list(store, &owner).await.expect("l").is_empty());
    let left = connectors::list(store, &owner).await.expect("l");
    assert!(left.iter().any(|c| c.slug == "kit-db"), "kept");
    assert!(
        !skills::list(store, &owner)
            .await
            .expect("l")
            .iter()
            .any(|s| s.name == "pg-explain"),
        "not kept: gone"
    );

    // Roles: a viewer reads and changes nothing; an editor uploads skills and nothing else here.
    let viewer = crate::organization::add_member(
        store,
        &owner,
        now,
        "vi",
        None,
        antilog_types::Role::Viewer,
    )
    .await
    .expect("v");
    let viewer = Principal {
        organization,
        member: viewer.id,
    };
    assert!(connectors::list(store, &viewer).await.is_ok());
    assert!(matches!(
        skills::add(store, &viewer, now, pack("nope", "")).await,
        Err(Refusal::Denied)
    ));
    assert!(matches!(
        connectors::remove(store, &viewer, now, added.id).await,
        Err(Refusal::Denied)
    ));
}
