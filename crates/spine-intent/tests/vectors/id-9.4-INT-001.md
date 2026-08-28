# INT-001: Add a health endpoint
Owner: @alice · Template: intent@2 · Constitution: v1

## Goal
The service answers a liveness probe without touching the database.

## Non-goals
- Readiness, which needs the database.
- Metrics of any kind.

## Acceptance criteria
AC-1: Given the process is running, when GET /healthz is called, then it answers 200.

## Touchpoints
Expected to change: src/http/
Must NOT change:
