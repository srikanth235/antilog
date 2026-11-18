use antilog_core::auth::{self, Proven};
use antilog_core::evaluations::{self, slug};
use antilog_core::fakes::FakeStore;
use antilog_core::organization::{self as org, Principal, Refusal};
use antilog_types::{EvaluationKind, Identity, NewEvaluationTemplate, Role};

const T0: i64 = 1_700_000_000_000;

fn new(name: &str) -> NewEvaluationTemplate {
    NewEvaluationTemplate {
        name: name.into(),
        summary: "Replays traffic".into(),
        kind: EvaluationKind::Speed,
        metric: "p95".into(),
        lower_is_better: true,
        command: "k6 run bench/load.js".into(),
        protected_paths: vec!["app:bench/".into()],
    }
}

#[test]
fn a_name_becomes_a_slug() {
    assert_eq!(slug("Load test (p95)"), "load-test-p95");
    assert_eq!(slug("  --  "), "");
}

#[tokio::test]
async fn editors_add_viewers_read_and_a_name_is_taken_once() {
    let store = FakeStore::default();
    let proven = |s: &str| Proven {
        identity: Identity {
            provider: "fake".into(),
            subject: s.into(),
        },
        name: s.into(),
    };
    let (organization, signed) = auth::create_organization(&store, T0, "acme", &proven("ada"))
        .await
        .unwrap();
    let owner = Principal {
        organization: organization.id,
        member: signed.member.id,
    };
    let who = |subject: &str, role| {
        let identity = proven(subject).identity;
        let store = &store;
        let subject = subject.to_owned();
        async move {
            let member = org::add_member(store, &owner, T0, &subject, Some(identity), role)
                .await
                .unwrap();
            Principal {
                organization: owner.organization,
                member: member.id,
            }
        }
    };
    let editor = who("ed", Role::Editor).await;
    let viewer = who("vi", Role::Viewer).await;

    assert!(matches!(
        evaluations::add(&store, &viewer, T0, new("Load test")).await,
        Err(Refusal::Denied)
    ));
    let made = evaluations::add(&store, &editor, T0, new("Load test"))
        .await
        .unwrap();
    assert_eq!(
        (made.id.as_str(), made.created_by),
        ("load-test", editor.member)
    );
    assert!(matches!(
        evaluations::add(&store, &editor, T0, new("load  TEST")).await,
        Err(Refusal::Conflict)
    ));
    assert!(matches!(
        evaluations::add(&store, &editor, T0, new("  ")).await,
        Err(Refusal::Invalid(_))
    ));
    assert_eq!(
        evaluations::list(&store, &viewer).await.unwrap(),
        vec![made.clone()]
    );
    assert_eq!(
        evaluations::get(&store, &viewer, "load-test")
            .await
            .unwrap(),
        made
    );
    assert!(matches!(
        evaluations::get(&store, &viewer, "nope").await,
        Err(Refusal::NotFound)
    ));
}
