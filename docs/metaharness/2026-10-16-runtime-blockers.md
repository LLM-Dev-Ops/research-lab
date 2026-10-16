# Runtime and aggregate CI recovery

## Failure and correction

The prior RuOS image lacked `pkg-config` and OpenSSL development headers.
Install the system prerequisites idempotently with `bash scripts/bootstrap-ruos.sh`.
On the existing Ubuntu 24.04 desktop, installation resolved OpenSSL 3.0.13;
`cargo test --workspace --locked` passed 291 tests (two existing ignored doc
examples), and formatting, strict Clippy and the workspace build exited zero.
The system packages may need reinstalling after a desktop image replacement;
the bootstrap script verifies prerequisites instead of trusting prior evidence.

The previous aggregate CI job could turn green before Docker completed. It now
depends on every other job, including coverage and Docker. Failure, cancellation
or skipping of either closes the gate. Docker runs on every event supported by
this workflow, including a push to develop. All 22 local gate scenarios passed.

## Offline Ruflo learning

Ruflo 3.56.3 lazily initializes Transformers during post-task learning. Its
default remote model lookup previously triggered an egress approval rejection.
The recovery uses the unchanged documented `hooks post-task` interface with:

- Ruflo 3.56.3 and Transformers.js 4.3.1, checked before execution.
- Public `Xenova/all-MiniLM-L6-v2` revision
  `751bff37182d3f1213fa05d7196b954e230abad9`, downloaded separately without task
  text, cookies or authorization headers.
- A committed manifest pinning all five model files by size and SHA-256.
- Documented Transformers `env.allowRemoteModels = false` and a local model path.
- `RUFLO_REQUIRE_REAL_EMBEDDINGS=1` and a real 384-dimensional model probe.
- Linux libseccomp restrictions denying socket creation, connect/send syscalls
  and io_uring setup, inherited by child processes. Existing non-stdio socket
  descriptors are closed. If restrictions cannot load, execution is refused.

The runner is specifically an offline command runner, not a general filesystem
sandbox. Run it with the normal unprivileged desktop account. It needs Python 3
and libseccomp2. Download/install dependencies separately, before this command.
No network exception or approval bypass is used for learning.

Example setup on the existing desktop:

```bash
bash scripts/bootstrap-ruos.sh
npm install --prefix "$HOME/.local/share/llm-dev-ops-tools" --save-exact \
  ruflo@3.56.3 @huggingface/transformers@4.3.1
python3 scripts/cache-ruflo-model.py "$HOME/.cache/llm-dev-ops-models"
```

Run from the repository/worktree whose task is being completed:

```bash
export RUFLO_MODEL_DIR="$HOME/.cache/llm-dev-ops-models"
python3 scripts/offline-exec.py node --import ./scripts/ruflo-local-model.mjs \
  "$HOME/.local/share/llm-dev-ops-tools/node_modules/ruflo/bin/ruflo.js" \
  hooks post-task --task-id TASK_ID --success true --quality 1 \
  --agent codex --task 'Completed task description' --store-results --format json
```

Use the actual task ID and outcome. Do not report learning success from exit code
alone: check `learningUpdates.available`, `outcomePersisted`, `feedback.recorded`
and `trajectory.recorded`. The recovered `task-mvax3du3` returned all four true,
with three observed feedback updates. No running pheromone swarm was present;
the returned `pheromone.active` was false and no swarm learning is claimed.

Four executable isolation tests cover IPv4/IPv6 socket denial, child-process
inheritance, closing inherited sockets, and preserving the command exit status.
They run in the existing Cloud Function CI job alongside the 15 Node tests.

The fixed `research-lab-runtime-recovery-v1` live evaluator passed 1/8 criteria
on main and 8/8 on the candidate. All four frozen behavior anchors passed for
both. Flywheel 0.1.12 promoted the implementation (+0.875) and verified all eight
signed replay checks. See the adjacent `2026-10-16-runtime-blockers-flywheel.json`.

## Rollback

Revert the resulting commit to restore the prior workflow and remove these
opt-in tooling files. The runner changes no global firewall settings. Installed
system prerequisites and the isolated tool/model cache can be retained. No
deployment, package publication, paid service or persistence service is added.
