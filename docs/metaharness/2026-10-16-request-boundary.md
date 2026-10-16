# MetaHarness evidence: Cloud Function request boundary

Date: 2026-10-16

Base commit: `4a292de7b1a43f1ed78455cc604bb8bd2701c89e`

Upstream MetaHarness revision: `e0dfd44da72b7adfb7c57fdd42d124be58ed7086`

## Frozen contract

The model and evaluator were fixed before candidate evaluation. The holdout checks:

1. reject untrusted CORS preflights;
2. enforce the health-route method contract;
3. require a JSON media type;
4. require an object-shaped JSON body;
5. reject bodies larger than 64 KiB;
6. refuse backend redirects;
7. sanitize caller-controlled correlation identifiers; and
8. avoid exposing upstream error details.

The frozen no-regression anchor preserves exact configured CORS reflection, the
404 route contract, the 503 missing-backend contract, and successful request
forwarding.

## Results

| Gate | Baseline | Candidate |
|---|---:|---:|
| Cloud Function tests | 7/7 | 15/15 |
| Frozen HTTP holdout | 0/8 | 8/8 |
| Frozen behavior anchor | 4/4 | 4/4 |
| Production npm audit | 0 | 0 |
| MetaHarness fit | 71 | 71 |
| MetaHarness hard constraints | 6/6 | 6/6 |

The Docker build on the base commit failed in hosted run `38786686022` because
the Dockerfile used Rust 1.86 and did not copy `Cargo.lock`; dependency resolution
selected crates requiring Rust 1.89 or newer. The candidate uses the same Rust
1.99 toolchain family as hosted CI, copies `Cargo.lock`, and builds with
`cargo build --locked`. The Docker job now runs for pull requests so this fix is
evaluated on the exact candidate before merge.

## Tool execution

- Ruflo `3.56.3`: `hooks pre-task` registered task `task-mvax3du3`.
- MetaHarness `0.4.17`: `metaharness score . --json` exited 0 for baseline and candidate.
- Darwin `0.10.3`: one generation, two children, concurrency 1, seed
  `2026101608`, real sandbox, Pareto selection, fixed `npm run test:cf`
  evaluator. Baseline and both variants scored `0.985`; the harness baseline was
  retained with delta `0.000`.
- Flywheel `0.1.12`: live deterministic evaluator, fixed holdout and anchor,
  Ed25519 receipts, lineage, frozen gate fingerprint, and replay verification.
  The implementation candidate was promoted from `0.000` to `1.000` with the
  anchor unchanged at `1.000`. All eight replay checks passed. The signed replay
  bundle is committed beside this report.
- RuOS: Rust `1.99.0`, Cargo `1.99.0`, Node `20.20.2`. A clean baseline
  `cargo test --workspace` reached real compilation and failed because that
  desktop lacks the system `pkg-config`/OpenSSL development prerequisites.
  Hosted GitHub Actions remains the authoritative Rust/Docker evaluator.

## Rollback

Revert the candidate commit. This restores the previous request behavior,
Dockerfile, workflow, and tests without changing deployment or persistent data.
