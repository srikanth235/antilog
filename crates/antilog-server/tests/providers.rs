#![expect(
    clippy::indexing_slicing,
    reason = "tests read a JSON body or a fixed block"
)]

//! The GitHub and OpenID Connect code paths, against a provider served from this test.

use std::collections::HashMap;

use antilog_core::auth::{Challenge, SignInError, SignInProvider};
use antilog_server::signin::{GitHub, GitHubConfig, Oidc, OidcConfig};
use axum::extract::{Form, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SECRET: &str = "client-secret";

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().fold(0u32, |n, b| (n << 8) | u32::from(*b)) << (8 * (3 - chunk.len()));
        for i in 0..=chunk.len() {
            let index = usize::try_from((n >> (18 - 6 * i)) & 63).unwrap_or(0);
            out.push(char::from(ALPHABET.get(index).copied().unwrap_or(b'A')));
        }
    }
    out
}

fn hmac(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    block[..key.len()].copy_from_slice(key);
    let pad = |byte: u8| block.map(|b| b ^ byte);
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize()
        .into()
}

fn id_token(issuer: &str, subject: &str, nonce: &str, signing: &[u8]) -> String {
    let header = base64url(br#"{"alg":"HS256","typ":"JWT"}"#);
    let claims = json!({
        "iss": issuer, "sub": subject, "aud": "antilog", "name": "Grace Hopper",
        "nonce": nonce, "iat": 1_700_000_000, "exp": 4_100_000_000_u64,
    });
    let payload = base64url(claims.to_string().as_bytes());
    let signed = format!("{header}.{payload}");
    format!("{signed}.{}", base64url(&hmac(signing, signed.as_bytes())))
}

#[derive(Clone)]
struct Fake {
    base: String,
}

async fn token(
    State(fake): State<Fake>,
    Form(form): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let Some(code) = form.get("code") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_request"})),
        );
    };
    if code.starts_with("bad") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_grant"})),
        );
    }
    let nonce = code.strip_prefix("nonce-").unwrap_or("");
    (
        StatusCode::OK,
        Json(json!({
            "access_token": "gho_token", "token_type": "bearer",
            "id_token": id_token(&fake.base, "108", nonce, SECRET.as_bytes()),
        })),
    )
}

async fn user(headers: HeaderMap) -> (StatusCode, Json<Value>) {
    let ok = headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer gho_token");
    if ok {
        (
            StatusCode::OK,
            Json(json!({"id": 4242, "login": "octocat", "name": null})),
        )
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({})))
    }
}

async fn discovery(State(fake): State<Fake>) -> Json<Value> {
    let base = &fake.base;
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "jwks_uri": format!("{base}/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["HS256"],
    }))
}

async fn serve() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    let base = format!(
        "http://{}",
        listener
            .local_addr()
            .unwrap_or_else(|e| unreachable!("{e}"))
    );
    let app = Router::new()
        .route("/token", post(token))
        .route("/user", get(user))
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(|| async { Json(json!({"keys": []})) }))
        .with_state(Fake { base: base.clone() });
    tokio::spawn(async move { axum::serve(listener, app).await });
    base
}

fn challenge() -> Challenge {
    Challenge {
        state: "state-1".into(),
        nonce: "nonce-1".into(),
    }
}

#[tokio::test]
async fn github_signs_in_by_its_numeric_user_id() {
    let base = serve().await;
    let github = GitHub(GitHubConfig {
        client_id: "antilog".into(),
        client_secret: SECRET.into(),
        redirect: "https://antilog.example/callback".into(),
        auth_url: format!("{base}/authorize"),
        token_url: format!("{base}/token"),
        user_url: format!("{base}/user"),
    });
    let url = github.authorize_url(&challenge()).await.unwrap_or_default();
    assert!(
        url.starts_with(&format!("{base}/authorize?")) && url.contains("state=state-1"),
        "{url}"
    );
    let proven = github
        .complete("good", &challenge())
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(proven.identity.provider, "github");
    assert_eq!(
        proven.identity.subject, "4242",
        "the numeric id, never the login or an email"
    );
    assert_eq!(proven.name, "octocat");
    assert!(matches!(
        github.complete("bad", &challenge()).await,
        Err(SignInError::Refused)
    ));
}

fn oidc(base: &str) -> Oidc {
    Oidc(OidcConfig {
        name: "google".into(),
        issuer: base.into(),
        client_id: "antilog".into(),
        client_secret: SECRET.into(),
        redirect: "https://antilog.example/callback".into(),
    })
}

#[tokio::test]
async fn openid_connect_verifies_the_id_token_and_its_nonce() {
    let base = serve().await;
    let provider = oidc(&base);
    let url = provider
        .authorize_url(&challenge())
        .await
        .unwrap_or_default();
    assert!(url.starts_with(&format!("{base}/authorize?")), "{url}");
    assert!(
        url.contains("state=state-1")
            && url.contains("nonce=nonce-1")
            && url.contains("scope=openid"),
        "{url}"
    );

    let proven = provider
        .complete("nonce-nonce-1", &challenge())
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        (
            proven.identity.provider.as_str(),
            proven.identity.subject.as_str()
        ),
        ("google", "108")
    );
    assert_eq!(proven.name, "Grace Hopper");
    assert!(
        matches!(
            provider.complete("nonce-other", &challenge()).await,
            Err(SignInError::Refused)
        ),
        "a replayed token has another nonce"
    );
    assert!(matches!(
        provider.complete("bad", &challenge()).await,
        Err(SignInError::Refused)
    ));

    let forged = Oidc(OidcConfig {
        client_secret: "other".into(),
        ..oidc(&base).0
    });
    assert!(
        forged
            .complete("nonce-nonce-1", &challenge())
            .await
            .is_err(),
        "a token signed with another key"
    );
}

#[tokio::test]
async fn an_unreachable_provider_is_unavailable_not_refused() {
    let provider = Oidc(OidcConfig {
        issuer: "http://127.0.0.1:1".into(),
        ..oidc("http://127.0.0.1:1").0
    });
    assert!(matches!(
        provider.complete("x", &challenge()).await,
        Err(SignInError::Unavailable(_))
    ));
}
