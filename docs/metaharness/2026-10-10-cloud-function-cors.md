# MetaHarness evidence: Cloud Function CORS allowlist

## Scope and frozen contract

- Baseline: `main` at `7e0ec19c3035e98176db0d31cdef2d3ea9616b31`.
- Source candidate: restrict `Access-Control-Allow-Origin` to an exact configured origin, normalize comma-separated configuration, and emit `Vary: Origin` for dynamic allowlists.
- Compatibility anchors: explicit wildcard behavior, handler export, unknown-route 404, missing-backend 503, and unchanged Rust core/Cargo manifests.
- Project-source optimization and harness optimization were evaluated separately. The project candidate was eligible for Flywheel promotion; Darwin did not promote a harness mutation.

## Pinned tooling

Upstream MetaHarness revision: `e0dfd44da72b7adfb7c57fdd42d124be58ed7086`.

| Component | Version |
| --- | --- |
| `metaharness` | `0.4.17` |
| `@metaharness/darwin` | `0.10.3` |
| `@metaharness/flywheel` | `0.1.12` |
| Node.js | `24.19.0` locally; CI pins Node.js 20 |
| Rust | `1.99.0` (`stable`, 2026-10-01 release manifest) |

## Reproducible results

| Gate | Baseline | Candidate | Status |
| --- | ---: | ---: | --- |
| Cloud Function contract (`npm run test:cf`) | 2/7 | 7/7 | PASS candidate |
| npm production audit, high threshold | 0 vulnerabilities | 0 vulnerabilities | PASS |
| Flywheel CORS holdout | 1/6 (0.1667) | 6/6 (1.0000) | PROMOTED |
| Flywheel frozen anchor | 5/5 | 5/5 | PASS, no regression |
| Darwin real-sandbox score | 0.435 | 0.435 | NO_IMPROVEMENT |
| `cargo fmt --all -- --check` | Existing workspace formatting drift | Unchanged | BLOCKED |
| `cargo check --workspace` | Local environment lacks `pkg-config`/OpenSSL development metadata | Unchanged | BLOCKED locally |

The untouched implementation returned the first configured trusted origin for both a disallowed request origin and a request without an `Origin` header. The candidate withholds the allow-origin header in both cases. The test suite also covers whitespace normalization, allowed-origin reflection, wildcard compatibility, preflight behavior, and routing fail-closed anchors.

MetaHarness repository scoring completed with exit 0: harness fit 71, compile confidence 100, task coverage 100, tool safety 95, memory usefulness 47, estimated cost per run USD 0.048, and all six hard constraints satisfied.

Darwin was invoked with a fixed deterministic mutator, seed `2026101014`, one generation, two children, concurrency one, and the real sandbox. The baseline and both children scored 0.435 with test pass rate 0; the baseline remained the winner with delta 0.000. No harness mutation was copied into project source.

Flywheel used a deterministic evidence proposer with a frozen six-item CORS holdout and five-item anchor. The default conjunctive gate promoted the exact candidate with primary delta `+0.8333333333333334`, no-op rate `0.8333 -> 0`, cost per win `1 -> 0.1667`, and no regression. Gate fingerprint: `c5942e7c9bf8f28fe7af47a6ec3887d11eeb3f8c8e8601cc6a7a54969ea0b981`. Independent replay passed Ed25519 receipts, root reconstruction, contiguous parents, promotion verdicts, the full diagnostic ledger, sealed score fields, and the pinned gate fingerprint.

## CI and rollback

The CI addition runs a clean npm install, the Cloud Function contract suite, and the production dependency audit on Linux with commit-pinned checkout and Node setup actions. It is included in the existing `All Checks Pass` aggregate and does not remove or relax any Rust gate.

The first exact GitHub run proved the Cloud Function job green but found every existing Rust job failing during job setup because the workflow referenced the nonexistent `dtolnay/rust-action`. The candidate replaces those references with the official `dtolnay/rust-toolchain` action pinned to `e2a55d2ffb04f378e9626c28d38b36d230d1e12f` and explicitly requests the stable toolchain. This repairs evaluator execution without changing a Rust test, lint rule, or aggregate requirement.

If a post-merge regression is discovered, revert the PR merge commit. This restores the previous CORS behavior, test script, and CI job as one atomic rollback. No deployment or package publication is part of this change.

## Strict Rust gate recovery (2026-10-15)

The recovery cycle kept the model and six-gate evaluation contract fixed. Its baseline was PR head `0e4304aaf3e8e9ced19b680b22f7ea0f6010645d`: Cloud Function tests and npm audit passed, while rustfmt, build, tests, coverage, and lint failed. The first compiler failure was an undeclared `database` feature guarding an `sqlx::Error` conversion. Formatting drift then concealed pre-existing missing documentation, dead code, Clippy findings, and a non-monotonic metric-confidence test.

Darwin `evolve-numeric` ran two generations with four children, concurrency one, seed `20261015`, and a frozen real evaluator: `cargo check -p llm-research-core --all-features` under `-Dwarnings`. Strategy `0` (unchanged) scored 0 and regressed; strategy `2` (restore optional `sqlx` plus `database = ["dep:sqlx"]`) scored 1 and won with delta `+1.0`. No prompt/harness mutation was copied into project source; this was bounded project-source selection.

The retained candidate also:

- restores and validates the optional database feature instead of declaring an empty feature;
- makes the metric confidence estimator strictly increase from its 0.1 floor to its 0.9 ceiling;
- emits successful metric-agent decisions through the existing Observatory-compatible telemetry path;
- documents the public agent API and clears all existing strict Clippy findings without suppressing lints;
- replaces an unusable nine-argument evaluation constructor with a typed `NewEvaluation` input;
- records `Cargo.lock` so Linux CI resolves the same dependency graph that was validated;
- formats the full Rust workspace with rustfmt.

| Gate | Baseline | Candidate | Result |
| --- | ---: | ---: | --- |
| `cargo fmt --all -- --check` | exit 1 | exit 0 | PASS |
| core all-features check (`-Dwarnings`) | exit 101 | exit 0 | PASS |
| workspace Clippy, all targets/features (`-D warnings`) | exit 101 | exit 0 | PASS |
| workspace Rust tests (`-Dwarnings`) | 58/59 before first failure | 291/291 | PASS |
| release workspace build (`-Dwarnings`) | exit 101 | exit 0 | PASS |
| Cloud Function contract | 7/7 | 7/7 | PASS, anchor preserved |
| npm high-severity audit | 0 vulnerabilities | 0 vulnerabilities | PASS |
| `cargo audit` 0.22.2 | prior hosted audit exit 0 | 0 vulnerabilities; 1 allowed unmaintained warning | PASS |
| MetaHarness hard constraints | 6/6 | 6/6 | PASS |

MetaHarness `0.4.17` at upstream revision `e0dfd44da72b7adfb7c57fdd42d124be58ed7086` scored the candidate at harness fit 71, compile confidence 100, task coverage 100, tool safety 95, memory usefulness 47, and estimated cost per run USD 0.048. Rust was bootstrapped in an isolated tool home at `rustc 1.99.1`; Darwin remained `0.10.3`, Flywheel `0.1.12`, and Ruflo `3.56.3`.

Flywheel used a frozen six-item CI holdout (rustfmt, core all-features, Clippy, Rust tests, Node tests, audit) and a seven-item CORS anchor. It promoted `baseline -> candidate` from 2/6 to 6/6 with the anchor unchanged at 7/7. Gate fingerprint `c5942e7c9bf8f28fe7af47a6ec3887d11eeb3f8c8e8601cc6a7a54969ea0b981`; signed replay passed receipts, root reachability, contiguous parents, promotion verdict re-execution, sealed fields, full ledger, and pinned-gate verification.

Rollback remains one atomic PR revert. Because `Evaluation::new` now accepts `NewEvaluation`, downstream Rust callers must migrate positional arguments to named fields; the repository tests cover every in-tree call site. No deployment, publishing, persistence-layer change, or database bypass is included.
