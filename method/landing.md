# Landing

How a target moves from what it is to what the installed release says it should be. The subject is one repository and one version: the one the operator installed. Migrating a corpus onto the method is the question of [Migration](./migration.md). Installing and owning the files is the question of the [instance guide](../instance/README.md). This chapter owns what every landing shares: one candidate, one comparison, one write.

## The candidate is the version you installed

A candidate is what the running binary would put in a target. It is rendered from the binary's own embedded sources, and from nothing else. There is no selector, because acquisition belongs to the project manager the operator already runs: to land the shape release 0.8.0 lands, install release 0.8.0 and run it.

That is the whole acquisition rule, and it buys the thing a cross-release engine could never prove. A release is the only honest witness of what it lands, so the binary that renders a candidate is the binary that published it.

## One projection owns every byte

The projection is a pure function. It takes the resolved profile, the documentation root, the recorded values a landing carries forward, and the evidence the caller read from the target. It returns an ordered list of destinations, each with its complete bytes, its ownership, its placement, and the source that produced it, plus the record that describes them.

It reads no filesystem, no clock, and no network. Observation happens before the call and arrives as values, which is why a stage and a production run that observed the same target render the same bytes.

## The stage is evidence, never input

`sdd stage` writes that candidate into a directory beside the target. It holds every destination under `artifacts/`, the installed version's own method, specs, templates and skills under `reference/`, and a receipt naming what it rendered.

The stage is for reading. Production renders the candidate again from scratch and accepts no staged byte, because bytes that reached production through a stage would make the stage a cache, and a stale cache lands a version nobody chose. What a landing may take from a stage is the choices its receipt records: `sdd init --stage <root>` and `sdd upgrade --stage <root>` land the profile, docs scratch, writing style, reserved paths, and acquisition decision the stage was rendered with, and refuse a receipt another release or another target wrote. Over an instance, `sdd stage` itself takes the recorded choices, so the stage describes that instance's next landing.

`sdd stage compare <root>` reads the stage against the target, one row per destination: whether the target holds the staged bytes, who owns the destination, and, where the two differ, its provenance. The record half of the provenance is the attribution check a landing refuses on, and the history half is whether any commit touched the path. Before a landing, the rows that differ with reduced or unknown provenance are the decisions an operator owes. After it, every managed and integration row reads `equal`, which is the proof the landing wrote what the stage showed.

The stage survives the landing it informed, so the comparison an operator wants most, the one after the write, is the one they can still make. Only `sdd stage clean` removes it.

## The classifications

A classification is read from the target and never from the verb the operator typed. `sdd status --json` reports it as `classification`, and every front reads the same rule. Seven values cover it.

- `setup`: no instance and nothing durable written.
- `migration`: no instance and a settled corpus in a recognized home, or a migration the declaration holds open, whatever the files beneath it show.
- `needs-decision`: no instance, and documents outside any recognized home. The operator names where they belong before anything lands.
- `upgrade`: an instance older than the installed version.
- `drift`: an instance at that version whose files have moved.
- `current`: an instance at that version with nothing to do.
- `invalid`: a record that reads and a declaration that does not, or a record naming a destination no operation may name.

A record that does not parse at all is a command error, exit 65, rather than a report. An unreadable target is a command error rather than a guessed landing. Absence and unreadability are different facts, and a tool that reports the second as the first lands seeds over something it never read.

## What decides before the first write

A landing settles everything it can refuse on before a byte moves.

- Containment. Every destination resolves beneath the opened target, checked component by component without following a link. A link, a parent traversal, or an absolute path refuses the run.
- Attribution. A whole file this tool would own must be one the record accounts for. A destination holding bytes nothing vouches for refuses the run and names it, because that file is exactly the one somebody else wrote.
- The lock. One writer holds a target for the whole run, and a second refuses at once naming the holder.

Missing provenance routes to the agent, never to an automatic overwrite. The setup skill owns that judgment, and [Migration](./migration.md) owns the procedure.

## What a landing writes, and in what order

Each destination is replaced in its own directory, one at a time, and the record is written last. Until the record lands, the previous one still describes the target, which is what the next run reads.

A managed file the release no longer lands is taken back, and only where the record vouches for the bytes that are there. An adopted file a release stops seeding stays: the project owns it from the moment it lands, and a version moving is not permission to take it back.

## Honest failure

There is no journal, no rollback, and no stored result. A run that stops partway leaves whole files, the previous record, a visible Git diff, and a command that runs again. It names the destinations that hold candidate bytes, and running it again finishes the rest.

This tool claims no universal filesystem atomicity and no all-or-nothing guarantee. What it claims is that every file is whole at every moment, that the record never describes a landing which did not finish, and that a rerun is safe.

## The fronts

| Verb                | Serves                    | Writes           |
| ------------------- | ------------------------- | ---------------- |
| `sdd stage`         | every classification      | the stage only   |
| `sdd init`          | setup                     | the destinations |
| `sdd upgrade`       | upgrade, drift, current   | the destinations |
| `sdd assess`        | every classification      | nothing          |
| `sdd status`        | every classification      | nothing          |
| `sdd stage compare` | one stage this tool wrote | nothing          |
| `sdd stage clean`   | one stage this tool wrote | nothing else     |

A front refuses a classification it does not serve, naming what the target turned out to be and what serves it. `sdd init` refuses a settled corpus with no instance, because landing seeds there starts a second convention beside the first, and it refuses documents with no recognized home until the operator names one. `sdd upgrade` refuses an absent instance. Neither widens its own scope, because a front that did would be the classification lying about the target.

`sdd status` reports. A version gap it prints is a fact about the target, not work anybody approved. It also lists what a release that stored plans left under the state and cache roots, by exact path, and whether it still binds a run: no verb of this release acts on that state, an unfinished run is finished or abandoned with the older binary, and removing the rest is the operator's. Where the instance is newer than the binary, the classification is `null`, because an older binary cannot judge a record a newer release wrote.

## The three reconciliations

None of the three is a single verb, and each has its own shape.

Managed drift. A managed file is a byte-for-byte projection, so an edit is a conflict rather than a preference. The landing refuses and names it, and the stage carries the candidate's bytes to compare against. Resolve it by name: `sdd upgrade --replace <path>` saves the edited bytes under the docs scratch, or under the state root where none is declared, then overwrites exactly that file, and every conflict not named still refuses. Keeping the edit means porting it from the saved copy into a file the project owns. Never widen the landing to keep the edit, because a managed file that keeps one is no longer managed. A managed file a release retires is compared the same way: an edited one is a conflict until it is named.

Adopted drift. An adopted file is seeded once and owned locally, and the record holds both the landed bytes and the upstream baseline. An edit is kept and the baseline moves. This is the ownership working, not a defect to fix.

A retired rule ID. A release that retires a rule leaves every citation of it dangling. `sdd upgrade --dry-run` lists each citation by file and line before anything is written, the changelog names the retirement, the operator re-points the citations, and the decision-record citation gate fails until they are re-pointed. This tool never rewrites a citation, because it cannot know which replacement the author meant.

## What a landing is not

A profile change or a documentation-root move is a named migration, not a consequence of a new version arriving. An operator asks for it deliberately, and the one resolved root serves the whole run: the root the candidate records is the root budget discovery measures and the landed gate configuration reads.

A migration's registry is written by the `sdd migrate` verbs alone, into a workspace under this tool's state root. [Migration](./migration.md) owns the registry's shape.

## One name, two surfaces

`sdd policy reconcile` appends a rule to the project's own specs. It is a direct policy edit an operator asks for, and it has nothing to do with landing. A reader who meets the word in both places deserves the sentence.
