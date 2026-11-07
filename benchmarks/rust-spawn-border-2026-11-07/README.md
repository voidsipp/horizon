# Natural-spawn chunk checks, 2026-11-07

Follow-up to [PR #11467](https://github.com/horizon-rust/horizon/pull/11467) and [issue #11748](https://github.com/horizon-rust/horizon/issues/11748), proposed against the original PR's branch at `4807f13288c50bb9291d8af1e341ad747cb1023d`. Validation below used local integration baseline `0000cd3e7d998d561533a3080660be1fe41b3a15` plus the original author's two changes, cherry-picked locally as `7aab071` and `d0e8253`. All modified existing files and `ticket_levels.rs` are identical between that test baseline and the proposed base. Full checks on the fork-based submission remain pending; the original contributor's changes are not repeated in this diff.

Eligibility now includes entity-ticking **loading** status, as vanilla 26.3 `PersistentEntitySectionManager.canPositionTick` does. Simulation range is a separate rule. Initial candidates already passed resident/ticking-chunk checks and avoid another resident-map lookup; cross-chunk attempts retain it. Filtering stays after shuffle and inhabited-time updates. A ticket snapshot is kept per borrowed level pass.

`BorderGeometry.is_chunk_within_bounds` reads one extent and performs four comparisons. Both shifted 32-bit block corners are exactly representable as doubles. Java wrapping shifts, inclusive minima and exclusive maxima are preserved, as are unrestricted placements and existing entities.

## Measurements

Median **nanoseconds per candidate**, not FPS or total spawning-tick time. All timed variants assert the same accepted count and include the same corrected status predicate. The map has ChunkMap's Fx hasher and one-word Arc values. Nine rotated-order samples, 8000 passes of 289 chunks each, pinned to logical CPU 2 on an i5-11260H; raw minima/maxima are in the CSVs. Desktop activity is uncontrolled.

| Profile | Scenario | Moving | Reference | One extent, keep lookup | One extent, resident candidates | Reduction |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| fast | inside | false | 10.697 | 7.134 | 5.407 | 49.5% |
| fast | edge | false | 8.696 | 6.711 | 5.183 | 40.4% |
| fast | outside | false | 7.238 | 6.064 | 4.427 | 38.8% |
| fast | inside | true | 18.288 | 11.096 | 9.012 | 50.7% |
| fast | edge | true | 12.624 | 10.968 | 9.023 | 28.5% |
| fast | outside | true | 10.803 | 9.789 | 7.942 | 26.5% |
| thin | inside | false | 9.253 | 6.998 | 5.403 | 41.6% |
| thin | edge | false | 7.358 | 6.660 | 5.090 | 30.8% |
| thin | outside | false | 6.623 | 6.067 | 4.449 | 32.8% |
| thin | inside | true | 17.002 | 11.120 | 8.956 | 47.3% |
| thin | edge | true | 12.335 | 11.639 | 8.930 | 27.6% |
| thin | outside | true | 11.038 | 10.206 | 7.935 | 28.1% |

Savings for one 289-candidate pass are only a few microseconds. Actual placement, collision checks, generation and rendering are outside this benchmark.

```sh
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo build --release -p horizon-core --example spawn_border_bench
taskset -c 2 target/release/examples/spawn_border_bench 8000
CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 cargo build --release -p horizon-core --example spawn_border_bench
taskset -c 2 target/release/examples/spawn_border_bench 8000
```

## Validation and limits

- Seven actual Game spawning tests passed, including a loaded inactive neighbor, nine shuffled candidates and border changes with an unchanged player/chunk cache. The shuffle RNG fingerprint comes from Java Random(12345) and Collections.shuffle of nine elements.
- Two core border tests passed. Client and server vanilla border oracles each matched 1,319,610 output lines, including chunk bounds, fractional/extreme edges and moving borders.
- 360 fixed flat-world natural-spawn cases matched byte for byte, with 126 actual spawns, covering generation, initial chunk gates and cross-chunk attempts. The oracle executes the real ServerLevel and entity-manager status predicates. Its Unsafe fixture explicitly initializes added state. The older general height-sampling gap is outside this report. Full hashes/logs are in ignored `.local/rust-natural-spawn-parity/report.json` and `.local/rust-world-border-parity/report.json`.
- Python tests: 719 run, two skipped, passed. Formatting and public-tree checks passed. All-target core Clippy passed with the existing `manual_is_multiple_of` warning in unchanged `storage.rs:223` allowed. Strict Clippy in pr_ready failed at that warning; the broad workspace-test phase was interrupted during resource pressure and is not reported as passed.
- The port-ledger comparison completed. Its new getter-gap location is a heuristic artifact: the adapter reads the active `game.border` directly rather than calling a method named getWorldBorder. The real-vanilla oracle covers that access; existing unrelated chunk-broadcast/piston gaps remain.
- The upstream baseline lock omits horizon-protocol's declared socket2 0.5 dependency. Local tests/builds used a minimally regenerated lock adding 0.5.10. This unrelated repair is excluded from the follow-up; the tested copy is retained in ignored `.local/Cargo.lock.tested`.
- The manual-test client uses fast dependencies with the gameplay crate at opt-level 1 and 64 codegen units to fit laptop memory. Both benchmark configurations use opt-level 3. No graphical play session or FPS test was performed. Build-limit feedback remains vanilla's existing height message.
