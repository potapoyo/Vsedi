# ADR 0002: Local First

- Status: Accepted
- Date: 2026-08-10

## Context

The primary user need is to save VRChat Unity work and recover from mistakes. Requiring GitHub or another hosting service would add account, authentication, network, and privacy concerns before the user receives that value.

VRChat's own SDK update guidance notes that version control is useful without uploading the repository to GitHub or a similar service.

## Decision

Vsedi will be local-first.

The core product must work with a local Git repository only:

- initialize repository
- inspect changes
- save work (commit)
- view history
- preview restore
- safely restore

Remote services are optional backup/synchronization features added after Vsedi Core.

## Consequences

Positive:

- first-use flow is simpler
- private/purchased assets are not uploaded as a prerequisite
- core functionality survives network/authentication failures
- Vsedi's value is not tied to one hosting provider

Negative:

- local-only repositories are not protection against disk loss
- onboarding must clearly explain the difference between local save and remote backup
- remote setup becomes a second, explicit workflow

## UX requirement

The UI must not imply that a local commit has been uploaded anywhere.

Suggested terminology:

- commit: `作業を保存`
- push: `リモートへバックアップ`

## Reference

- https://creators.vrchat.com/sdk/updating-the-sdk/
