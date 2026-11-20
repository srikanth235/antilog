#![expect(
    clippy::indexing_slicing,
    reason = "tests read a JSON body or a fixed block"
)]

//! The organization floor over HTTP, on the in-memory store and the fake sign-in provider.

use std::sync::Arc;

use antilog_core::auth::FakeSignIn;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};
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
        deps: Arc::new(FakeDeps::new(
            FakeMachine::default(),
            FakeCodeHost::default(),
        )),
        settings: Settings {
            providers,
            open_organizations: open,
            secure_cookies: true,
            code_host_webhook_secret: None,
            e2e_seed: false,
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

    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "checkout", "description": ""})),
        )
        .await;
    assert_eq!(project.status, StatusCode::CREATED);
    let project_id = project.body["id"].as_str().unwrap_or_default().to_owned();
    assert_eq!(
        org.call(
            Method::POST,
            "/projects",
            &viewer,
            Some(json!({"name": "x", "description": ""}))
        )
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
            Some(json!({"card": "keep_going", "mode": "allow", "limitCents": null}))
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
            &format!("/projects/{project_id}/limits"),
            &editor,
            Some(json!({"monthlyCapCents": 5, "defaultBudgetCents": null}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    for who in [&viewer, &editor] {
        assert_eq!(
            org.call(Method::GET, "/projects", who, None).await.status,
            StatusCode::OK,
            "every member reads"
        );
        assert_eq!(
            org.call(Method::GET, "/policies", who, None)
                .await
                .body
                .as_array()
                .map(Vec::len),
            Some(10)
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
        "project.create",
        "member.set_role",
        "policy.set",
        "audit.read",
        "project.set_limits",
    ] {
        assert!(denied.contains(&action), "{action} in {denied:?}");
    }
    let done = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["action"] == "project.create" && e["outcome"] == "done")
        .count();
    assert_eq!(done, 1);
}

#[tokio::test]
async fn an_organization_and_its_projects_keep_an_owner() {
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
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "p", "description": ""})),
        )
        .await;
    let id = project.body["id"].as_str().unwrap_or_default().to_owned();
    let step_back = org
        .call(
            Method::PUT,
            &format!("/projects/{id}/roles/{owner_id}"),
            &org.owner,
            Some(json!({"role": "viewer"})),
        )
        .await;
    assert_eq!(
        (step_back.status, step_back.body["code"].as_str()),
        (StatusCode::CONFLICT, Some("last_owner"))
    );
    let roles = org
        .call(
            Method::GET,
            &format!("/projects/{id}/roles"),
            &org.owner,
            None,
        )
        .await;
    assert_eq!(roles.body.as_array().map(Vec::len), Some(1));
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
            "/projects",
            &secret,
            Some(json!({"name": "x", "description": ""}))
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
        put(json!({"card": "keep_going", "mode": "allow_up_to", "limitCents": null}))
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
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "p", "description": ""})),
        )
        .await;
    let id = project.body["id"].as_str().unwrap_or_default().to_owned();
    let over = org
        .call(
            Method::PUT,
            &format!("/projects/{id}/limits"),
            &org.owner,
            Some(json!({"monthlyCapCents": 20_000, "defaultBudgetCents": null})),
        )
        .await;
    assert_eq!(
        (over.status, over.body["code"].as_str()),
        (StatusCode::CONFLICT, Some("over_cap"))
    );
    let summary = org.call(Method::GET, "/budgets", &org.owner, None).await;
    assert_eq!(
        summary.body["organization"],
        json!({"capCents": 10_000, "spentCents": 0})
    );
    assert_eq!(summary.body["projects"].as_array().map(Vec::len), Some(1));
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
            &format!("/experiments/{nobody}/approvals"),
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
            &format!("/experiments/{nobody}/budget"),
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
    let deps = Arc::new(FakeDeps::new(
        FakeMachine::default(),
        FakeCodeHost::default(),
    ));
    let app = router(
        deps.clone(),
        Settings {
            providers: vec![Provider::Fake(FakeSignIn)],
            open_organizations: true,
            secure_cookies: true,
            code_host_webhook_secret: None,
            e2e_seed: false,
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
    use antilog_types::{
        CardKind, ExperimentId, OrganizationId, Session, Spend, SpendId, SpendKind,
    };

    let (org, deps) = org_with_store().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "checkout", "description": ""})),
        )
        .await;
    let project_id = project.body["id"].as_str().unwrap_or_default().to_owned();
    let experiment = org
        .call(
            Method::POST,
            &format!("/projects/{project_id}/experiments"),
            &editor,
            Some(json!({"goal": "cut p95", "budgetCapCents": 500})),
        )
        .await;
    assert_eq!(experiment.status, StatusCode::CREATED);
    let experiment_id = experiment.body["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let organization = OrganizationId::from_uuid(org.organization.parse().unwrap_or_default());
    let experiment_uuid = ExperimentId::from_uuid(experiment_id.parse().unwrap_or_default());
    let card = approval::raise(
        &deps.store,
        1_700_000_000_000,
        None,
        CardRequest {
            organization,
            experiment: experiment_uuid,
            session: None,
            card: CardKind::RaiseBudget,
            summary: "Raise the budget by $5".into(),
            payload: None,
            cost_cents: Some(500),
            needs_owner: false,
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

    // A session with a conversation and spend.
    let session = Session::new(organization, experiment_uuid, "claude", 1_700_000_000_000);
    deps.store
        .create_session(
            &session,
            &antilog_types::AuditEntry::new(
                organization,
                None,
                "session.start",
                session.id,
                antilog_types::AuditOutcome::Done,
                1_700_000_000_000,
            ),
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
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
            &format!("/sessions/{}/events?limit=2", session.id),
            &viewer,
            None,
        )
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.body["events"].as_array().map(Vec::len), Some(2));
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

    for (kind, cents) in [(SpendKind::Tokens, 40), (SpendKind::Judge, 25)] {
        deps.store
            .record_spend(&Spend {
                id: SpendId::new(),
                organization_id: organization,
                session_id: session.id,
                kind,
                amount_cents: cents,
                tokens: 0,
                occurred_at: 1_700_000_000_000,
                trial_id: None,
            })
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
    }
    let spend = org
        .call(
            Method::GET,
            &format!("/experiments/{experiment_id}/spend"),
            &viewer,
            None,
        )
        .await;
    assert_eq!(spend.status, StatusCode::OK);
    assert_eq!(spend.body["spentCents"], 65);
    assert_eq!(spend.body["budgetCapCents"], 500);
    assert_eq!(spend.body["byKind"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        spend.body["bySession"][0]["sessionId"],
        json!(session.id.to_string())
    );
    let all = org.call(Method::GET, "/spend", &viewer, None).await;
    assert_eq!(all.body[0]["spentCents"], 65);
}

#[tokio::test]
async fn agents_auto_and_environments_are_read_by_members_and_set_by_owners() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "checkout", "description": ""})),
        )
        .await;
    let project_id = project.body["id"].as_str().unwrap_or_default().to_owned();
    let experiment = org
        .call(
            Method::POST,
            &format!("/projects/{project_id}/experiments"),
            &editor,
            Some(json!({"goal": "cut p95", "budgetCapCents": 500})),
        )
        .await;
    let experiment_id = experiment.body["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

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
            &format!("/experiments/{experiment_id}/sessions"),
            &editor,
            Some(json!({"agent": "auto", "prompt": "go", "repos": []})),
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
            &format!("/experiments/{experiment_id}/sessions"),
            &editor,
            Some(json!({"agent": "auto", "prompt": "go", "repos": []})),
        )
        .await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    let named = org
        .call(
            Method::POST,
            &format!("/experiments/{experiment_id}/sessions"),
            &editor,
            Some(json!({"agent": "gemini", "prompt": "go", "repos": []})),
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
async fn evaluations_are_listed_read_and_added_by_role() {
    let org = org().await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let (_, editor) = org.member("ed", "editor").await;
    let body = json!({"name": "Load test", "summary": "Replays traffic", "kind": "speed", "metric": "p95", "lowerIsBetter": true, "command": "k6 run bench/load.js", "protectedPaths": ["app:bench/"]});

    let empty = org.call(Method::GET, "/evaluations", &viewer, None).await;
    assert_eq!(
        (empty.status, empty.body.as_array().map(Vec::len)),
        (StatusCode::OK, Some(0))
    );
    let denied = org
        .call(Method::POST, "/evaluations", &viewer, Some(body.clone()))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let added = org
        .call(Method::POST, "/evaluations", &editor, Some(body.clone()))
        .await;
    assert_eq!(
        (added.status, &added.body["id"]),
        (StatusCode::CREATED, &json!("load-test"))
    );
    let again = org
        .call(Method::POST, "/evaluations", &editor, Some(body))
        .await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    let one = org
        .call(Method::GET, "/evaluations/load-test", &viewer, None)
        .await;
    assert_eq!(one.body["protectedPaths"][0], "app:bench/");
    let missing = org
        .call(Method::GET, "/evaluations/nope", &viewer, None)
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    let log = org.call(Method::GET, "/audit", &org.owner, None).await;
    let actions: Vec<_> = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["action"] == "evaluation.add")
        .map(|e| e["outcome"].clone())
        .collect();
    assert_eq!(actions.len(), 2);
}

#[tokio::test]
async fn value_models_are_set_by_project_owners_and_value_and_return_are_read_by_all() {
    use antilog_core::judge::fixtures;
    use antilog_core::store::judge::JudgeStore;
    use antilog_core::value::record_win_event;
    use antilog_types::judge::*;

    let (org, deps) = org_with_store().await;
    let (_, editor) = org.member("ed", "editor").await;
    let (_, viewer) = org.member("vi", "viewer").await;
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "Checkout", "description": "the path"})),
        )
        .await;
    let project = project.body["id"].as_str().unwrap_or_default().to_owned();
    let model = json!({"metric": "p95", "unit": "ms", "centsPerUnit": 360.0, "source": {"text": "Finance study, Q3", "link": "https://example.com/q3"}});

    // Only a project owner sets one; the refusal is audited.
    let uri = format!("/projects/{project}/value-models");
    for who in [&editor, &viewer] {
        let denied = org.call(Method::POST, &uri, who, Some(model.clone())).await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN);
    }
    let set = org
        .call(Method::POST, &uri, &org.owner, Some(model.clone()))
        .await;
    assert_eq!(set.status, StatusCode::CREATED);
    assert_eq!(set.body["model"]["version"], 1);
    assert_eq!(set.body["model"]["centsPerUnit"], 360.0);
    let again = org
        .call(Method::POST, &uri, &org.owner, Some(json!({"metric": "p95", "unit": "ms", "centsPerUnit": 720.0, "source": {"text": "Re-estimated", "link": null}})))
        .await;
    assert_eq!(
        (
            again.body["model"]["version"].clone(),
            again.body["model"]["id"].clone()
        ),
        (json!(2), set.body["model"]["id"].clone())
    );
    for bad in [
        json!({"metric": "p95", "unit": "ms", "centsPerUnit": 0, "source": {"text": "x", "link": null}}),
        json!({"metric": "", "unit": "ms", "centsPerUnit": 1, "source": {"text": "x", "link": null}}),
        json!({"metric": "p95", "unit": "ms", "centsPerUnit": 1, "source": {"text": "", "link": null}}),
        json!({"metric": "p95", "unit": "ms", "centsPerUnit": 1, "source": {"text": "x", "link": "javascript:1"}}),
    ] {
        let reply = org.call(Method::POST, &uri, &org.owner, Some(bad)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let listed = org.call(Method::GET, &uri, &viewer, None).await;
    assert_eq!(listed.body["models"].as_array().map(Vec::len), Some(1));
    assert_eq!(listed.body["models"][0]["model"]["version"], 2);
    let log = org.call(Method::GET, "/audit", &org.owner, None).await;
    let actions: Vec<_> = log.body["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["action"] == "value_model.set")
        .map(|entry| entry["outcome"].clone())
        .collect();
    assert_eq!(
        actions.len(),
        4,
        "two refusals and two versions: {actions:?}"
    );

    // With no win there is nothing delivered, and every figure says it is measured offline.
    let none = org
        .call(
            Method::GET,
            &format!("/projects/{project}/value"),
            &viewer,
            None,
        )
        .await;
    assert_eq!(none.status, StatusCode::OK);
    assert_eq!(
        (
            none.body["basis"].clone(),
            none.body["deliveredCents"].clone(),
            none.body["returnX"].clone()
        ),
        (json!("measuredOffline"), json!(0), Value::Null)
    );

    // An experiment, sealed with the owner's model and one Kept trial.
    let made = org
        .call(
            Method::POST,
            &format!("/projects/{project}/experiments"),
            &editor,
            Some(json!({"goal": "cut p95", "budgetCapCents": 100_000})),
        )
        .await;
    assert_eq!(made.status, StatusCode::CREATED, "{:?}", made.body);
    let experiment: antilog_types::ExperimentId =
        serde_json::from_value(made.body["id"].clone()).unwrap_or_else(|e| unreachable!("{e}"));
    let organization: antilog_types::OrganizationId =
        serde_json::from_value(json!(org.organization)).unwrap_or_else(|e| unreachable!("{e}"));
    let judge = deps.judge.clone();
    let mut definition = fixtures::definition();
    definition.value_model = Some(ValueModelRef {
        id: serde_json::from_value(set.body["model"]["id"].clone())
            .unwrap_or_else(|e| unreachable!("{e}")),
        version: 2,
    });
    let sealed = SealedExperiment::seal(experiment, 1, definition, vec![], fixtures::checks(), 1)
        .unwrap_or_else(|e| unreachable!("{e}"));
    judge
        .put_sealed(&sealed)
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    let trial = fixtures::trial(&sealed, None, "a");
    judge
        .begin_trial(&trial)
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    judge
        .finish_trial(
            trial.id,
            &Finished {
                measurement: Some(Measurement {
                    trial: trial.id,
                    experiment,
                    version: 1,
                    seal: sealed.seal_hash().clone(),
                    stage: Stage::Full,
                    k: 1,
                    noise: 2.0,
                    parent_value: 100.0,
                    value: 90.0,
                    gain: None,
                    confirmation: None,
                    guardrails: vec![],
                    holdout: Some(HoldoutResult::Passed),
                }),
                decision: Decision {
                    outcome: Outcome::Kept,
                    reasons: vec![],
                    criteria: 1,
                },
            },
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    let now = 1_000;
    for kind in [WinEventKind::Opened, WinEventKind::Merged] {
        record_win_event(
            &deps.store,
            &judge,
            organization,
            WinEvent {
                experiment,
                version: 1,
                trial: trial.id,
                kind,
                value: None,
                link: None,
                at: now,
            },
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e:?}"));
    }

    let seen = org
        .call(
            Method::GET,
            &format!("/experiments/{experiment}/value"),
            &viewer,
            None,
        )
        .await;
    assert_eq!(seen.status, StatusCode::OK, "{:?}", seen.body);
    assert_eq!(seen.body["valueModel"]["model"]["version"], 2);
    assert_eq!(seen.body["wins"][0]["state"], "delivered");
    // 10 ms at 720 cents a year per ms.
    assert_eq!(seen.body["wins"][0]["yearlyCents"], 7_200);
    assert_eq!(seen.body["figures"]["deliveredCents"], 7_200);
    // The organization's figures and report are for owners and editors; a viewer reads a project's.
    for uri in ["/value", "/report?month=2026-11"] {
        let denied = org.call(Method::GET, uri, &viewer, None).await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN, "{uri}");
    }
    let everything = org.call(Method::GET, "/value", &editor, None).await;
    assert_eq!(everything.body["deliveredCents"], 7_200);
    let outside = org
        .call(Method::GET, "/value?from=5000&to=9000", &editor, None)
        .await;
    assert_eq!(outside.body["deliveredCents"], 0);
    let backwards = org
        .call(Method::GET, "/value?from=9&to=3", &editor, None)
        .await;
    assert_eq!(backwards.status, StatusCode::UNPROCESSABLE_ENTITY);

    let report = org
        .call(Method::GET, "/report?month=2026-11", &editor, None)
        .await;
    assert_eq!(report.status, StatusCode::OK, "{:?}", report.body);
    assert_eq!(report.body["month"], "2026-11");
    let current = org.call(Method::GET, "/report", &editor, None).await;
    assert_eq!(current.status, StatusCode::OK);
    let scoped = org
        .call(
            Method::GET,
            &format!("/report?month=2026-11&project={project}"),
            &viewer,
            None,
        )
        .await;
    assert_eq!(scoped.body["projectId"], json!(project));
    for bad in ["2026-13", "nov", "2026-1"] {
        let reply = org
            .call(Method::GET, &format!("/report?month={bad}"), &editor, None)
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    let missing = org
        .call(
            Method::GET,
            "/projects/018f3b1e-0000-7000-8000-000000000000/value",
            &viewer,
            None,
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

// ---- live updates: GET /events/stream ----

mod live {
    use std::time::Duration;

    use antilog_core::approval::{self, CardRequest};
    use antilog_core::clock::Clock;
    use antilog_core::deps::Deps;
    use antilog_core::store::{NewEvent, SessionStore};
    use antilog_server::api::auth::App;
    use antilog_server::api::live::{self, Options};
    use antilog_types::{
        AuditEntry, AuditOutcome, CardKind, EventKind, ExperimentId, LiveEvent, OrganizationId,
        Session, SessionId,
    };
    use tokio_stream::StreamExt;

    use super::*;

    /// One open `GET /events/stream`, parsed into (event name, id, data).
    struct Sse {
        body: axum::body::BodyDataStream,
        pending: String,
    }

    impl Sse {
        async fn open(
            org: &Org,
            token: &str,
            last: Option<&str>,
        ) -> (StatusCode, axum::http::HeaderMap, Self) {
            let mut request = Request::get("/events/stream")
                .header(header::AUTHORIZATION, format!("Bearer {token}"));
            if let Some(last) = last {
                request = request.header("last-event-id", last);
            }
            let response = org
                .app
                .clone()
                .oneshot(
                    request
                        .body(Body::empty())
                        .unwrap_or_else(|e| unreachable!("{e}")),
                )
                .await
                .unwrap_or_else(|e| unreachable!("{e}"));
            let (parts, body) = response.into_parts();
            (
                parts.status,
                parts.headers,
                Self {
                    body: body.into_data_stream(),
                    pending: String::new(),
                },
            )
        }

        /// The next event, skipping keep-alive comments. Fails the test if none comes in 15 s.
        async fn next(&mut self) -> (String, Option<String>, Value) {
            loop {
                if let Some(end) = self.pending.find("\n\n") {
                    let block: String = self.pending.drain(..end + 2).collect();
                    let (mut name, mut id, mut data) = (String::new(), None, String::new());
                    for line in block.lines() {
                        if let Some(v) = line.strip_prefix("event: ") {
                            name = v.to_owned();
                        } else if let Some(v) = line.strip_prefix("id: ") {
                            id = Some(v.to_owned());
                        } else if let Some(v) = line.strip_prefix("data: ") {
                            data.push_str(v);
                        }
                    }
                    if name.is_empty() {
                        continue;
                    }
                    return (name, id, serde_json::from_str(&data).unwrap_or(Value::Null));
                }
                let chunk = tokio::time::timeout(Duration::from_secs(15), self.body.next())
                    .await
                    .unwrap_or_else(|_| unreachable!("no event within 15 seconds"))
                    .unwrap_or_else(|| unreachable!("the stream ended"))
                    .unwrap_or_else(|e| unreachable!("{e}"));
                self.pending.push_str(&String::from_utf8_lossy(&chunk));
            }
        }

        /// Reads until an event named `name` arrives and returns it, with everything before it.
        async fn until(&mut self, name: &str) -> Vec<(String, Option<String>, Value)> {
            let mut seen = Vec::new();
            loop {
                let got = self.next().await;
                let done = got.0 == name;
                seen.push(got);
                if done {
                    return seen;
                }
            }
        }
    }

    struct World {
        org: Org,
        deps: Arc<FakeDeps>,
        organization: OrganizationId,
        experiment: ExperimentId,
    }

    async fn world() -> World {
        let (org, deps) = org_with_store().await;
        let organization = OrganizationId::from_uuid(org.organization.parse().unwrap_or_default());
        let (experiment, _) = experiment_in(&org, &org.owner).await;
        World {
            org,
            deps,
            organization,
            experiment,
        }
    }

    async fn experiment_in(org: &Org, token: &str) -> (ExperimentId, String) {
        let project = org
            .call(
                Method::POST,
                "/projects",
                token,
                Some(json!({"name": "checkout", "description": ""})),
            )
            .await;
        let project_id = project.body["id"].as_str().unwrap_or_default().to_owned();
        let experiment = org
            .call(
                Method::POST,
                &format!("/projects/{project_id}/experiments"),
                token,
                Some(json!({"goal": "cut p95", "budgetCapCents": 500})),
            )
            .await;
        assert_eq!(experiment.status, StatusCode::CREATED);
        (
            ExperimentId::from_uuid(
                experiment.body["id"]
                    .as_str()
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or_default(),
            ),
            project_id,
        )
    }

    impl World {
        fn now(&self) -> i64 {
            self.deps.clock().now().as_millisecond()
        }

        async fn session(&self, organization: OrganizationId, experiment: ExperimentId) -> Session {
            let session = Session::new(organization, experiment, "claude", self.now());
            self.deps
                .store
                .create_session(
                    &session,
                    &AuditEntry::new(
                        organization,
                        None,
                        "session.start",
                        session.id,
                        AuditOutcome::Done,
                        self.now(),
                    ),
                )
                .await
                .unwrap_or_else(|e| unreachable!("{e}"));
            session
        }

        async fn say(&self, organization: OrganizationId, session: SessionId, key: &str) {
            self.deps
                .store
                .append_event(&NewEvent {
                    organization,
                    session,
                    key: key.to_owned(),
                    kind: EventKind::Agent,
                    member: None,
                    text: "Profiling.".into(),
                    at: self.now(),
                })
                .await
                .unwrap_or_else(|e| unreachable!("{e}"));
        }
    }

    #[tokio::test]
    async fn a_stream_ends_soon_after_its_token_is_revoked() {
        let world = world().await;
        let created = world
            .org
            .call(
                Method::POST,
                "/tokens",
                &world.org.owner,
                Some(json!({"memberId": null, "name": "tab"})),
            )
            .await;
        let secret = created.body["secret"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let id = created.body["token"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let me = world.org.call(Method::GET, "/me", &secret, None).await;
        let who = antilog_core::organization::Principal {
            organization: world.organization,
            member: antilog_types::MemberId::from_uuid(
                me.body["member"]["id"]
                    .as_str()
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or_default(),
            ),
        };
        let app = Arc::new(App {
            deps: world.deps.clone(),
            settings: Settings {
                providers: Vec::new(),
                open_organizations: false,
                secure_cookies: true,
                code_host_webhook_secret: None,
                e2e_seed: false,
                runner: None,
            },
        });
        let options = Options {
            poll: Duration::from_millis(5),
            recheck: Duration::from_millis(50),
            max_age: Duration::from_secs(60),
            ..Options::default()
        };
        let mut events = live::open(app, who, secret, None, options);
        assert!(matches!(events.recv().await, Some(LiveEvent::Hello { .. })));
        let quiet = tokio::time::timeout(Duration::from_millis(300), events.recv()).await;
        assert!(quiet.is_err(), "a valid credential keeps the stream open");
        assert_eq!(
            world
                .org
                .call(
                    Method::DELETE,
                    &format!("/tokens/{id}"),
                    &world.org.owner,
                    None
                )
                .await
                .status,
            StatusCode::NO_CONTENT
        );
        let ended = tokio::time::timeout(Duration::from_secs(5), async {
            while events.recv().await.is_some() {}
        })
        .await;
        assert!(ended.is_ok(), "the stream outlived its revoked token");
    }

    #[tokio::test]
    async fn reading_history_opens_no_session() {
        let world = world().await;
        let path = format!("/experiments/{}/history", world.experiment);
        for _ in 0..5 {
            let read = world
                .org
                .call(Method::GET, &path, &world.org.owner, None)
                .await;
            assert_eq!(read.status, StatusCode::OK);
            assert_eq!(read.body["trials"], json!([]));
        }
        let sessions = world
            .org
            .call(
                Method::GET,
                &format!("/experiments/{}/sessions", world.experiment),
                &world.org.owner,
                None,
            )
            .await;
        assert_eq!(sessions.body, json!([]), "no session from a read");
        let nobody = send(&world.org.app, Method::GET, &path, None, None, None).await;
        assert_eq!(nobody.status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_stream_needs_sign_in_and_is_an_uncached_event_stream() {
        let world = world().await;
        let unauthenticated = send(
            &world.org.app,
            Method::GET,
            "/events/stream",
            None,
            None,
            None,
        )
        .await;
        assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);

        let (status, headers, mut stream) = Sse::open(&world.org, &world.org.owner, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CONTENT_TYPE], "text/event-stream");
        assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
        let (name, id, data) = stream.next().await;
        assert_eq!((name.as_str(), id.as_deref()), ("hello", Some("0")));
        assert_eq!(data, json!({"type": "hello", "cursor": 0}));
    }

    #[tokio::test]
    async fn another_organizations_events_never_arrive() {
        let world = world().await;
        let globex = create_organization(&world.org.app, "globex", "grace").await;
        let globex_token = globex.body["session"]["secret"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let globex_org = OrganizationId::from_uuid(
            globex.body["organization"]["id"]
                .as_str()
                .unwrap_or_default()
                .parse()
                .unwrap_or_default(),
        );
        let (globex_experiment, _) = experiment_in(&world.org, &globex_token).await;

        let (_, _, mut acme) = Sse::open(&world.org, &world.org.owner, None).await;
        let (_, _, mut globex_stream) = Sse::open(&world.org, &globex_token, None).await;
        assert_eq!(acme.next().await.0, "hello");
        assert_eq!(globex_stream.next().await.0, "hello");

        // Globex speaks first, acme after. Events arrive in `seq` order, so if globex's leaked
        // into acme's stream it would come before acme's own message.
        let theirs = world.session(globex_org, globex_experiment).await;
        world.say(globex_org, theirs.id, "a").await;
        let ours = world.session(world.organization, world.experiment).await;
        world.say(world.organization, ours.id, "a").await;

        let heard = acme.until("message").await;
        for (name, _, data) in &heard {
            assert_ne!(data["sessionId"], json!(theirs.id), "{name} leaked");
            assert_ne!(data["experimentId"], json!(globex_experiment));
        }
        let (_, _, message) = heard.last().cloned().unwrap_or_default();
        assert_eq!(message["sessionId"], json!(ours.id));
        assert_eq!(message["experimentId"], json!(world.experiment));
        // The session itself was announced to acme, and its status too.
        assert!(heard.iter().all(|(name, ..)| name != "cards"));

        let heard = globex_stream.until("message").await;
        let (_, _, message) = heard.last().cloned().unwrap_or_default();
        assert_eq!(message["sessionId"], json!(theirs.id));
        for (_, _, data) in &heard {
            assert_ne!(data["sessionId"], json!(ours.id));
        }
    }

    #[tokio::test]
    async fn a_reconnect_replays_from_last_event_id_and_resyncs_when_it_cannot() {
        let world = world().await;
        let session = world.session(world.organization, world.experiment).await;
        for key in ["one", "two", "three"] {
            world.say(world.organization, session.id, key).await;
        }
        let owner = &world.org.owner;

        // No Last-Event-ID: start at now, no replay.
        let (_, _, mut fresh) = Sse::open(&world.org, owner, None).await;
        let (name, id, _) = fresh.next().await;
        assert_eq!((name.as_str(), id.as_deref()), ("hello", Some("3")));

        // After seq 1: hello at 1, then 2 and 3, in order, each with its seq as the id.
        let (_, _, mut resumed) = Sse::open(&world.org, owner, Some("1")).await;
        let (name, id, _) = resumed.next().await;
        assert_eq!((name.as_str(), id.as_deref()), ("hello", Some("1")));
        for want in ["2", "3"] {
            let (name, id, data) = resumed.next().await;
            assert_eq!((name.as_str(), id.as_deref()), ("message", Some(want)));
            assert_eq!(data["seq"], json!(want.parse::<i64>().unwrap_or_default()));
            assert_eq!(data["sessionId"], json!(session.id));
        }
        // What arrives after the gap continues the same stream.
        world.say(world.organization, session.id, "four").await;
        let heard = resumed.until("message").await;
        assert_eq!(heard.last().and_then(|e| e.1.as_deref()), Some("4"));

        // A cursor from a different ledger: resync, then continue from the latest.
        let (_, _, mut ahead) = Sse::open(&world.org, owner, Some("99999")).await;
        let (name, id, data) = ahead.next().await;
        assert_eq!((name.as_str(), id.as_deref()), ("resync", Some("4")));
        assert_eq!(data["type"], "resync");
    }

    #[tokio::test]
    async fn cards_and_session_status_are_announced() {
        let world = world().await;
        let (_, _, mut stream) = Sse::open(&world.org, &world.org.owner, None).await;
        assert_eq!(stream.next().await.0, "hello");

        let session = world.session(world.organization, world.experiment).await;
        let heard = stream.until("session").await;
        let (_, id, data) = heard.last().cloned().unwrap_or_default();
        assert_eq!(id, None);
        assert_eq!(data["sessionId"], json!(session.id));
        assert_eq!(data["status"], "working");

        let card = approval::raise(
            &world.deps.store,
            world.now(),
            None,
            CardRequest {
                organization: world.organization,
                experiment: world.experiment,
                session: Some(session.id),
                card: CardKind::RaiseBudget,
                summary: "Raise the budget by $5".into(),
                cost_cents: Some(500),
                needs_owner: false,
                follow_policy: false,
                payload: None,
            },
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
        let heard = stream.until("cards").await;
        assert_eq!(
            heard.last().map(|e| e.2.clone()),
            Some(json!({"type": "cards", "pending": 1}))
        );
        // Never the card's words.
        assert!(!format!("{heard:?}").contains("Raise the budget"));

        let decided = world
            .org
            .call(
                Method::POST,
                &format!("/approvals/{}/decision", card.id),
                &world.org.owner,
                Some(json!({"decision": "approve"})),
            )
            .await;
        assert_eq!(decided.status, StatusCode::OK);
        let heard = stream.until("cards").await;
        assert_eq!(
            heard.last().map(|e| e.2.clone()),
            Some(json!({"type": "cards", "pending": 0}))
        );
    }

    #[tokio::test]
    async fn a_stream_ends_at_its_age_limit_so_the_browser_can_reconnect() {
        let world = world().await;
        let who = antilog_core::organization::Principal {
            organization: world.organization,
            member: {
                let me = world
                    .org
                    .call(Method::GET, "/me", &world.org.owner, None)
                    .await;
                antilog_types::MemberId::from_uuid(
                    me.body["member"]["id"]
                        .as_str()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                )
            },
        };
        let app = Arc::new(App {
            deps: world.deps.clone(),
            settings: Settings {
                providers: Vec::new(),
                open_organizations: false,
                secure_cookies: true,
                code_host_webhook_secret: None,
                e2e_seed: false,
                runner: None,
            },
        });
        let options = Options {
            poll: Duration::from_millis(5),
            max_age: Duration::from_millis(60),
            ..Options::default()
        };
        let mut events = live::open(app, who, String::new(), None, options);
        let first = events.recv().await;
        assert_eq!(first, Some(LiveEvent::Hello { cursor: 0 }));
        let ended = tokio::time::timeout(Duration::from_secs(10), async {
            while events.recv().await.is_some() {}
        })
        .await;
        assert!(ended.is_ok(), "the stream did not end");
    }
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
    let project = org
        .call(
            Method::POST,
            "/projects",
            &org.owner,
            Some(json!({"name": "checkout", "description": ""})),
        )
        .await;
    let project_id = project.body["id"].as_str().unwrap_or_default().to_owned();
    let experiment = org
        .call(
            Method::POST,
            &format!("/projects/{project_id}/experiments"),
            &editor,
            Some(json!({"goal": "cut p95", "budgetCapCents": 500})),
        )
        .await;
    let sessions = format!(
        "/experiments/{}/sessions",
        experiment.body["id"].as_str().unwrap_or_default()
    );
    let start = |environment: &str| json!({"agent": "auto", "prompt": "go", "repos": [], "environment": environment});
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
