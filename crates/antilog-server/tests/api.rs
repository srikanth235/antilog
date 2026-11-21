#![expect(
    clippy::indexing_slicing,
    reason = "tests read a JSON body or a fixed block"
)]

//! The organization floor over HTTP, on the in-memory store and the fake sign-in provider.

use std::sync::Arc;

use antilog_core::auth::FakeSignIn;
use antilog_core::fakes::FakeDeps;
use antilog_server::api::auth::App;
use antilog_server::api::auth::{Provider, Settings};
use antilog_server::api::customize::Backend;
use antilog_server::api::{openapi_json, router, router_with_reach};
use antilog_server::connectors::HttpConnectors;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app(fake: bool, open: bool) -> Router {
    let providers = if fake {
        vec![Provider::Fake(FakeSignIn)]
    } else {
        Vec::new()
    };
    let app = Arc::new(App {
        deps: Arc::new(FakeDeps::new()),
        settings: Settings {
            providers,
            open_organizations: open,
            secure_cookies: true,
            runner: None,
        },
    });
    router_with_reach(
        app,
        antilog_core::fakes::FakeCredentials::new(Arc::new(antilog_core::clock::SystemClock)),
        Backend {
            client: HttpConnectors::new(false).unwrap_or_else(|_| unreachable!("a client builds")),
            sealer: None,
        },
    )
}

struct Reply {
    status: StatusCode,
    cookies: Vec<String>,
    body: Value,
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    auth: Option<&str>,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Reply {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(secret) = auth {
        request = request.header(header::AUTHORIZATION, format!("Bearer {secret}"));
    }
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    };
    let response = app
        .clone()
        .oneshot(request.unwrap_or_else(|e| unreachable!("{e}")))
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_owned))
        .collect();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap_or_default();
    Reply {
        status,
        cookies,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

fn cookie_value(cookies: &[String], name: &str) -> Option<String> {
    cookies
        .iter()
        .find_map(|c| {
            c.strip_prefix(&format!("{name}="))
                .and_then(|rest| rest.split(';').next())
                .map(str::to_owned)
        })
        .filter(|value| !value.is_empty())
}

/// Starts a sign-in and returns the state and the challenge cookie to send back.
async fn start(app: &Router) -> (String, String) {
    let started = send(app, Method::GET, "/auth/fake/start", None, None, None).await;
    assert_eq!(started.status, StatusCode::OK);
    let state = started.body["state"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let challenge = cookie_value(&started.cookies, "antilog_challenge").unwrap_or_default();
    (state, format!("antilog_challenge={challenge}"))
}

async fn create_organization(app: &Router, name: &str, subject: &str) -> Reply {
    let (state, cookie) = start(app).await;
    send(
        app,
        Method::POST,
        "/auth/fake/organization",
        None,
        Some(&cookie),
        Some(json!({"name": name, "code": format!("fake:{subject}"), "state": state})),
    )
    .await
}

struct Org {
    app: Router,
    owner: String,
    organization: String,
}

async fn org() -> Org {
    let app = app(true, true);
    let created = create_organization(&app, "acme", "ada").await;
    assert_eq!(created.status, StatusCode::CREATED);
    Org {
        owner: created.body["session"]["secret"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        organization: created.body["organization"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        app,
    }
}

impl Org {
    async fn call(&self, method: Method, uri: &str, who: &str, body: Option<Value>) -> Reply {
        send(&self.app, method, uri, Some(who), None, body).await
    }

    /// A member of this organization with `role`, and a bearer token for them.
    async fn member(&self, subject: &str, role: &str) -> (String, String) {
        let added = self.call(Method::POST, "/members", &self.owner, Some(json!({"name": subject, "role": role, "identity": {"provider": "fake", "subject": subject}}))).await;
        assert_eq!(added.status, StatusCode::CREATED);
        let (state, cookie) = start(&self.app).await;
        let signed = send(&self.app, Method::POST, "/auth/fake/callback", None, Some(&cookie), Some(json!({"organization": self.organization, "code": format!("fake:{subject}"), "state": state}))).await;
        assert_eq!(signed.status, StatusCode::OK);
        (
            added.body["id"].as_str().unwrap_or_default().to_owned(),
            signed.body["secret"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        )
    }
}

#[tokio::test]
async fn every_route_that_names_a_bearer_refuses_a_caller_without_one() {
    let doc: Value = serde_json::from_str(&openapi_json()).unwrap_or_default();
    let app = app(true, true);
    let mut walked = 0;
    for (path, item) in doc["paths"].as_object().into_iter().flatten() {
        for (method, operation) in item.as_object().into_iter().flatten() {
            if operation.get("security").is_none() {
                continue;
            }
            let uri = path
                .split('/')
                .map(|part| {
                    if part.starts_with('{') {
                        "018f3b1e-0000-7000-8000-000000000000"
                    } else {
                        part
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            let method =
                Method::from_bytes(method.to_uppercase().as_bytes()).unwrap_or(Method::GET);
            for auth in [None, Some("alt_x_y"), Some("als_x_y")] {
                let reply = send(&app, method.clone(), &uri, auth, None, Some(json!({}))).await;
                assert_eq!(
                    reply.status,
                    StatusCode::UNAUTHORIZED,
                    "{method} {path} with {auth:?}"
                );
                assert_eq!(reply.body["code"], "unauthenticated");
            }
            walked += 1;
        }
    }
    assert!(walked >= 18, "only {walked} routes were walked");
}

#[tokio::test]
async fn sign_in_by_cookie_and_sign_out() {
    let org = org().await;
    let (_, _) = org.member("grace", "viewer").await;
    let (state, cookie) = start(&org.app).await;
    let signed = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(json!({"organization": org.organization, "code": "fake:grace", "state": state})),
    )
    .await;
    assert_eq!(signed.status, StatusCode::OK);
    let raw = signed
        .cookies
        .iter()
        .find(|c| c.starts_with("antilog_session="))
        .cloned()
        .unwrap_or_default();
    for flag in [
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/",
        "Max-Age=43200",
    ] {
        assert!(raw.contains(flag), "{flag} in {raw}");
    }
    let session = format!(
        "antilog_session={}",
        cookie_value(&signed.cookies, "antilog_session").unwrap_or_default()
    );
    let me = send(
        &org.app,
        Method::GET,
        "/organization",
        None,
        Some(&session),
        None,
    )
    .await;
    assert_eq!(
        (me.status, me.body["name"].as_str()),
        (StatusCode::OK, Some("acme"))
    );
    let out = send(
        &org.app,
        Method::POST,
        "/auth/sign-out",
        None,
        Some(&session),
        None,
    )
    .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    let after = send(
        &org.app,
        Method::GET,
        "/organization",
        None,
        Some(&session),
        None,
    )
    .await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_sign_in_needs_its_own_challenge_and_a_member() {
    let org = org().await;
    let (state, cookie) = start(&org.app).await;
    let body = |state: &str, code: &str| json!({"organization": org.organization, "code": code, "state": state});
    let stranger = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(body(&state, "fake:mallory")),
    )
    .await;
    assert_eq!(stranger.status, StatusCode::FORBIDDEN);
    let forged_state = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(body("other", "fake:ada")),
    )
    .await;
    assert_eq!(forged_state.status, StatusCode::UNAUTHORIZED);
    let no_cookie = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        None,
        Some(body(&state, "fake:ada")),
    )
    .await;
    assert_eq!(no_cookie.status, StatusCode::UNAUTHORIZED);
    let bad_code = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(body(&state, "ada")),
    )
    .await;
    assert_eq!(bad_code.status, StatusCode::UNAUTHORIZED);
    let elsewhere = send(&org.app, Method::POST, "/auth/fake/callback", None, Some(&cookie), Some(json!({"organization": "018f3b1e-0000-7000-8000-000000000000", "code": "fake:ada", "state": state}))).await;
    assert_eq!(elsewhere.status, StatusCode::NOT_FOUND);
    let log = org.call(Method::GET, "/audit", &org.owner, None).await;
    let denied = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["action"] == "sign_in" && e["outcome"] == "denied")
        .count();
    assert_eq!(denied, 1);
}

#[tokio::test]
async fn providers_and_organization_creation_are_configuration() {
    assert_eq!(
        send(
            &app(false, true),
            Method::GET,
            "/auth/providers",
            None,
            None,
            None
        )
        .await
        .body,
        json!({"providers": []})
    );
    assert_eq!(
        send(
            &app(true, true),
            Method::GET,
            "/auth/providers",
            None,
            None,
            None
        )
        .await
        .body,
        json!({"providers": ["fake"]})
    );
    assert_eq!(
        send(
            &app(false, true),
            Method::GET,
            "/auth/fake/start",
            None,
            None,
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        create_organization(&app(true, false), "acme", "ada")
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let both = app(true, true);
    assert_eq!(
        create_organization(&both, "acme", "ada").await.status,
        StatusCode::CREATED
    );
    assert_eq!(
        create_organization(&both, "acme", "grace").await.status,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn roles_gate_the_writes_and_every_denial_is_audited() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (editor_id, editor) = org.member("ed", "editor").await;

    let start = json!({"prompt": "cut p95", "model": "opencode/test"});
    let session = org
        .call(Method::POST, "/sessions", &editor, Some(start.clone()))
        .await;
    assert_eq!(session.status, StatusCode::CREATED, "an editor starts one");
    assert_eq!(
        org.call(Method::POST, "/sessions", &viewer, Some(start))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        org.call(
            Method::PUT,
            &format!("/members/{editor_id}/role"),
            &editor,
            Some(json!({"role": "owner"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        org.call(
            Method::PUT,
            "/policies",
            &editor,
            Some(json!({"card": "change_environment", "mode": "allow", "limitCents": null}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        org.call(Method::GET, "/audit", &viewer, None).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        org.call(
            Method::PUT,
            "/organization/limits",
            &editor,
            Some(json!({"monthlyCapCents": 5, "defaultBudgetCents": null}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    for who in [&viewer, &editor] {
        assert_eq!(
            org.call(Method::GET, "/sessions", who, None).await.status,
            StatusCode::OK,
            "every member reads"
        );
        assert_eq!(
            org.call(Method::GET, "/policies", who, None)
                .await
                .body
                .as_array()
                .map(Vec::len),
            Some(6)
        );
    }

    let log = org.call(Method::GET, "/audit", &org.owner, None).await;
    let denied: Vec<&str> = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["outcome"] == "denied")
        .filter_map(|e| e["action"].as_str())
        .collect();
    for action in [
        "session.start",
        "member.set_role",
        "policy.set",
        "audit.read",
        "organization.set_limits",
    ] {
        assert!(denied.contains(&action), "{action} in {denied:?}");
    }
    let done = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["action"] == "session.start" && e["outcome"] == "done")
        .count();
    assert_eq!(done, 1);
}

#[tokio::test]
async fn an_organization_keeps_an_owner() {
    let org = org().await;
    let me = org.call(Method::GET, "/members", &org.owner, None).await;
    let owner_id = me.body[0]["id"].as_str().unwrap_or_default().to_owned();
    let demote = org
        .call(
            Method::PUT,
            &format!("/members/{owner_id}/role"),
            &org.owner,
            Some(json!({"role": "editor"})),
        )
        .await;
    assert_eq!(
        (demote.status, demote.body["code"].as_str()),
        (StatusCode::CONFLICT, Some("last_owner"))
    );
}

#[tokio::test]
async fn tokens_act_as_their_member_and_secrets_are_shown_once() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let created = org
        .call(
            Method::POST,
            "/tokens",
            &viewer,
            Some(json!({"memberId": null, "name": "laptop"})),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let secret = created.body["secret"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let id = created.body["token"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(secret.starts_with("alt_"));
    assert_eq!(
        org.call(Method::GET, "/organization", &secret, None)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        org.call(
            Method::POST,
            "/sessions",
            &secret,
            Some(json!({"prompt": "x", "model": "opencode/test"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN,
        "a token has its member's role"
    );

    let listed = org.call(Method::GET, "/tokens", &secret, None).await;
    let everything = format!(
        "{} {} {}",
        listed.body,
        org.call(Method::GET, "/members", &org.owner, None)
            .await
            .body,
        org.call(Method::GET, "/audit", &org.owner, None).await.body
    );
    assert!(
        !everything.contains(&secret) && !everything.contains("als_"),
        "no secret outside its creation"
    );
    assert_eq!(
        org.call(Method::DELETE, &format!("/tokens/{id}"), &secret, None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        org.call(Method::GET, "/organization", &secret, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let service = org
        .call(
            Method::POST,
            "/members",
            &org.owner,
            Some(json!({"name": "ci", "role": "editor", "identity": null})),
        )
        .await;
    let service_id = service.body["id"].as_str().unwrap_or_default().to_owned();
    assert_eq!(
        org.call(
            Method::POST,
            "/tokens",
            &viewer,
            Some(json!({"memberId": service_id, "name": "x"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        org.call(
            Method::POST,
            "/tokens",
            &org.owner,
            Some(json!({"memberId": service_id, "name": "ci"}))
        )
        .await
        .status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn policy_budgets_and_the_audit_page() {
    let org = org().await;
    let put = |body: Value| org.call(Method::PUT, "/policies", &org.owner, Some(body));
    assert_eq!(
        put(json!({"card": "raise_budget", "mode": "allow_up_to", "limitCents": 500}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let credential =
        put(json!({"card": "add_credential", "mode": "allow", "limitCents": null})).await;
    assert_eq!(
        (credential.status, credential.body["code"].as_str()),
        (StatusCode::CONFLICT, Some("always_ask"))
    );
    assert_eq!(
        put(json!({"card": "change_environment", "mode": "allow_up_to", "limitCents": null}))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let policies = org.call(Method::GET, "/policies", &org.owner, None).await;
    assert!(
        policies
            .body
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| p["card"] == "raise_budget" && p["limitCents"] == 500)
    );

    let limits = org
        .call(
            Method::PUT,
            "/organization/limits",
            &org.owner,
            Some(json!({"monthlyCapCents": 10_000, "defaultBudgetCents": 1_000})),
        )
        .await;
    assert_eq!(limits.status, StatusCode::NO_CONTENT);
    let cap = |value: Option<u32>| json!({"monthlyCapCents": 10_000, "defaultBudgetCents": 1_000, "maxRunningSessions": value});
    let own = org
        .call(Method::GET, "/organization", &org.owner, None)
        .await;
    assert_eq!(
        own.body["maxRunningSessions"], 10,
        "ten unless an owner says otherwise"
    );
    let set = org
        .call(
            Method::PUT,
            "/organization/limits",
            &org.owner,
            Some(cap(Some(3))),
        )
        .await;
    assert_eq!(set.status, StatusCode::NO_CONTENT);
    let own = org
        .call(Method::GET, "/organization", &org.owner, None)
        .await;
    assert_eq!(own.body["maxRunningSessions"], 3);
    // Omitted: unchanged. Zero: refused.
    org.call(
        Method::PUT,
        "/organization/limits",
        &org.owner,
        Some(cap(None)),
    )
    .await;
    let own = org
        .call(Method::GET, "/organization", &org.owner, None)
        .await;
    assert_eq!(own.body["maxRunningSessions"], 3);
    let zero = org
        .call(
            Method::PUT,
            "/organization/limits",
            &org.owner,
            Some(cap(Some(0))),
        )
        .await;
    assert_eq!(zero.status, StatusCode::UNPROCESSABLE_ENTITY);
    let session = org
        .call(
            Method::POST,
            "/sessions",
            &org.owner,
            Some(json!({"prompt": "go", "model": "opencode/test", "budgetCapCents": 500})),
        )
        .await;
    let id = session.body["id"].as_str().unwrap_or_default().to_owned();
    let over = org
        .call(
            Method::PUT,
            &format!("/sessions/{id}/budget"),
            &org.owner,
            Some(json!({"budgetCapCents": 20_000})),
        )
        .await;
    assert_eq!(
        (over.status, over.body["code"].as_str()),
        (StatusCode::CONFLICT, Some("over_cap")),
        "a session's budget stays inside the organization's cap"
    );
    let own = org
        .call(
            Method::PUT,
            &format!("/sessions/{id}/budget"),
            &org.owner,
            Some(json!({"budgetCapCents": 2_000})),
        )
        .await;
    assert_eq!(own.status, StatusCode::NO_CONTENT);
    let defaulted = org
        .call(
            Method::POST,
            "/sessions",
            &org.owner,
            Some(json!({"prompt": "go", "model": "opencode/test"})),
        )
        .await;
    assert_eq!(
        defaulted.body["budgetCapCents"], 1_000,
        "the organization's default per-session cap"
    );
    let summary = org.call(Method::GET, "/budgets", &org.owner, None).await;
    assert_eq!(
        summary.body["organization"],
        json!({"capCents": 10_000, "spentCents": 0})
    );
    assert_eq!(
        org.call(
            Method::PUT,
            "/organization/limits",
            &org.owner,
            Some(json!({"monthlyCapCents": -1, "defaultBudgetCents": null}))
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let first = org
        .call(Method::GET, "/audit?limit=2", &org.owner, None)
        .await;
    assert_eq!(first.body["entries"].as_array().map(Vec::len), Some(2));
    let cursor = &first.body["next"];
    let uri = format!(
        "/audit?limit=200&beforeAt={}&beforeId={}",
        cursor["at"],
        cursor["id"].as_str().unwrap_or_default()
    );
    let rest = org.call(Method::GET, &uri, &org.owner, None).await;
    let seen: Vec<&Value> = first.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(rest.body["entries"].as_array().into_iter().flatten())
        .collect();
    let mut ids: Vec<&str> = seen.iter().filter_map(|e| e["id"].as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), seen.len(), "pages do not overlap");
    assert_eq!(
        org.call(Method::GET, "/audit?beforeAt=5", &org.owner, None)
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn unknown_ids_are_not_found_and_leak_nothing() {
    let org = org().await;
    let nobody = "018f3b1e-0000-7000-8000-000000000000";
    assert_eq!(
        org.call(
            Method::GET,
            &format!("/sessions/{nobody}/approvals"),
            &org.owner,
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        org.call(
            Method::POST,
            &format!("/approvals/{nobody}/decision"),
            &org.owner,
            Some(json!({"decision": "approve"}))
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        org.call(
            Method::PUT,
            &format!("/sessions/{nobody}/budget"),
            &org.owner,
            Some(json!({"budgetCapCents": 1}))
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        org.call(
            Method::DELETE,
            &format!("/tokens/{nobody}"),
            &org.owner,
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        org.call(
            Method::PUT,
            &format!("/members/{nobody}/role"),
            &org.owner,
            Some(json!({"role": "viewer"}))
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let other = create_organization(&org.app, "other", "bob").await;
    let theirs = other.body["session"]["secret"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let mine = org
        .call(
            Method::POST,
            "/tokens",
            &org.owner,
            Some(json!({"memberId": null, "name": "t"})),
        )
        .await;
    let id = mine.body["token"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(
        org.call(Method::DELETE, &format!("/tokens/{id}"), &theirs, None)
            .await
            .status,
        StatusCode::NOT_FOUND,
        "another organization's token does not exist here"
    );
    assert_eq!(
        org.call(Method::GET, "/members", &theirs, None)
            .await
            .body
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

/// Generated clients key operations by id and read parameters by where they sit, so both must
/// be right in the document: ids unique, and every path parameter named in its path.
#[test]
fn openapi_operation_ids_are_unique_and_path_parameters_are_in_the_path() {
    let doc: serde_json::Value =
        serde_json::from_str(&antilog_server::api::openapi_json()).expect("the document is JSON");
    let mut ids = std::collections::BTreeSet::new();
    for (path, item) in doc["paths"].as_object().expect("paths") {
        for (method, op) in item.as_object().expect("a path item") {
            let id = op["operationId"]
                .as_str()
                .expect("every operation has an id");
            assert!(
                ids.insert(id.to_owned()),
                "operationId {id} is used twice ({method} {path})"
            );
            for param in op["parameters"].as_array().into_iter().flatten() {
                if param["in"] == "path" {
                    let name = param["name"].as_str().expect("a parameter name");
                    assert!(
                        path.contains(&format!("{{{name}}}")),
                        "{method} {path}: path parameter {name} is not in the path"
                    );
                }
            }
        }
    }
}

/// An organization on a store the test can also reach, for what only a tool writes.
async fn org_with_store() -> (Org, Arc<FakeDeps>) {
    let deps = Arc::new(FakeDeps::new());
    let app = router(
        deps.clone(),
        Settings {
            providers: vec![Provider::Fake(FakeSignIn)],
            open_organizations: true,
            secure_cookies: true,
            runner: None,
        },
    );
    let created = create_organization(&app, "acme", "ada").await;
    assert_eq!(created.status, StatusCode::CREATED);
    let org = Org {
        owner: created.body["session"]["secret"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        organization: created.body["organization"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        app,
    };
    (org, deps)
}

#[tokio::test]
async fn me_names_the_member_and_signing_in_needs_no_organization_id() {
    let org = org().await;
    let (_, token) = org.member("grace", "viewer").await;

    let me = org.call(Method::GET, "/me", &token, None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["member"]["name"], "grace");
    assert_eq!(me.body["role"], "viewer");
    assert_eq!(me.body["organization"]["name"], "acme");
    assert_eq!(me.body["organizations"][0]["id"], json!(org.organization));
    assert!(me.body["member"]["identity"]["provider"] == "fake");

    // No organization in the request: the identity's membership is found.
    let (state, cookie) = start(&org.app).await;
    let signed = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(json!({"code": "fake:grace", "state": state})),
    )
    .await;
    assert_eq!(signed.status, StatusCode::OK);
    assert_eq!(
        signed.body["member"]["organizationId"],
        json!(org.organization)
    );
    let (state, cookie) = start(&org.app).await;
    let stranger = send(
        &org.app,
        Method::POST,
        "/auth/fake/callback",
        None,
        Some(&cookie),
        Some(json!({"code": "fake:mallory", "state": state})),
    )
    .await;
    assert_eq!(stranger.status, StatusCode::FORBIDDEN);

    // The same person in a second organization sees both, and each session is its own.
    let globex = create_organization(&org.app, "globex", "grace").await;
    assert_eq!(globex.status, StatusCode::CREATED);
    let secret = globex.body["session"]["secret"]
        .as_str()
        .unwrap_or_default();
    let there = org.call(Method::GET, "/me", secret, None).await;
    assert_eq!(there.body["organization"]["name"], "globex");
    let names: Vec<_> = there.body["organizations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| o["name"].as_str())
        .collect();
    assert_eq!(names, ["acme", "globex"]);
    let still = org.call(Method::GET, "/me", &token, None).await;
    assert_eq!(still.body["organization"]["name"], "acme");
}

#[tokio::test]
async fn pending_cards_the_transcript_and_spend_are_read_across_roles() {
    use antilog_core::approval::{self, CardRequest};
    use antilog_core::store::{BudgetStore, NewEvent, SessionStore};
    use antilog_types::{CardKind, OrganizationId, SessionId, Spend, SpendId, SpendKind};

    let (org, deps) = org_with_store().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;
    let made = org
        .call(
            Method::POST,
            "/sessions",
            &editor,
            Some(json!({"prompt": "make it faster", "model": "opencode/test", "budgetCapCents": 500})),
        )
        .await;
    assert_eq!(made.status, StatusCode::CREATED);
    let session_id = made.body["id"].as_str().unwrap_or_default().to_owned();
    let session_uuid = SessionId::from_uuid(session_id.parse().unwrap_or_default());
    let organization = OrganizationId::from_uuid(org.organization.parse().unwrap_or_default());
    let card = approval::raise(
        &deps.store,
        1_700_000_000_000,
        None,
        CardRequest {
            organization,
            session: session_uuid,
            card: CardKind::RaiseBudget,
            summary: "Raise the budget by $5".into(),
            payload: None,
            cost_cents: Some(500),
            follow_policy: false,
        },
    )
    .await
    .unwrap_or_else(|e| unreachable!("{e}"));

    let seen = |reply: &Reply| -> Vec<String> {
        reply
            .body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["id"].as_str().map(str::to_owned))
            .collect()
    };
    let for_editor = org
        .call(Method::GET, "/approvals/pending", &editor, None)
        .await;
    assert_eq!(seen(&for_editor), vec![card.id.to_string()]);
    let for_viewer = org
        .call(Method::GET, "/approvals/pending", &viewer, None)
        .await;
    assert_eq!(
        (for_viewer.status, seen(&for_viewer)),
        (StatusCode::OK, Vec::new())
    );
    let decided = org
        .call(
            Method::POST,
            &format!("/approvals/{}/decision", card.id),
            &editor,
            Some(json!({"decision": "approve"})),
        )
        .await;
    assert_eq!(decided.status, StatusCode::OK);
    let after = org
        .call(Method::GET, "/approvals/pending", &editor, None)
        .await;
    assert!(seen(&after).is_empty());

    // The session's conversation and spend.
    let session = deps
        .store
        .session(organization, session_uuid)
        .await
        .unwrap_or_else(|e| unreachable!("{e}"))
        .unwrap_or_else(|| unreachable!("the session exists"));
    for (i, (kind, text)) in [
        (antilog_types::EventKind::Person, "make it faster"),
        (antilog_types::EventKind::Agent, "Profiling."),
        (antilog_types::EventKind::Tool, "measure"),
    ]
    .into_iter()
    .enumerate()
    {
        deps.store
            .append_event(&NewEvent {
                organization,
                session: session.id,
                key: format!("k{i}"),
                kind,
                member: None,
                text: text.into(),
                at: 1_700_000_000_000 + i64::try_from(i).unwrap_or_default(),
            })
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
    }
    let page = org
        .call(
            Method::GET,
            &format!("/sessions/{}/events?limit=4", session.id),
            &viewer,
            None,
        )
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.body["events"].as_array().map(Vec::len), Some(4));
    let next = page.body["next"].as_i64().unwrap_or_default();
    let rest = org
        .call(
            Method::GET,
            &format!("/sessions/{}/events?after={next}", session.id),
            &viewer,
            None,
        )
        .await;
    assert_eq!(rest.body["events"][0]["text"], "measure");
    assert_eq!(rest.body["events"][0]["kind"], "tool");
    assert!(rest.body["next"].is_null());
    let missing = org
        .call(
            Method::GET,
            "/sessions/018f3b1e-0000-7000-8000-000000000000/events",
            &viewer,
            None,
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    for (kind, cents) in [(SpendKind::Tokens, 40), (SpendKind::Machine, 25)] {
        deps.store
            .record_spend(&Spend {
                id: SpendId::new(),
                organization_id: organization,
                session_id: session.id,
                kind,
                amount_cents: cents,
                tokens: 0,
                occurred_at: 1_700_000_000_000,
            })
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
    }
    let spend = org
        .call(
            Method::GET,
            &format!("/sessions/{session_id}/spend"),
            &viewer,
            None,
        )
        .await;
    assert_eq!(spend.status, StatusCode::OK);
    assert_eq!(spend.body["spentCents"], 65);
    assert_eq!(
        spend.body["budgetCapCents"], 1_000,
        "the approved card raised the budget by $5"
    );
    assert_eq!(spend.body["byKind"].as_array().map(Vec::len), Some(2));
    assert_eq!(spend.body["sessionId"], json!(session.id.to_string()));
}

#[tokio::test]
async fn agents_auto_and_environments_are_read_by_members_and_set_by_owners() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;

    let listed = org.call(Method::GET, "/agents", &viewer, None).await;
    assert_eq!(listed.body[0]["id"], "opencode");
    assert_eq!(listed.body[0]["builtIn"], true);
    let auto = org.call(Method::GET, "/agents/auto", &viewer, None).await;
    assert_eq!(auto.body["agent"]["id"], "opencode");
    let environments = org.call(Method::GET, "/environments", &viewer, None).await;
    assert_eq!(environments.body[0]["id"], "default");

    // "auto" in a new session is resolved by the server to a named agent.
    let started = org
        .call(
            Method::POST,
            "/sessions",
            &editor,
            Some(json!({"agent": "auto", "prompt": "go", "repos": [], "model": "opencode/test"})),
        )
        .await;
    assert_eq!(
        (started.status, &started.body["agent"]),
        (StatusCode::CREATED, &json!("opencode"))
    );

    let body = json!({"name": "Codex", "enabled": true, "isDefault": false, "costTier": "high"});
    let denied = org
        .call(Method::PUT, "/agents/codex", &editor, Some(body.clone()))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let set = org
        .call(Method::PUT, "/agents/codex", &org.owner, Some(body))
        .await;
    assert_eq!(
        (set.status, &set.body["builtIn"]),
        (StatusCode::OK, &json!(false))
    );
    let two = org
        .call(
            Method::PUT,
            "/agents/gemini",
            &org.owner,
            Some(json!({"name": "Gemini", "enabled": true, "isDefault": false, "costTier": "low"})),
        )
        .await;
    assert_eq!(two.status, StatusCode::OK);
    let asks = org.call(Method::GET, "/agents/auto", &viewer, None).await;
    assert_eq!(
        (asks.status, &asks.body["code"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("invalid"))
    );
    let refused = org
        .call(
            Method::POST,
            "/sessions",
            &editor,
            Some(json!({"agent": "auto", "prompt": "go", "repos": [], "model": "opencode/test"})),
        )
        .await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    let named = org
        .call(
            Method::POST,
            "/sessions",
            &editor,
            Some(json!({"agent": "gemini", "prompt": "go", "repos": [], "model": "opencode/test"})),
        )
        .await;
    assert_eq!(named.status, StatusCode::CREATED);
    let bad = org
        .call(
            Method::PUT,
            "/agents/Bad",
            &org.owner,
            Some(json!({"name": "x", "enabled": true, "isDefault": false, "costTier": "low"})),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn model_keys_are_set_by_an_owner_listed_by_anyone_and_never_returned() {
    let org = org().await;
    let (_, editor) = org.member("ed", "editor").await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let secret = "sk-ant-test-value";

    assert_eq!(
        org.call(Method::GET, "/model-keys", &viewer, None)
            .await
            .body,
        json!([])
    );
    let denied = org
        .call(
            Method::PUT,
            "/model-keys/anthropic",
            &editor,
            Some(json!({"key": secret})),
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    for bad in ["", "has space", "line\nbreak"] {
        let refused = org
            .call(
                Method::PUT,
                "/model-keys/anthropic",
                &org.owner,
                Some(json!({"key": bad})),
            )
            .await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad:?}");
    }
    let unknown = org
        .call(
            Method::PUT,
            "/model-keys/gemini",
            &org.owner,
            Some(json!({"key": secret})),
        )
        .await;
    assert!(unknown.status.is_client_error());

    let set = org
        .call(
            Method::PUT,
            "/model-keys/anthropic",
            &org.owner,
            Some(json!({"key": secret})),
        )
        .await;
    assert_eq!(set.status, StatusCode::OK);
    assert_eq!(set.body["provider"], "anthropic");
    // Replacing is the same call.
    let replaced = org
        .call(
            Method::PUT,
            "/model-keys/anthropic",
            &org.owner,
            Some(json!({"key": "sk-ant-new"})),
        )
        .await;
    assert_eq!(replaced.status, StatusCode::OK);

    let listed = org.call(Method::GET, "/model-keys", &viewer, None).await;
    assert_eq!(listed.body.as_array().map(Vec::len), Some(1));
    assert_eq!(listed.body[0]["provider"], "anthropic");
    for reply in [&set, &replaced, &listed] {
        let text = reply.body.to_string();
        assert!(!text.contains("sk-ant"), "a key came back: {text}");
    }

    let denied = org
        .call(Method::DELETE, "/model-keys/anthropic", &editor, None)
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let removed = org
        .call(Method::DELETE, "/model-keys/anthropic", &org.owner, None)
        .await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let again = org
        .call(Method::DELETE, "/model-keys/anthropic", &org.owner, None)
        .await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);

    // The log names the provider and the actor, never the key.
    let audit = org.call(Method::GET, "/audit", &org.owner, None).await;
    let log = audit.body.to_string();
    assert!(log.contains("model_key.set") && log.contains("model_key.remove"));
    assert!(log.contains("model_key.set") && !log.contains("sk-ant"));
}

fn environment_body(name: &str) -> Value {
    json!({
        "name": name, "network": "limited", "allowedHosts": ["API.stripe.com"],
        "allowPackageManagers": true, "variables": {"STRIPE_KEY": "sk_test"},
        "setupScript": "npm ci", "cpus": 4, "memoryMb": 16384
    })
}

#[tokio::test]
async fn environments_are_read_by_every_member_and_managed_by_owners_and_editors() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;

    // The built-in default is always there, and a viewer reads.
    let listed = org.call(Method::GET, "/environments", &viewer, None).await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.body.as_array().map(Vec::len), Some(1));
    assert_eq!(listed.body[0]["builtIn"], true);

    // A viewer may not make, change or archive one, and the denial is audited.
    let body = environment_body("Checkout staging");
    for (method, uri) in [
        (Method::POST, "/environments"),
        (Method::PUT, "/environments/checkout-staging"),
        (Method::POST, "/environments/checkout-staging/archive"),
    ] {
        let denied = org.call(method, uri, &viewer, Some(body.clone())).await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN, "{uri}");
    }
    let log = org.call(Method::GET, "/audit", &org.owner, None).await;
    let denials = log.body["entries"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row["outcome"] == "denied"
                        && row["action"]
                            .as_str()
                            .is_some_and(|a| a.starts_with("environment."))
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(denials, 3, "{:?}", log.body);

    // An editor makes one: version 1, the id from the name, host names cleaned.
    let made = org
        .call(Method::POST, "/environments", &editor, Some(body.clone()))
        .await;
    assert_eq!(made.status, StatusCode::CREATED, "{:?}", made.body);
    assert_eq!(made.body["id"], "checkout-staging");
    assert_eq!(made.body["version"], 1);
    assert_eq!(made.body["allowedHosts"], json!(["api.stripe.com"]));
    assert_eq!(made.body["builtIn"], false);
    let again = org
        .call(
            Method::POST,
            "/environments",
            &org.owner,
            Some(body.clone()),
        )
        .await;
    assert_eq!(again.status, StatusCode::CONFLICT);

    // An owner's edit is the next version.
    let mut changed = body.clone();
    changed["network"] = json!("none");
    let edited = org
        .call(
            Method::PUT,
            "/environments/checkout-staging",
            &org.owner,
            Some(changed),
        )
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{:?}", edited.body);
    assert_eq!(
        (
            edited.body["version"].clone(),
            edited.body["network"].clone()
        ),
        (json!(2), json!("none"))
    );
    let listed = org.call(Method::GET, "/environments", &viewer, None).await;
    assert_eq!(listed.body.as_array().map(Vec::len), Some(2));
    assert_eq!(listed.body[1]["version"], 2);

    // Nothing the machine's wiring depends on, and no host that is not a name, is accepted.
    for bad in [
        json!({"variables": {"HTTPS_PROXY": "http://evil"}}),
        json!({"variables": {"ANTILOG_TOKEN": "x"}}),
        json!({"variables": {"1BAD": "x"}}),
        json!({"allowedHosts": ["*.stripe.com"]}),
        json!({"allowedHosts": ["10.0.0.1"]}),
        json!({"allowedHosts": ["https://stripe.com"]}),
        json!({"setupScript": "x".repeat(64 * 1024 + 1)}),
        json!({"cpus": 0}),
        json!({"memoryMb": 1}),
        json!({"name": "  "}),
    ] {
        let mut body = environment_body("Other");
        for (key, value) in bad.as_object().into_iter().flatten() {
            body[key] = value.clone();
        }
        let refused = org
            .call(Method::POST, "/environments", &editor, Some(body))
            .await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    // The built-in default cannot be taken, changed or archived.
    let default = org
        .call(
            Method::POST,
            "/environments",
            &editor,
            Some(environment_body("Default")),
        )
        .await;
    assert_eq!(default.status, StatusCode::CONFLICT);
    let default = org
        .call(
            Method::PUT,
            "/environments/default",
            &editor,
            Some(environment_body("Default")),
        )
        .await;
    assert_eq!(default.status, StatusCode::UNPROCESSABLE_ENTITY);
    let missing = org
        .call(
            Method::PUT,
            "/environments/nothing",
            &editor,
            Some(environment_body("Nothing")),
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    // A session starts on the stored environment, then not once it is archived.
    let sessions = "/sessions".to_owned();
    let start = |environment: &str| json!({"agent": "auto", "prompt": "go", "repos": [], "environment": environment, "model": "opencode/test"});
    let started = org
        .call(
            Method::POST,
            &sessions,
            &editor,
            Some(start("checkout-staging")),
        )
        .await;
    assert_eq!(started.status, StatusCode::CREATED, "{:?}", started.body);
    let unknown = org
        .call(Method::POST, &sessions, &editor, Some(start("nothing")))
        .await;
    assert_eq!(unknown.status, StatusCode::UNPROCESSABLE_ENTITY);
    let archived = org
        .call(
            Method::POST,
            "/environments/checkout-staging/archive",
            &editor,
            None,
        )
        .await;
    assert_eq!(archived.status, StatusCode::NO_CONTENT);
    let refused = org
        .call(
            Method::POST,
            &sessions,
            &editor,
            Some(start("checkout-staging")),
        )
        .await;
    assert_eq!(
        (refused.status, &refused.body["code"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!("invalid_environment")
        )
    );
    let listed = org.call(Method::GET, "/environments", &viewer, None).await;
    assert_eq!(listed.body[1]["archived"], true);
    let frozen = org
        .call(
            Method::PUT,
            "/environments/checkout-staging",
            &editor,
            Some(environment_body("Checkout staging")),
        )
        .await;
    assert_eq!(frozen.status, StatusCode::UNPROCESSABLE_ENTITY);
    // The default is always offered.
    let on_default = org
        .call(Method::POST, &sessions, &editor, Some(start("default")))
        .await;
    assert_eq!(on_default.status, StatusCode::CREATED);
}
