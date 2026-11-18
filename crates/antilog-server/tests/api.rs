#![expect(
    clippy::indexing_slicing,
    reason = "tests read a JSON body or a fixed block"
)]

//! The organization floor over HTTP, on the in-memory store and the fake sign-in provider.

use std::sync::Arc;

use antilog_core::auth::FakeSignIn;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakePlatform};
use antilog_server::api::auth::{Provider, Settings};
use antilog_server::api::{openapi_json, router};
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
    router(
        Arc::new(FakeDeps::new(
            FakePlatform::default(),
            FakeCodeHost::default(),
        )),
        Settings {
            providers,
            open_organizations: open,
            secure_cookies: true,
            wake_secret: None,
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
            Some(8)
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
