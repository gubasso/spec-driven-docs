# Guidance

What a release asks of an instance that takes it, as data a plan can filter rather than prose an operator reads whole.

A release that asks nothing declares `none` in `index.toml`. A release that asks something carries `<version>.toml` beside this file and one prose body per step under `<version>/`. Absence never means two things: the index has one entry per release from the capability floor onward, so a missing prose file cannot strand an interval and a missing entry is a release-time failure rather than a silent gap.

## Authoring one

1. Add the index entry in the same change as the release. Every release has exactly one, selecting `none` or a file.
2. Where the release changes a managed projection's bytes, drops an adopted seed, retires a rule ID, adds a declaration key, or widens a gate's judged set, `none` is not available. Write the file.
3. Each step declares `id`, `kind`, `breaking`, `destinations`, `actor`, and `text`. The identifier is a slug that stays put for the life of the release, because a decision identifier derives from it and a reworded body must move no plan's identity.
4. Keep the prose body to what a person does. The plan reads the TOML and hands the body over; it never parses the Markdown.

## The step kinds

- `seed-added`: an adopted seed the projection now lands.
- `rule-retired`: a rule ID that no longer resolves, whose citations must re-point.
- `managed-changed`: a managed file whose bytes changed.
- `declaration-key-added`: a key the project can now set.
- `gate-widened`: a gate whose judged set grew.
