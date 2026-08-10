#!/usr/bin/env sh
set -eu

pnpm generate-types
if ! git diff --exit-code -- src/generated/bindings.ts; then
  echo "Generated TypeScript bindings are out of date. Run pnpm generate-types and commit the result." >&2
  exit 1
fi
