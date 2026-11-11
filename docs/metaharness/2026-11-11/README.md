# November 11, 2026 — research-lab
EXECUTED; component VALIDATED; full promotion BLOCKED; MERGED:none.
Ready PR https://github.com/LLM-Dev-Ops/research-lab/pull/5
Candidate 1c3edc64bc650e8cb0e8ad7788cc889b285d244c; baseline/current main83c4850c04d6243c1f700ec4a6a0e406a7d7198a. No conflicts; main remains ancestor. No existing open research-lab PR at start; resumed recovery implemented on main by PR4, not duplicate work. Selected this long-unworked recovery surface after recent Registry/schema/inference cycles.

## Implementation and frozen results
scripts/offline-exec.py now refuses socket-backed stdin/stdout/stderr before task exec. Ordinary write() on already-connected stdio sockets bypassed connect/send restrictions and survived into child processes. Other inherited sockets still close; ordinary pipes/files/terminals remain supported.
Frozen six-case probe/contract committed locally8c474fb before source fix; published unchanged in candidate. Three alternating baseline/candidate trials: containment0/6→6/6, synthetic canary transmissions6→0. UNIX socketpairs only, no external endpoint/credentials/task data. Original four kernel-isolation test methods and15Node assertions retained byte-identically. Candidate full Python suite5methods (new method includes6subtests) passes. Regular-file stdout and exit23 preservation check passes. No Rust/lock/deployed IAM/persistence changes.
This runner is not a filesystem sandbox or proof against a trusted caller forwarding a pipe. Node child_process stdin pipe is implemented with a socketpair here and is intentionally rejected; evaluator needs no input and sets stdin ignore.

## Exact commands/results
python3 scripts/probe-offline-stdio.py BASE:1/0of6; EXACT:0/6of6.
python3 -m unittest discover -s test -p 'offline_runtime_test.py':0,4methods both.
python3 -m unittest discover -s test -p '*_test.py':0,5methods candidate.
npm ci --ignore-scripts:0; npm run test:cf:0/15of15; npm audit --json:0/zero.
git diff --check:0; merge-base ancestor and native merge-tree:0.
Local cargo test --workspace --locked:127 cargo absent. No missing OpenSSL inference: existing RuOS bootstrap/current OpenSSL/durable recovery checkout could not be accessed.
Exact-head CI43025148490 completed: CloudFunction,Lint,Test,Build,CodeCoverage,DockerBuild SUCCESS; SecurityAudit FAILURE; AllChecksPass FAILURE.
Fresh hosted cargo test --workspace:291passed,0failed,2ignored (before repeated doc step). Hosted command is NOT --locked. Security cargo-audit0.22.2:3vulnerabilities,2warnings,1458advisories/380crate dependencies. Findings libsqlite3-sys0.30.1 RUSTSEC-2026-0459; rustls-webpki0.103.15 RUSTSEC-2026-0382; webpki-roots1.0.9 RUSTSEC-2026-0381. No suppression or lock rewrite.
CI link https://github.com/LLM-Dev-Ops/research-lab/actions/runs/43025148490

## Actual tools — no promotion claimed
Installed current package READMEs/interfaces inspected, exact lock/integrity adjacent. npm install --prefix /tmp/research11-tools --ignore-scripts --no-audit --no-fund --save-exact metaharness@0.4.2 @metaharness/darwin@0.10.3 @metaharness/flywheel@0.1.12 succeeded. Earlier incorrect @ruvnet/unscoped package-name attempts404/ETARGET corrected, not execution proof.
node /tmp/research11-tools/node_modules/.bin/metaharness score . --json
exit0 fit71,compileConfidence100,taskCoverage79,safety100,memory45,6of6 hard constraints; static heuristic, not build/cost proof.
timeout --kill-after=3 45 /tmp/research11-tools/node_modules/.bin/metaharness-darwin evolve . --generations 1 --children 1 --concurrency 1 --seed 2026111108 --risk-budget 0 --sandbox real --mutator deterministic
Fresh exact-source archive,exit0 baseline/scorePolicy child0.435/pass0,delta0; baseline retained, NO_IMPROVEMENT. Native test commands failed127 cargo absent. CLI warns repo tests cannot distinguish harness mutations. Archive baseline bookkeeping is NOT validated implementation promotion. Raw traces darwin.json; preserved archive not overwritten.
node docs/metaharness/2026-11-11/evaluate.cjs BASE EXACT /tmp/research11-tools /tmp/research11-flywheel.json
exit0, actual runFlywheelGenerations/meetsPromotionRule/makeSigner/gateFingerprint/verifyReplayBundle, real subprocess native probes and retained suites.0promotions; fullgate explicit regressed=true veto for incomplete Rust/security/Ruflo clearance, not newly regressed assertion. Default gate unchanged; noop/cost0 neutral/unmeasured, no savings claim. Signed root-only replay8of8, not candidate acceptance. Inverse exact parent launcher bytes/SHA against Git preserved in flywheel.json.
Initial evaluator wrong test/index.test.js path failed before evaluation; corrected actual cloud-function.test.js. Next inherited Node socket stdin caused original anchors to fail; supplied non-socket stdin to BOTH revisions, no tests weakened. Final complete evaluator/results adjacent. No LLM/model optimization claim.

## Runtime failures separately verified
Primary Node24.22.1/Python/Linux; actual kernel restrictions exercised by four original tests.
Fresh RuOS guide/status/peers/activity read. Existing machine40eb5828d43bfce01110e62d334a6de0 Fly8ee959f7363638 stopped/asleep0connections. desktop_start returned502 unresolved launch cancellation; a new start was not sent. No retry/reset/alternate identity/provisioning.
node /home/ruv/.local/share/llm-dev-ops-tools/node_modules/ruflo/bin/ruflo.js swarm init --topology hierarchical --max-agents 1 --auto-scale false --strategy specialized --with-permissions strict --format json
exit1 MODULE_NOT_FOUND, no orchestration/worker.
RUFLO_MODEL_DIR=/home/ruv/.cache/llm-dev-ops-models python3 /home/ruv/work/research-lab-runtime-recovery-20261016/scripts/offline-exec.py node --import /home/ruv/work/research-lab-runtime-recovery-20261016/scripts/ruflo-local-model.mjs /home/ruv/.local/share/llm-dev-ops-tools/node_modules/ruflo/bin/ruflo.js hooks post-task --task-id research-stdio-20261111 --success false --quality 0.7 --agent codex --task 'Socket stdio containment fixed; promotion blocked' --store-results --format json
exit2 ENOENT at Python launcher in primary. No model/kernel loading for this learning attempt; all learningUpdates.available/outcomePersisted,feedback.recorded,trajectory.recorded UNCONFIRMED. Historical Ruflo3.56.3/Transformers4.3.1/MiniLM751bff37182d3f1213fa05d7196b954e230abad9 pins unchanged, not installed-runtime evidence. No network hook/hashfallback/task data to HuggingFace. Fresh hosted291 passes above are not historical recovery substitution.

## Coverage/guidance/safeguards
Installation168979216 fully paginated58offset0,0offset100. Initial58repos/163openPRs; final58repos/164PRs includes ownPR5; full current main/rootinstruction-manifest inventory/latestmainCI/allPRpages refreshed with0final errors. Initial RuVector transient disconnect recovered. Deep source/manifests/README/CI/recoverydocs only research-lab; no AGENTS/CLAUDE found.57named DEFERRED in coverage.json; inventory is not58builds.
Core-memory qualified namespace guidance applied: avoid duplicate PRs, frozen anchors, fresh Darwin state, no mechanical conflict union. Human-only historical lesson does not supersede current user merge authorization/protections. Append-only outcome accepted for candidate, not Ruflo learning substitute.
Single writer, isolated baseline/candidate, atomic claim released. Public connector tree/commit/branch/PR writes verified; public shellfetch only, shellpush auth not implied. Evidence publication interrupted once before any evidencebranch existed; checked404 before retry; source untouched. Architecture unchanged: no new database, deployment, IAM, paid service or publishing. Existing architecture not certified.
Schedule untouched; no disablement. Desktop final stopped/asleep0connections.

## Rollback/remaining
Not merged. Close PR to discard before merge; after future authorized merge revert candidate; signed inverse holds original launcher bytes. Source rollback restores stdio leakage, not security-safe production rollback. No data/schema/globalfirewall change.
Remaining:3Rust advisories/full required CI/coverage/approval gates; recover authorized runtime; inspect durable revision/upstream, run bootstrap/currentOpenSSL and lockedRust; real bounded Ruflo execution and offline real embeddings with all4flags; rerun full Flywheel/latestmain/exacthead gates before merge. No speed/dollar or deployed integration claim.
