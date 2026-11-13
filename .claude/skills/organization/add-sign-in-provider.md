# Add a sign-in provider

Sign-in is OpenID Connect only. Ratchet never stores a password. None of it exists yet; today the only credential is the per-boot bearer token. GitHub device flow in `crates/ratchet-github/src/signin.rs` grants access to repos and is not a Ratchet sign-in.

1. **OpenID Connect only.** The provider speaks OpenID Connect with discovery. A provider that needs a password or a custom protocol is refused. (target)
2. **Endpoints are configuration.** Issuer, client id and redirect are settings. No provider URL is a constant, so a self-hosted server contacts only what its administrator set. (target)
3. **Hosted and self-hosted.** The hosted service offers GitHub and Google. A self-hosted server offers the customer's own identity provider and no other. (target)
4. **Identity becomes a member.** The provider's stable subject id maps to a member. An email is not an identity. (target)
5. **Secrets.** The client secret is stored like any secret (see `crates/ratchet-connectors/src/secrets.rs` for the pattern) and never appears in a response, event or log.
6. **Group claims.** If the provider sends groups, map them to roles (see [add-role.md](add-role.md)). (target)
7. **Session.** A successful sign-in starts a short-lived session. Sign-out and expiry end it. (target)
8. **Audit.** Sign-in, sign-out and a failed sign-in are audited actions (see [add-audited-action.md](add-audited-action.md)). (target)
9. **One cryptography stack.** Use the stack the rest of Ratchet uses. Do not add a second library for tokens or signatures. (target)
10. **Tests.** A fake provider in tests, a rejected token, an expired session, and a check that no password field exists anywhere.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).
