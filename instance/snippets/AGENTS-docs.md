## Documentation

- Load the affected specs before editing governed content: `{docs_root}/specs/SPEC-<domain>.md`.
- Treat decision records as immutable rationale and load them only when asked why.
- Write technical text in SimpleEnglish `Plain` mode, the default for documentation, guides, agent instructions, reference, and error messages. Load `{docs_root}/specs/SPEC-simple-english.md` and `.spec-driven-docs/upstreams/simpleenglish/skills/simple-english/SKILL.md`.
- Write and edit step-by-step guides to the adopted guides spec, `{docs_root}/specs/SPEC-guides.md`.
- Run `sdd verify` before handoff.
- Keep adopted specs, the tracking registry, and local integration instance-owned.
