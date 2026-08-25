# Glossary

Terms this framework fixes, each resolved at the chapter that owns it. A term is defined once; this page points rather than restates.

| Term                     | Means                                                                   | Owner                 |
| ------------------------ | ----------------------------------------------------------------------- | --------------------- |
| Spec                     | the current contract for one domain, loaded before work                 | `02-specs.md`         |
| Domain                   | a capability a spec covers; a shelf in a knowledge base                 | `01-placement.md`     |
| Shelf                    | one top-level subject directory, holding its own chapters and index     | `01-placement.md`     |
| Requirement              | one `` ### `<rule-id>` `` block: title, ID, statement, scenario, verify | `03-rules.md`         |
| Rule                     | the same block, named by what it does rather than where it sits         | `03-rules.md`         |
| Rule ID                  | `<spec-slug>:<rule-slug>`, the citation token                           | `03-rules.md`         |
| Statement                | the single binding sentence in an EARS pattern                          | `03-rules.md`         |
| Scenario                 | the GIVEN / WHEN / THEN case separating compliance from breach          | `03-rules.md`         |
| Verification command     | the command that exits non-zero when a rule is violated                 | `03-rules.md`         |
| Decision record          | one immutable entry in the log of why                                   | `04-decisions.md`     |
| Disposition              | what happened to a considered option: chosen, rejected, deferred        | `04-decisions.md`     |
| Reopening condition      | what would put a deferred option back on the table                      | `04-decisions.md`     |
| Kind prefix              | the uppercase `SPEC-`, `ADR-`, `KI-`, or `TEMPLATE-` leading a name     | `01-placement.md`     |
| Companion directory      | `SPEC-<domain>/`, holding artifacts a requirement names                 | `02-specs.md`         |
| Docs root                | `docs/` for a codebase, `_docs/` for a content tree                     | `01-placement.md`     |
| Zone                     | a directory under the docs root serving one reader need                 | `01-placement.md`     |
| Author-instructions file | the always-loaded digest and router at a directory                      | `05-agent-context.md` |
| Entry document           | the one document a unit of work loads, which names its sources          | `05-agent-context.md` |
| Draft                    | exploratory material in the gitignored workshop                         | `07-lifecycle.md`     |
| Promotion                | rewriting a draft into its owning zone and deleting the draft           | `07-lifecycle.md`     |
| Tracking registry        | the machine-readable record of facts that expire                        | `07-lifecycle.md`     |
| Unenforced rule          | a real rule that no command decides, declared as such                   | `08-gates.md`         |
| Enact                    | to change behavior so a rule's verification command passes              | `09-spec-to-code.md`  |
| Typed clause             | `ADDED`, `MODIFIED`, or `REMOVED` plus a rule ID, in an entry doc       | `09-spec-to-code.md`  |
| Coverage tag             | `SATISFIES` or `VERIFIES` plus a rule ID, in a source comment           | `09-spec-to-code.md`  |
| Suppression              | a failing or expected-failing test naming its case and its exit         | `09-spec-to-code.md`  |
| Known-issue case         | a live defect in a system the project does not own, recorded once       | `07-lifecycle.md`     |
| Case id                  | `KI-<slug>`, the known-issue record's filename and citation token       | `07-lifecycle.md`     |
| Mask                     | a temporary workaround, carrying the condition that removes it          | `07-lifecycle.md`     |
| Mitigation               | a permanent guard that stays after the upstream fix                     | `07-lifecycle.md`     |
| Clarification marker     | `[NEEDS CLARIFICATION: <question>]` at an undecided requirement         | `03-rules.md`         |
| Catalog                  | a document holding one entry per rule and no argument                   | `06-format.md`        |
| Agreed behavior          | behavior an open unit of work cites by rule ID; the spec wins           | `00-model.md`         |
| Observed behavior        | behavior no open work cites; the code wins                              | `00-model.md`         |
| Canon                    | this repository: the method, its templates, and its gates               | `instance/README.md`  |
| Instance                 | a project that installed the canon, holding its own specs               | `instance/README.md`  |
| Managed file             | a file the canon owns in an instance; an edit is a conflict             | `instance/README.md`  |
| Adopted file             | a file the canon seeds and the instance then owns                       | `instance/README.md`  |
| Integration block        | a marked region the canon owns inside a file the instance owns          | `instance/README.md`  |

Terms this framework deliberately does not use, because each is a synonym that weakens retrieval for the one it duplicates: specification document, contract, requirements doc, ADR log, decision log, constitution, steering file, policy.
