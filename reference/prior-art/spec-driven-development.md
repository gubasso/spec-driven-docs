# Spec-Driven Development — Best Practices

A reference guide on using specifications to drive software development, particularly with LLM coding agents. Synthesized from industry research, Anthropic recommendations, and practitioner experience.

---

## Core Principles

Source of Truth (SoT): Every concern has exactly one authoritative source. Code+tests own behavioral truth post-implementation; specs own intent and rationale pre-implementation. Violating this creates divergence that compounds over time.

Docs-as-Code: Specifications live in the repo, version-controlled alongside code. They are reviewed in PRs, linted in CI, and subject to the same quality standards as source code.

Spec Rot is Real: "By Sprint 3, the HLD is outdated. By release 2, the SRS no longer matches the product" (Thoughtworks). Every spec that describes implemented behavior will eventually lie unless actively maintained or deliberately archived.

---

## Research Findings

### AGENTS.md Benchmark Study

arXiv study (January 2026, 124 PRs across 10 repositories) measuring impact of project-context files on AI agent performance:

| Metric                 | Without AGENTS.md | With AGENTS.md | Improvement   |
| ---------------------- | ----------------- | -------------- | ------------- |
| Median completion time | 98.57s            | 70.34s         | 28.6% faster  |
| Mean completion time   | 162.94s           | 129.91s        | 20.3% faster  |
| Median output tokens   | 2,925             | 2,440          | 16.6% fewer   |
| Mean output tokens     | 5,744             | 4,591          | 20.1% fewer   |
| Mean input tokens      | —                 | —              | 9.7% decrease |

The improvement is not uniform — AGENTS.md primarily reduces token usage in high-cost outlier runs rather than providing consistent gains across all tasks.

Adoption: over 60,000 repositories as of early 2026.

### Anthropic Recommendations

- CLAUDE.md should contain only broadly applicable rules. "For each line, ask: 'Would removing this cause Claude to make mistakes?' If not, cut it."
- Don't include code snippets in context files — they go stale. Use `file:line` references instead.
- Use `.claude/skills/` for domain knowledge that is only sometimes relevant.
- Spec-then-implement workflow: Have the agent interview you about requirements, write a spec, then start a fresh session to implement. The fresh session gets clean context focused entirely on implementation.
- Progressive disclosure: "Don't tell Claude all the information you could possibly want it to know; rather, tell it how to find important information."

### Martin Fowler's Analysis

Fowler identifies three implementation levels for spec-driven development:

1. Spec-first — Specs guide initial development, then are discarded
2. Spec-anchored — Specs persist through feature evolution
3. Spec-as-source — Specs are the primary artifact; code is generated

His assessment is cautious:

- Spec-kit output was "repetitive" and "tedious to review"
- Even with detailed specs, agents "ultimately did not follow all the instructions"
- Spec-as-source risks combining "inflexibility and non-determinism" — the worst of both worlds
- Draws parallels to failed model-driven development (MDD) approaches of the 2000s
- Spec-implementation synchronization "remains largely unresolved"

### Thoughtworks Conservative Position

- "Executable code remains the source of truth you need to maintain"
- Specs drive initial generation but cannot be trusted alone because "code generation from spec to LLMs isn't deterministic"
- Specs are inputs to the process, not outputs that track the system

### Addy Osmani (Google Chrome)

- Recommends creating a `spec.md` before coding — describes it as "doing a waterfall in 15 minutes"
- Treats specs as "foundational infrastructure that multiplies AI productivity"

---

## Spec Lifecycle Tiers

Different document types serve different lifecycle stages. Conflating them causes SoT divergence.

| Stage               | Artifact              | Lifecycle                             | SoT Role                         |
| ------------------- | --------------------- | ------------------------------------- | -------------------------------- |
| Pre-implementation  | Spec / PRD            | Drives implementation → archived      | Intent and rationale (temporary) |
| At decision time    | ADR                   | Immutable, kept forever               | Decision rationale (permanent)   |
| During development  | CLAUDE.md / AGENTS.md | Living file, updated with conventions | Project context (evolving)       |
| Post-implementation | Tests                 | Executable, verified every CI run     | Behavioral truth (permanent)     |
| Ongoing             | Code + inline docs    | Updated with code changes             | Implementation truth (permanent) |

---

## SoT Divergence Strategies

### 1. Specs as Input Artifacts

Treat specs as generative inputs that produce code, then evolve into other artifacts. Anthropic's recommended workflow: write spec → fresh session → implement → spec served its purpose. The spec is not maintained; the code is.

### 2. Executable Specifications

BDD tools (Cucumber, SpecFlow) turn specs into tests. The spec IS the test. Eliminates divergence by construction — "a single source of truth eliminates discrepancies between documentation and actual system behavior."

### 3. Immutable Decision Records (ADRs)

Specs describe what was decided and why, not what the system currently does. Append-only. When decisions change, a new record supersedes the old one. Amazon, Google Cloud, Azure, and Red Hat all recommend this pattern.

### 4. Automated Spec-Code Verification

Tools like SpecTBD cross-validate specs against code, flagging drift. Promising but still emerging.

### 5. Tiered Document Lifecycle

Assign each document type a clear lifecycle stage (see table above). Archive behavioral specs once implemented. Keep philosophy/rationale specs permanently. Never maintain two documents that describe the same truth.

### 6. Conservative Approach (Thoughtworks)

Accept that code is the only reliable SoT. Use specs to drive initial generation, then let them go. Tests are the living specification.

---

## When Specs Help vs. Hurt

### Specs help when

- Pre-implementation alignment — Clarifying requirements before writing code
- LLM agent context — Giving agents focused, clean context for implementation
- Cross-team communication — Capturing intent that code alone cannot convey
- Decision rationale — Recording why, not just what (ADRs)
- Complex domain logic — Where behavioral intent is not obvious from implementation

### Specs hurt when

- Maintained alongside code — Two sources of truth for the same concern
- Over-specified — Specs that constrain implementation without adding clarity
- Generated boilerplate — Spec-kit style scaffolding that is "tedious to review"
- Mistaken for guarantees — Specs don't prevent hallucinations or non-deterministic generation
- Documentation for documentation's sake — "Taken to its logical extreme, all documentation is waste" (Agile Alliance)

---

## AI Agent Efficiency

Key data points for justifying spec-driven workflows:

- 28% faster median task completion with project-context files
- 20% fewer output tokens (mean), primarily reducing high-cost outlier runs
- 65% of developers report missing context as the top issue during AI-assisted refactoring (Qodo)
- Fresh session per implementation prevents context pollution (Anthropic recommendation)
- Progressive disclosure over front-loading: tell agents how to find information, not everything at once

---

## Quick Reference

| Artifact     | Lives In                 | Lifecycle          | SoT For             | Update Frequency       |
| ------------ | ------------------------ | ------------------ | ------------------- | ---------------------- |
| Feature spec | `docs/spec/` or `specs/` | Pre-impl → archive | Intent (temporary)  | Never post-archive     |
| ADR          | `docs/decisions/`        | Immutable          | Decision rationale  | Never (supersede)      |
| CLAUDE.md    | Repo root                | Living             | Project conventions | As conventions change  |
| Tests        | `tests/`                 | Living             | Behavior            | With every code change |
| Code         | `src/` / `lib/`          | Living             | Implementation      | Continuously           |
| API docs     | Generated                | Living             | Interface contracts | Auto-generated         |

---

## Sources

> Citation note: several papers below are 2025–2026 work that may postdate an LLM reviewer's training cutoff. Fetch the cited primary source before treating a recent citation as invalid.

### Research Papers

- [On the Impact of AGENTS.md Files on AI Coding Agent Efficiency](https://arxiv.org/html/2601.20404v1) — arXiv, January 2026
- [Spec-Driven Development — From Code to Contract](https://arxiv.org/html/2602.00180v1) — arXiv, February 2026

### Anthropic / Claude

- [Claude Code Best Practices](https://code.claude.com/docs/en/best-practices)
- [How Anthropic Teams Use Claude Code](https://www-cdn.anthropic.com/58284b19e702b49db9302d5b6f135ad8871e7658.pdf)

### Martin Fowler / Thoughtworks

- [Understanding SDD — Kiro, spec-kit, and Tessl](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [SDD — Unpacking 2025's Key Practices](https://www.thoughtworks.com/en-us/insights/blog/agile-engineering-practices/spec-driven-development-unpacking-2025-new-engineering-practices)
- [Spec-Driven Development](https://thoughtworks.medium.com/spec-driven-development-d85995a81387) — Thoughtworks Medium

### Practitioner Guides

- [My LLM Coding Workflow Going Into 2026](https://addyosmani.com/blog/ai-coding-workflow/) — Addy Osmani
- [How to Write a Good Spec for AI Agents](https://addyosmani.com/blog/good-spec/) — Addy Osmani
- [Writing a Good CLAUDE.md](https://www.humanlayer.dev/blog/writing-a-good-claude-md) — HumanLayer

### GitHub / Tooling

- [Spec-Driven Development with AI (spec-kit)](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)
- [spec-kit spec-driven.md](https://github.com/github/spec-kit/blob/main/spec-driven.md)

### Industry Reports

- [State of AI Code Quality](https://www.qodo.ai/reports/state-of-ai-code-quality/) — Qodo
- [How SDD Improves AI Coding Quality](https://developers.redhat.com/articles/2025/10/22/how-spec-driven-development-improves-ai-coding-quality) — Red Hat

### Docs-as-Code

- [What is Docs as Code?](https://konghq.com/blog/learning-center/what-is-docs-as-code) — Kong
- [Docs-as-Code Explained](https://buildwithfern.com/post/docs-as-code) — Fern
- [Docs-as-Code Benefits](https://www.techtarget.com/searchapparchitecture/tip/Docs-as-Code-explained-Benefits-tools-and-best-practices) — TechTarget
- [Docs Culture at Amazon, Google, Meta, Stripe, Anthropic](https://twocentspm.substack.com/p/docs-culture-at-amazon-google-and)

### Architecture Decision Records

- [Master ADR Best Practices](https://aws.amazon.com/blogs/architecture/master-architecture-decision-records-adrs-best-practices-for-effective-decision-making/) — AWS
- [ADR Overview](https://cloud.google.com/architecture/architecture-decision-records) — Google Cloud
- [Maintain an ADR](https://learn.microsoft.com/en-us/azure/well-architected/architect-role/architecture-decision-record) — Microsoft Azure
- [Why You Should Be Using ADRs](https://www.redhat.com/en/blog/architecture-decision-records) — Red Hat

### Executable Specs / Living Documentation

- [Living Document Specifications](https://www.spectbd.com/) — SpecTBD
- [SDD and AI Agents Explained](https://www.augmentcode.com/guides/spec-driven-development-ai-agents-explained) — Augment Code
- [Specification by Example](https://fastercapital.com/content/Specification-by-Example--How-to-Create-Living-Documentation-for-Your-Software.html) — FasterCapital

### Agile / Counterarguments

- [An Agile Focus on Minimalism](https://www.agilealliance.org/an-agile-focus-on-minimalism/) — Agile Alliance
- [Documentation in Agile — How Much?](https://www.infoq.com/news/2014/01/documentation-agile-how-much/) — InfoQ
