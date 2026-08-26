# Ship cross-agent skills inside the binary

## Context and Problem Statement

Coding agents discover operating knowledge through skill files in agent-specific directories. The distribution needs every agent — Claude Code, Codex, Gemini CLI, Copilot — to land and operate an instance without a network, without per-agent authoring, and without the skills drifting from the CLI they describe.

## Considered Options

- `embed the skills in the binary with spec-only frontmatter` — chosen.
- `per-agent skill variants with a render or merge step` — rejected: two renders of one skill drift, and every field outside the portable intersection binds the payload to one vendor.
- `publish the skills as a separate download` — rejected: it reintroduces the network dependency the payload embedding removed, and versions skew between skill and binary.
- `Claude-specific overlay fields on the shared body` — deferred: revisit if an agent requires a field the portable Agent Skills format cannot carry.

## Decision Outcome

Chosen option: `embed the skills in the binary with spec-only frontmatter`. The Agent Skills format is the portable intersection every listed agent reads, Claude Code loads it unchanged, and `include_dir!` keeps the shipped bytes identical to the authored `skills/` tree. Instances receive the skills managed at `.claude/skills/` and `.agents/skills/`, and `sdd skill install` writes the same bytes at user scope.

Enforced by `distribution:skills-are-part-of-the-payload` and `distribution:a-skill-obeys-the-portable-format`.

## Consequences

- Good: one authored file per skill serves every agent byte-identically, at project scope through `sdd init` and at user scope through `sdd skill install`.
- Good: the skills upgrade with the binary as managed files, so they cannot describe a CLI they no longer match.
- Bad: no skill may use a vendor-only capability field, and the conformance test rejects any attempt.

## Status

Accepted
