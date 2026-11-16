# Deterministic checkpoint contract (schema 8)

A current checkpoint is an exact continuation point at a completed tick, for the
same engine build, configuration and deterministic execution environment. A seed
alone is not a continuation point. The complete ChaCha8 state, ordered entity
collections and all behaviour-relevant temporal state must survive.

## Restore, migrate, recover

- `Simulation::from_checkpoint_bytes` is the untrusted-input boundary used by
  native strict loading and WASM import. Schema 8 requires its runtime sections,
  RNG, complete grid layers, and recursively all fields emitted by the writer. A current
  runtime section without its current version tag is rejected, not migrated.
  Sparse fields deliberately omitted by the format retain their documented
  defaults. It parses the original bytes into ordered maps; rebuilding a world
  from `serde_json::Value` would reorder keys.
- Current `from_save` restores a trusted, already decoded in-memory checkpoint.
  It validates the current runtime section and never runs legacy normalization.
  External callers must use the byte decoder to retain missing-field detection.
- `migrate_legacy_checkpoint` accepts schemas 0–7, checks supported map sizes and
  runs the older import/repair rules with a migration warning. These versions
  omitted information, so migration establishes a new continuity baseline. It
  cannot reproduce an unknowable historical heading, route, work plan or precise
  learned value. It does not claim equivalence to the old binary's future.
- `load_or_new` is explicitly the recovery/startup policy: it preserves an invalid
  input under a backup name before starting a fresh world. `load_strict` returns
  an error without changing the input. No current-schema corruption is repaired
  inside exact restoration.

## Causes addressed

The original loader reapplied ocean normalization, generated empty vocabularies,
recalculated settlement levels, sorted religions and repaired counters even for
current saves. The save also omitted animal headings and activity flags, navigation and blocked-goal
memory, work plans, recent harm, motion history used by steering/rewards, demographic configuration, ferry state, social
cooldowns, prior census/aggregate state and the last claimant of overlapping
territory. Skills, work feelings and lessons were quantized by legacy named-map
export helpers. Some in-memory history queues were shortened at save time. Building
`last_work_tick` was skipped by Serde even though it controls abandonment.

Schema 8 keeps the existing named skill/lesson projections for compatibility and
inspection, but the exact arrays in the organism runtime section are authoritative.
Restoration replaces those projections with the exact arrays. Learning and social
algorithms are unchanged. Empty memory stores and empty vocabularies remain empty.
Seed zero is a valid seed. A missing RNG in a current checkpoint is an error.

The new runtime section preserves physics state, dirty-trail order, active
structure order, ferry surveys/queues, animal census, lineage aggregates and craft
state, smog, peril/death windows, evening/school destinations, festival/teaching/
revival/faith cooldowns, graves/cemeteries, cared-for orphans and pending memory
flushes. Tuple-keyed maps are encoded as ordered entry pairs.

## Deliberately transient state

Network frame bases, dirty UI flags, serialized
revisions, cached JSON views, command-result presentation, reporting counters and
the display-only tribe chronicle are not authoritative simulation state. They
start fresh for a new observer/session. They do not drive decisions or consume
simulation randomness. This change does not add a permanent historical archive.

Settlement census snapshots are also preserved: although tick-keyed, commands
between ticks may still read the cached view. Navigation, lineage aggregates and
ferry surveys likewise contain history or temporal decisions and are persisted.
The building position index and discovery bit masks are reconstructed from the
same ordered buildings and discovery names; their definitions do not depend on
historical state. Regression continuation checks exercise their reconstruction.

## Storage and scope

ChaCha8, exact packed numeric grid layers, buffered JSON output, temporary-file
writing, file synchronization and atomic replacement are retained. Checkpoints
copy the current in-memory history queues without imposing additional retention.
The engine's existing retention rules are not changed.

Checkpoint size and load validation cost increase. The byte decoder temporarily
builds JSON validation trees in addition to the typed state; this is a known
memory cost, not an incremental-checkpoint implementation. No performance claims
are made for an MSI machine or for cross-platform determinism without tests there.
The existing server memory-pressure policy still changes cognition under pressure;
reproduction requires the same external policy/input sequence. That policy is
outside this persistence-only change.

## Reproduction and acceptance

The baseline executable from commit
`4667dc190f1a3531466d3601e55d5a01d6d97711` was run before changes:

```sh
headless --seed 42 --ticks 6100 --every 1000 --save continuous.json
headless --seed 42 --ticks 6000 --every 1000 --save checkpoint.json
headless --load checkpoint.json --ticks 6100 --every 1000 --save resumed.json
```

The baseline complete-state and RNG comparisons both failed. The same experiment
must pass with schema 8. Regression tests compare serialized authoritative state
byte for byte (including map/array order) and compare RNG state independently.
They cover zero-tick restoration, ticks 6001 and 6100, seeds 0/1/7/42, temporal
state and memory, custom population limits, incomplete current checkpoints and
repeatable legacy migration. A dedicated RNG test also covers a non-default
stream and a word position greater than u64, across buffer refills.

Required verification:

```sh
cargo test --release --locked --workspace
```

Actual final results are recorded in the pull request and accompanying validation
report; this contract is not itself a claim that every platform has been tested.

## Declared legacy reconstruction (schemas 0–7)

Missing state is irrecoverable, not silently treated as an exact checkpoint:

| Missing older state | Migration policy |
| --- | --- |
| Population limit | Existing engine default, currently 350 |
| RNG | Historical seed/tick fallback if absent; never used for schema 8 |
| Animal heading/activity | Constructor heading from ID; inactive activity flags |
| Navigation/work/recent harm | Empty route, no work plan or recent harm |
| Motion history | Constructor position and zero smoothed velocity |
| Exact learned arrays | Import the older quantized named maps; lost precision cannot be recovered |
| Social/ferry/death-window runtime | Empty/default state, then normal future scheduling |
| Physics | Historical tick/5 cadence and fire scan |
| Territory owner, settlement levels, infrastructure | Historical reconstruction/repair rules |
| Building last work tick | None; existing simulation falls back to construction start |

The migration is repeatable for a given input and engine, but it starts a new
experimental baseline. A migrated world written as schema 8 then supports exact
continuation. Keep the original file when migrating: headless `--load old.json
--save migrated.json` reads without replacing its input.
