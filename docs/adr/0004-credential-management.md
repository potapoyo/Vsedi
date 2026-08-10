# ADR 0004: Delegate Git Credentials to Git Credential Helpers

- Status: Accepted
- Date: 2026-08-10

## Context

Remote Git operations may require credentials. Storing access tokens or passwords in Vsedi's own plaintext configuration would create unnecessary security risk and duplicate platform-specific secure-storage behavior.

Git already supports credential helpers, including helpers that integrate with OS keychains or secure credential stores.

## Decision

Vsedi will not initially implement its own persistent Git credential store.

Remote operations performed through the system Git CLI will use the user's configured Git credential helper / askpass mechanism.

Vsedi must never intentionally write tokens or passwords into:

- application settings
- repository configuration
- diagnostic logs
- remote URLs

## Consequences

Positive:

- reuses established Git authentication behavior
- can integrate with OS secure storage through existing helpers
- avoids becoming a password manager

Negative:

- authentication UX can differ depending on the user's Git installation and helper
- GUI prompt integration may need additional work for environments without a usable helper / askpass setup

## Future work

If Vsedi later provides first-class GitHub OAuth, that should be a separate ADR and must use secure OS-backed storage rather than replacing this decision implicitly.

## Reference

- https://git-scm.com/docs/gitcredentials.html
