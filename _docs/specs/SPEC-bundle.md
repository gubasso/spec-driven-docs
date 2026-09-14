# Bundle Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`bundle:a-release-is-read-through-one-seam` — A release is read through one seam](#bundlea-release-is-read-through-one-seam--a-release-is-read-through-one-seam)
  - [`bundle:a-release-declares-what-it-lands` — A release declares what it lands](#bundlea-release-declares-what-it-lands--a-release-declares-what-it-lands)
  - [`bundle:the-protocol-version-is-a-range-the-engine-declares` — The protocol version is a range the engine declares](#bundlethe-protocol-version-is-a-range-the-engine-declares--the-protocol-version-is-a-range-the-engine-declares)
  - [`bundle:a-pre-schema-release-is-cataloged-or-unavailable` — A pre-schema release is cataloged or unavailable](#bundlea-pre-schema-release-is-cataloged-or-unavailable--a-pre-schema-release-is-cataloged-or-unavailable)
  - [`bundle:a-fetched-archive-is-verified-before-it-is-read` — A fetched archive is verified before it is read](#bundlea-fetched-archive-is-verified-before-it-is-read--a-fetched-archive-is-verified-before-it-is-read)
  - [`bundle:resolution-happens-once-and-writes-only-the-cache` — Resolution happens once and writes only the cache](#bundleresolution-happens-once-and-writes-only-the-cache--resolution-happens-once-and-writes-only-the-cache)

<!--TOC-->

## Purpose

Rules governing how this binary reads a release other than the one compiled into it. They cover the content seam, the projection declaration, the protocol version between an engine and a bundle, the finite catalog of releases that predate that declaration, and the one network read the tool makes. No instance adopts this spec: its subject is the engine, so an instance holding these rules holds obligations it cannot violate and verifications it cannot run. What a release lands into a target is stated in `SPEC-distribution.md`. What this repository owes at release time is stated in `SPEC-release.md`.

## Requirements

### `bundle:a-release-is-read-through-one-seam` — A release is read through one seam

Every verb that lands, verifies, or reconciles a target MUST read a release's bytes through the bundle interface and MUST NOT name an embedded payload accessor of its own. The interface MUST answer with a manifest and with blobs by digest, and nothing else.

#### Scenario: A landing verb reaches past the seam

- GIVEN a landing service edited to read the embedded payload directly
- WHEN the canon test suite runs
- THEN it fails naming the service, because a verb that names the embedded payload can only ever describe the release it was compiled with

Verify: `cargo nextest run -E 'binary(canon)'`

### `bundle:a-release-declares-what-it-lands` — A release declares what it lands

A release MUST carry `instance/projection.toml`, declaring its profiles, their documentation roots, its managed and adopted pairs, its canon templates, and its sentinels. The engine MUST read that declaration from the bundle it was asked about and MUST NOT substitute its own.

#### Scenario: A plan is computed toward an older release

- GIVEN a release whose adopted set is smaller than this engine's
- WHEN the engine computes what that release lands
- THEN it lands that release's set, because the release is the witness of what it landed and the running engine is not

Verify: `cargo nextest run -E 'binary(canon)'`

### `bundle:the-protocol-version-is-a-range-the-engine-declares` — The protocol version is a range the engine declares

The declaration MUST carry `payload_schema`, and the engine MUST declare the inclusive range it decodes and carry one decoder per schema in it. A schema outside that range MUST be refused by naming an engine that reads it. The installed-record schema is a separate axis and MUST fail independently.

#### Scenario: A bundle declares a schema this engine does not carry

- GIVEN a bundle whose declaration reads a schema above the engine's range
- WHEN the engine reads it
- THEN it refuses naming which engine to install, because assuming every lower integer stays readable forever is how a decoder is silently retired

Verify: `cargo nextest run -E 'binary(canon)'`

### `bundle:a-pre-schema-release-is-cataloged-or-unavailable` — A pre-schema release is cataloged or unavailable

A release published before the declaration MUST be read only through an audited catalog entry, whose descriptor supplies that release's projection and source-path facts and whose digest the index records. An uncataloged or ineligible pre-schema release MUST be reported unavailable with its evidence, and the engine MUST NOT apply the current projection to it.

#### Scenario: A destination below the capability floor is named

- GIVEN a release whose published archive carries no seed root
- WHEN the engine is asked to read it
- THEN it reports the release unavailable, names the missing root and the capability floor, and fetches nothing

Verify: `cargo nextest run -E 'binary(cmd_payload)'`

### `bundle:a-fetched-archive-is-verified-before-it-is-read` — A fetched archive is verified before it is read

A fetched archive MUST be verified against the registry index checksum before it is parsed, MUST be read under caps on compressed bytes, expanded bytes, entry count, and per-file bytes, and MUST admit only regular files under a declared payload root. An absolute path, a parent traversal, a link, a duplicate logical path, and an entry outside the package prefix MUST each refuse the whole archive. A refused archive MUST leave nothing cached.

#### Scenario: An archive carries an entry that climbs out of the package

- GIVEN a published archive holding an entry named above its own root
- WHEN the engine reads it
- THEN it refuses the archive naming the entry, writes no cache entry, and touches no target

Verify: `cargo nextest run -E 'binary(canon)'`

### `bundle:resolution-happens-once-and-writes-only-the-cache` — Resolution happens once and writes only the cache

Resolution MUST freeze one exact version, one registry checksum, and one payload digest, and content access MUST NOT resolve again. A verified bundle MUST be cached under the cache root and nowhere else, and `--offline` MUST accept an exact selector only from that cache and MUST refuse `latest` with its reason.

#### Scenario: An operator asks for the newest release with no network

- GIVEN `--offline` and the selector `latest`
- WHEN the engine resolves
- THEN it refuses, because only the index says which release is newest and a cached answer cannot prove freshness

Verify: `cargo nextest run -E 'binary(cmd_payload)'`
