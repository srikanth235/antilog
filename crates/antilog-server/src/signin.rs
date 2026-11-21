//! The real sign-in providers: GitHub (OAuth and its user API) and OpenID Connect
//! (Google, or a self-hosted server's own provider). Every endpoint is configuration.

use antilog_core::auth::{Challenge, Proven, SignInError, SignInProvider};
use antilog_types::Identity;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{IssuerUrl, Nonce, TokenResponse as _};

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> SignInError {
    SignInError::Unavailable(Box::new(error))
}

fn http() -> Result<oauth2::reqwest::Client, SignInError> {
    oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(unavailable)
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Malformed(&'static str);

#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect: String,
    pub auth_url: String,
    pub token_url: String,
    /// The user API: `https://api.github.com/user`.
    pub user_url: String,
}

#[derive(Debug, Clone)]
pub struct GitHub(pub GitHubConfig);

impl SignInProvider for GitHub {
    fn name(&self) -> &str {
        "github"
    }

    async fn authorize_url(&self, challenge: &Challenge) -> Result<String, SignInError> {
        let config = &self.0;
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_auth_uri(AuthUrl::new(config.auth_url.clone()).map_err(unavailable)?)
            .set_redirect_uri(RedirectUrl::new(config.redirect.clone()).map_err(unavailable)?);
        let state = challenge.state.clone();
        let (url, _) = client
            .authorize_url(move || CsrfToken::new(state))
            .add_scope(Scope::new("read:user".to_owned()))
            .url();
        Ok(url.into())
    }

    async fn complete(&self, code: &str, _: &Challenge) -> Result<Proven, SignInError> {
        let config = &self.0;
        let client = BasicClient::new(ClientId::new(config.client_id.clone()))
            .set_client_secret(ClientSecret::new(config.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(config.auth_url.clone()).map_err(unavailable)?)
            .set_token_uri(TokenUrl::new(config.token_url.clone()).map_err(unavailable)?)
            .set_redirect_uri(RedirectUrl::new(config.redirect.clone()).map_err(unavailable)?);
        let http = http()?;
        let token = client
            .exchange_code(AuthorizationCode::new(code.to_owned()))
            .request_async(&http)
            .await
            .map_err(|_| SignInError::Refused)?;
        let body = http
            .get(&config.user_url)
            .bearer_auth(token.access_token().secret())
            .header("user-agent", "antilog")
            .header("accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(unavailable)?
            .error_for_status()
            .map_err(|_| SignInError::Refused)?
            .bytes()
            .await
            .map_err(unavailable)?;
        let user: serde_json::Value = serde_json::from_slice(&body).map_err(unavailable)?;
        let id = user
            .get("id")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| unavailable(Malformed("the user API sent no numeric id")))?;
        let name = ["name", "login"]
            .iter()
            .find_map(|key| user.get(*key).and_then(serde_json::Value::as_str))
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("GitHub user");
        Ok(Proven {
            identity: Identity {
                provider: "github".to_owned(),
                subject: id.to_string(),
            },
            name: name.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// The identity provider's name in an identity: `google`, or `oidc` self-hosted.
    pub name: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect: String,
}

#[derive(Debug, Clone)]
pub struct Oidc(pub OidcConfig);

impl Oidc {
    async fn client(
        &self,
        http: &oauth2::reqwest::Client,
    ) -> Result<
        CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
        SignInError,
    > {
        let config = &self.0;
        let metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(config.issuer.clone()).map_err(unavailable)?,
            http,
        )
        .await
        .map_err(unavailable)?;
        Ok(CoreClient::from_provider_metadata(
            metadata,
            openidconnect::ClientId::new(config.client_id.clone()),
            Some(openidconnect::ClientSecret::new(
                config.client_secret.clone(),
            )),
        )
        .set_redirect_uri(
            openidconnect::RedirectUrl::new(config.redirect.clone()).map_err(unavailable)?,
        ))
    }
}

impl SignInProvider for Oidc {
    fn name(&self) -> &str {
        &self.0.name
    }

    async fn authorize_url(&self, challenge: &Challenge) -> Result<String, SignInError> {
        let http = http()?;
        let client = self.client(&http).await?;
        let (state, nonce) = (challenge.state.clone(), challenge.nonce.clone());
        let (url, _, _) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                move || CsrfToken::new(state),
                move || Nonce::new(nonce),
            )
            .add_scope(openidconnect::Scope::new("profile".to_owned()))
            .url();
        Ok(url.into())
    }

    async fn complete(&self, code: &str, challenge: &Challenge) -> Result<Proven, SignInError> {
        let http = http()?;
        let client = self.client(&http).await?;
        let token = client
            .exchange_code(openidconnect::AuthorizationCode::new(code.to_owned()))
            .map_err(unavailable)?
            .request_async(&http)
            .await
            .map_err(|_| SignInError::Refused)?;
        let id_token = token.id_token().ok_or(SignInError::Refused)?;
        let claims = id_token
            .claims(
                &client.id_token_verifier(),
                &Nonce::new(challenge.nonce.clone()),
            )
            .map_err(|_| SignInError::Refused)?;
        let name = claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.as_str().to_owned())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Member".to_owned());
        Ok(Proven {
            identity: Identity {
                provider: self.0.name.clone(),
                subject: claims.subject().as_str().to_owned(),
            },
            name,
        })
    }
}
