# Add a sign-in provider

Sign-in is OpenID Connect, plus GitHub's OAuth on Antilog cloud, where only GitHub and Google are offered. Antilog never stores a password. Built: GitHub (`oauth2` and the user API), OpenID Connect (`openidconnect`, Google or a self-hosted provider) and a fake provider for agents and tests, in `antilog-server/src/signin.rs`, `api/auth.rs` and `antilog-core/src/auth.rs`, configured by `ANTILOG_GITHUB_*`, `ANTILOG_GOOGLE_*` and `ANTILOG_OIDC_*` (issuer, client id, redirect and a client secret read from `NAME` or the file `NAME_FILE`). The fake provider is off unless `ANTILOG_FAKE_SIGN_IN=1`, which only tests and agents set. Creating an organization by signing in is off unless `ANTILOG_OPEN_ORGANIZATIONS=1`. Sign-in providers are set up in Settings, Single sign-on, one of the two things set up in Settings ([approvals.md](approvals.md)).

1. **OpenID Connect.** The provider speaks OpenID Connect with discovery (`openidconnect`). The one exception is GitHub on Antilog cloud, which offers only OAuth: it signs in with `oauth2` and GitHub's user API, and its numeric user id is the subject. Any other provider that needs a password or a custom protocol is refused.
2. **Endpoints are configuration.** Issuer, client id and redirect are settings. No provider URL is a constant, so a self-hosted server contacts only what its administrator set.
3. **Antilog cloud and self-hosted.** Antilog cloud offers GitHub and Google to everyone and nothing else: no per-organization single sign-on (owner's call, 17 Nov 2026). A self-hosted server offers the customer's own identity provider and no other; only an owner sets it up, in Settings, Single sign-on, its settings live in the organization's own database, and the client secret is a reference, never the value. (target)
4. **Identity becomes a member.** The provider's stable subject id maps to a member. An email is not an identity. The callback needs no organization id: the identity's memberships are looked up (a read-only row-level-security window on that one identity), the oldest signs in, and `GET /me` lists the others. A switch between organizations is target.
5. **Secrets.** The client secret is stored like any secret and never appears in a response, event or log.
6. **Group claims.** If the provider sends groups, map them to roles (see [add-role.md](add-role.md)). (target)
7. **Session.** A successful sign-in starts a short-lived sign-in session: a random token in a secure, HTTP-only cookie, stored only as a hash through the `Store`. Sign-out and expiry end it.
8. **Audit.** Sign-in, sign-out and a failed sign-in are audited actions (see [add-audited-action.md](add-audited-action.md)).
9. **One cryptography stack.** Use the pure-Rust crates named in [../stack/SKILL.md](../stack/SKILL.md) (`sha2`, `subtle`, `ed25519-dalek`) . Do not add a second library for tokens or signatures.
10. **Tests.** A fake provider in tests, a rejected token, an expired session, and a check that no password field exists anywhere.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
