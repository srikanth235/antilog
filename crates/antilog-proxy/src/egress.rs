//! The session's outbound allowlist: a CONNECT forward proxy. A session's machine is told to send
//! its traffic here with its token as the proxy password. The tunnel is opaque bytes; nothing is
//! decrypted. A host that is not allowed is refused and logged.

use std::future::Future;
use std::net::IpAddr;

use antilog_core::credentials::Purpose;
use antilog_types::SessionId;
use axum::body::Body;
use axum::http::{HeaderMap, Request, Response, StatusCode, header};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::{Backend, Shared, Time, Tokens};

/// Hosts of the package registries an environment may allow with one switch. A registry is a
/// service the build needs, not a place the agent chooses to talk to.
pub const PACKAGE_REGISTRIES: &[&str] = &[
    "registry.npmjs.org",
    "registry.yarnpkg.com",
    "pypi.org",
    "files.pythonhosted.org",
    "crates.io",
    "index.crates.io",
    "static.crates.io",
    "proxy.golang.org",
    "sum.golang.org",
    "rubygems.org",
    "index.rubygems.org",
    "repo.maven.apache.org",
    "repo1.maven.org",
    "plugins.gradle.org",
    "services.gradle.org",
    "deb.debian.org",
    "security.debian.org",
    "archive.ubuntu.com",
    "security.ubuntu.com",
];

/// An environment's network level (`docs/V0.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkAccess {
    None,
    Limited {
        hosts: Vec<String>,
        package_registries: bool,
    },
    Full,
}

fn normal(host: &str) -> String {
    host.trim().trim_end_matches('.').to_ascii_lowercase()
}

impl NetworkAccess {
    pub fn allows(&self, host: &str) -> bool {
        let host = normal(host);
        match self {
            Self::None => false,
            Self::Full => true,
            Self::Limited {
                hosts,
                package_registries,
            } => {
                hosts.iter().any(|allowed| normal(allowed) == host)
                    || (*package_registries && PACKAGE_REGISTRIES.contains(&host.as_str()))
            }
        }
    }
}

/// What a session's environment lets it reach. Environments are not stored yet, so v0 gives every
/// session one answer (`StaticEnvironment`); the real one reads the session's environment.
pub trait Environments: Send + Sync {
    fn network(&self, session: SessionId) -> impl Future<Output = NetworkAccess> + Send;
}

pub struct StaticEnvironment(pub NetworkAccess);

impl Environments for StaticEnvironment {
    async fn network(&self, _session: SessionId) -> NetworkAccess {
        self.0.clone()
    }
}

/// Where an allowed host may not lead: an address inside our own network, which would turn the
/// allowlist into a way past it.
fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let [a0, a1, ..] = a.octets();
            !(a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_unspecified()
                || a.is_broadcast()
                || a.is_documentation()
                || (a0 == 100 && (a1 & 0xc0) == 64))
        }
        IpAddr::V6(a) => {
            let [s0, ..] = a.segments();
            !(a.is_loopback()
                || a.is_unspecified()
                || (s0 & 0xfe00) == 0xfc00
                || (s0 & 0xffc0) == 0xfe80
                || a.to_ipv4_mapped().is_some_and(|v4| !public(IpAddr::V4(v4))))
        }
    }
}

fn reply(status: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    response
}

fn challenge() -> Response<Body> {
    let mut response = reply(StatusCode::PROXY_AUTHENTICATION_REQUIRED);
    response.headers_mut().insert(
        header::PROXY_AUTHENTICATE,
        header::HeaderValue::from_static("Basic realm=\"antilog\""),
    );
    response
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut bits, mut have) = (0u32, 0u32);
    for c in text.trim_end_matches('=').bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push(u8::try_from((bits >> have) & 0xff).ok()?);
        }
    }
    Some(out)
}

/// The session token a machine presents: the password of a Basic proxy login (any user name), or
/// a Bearer value.
fn presented(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(header::PROXY_AUTHORIZATION)?
        .to_str()
        .ok()?
        .trim();
    if let Some(token) = value.strip_prefix("Bearer ") {
        return Some(token.trim().to_owned());
    }
    let login = String::from_utf8(decode_base64(value.strip_prefix("Basic ")?.trim())?).ok()?;
    let (user, password) = login.split_once(':')?;
    let token = if password.is_empty() { user } else { password };
    Some(token.to_owned())
}

/// Answers a CONNECT: checks the token, the allowlist and where the name leads, then splices the
/// two connections together until either side closes.
pub async fn connect<S, C, K, E>(
    proxy: Shared<S, C, K, E>,
    request: Request<Incoming>,
) -> Response<Body>
where
    S: Backend,
    C: Tokens,
    K: Time,
    E: Environments + 'static,
{
    let Some(authority) = request.uri().authority().cloned() else {
        return reply(StatusCode::BAD_REQUEST);
    };
    let host = normal(authority.host().trim_matches(['[', ']']));
    let port = authority.port_u16().unwrap_or(443);

    let Some(token) = presented(request.headers()) else {
        return challenge();
    };
    let session = match proxy.credentials.verify(&token, Purpose::Model).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            tracing::warn!(%host, "egress refused: no valid session credential");
            return challenge();
        }
        Err(error) => {
            tracing::error!(%error, "egress: the credential store is unavailable");
            return reply(StatusCode::SERVICE_UNAVAILABLE);
        }
    };
    if !proxy.environments.network(session).await.allows(&host) {
        tracing::warn!(%session, %host, port, "egress refused: not on the allowlist");
        return reply(StatusCode::FORBIDDEN);
    }

    let addresses = match tokio::net::lookup_host((host.as_str(), port)).await {
        Ok(found) => found.collect::<Vec<_>>(),
        Err(error) => {
            tracing::warn!(%session, %host, %error, "egress: the name does not resolve");
            return reply(StatusCode::BAD_GATEWAY);
        }
    };
    let reachable: Vec<_> = addresses
        .into_iter()
        .filter(|addr| proxy.allow_private_addresses || public(addr.ip()))
        .collect();
    if reachable.is_empty() {
        tracing::warn!(%session, %host, "egress refused: the name leads only to a private address");
        return reply(StatusCode::FORBIDDEN);
    }
    let mut upstream = None;
    for addr in reachable {
        if let Ok(stream) = TcpStream::connect(addr).await {
            upstream = Some(stream);
            break;
        }
    }
    let Some(mut upstream) = upstream else {
        tracing::warn!(%session, %host, "egress: the host did not answer");
        return reply(StatusCode::BAD_GATEWAY);
    };

    tracing::info!(%session, %host, port, "egress allowed");
    let upgrade = hyper::upgrade::on(request);
    tokio::spawn(async move {
        match upgrade.await {
            Ok(upgraded) => {
                let mut client = TokioIo::new(upgraded);
                if let Err(error) = tokio::io::copy_bidirectional(&mut client, &mut upstream).await
                {
                    tracing::debug!(%error, "egress tunnel ended");
                }
            }
            Err(error) => tracing::debug!(%error, "egress upgrade failed"),
        }
    });
    reply(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowlist_is_exact_and_registries_are_one_switch() {
        let limited = |registries| NetworkAccess::Limited {
            hosts: vec!["api.stripe.com".into()],
            package_registries: registries,
        };
        assert!(limited(false).allows("API.stripe.com."));
        assert!(!limited(false).allows("evil.api.stripe.com"));
        assert!(!limited(false).allows("registry.npmjs.org"));
        assert!(limited(true).allows("registry.npmjs.org"));
        assert!(!NetworkAccess::None.allows("api.stripe.com"));
        assert!(NetworkAccess::Full.allows("anything.example"));
    }

    #[test]
    fn private_and_metadata_addresses_are_not_public() {
        for ip in [
            "127.0.0.1",
            "10.0.0.5",
            "169.254.169.254",
            "192.168.1.1",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:10.0.0.1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "140.82.112.3", "2606:4700::1111"] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn the_token_is_the_password_of_a_basic_login() {
        let mut headers = HeaderMap::new();
        // "agent:alc_abc"
        headers.insert(
            header::PROXY_AUTHORIZATION,
            "Basic YWdlbnQ6YWxjX2FiYw==".parse().unwrap(),
        );
        assert_eq!(presented(&headers).as_deref(), Some("alc_abc"));
        assert_eq!(presented(&HeaderMap::new()), None);
    }
}
