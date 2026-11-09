# Phase 30: deploy health on Deployed cards and a fuller `d` pane

## Context

A Deployed row only reads the Deployment's image, so a card says `✓ at main` while the new
version sits in CrashLoopBackOff and the old pods keep serving. The card should flag that
(yellow warning), flag a Deployment that is not serving (red alert), and `d` should show enough
to skip a `kubectl describe`.

Decisions taken with the user:

- **Placement:** the version glyph (`✓ ◐ ○ ✗`) stays. Health goes in the card's icon slot
  (the one `pr_row` uses), plus one coloured reason line. A healthy card is unchanged.
- **Glyphs:** yellow `⚠` warning, red `‼` alert. Plain Unicode, no Nerd Font.
- **Alert (red):** the Deployment's `Available` condition is not `True`.
- **Warning (yellow):** Available, but a pod container waits in `CrashLoopBackOff`,
  `ImagePullBackOff`, `ErrImagePull` or `CreateContainerConfigError`, or `Progressing` is
  `False` with reason `ProgressDeadlineExceeded`.
- **Details pane adds:** target, replicas and conditions, pod list, why a container crashed.

## Design

### kubectl (`src/sources/kube.rs`)

`Kube::image` is replaced by two calls returning raw JSON; parsing lives in the model.

```rust
pub trait Kube {
    fn deployment(&self, env: &DeployEnv) -> io::Result<String>;
    fn pods(&self, env: &DeployEnv, selector: &str) -> io::Result<String>;
}
```

```bash
kubectl --context C -n NS get deploy D -o json --request-timeout=10s
kubectl --context C -n NS get pods -l <k=v,k=v> -o json --request-timeout=10s
```

Both go through one private runner that keeps today's behaviour: null stdin, `MISSING` on
`NotFound`, `Log::call` before the `?`, stderr as the error. `image_args` becomes
`deployment_args` and `pods_args`.

The image now comes from `.spec.template.spec.containers[0].image` of the Deployment JSON, so
the per-env cost is 2 calls per refresh (was 1), worst case 20 s on a dead session. The pods
call is skipped when the Deployment call fails or `spec.selector.matchLabels` is empty.

### Model (`src/model/deployed.rs`)

- `Rollout { image, selector, desired, updated, ready, available, conditions: Vec<Condition> }`,
  `Condition { kind, status, reason, message }`, `parse_deployment(&str) -> Result<Rollout, String>`.
- `Pod { name, phase, ready, restarts, started_at, sha, waiting: Option<Waiting>, last_exit: Option<LastExit>, terminating }`
  with `Waiting { reason, message }` and `LastExit { reason, exit_code }`;
  `parse_pods(&str) -> Vec<Pod>`, lenient like the gh parsers, reading `containerStatuses` and
  `initContainerStatuses`, problems first.
- `enum Health { Unknown, Healthy, Warning(String), Alert(String) }` (default `Unknown`) and the
  pure `health(&Rollout, &[Pod]) -> Health`. Terminating pods never count. Reason strings:
  `0 of 2 available`, `1 of 3 pods CrashLoopBackOff`, `rollout stalled`.
- `Deployment` gains `health: Health`, `rollout: Option<Rollout>`, `pods: Vec<Pod>`,
  `pods_error: Option<String>`, `target: String` (`context / namespace / deployment`).

### Fetch (`src/sources/fetch.rs`, `DeploySource::fetch_env`)

Deployment call → `parse_deployment` → image and sha exactly as today → pods call → `health`.
A failed pods call is **not** a row error: it sets `pods_error`, and health is judged from the
Deployment alone (the alert rule still works, the pod warning cannot).

`App::keep_last_known` (`src/app.rs:1636`) stays as is: a row whose kubectl call failed keeps
its last sha and pull but not its last health or pods, which would be stale under a `✗`.

### Card (`src/ui/rows.rs::deployed_row`, `src/ui/style.rs`)

- New `style::health_icon(&Health) -> Option<(char, Color)>`, passed as `card()`'s icon
  argument (currently `None`).
- One reason line after the author and before the sha line, yellow or red, truncated to
  `width - INDENT` like the error line.

### Details (`src/ui/details.rs::deployed_details`)

Existing lines, then:

1. `target` line, dim.
2. `replicas: 3 desired · 1 updated · 2 ready · 2 available`.
3. One line per condition that is not healthy: `Progressing False · ProgressDeadlineExceeded · <message>`.
4. Pods, built like `job_lines`/`job_line`: glyph, name, status (waiting reason, else phase),
   `restarts N`, `text::age`, `sha8`; a failing pod gets an indented second line
   `last exit: OOMKilled (137) · <waiting message>`.
5. `pods: <error>` in red when `pods_error` is set.

The pane already scrolls (`scrolled()`), so the pod list is not truncated.

## Tasks (one red-green-refactor cycle each, shipped with `scripts/ship.sh`)

1. `parse_deployment` + `sha` from its image. Fixture `tests/fixtures/deployment.json`.
2. `parse_pods`. Fixture `tests/fixtures/pods.json` (one running old pod, one crashlooping new).
3. `health()`: one test per rule, plus terminating pods and scaled-to-zero.
4. `CliKubectl::deployment`/`pods` argv, logging and error mapping (`tests/kube.rs`, under `SHIMS`).
5. `DeploySource` uses both calls; pods failure lands in `pods_error` (`tests/fetch.rs`, extend `FakeKube`).
6. Card icon and reason line (`tests/deployed.rs`; `support::deployment` gets a health variant).
7. Details pane lines (`tests/deployed.rs`, taller `Harness::with_size`).
8. e2e and screenshot `kubectl` shims dispatch on `get pods` before the context and print JSON
   (`tests/e2e.rs:134`, `scripts/screenshots.sh:115`); one screenshot env crashloops; rerun
   `scripts/screenshots.sh`.
9. `AGENTS.md` (Deployed data source, Layout line for `kube.rs`), `PLAN.md` outcome,
   `Cargo.toml` to `0.23.0`, `CHANGELOG.md` entry.

Repo process: worktree `phase-30-deploy-health` off `origin/main`, this plan copied to
`plans/phase-30-deploy-health.md` with the first ship, draft PR on first push, `/pr-review`
before ready.

## Risks and assumptions

- **Fixtures are synthetic** (`acme/*`), like `queue.json`: the repo is public and the only
  clusters at hand are private. The JSON field paths get checked read-only against a live
  cluster before task 1, nothing is copied from it.
- **RBAC:** `get pods` may be denied where `get deploy` is allowed. Handled by `pods_error`.
- **Selector:** only `matchLabels` is used; a `matchExpressions`-only Deployment gets no pod
  list. Pods of another workload sharing the labels would be listed too.
- **`‼` width:** U+203C has an emoji presentation in some terminals and could draw 2 columns
  wide. Checked in the screenshot; fallback is `!`.
- **Refresh cost:** call count per env doubles, still every `refresh_secs` (default 120 s).
- Health is not added to the `/` filter.

## Verification

- Related tests only: `cargo test --test kube --test fetch --test deployed`, then
  `scripts/gates.sh` through `ship.sh`.
- `scripts/dev-install.sh`, then `conveyor-dev` against the real config: break nothing on
  healthy envs, and `d` on a Deployed row lists pods.
- `conveyor log` shows two kubectl lines per env per refresh.
- `docs/details.png` shows the warning card.
