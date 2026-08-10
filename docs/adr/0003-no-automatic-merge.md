# ADR 0003: Do Not Automatically Merge Diverged History in MVP

- Status: Accepted
- Date: 2026-08-10

## Context

A beginner-oriented client can easily turn a remote synchronization conflict into data loss or an incomprehensible working tree if it silently chooses merge or rebase behavior.

Unity projects also contain scenes, prefabs, metadata, and binary assets where a generic text-conflict flow may not be sufficient.

## Decision

MVP remote synchronization will only apply incoming history automatically when it is safe to fast-forward.

When local and remote histories diverge, Vsedi will:

1. fetch remote history
2. detect divergence
3. stop before modifying the worktree or history
4. show that local and remote both contain unique changes
5. explain that Vsedi does not automatically combine them

MVP will not ask a beginner to choose between merge and rebase.

## Consequences

Positive:

- synchronization behavior is predictable
- conflict-heavy states are not created silently
- implementation can prioritize reliable detection and explanation first

Negative:

- users with diverged histories need an external Git tool or a later Vsedi conflict workflow
- collaboration scenarios are intentionally limited in early versions

## Revisit when

Add a dedicated conflict-resolution design only after local save/restore and basic remote backup are stable and real usage demonstrates the need.
