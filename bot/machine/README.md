# The night bot's machine

The night bot's model sessions for the subscriptions that log in here run on this machine, the
night box: one virtual machine on AWS, where each subscription in
[`.harness/providers.json`](../../.harness/providers.json) whose `runs_on` is `night-vm-<id>`
(Codex, agy, Muse and Devin) has a Linux user of its own and a GitHub runner of its own per lane.
The Claude accounts log in from secrets, so their jobs run on GitHub's own runners (free for this
public repository, four vCPUs each), and so do `gate`, `plan` and `deliver`, the jobs that hold a
GitHub write token. A second box, the training box (below), runs the two AI training lanes,
outside the harness.

**After the v0.3.0 cutover (#306), run `setup.sh` on the night box again, once**, with the same
ids as before (Building it, step 2). The repository's checks are Rust now, and that run gives every
agent user the Rust toolchain `rust-toolchain.toml` pins and puts it on each runner's `PATH`; until
it runs, a machine job finds no `rustup` or `cargo`, so its install and its Rust checks fail. It
touches neither the logins nor the runners' registrations, and a runner in the middle of a job
picks up the new `PATH` at its next restart (the script says which):

```sh
bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse devin
```

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus 8 GB swap), Ubuntu 24.04, 120 GB gp3, tagged `Name=jackioh-night-vm`, in the project's Region (`us-east-2`) |
| Way in | Session Manager only (`aws ssm start-session --target <instance>`): no inbound port, no key pair. The instance role has `AmazonSSMManagedInstanceCore` and nothing else. |
| Users | `agent-<id>` per subscription (`agent-gpt`, `agent-agy`, `agent-muse`, `agent-devin`): a home only it can read, no `sudo`, no Docker |
| Runners | `~agent-<id>/actions-runner` (and `actions-runner-2` … for a subscription with `lanes` over 1), registered as `night-vm-<id>` (`night-vm-<id>-2` …) with the one label `night-vm-<id>`, each a systemd service under that user |
| CLIs | `claude`, `codex`, `agy`, `muse` and `devin`, installed for every user; Node 24, pnpm (corepack) and Python 3.12 |
| Rust | in each user's home: rustup, and the toolchain `rust-toolchain.toml` pins (1.97.0) on the minimal profile with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target; `~/.cargo/bin` is first on every runner's `PATH` |
| Idle stop | a timer powers it off after 30 minutes with no job and no Session Manager session |
| Starter | a Lambda run every five minutes starts it when a job waits for one of its runners |

## Why each subscription is its own user

A machine login (`"login": "machine"`) is a file in a home directory that its CLI refreshes in
place. With one user per subscription, a session can read only its own login and leave things
only in its own home, and its runner, labelled with its id alone, takes only its jobs. A Claude
account's token comes from its GitHub secret, handed to that job alone, and is never written to
the home. At most `machine_parallel` (6) jobs run here at once, each as its own user; Devin has
`"lanes": 6` on the six runners here, and Muse has four (`night-vm-muse` and `night-vm-muse-2`
to `night-vm-muse-4`, all under `agent-muse` and its one login). A lane the harness counts but no
runner serves only queues its job until another runner of that label is free, so after raising a
subscription's `lanes`, run `register-runners.sh` (Building it, step 4) to add the new runners.

Every repository workflow could ask for these labels, so the repository makes outside
contributors' pull requests wait for approval before any workflow runs (Settings → Actions →
"Require approval for all external contributors"). Keep it that way.

## The files

| File | Runs | What it does |
|---|---|---|
| `setup.sh` | on the machine, as root | everything above except the logins and the registration; idempotent, run it again to update the CLIs or Rust, or to add a subscription. With `--training`, the training box instead (below) |
| `clean.sh` | on the machine, as each agent user | gives back the disk that user's jobs can do without (Disk, below); installed as `/usr/local/bin/night-vm-clean.sh` |
| `disk-report.sh` | on the machine, as root | read-only: what fills the disk, biggest first, the runner versions kept and the disk timer's last runs |
| `on-machine.sh` | on your computer | runs a local script on the machine through Session Manager, with the scripts beside it, starting the machine first if it is stopped |
| `register-runners.sh` | on your computer | registers each `night-vm-*` subscription's runner with GitHub and starts it as a service |
| `starter.py` | AWS Lambda | starts the machine when a bot-night or triage job is queued for a `night-vm-*` runner (tested in `bot/tests/test_machine.py`) |
| `deploy-starter.sh` | on your computer | creates or updates the starter, its role and its five-minute schedule |

The scripts on your computer need the AWS CLI signed in to the project (`aws login`) and, for the
runners, `gh` signed in as a repository admin. They find the machine by its `Name` tag, or take
`INSTANCE_ID`.

## Building it

1. **The instance.** Launch Ubuntu 24.04 (x86_64) as `m7i-flex.large` with a 120 GB gp3 disk, an
   instance profile holding `AmazonSSMManagedInstanceCore`, a security group with no inbound
   rule, no key pair, shutdown behaviour **stop**, and the tag `Name=jackioh-night-vm`.
2. **Set it up**, with the ids from `providers.json`:

   ```sh
   bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse devin
   ```
3. **Log in** each machine subscription, once, as its own user
   (`aws ssm start-session --target <instance>`):

   ```sh
   sudo -iu agent-gpt codex login --device-auth   # Sign in with ChatGPT
   sudo -iu agent-agy agy                         # sign in with Google, then quit
   sudo -iu agent-muse muse login
   sudo -iu agent-devin devin auth login --force-manual-token-flow   # paste the page's token
   ```
4. **Register the runners**, from the repository's root:

   ```sh
   bot/machine/register-runners.sh OWNER/REPO
   ```

   It prints each runner's state; they also appear under Settings → Actions → Runners.
5. **Deploy the starter**:

   ```sh
   REPO_ID=$(gh api repos/OWNER/REPO --jq .id) bot/machine/deploy-starter.sh
   ```

## The training box

The two AI training lanes (`training/README.md`, #306 part 39) run on a second, bigger box that
is always on: `jackioh-train@improve` and `jackioh-train@unban`, two systemd services that run
`training/loop.sh <lane>` forever, each as its own user with its own checkout, Rust, Devin login
and GitHub token. They run outside the harness: no runner, no GitHub Actions job, no slot of
`machine_parallel`, no `training` label. A lane opens its own pull requests from `ai/<lane>`, and
CI's `training gate` re-runs its promotion before anything merges.

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus 8 GB swap), Ubuntu 24.04 x86_64, 100 GB gp3 encrypted, tagged `Name=jackioh-train-box`, in the project's Region (`us-east-2`). The plan was an `m7i.xlarge` (4 vCPUs, 16 GB); the project's Free plan refuses instance types outside the Free Tier, so the box built on 2026-10-07 (`i-017d1843ea5e68cef`, #427) is the `m7i-flex.large`; on a paid plan the bigger one halves a lane's cycle |
| Way in | Session Manager only, like the night box: no inbound port, no key pair. It uses the night box's subnet and instance profile (`jackioh-night-vm`: `AmazonSSMManagedInstanceCore`, and `CloudWatchAgentServerPolicy`). IMDSv2 required. |
| Users | `agent-train-improve` and `agent-train-unban`: a home only each can read, no `sudo`, no Docker |
| In each home | `~/JackiOh`, the lane's checkout (`loop.sh` resets it to `main` every cycle; its `target/` stays, so builds after the first are incremental); Rust (rustup and the toolchain `rust-toolchain.toml` pins, with `rustfmt` and `clippy`); `~/training-out/<lane>/` (the games' records, `attempts.md`, the gate's reports); `~/logs/<lane>.log` (the loop's log) |
| CLIs | `devin` (as on the night box), `gh` (GitHub's apt repository), `git` (pushing through `gh`'s login), Node 24 and pnpm (corepack), with which the loop and Devin run the web's tests that play the AI before a promotion goes up |
| Services | `jackioh-train@.service`: `User=agent-train-%i`, `training/loop.sh %i` from the lane's checkout, `Restart=always` after `RestartSec=60`, `DEVIN_MODEL=swe-2-max` (the knob for Devin's model), `RAYON_NUM_THREADS=2` (the two lanes' games share the box's vCPUs), `JACKIOH_TRAINING_OUT` the lane's `~/training-out/<lane>`; both enabled, so they start at boot |
| Idle stop | none: the box never powers itself off, and no starter wakes it |

Build it once, after the cutover has put `training/loop.sh` on `main` (a lane runs whatever `main`
holds; before that its service restarts every minute and does nothing):

1. **The instance**, as in the table above.
2. **Set it up**: `TAG=jackioh-train-box bot/machine/on-machine.sh bot/machine/setup.sh --training`.
   It installs the CLIs, makes both users with their checkouts and Rust, writes the service and
   starts both lanes; they wait, retrying every minute, until the logins below are in.
3. **Log each lane in once**, as its own user (`aws ssm start-session --target <instance>`): Devin,
   then a fine-grained GitHub token that can write contents and pull requests on this repository
   only, one token per lane:

   ```sh
   sudo -iu agent-train-improve devin auth login --force-manual-token-flow   # paste the page's token
   sudo -iu agent-train-improve gh auth login --hostname github.com --git-protocol https --with-token < improve-token.txt
   sudo -iu agent-train-unban devin auth login --force-manual-token-flow
   sudo -iu agent-train-unban gh auth login --hostname github.com --git-protocol https --with-token < unban-token.txt
   ```

   `setup.sh` already made `gh` git's credential helper for each user, and gave each a commit name
   of its own (`JackiOh training (<lane>)`); set `git config --global user.name`/`user.email` as
   that user to commit as someone else. Delete the token files after.
4. **Check it**: `systemctl status jackioh-train@improve jackioh-train@unban`, then `sudo reboot`
   and the same again (both come back by themselves). A lane's first cycle builds `jackioh` from
   `main` (minutes), starts Devin, and writes `GameRecord` lines to
   `~agent-train-<lane>/training-out/<lane>/<date>.jsonl`:
   `sudo tail -f ~agent-train-improve/logs/improve.log`. `devin --version` and a one-line
   `devin -p` as each user show whether Devin answers; if it does not, leave the services as they
   are: they retry every minute, which costs nothing, and start working once it does.

Running the lanes:

- **Stop a lane** (its Devin session ends with it): `sudo systemctl stop jackioh-train@improve`;
  `sudo systemctl disable jackioh-train@improve` keeps it stopped across reboots, and `sudo
  systemctl enable --now jackioh-train@improve` starts it again.
- **Watch a lane**: each keeps a pull request open from `ai/<lane>`, a draft while Devin works, with
  one comment per session rewritten every 15 minutes (`training/README.md`, Watching a lane on
  GitHub). To refuse one promotion, close its pull request: the lane starts again from `main` with a
  new draft.
- **Change Devin's model**: edit `DEVIN_MODEL` in `setup.sh`'s unit, run `setup.sh --training`
  again, then `sudo systemctl restart jackioh-train@improve jackioh-train@unban` (a running lane
  reads a changed unit only when it restarts). SWE-2 is free on Devin's CLI only through
  2026-10-16.
- **Updating** the CLIs or Rust: run `setup.sh --training` again; it changes nothing else and never
  restarts a lane.
- **Disk**: each lane's `target/` (a few GB), its records and logs, and Devin's session database in
  `~/.local/share/devin/cli/` (about 700 MB a day of sessions on the night box; the loop never
  resumes one) grow on the 100 GB disk; the night box's clean-up does not run here. To give the
  database back, stop the lane, delete `sessions.db*` there as that user, and start it again.
- **Cost**: the box is always on: about $70 a month for the instance (on-demand `m7i-flex.large` in
  `us-east-2`; an `m7i.xlarge` would be about $150) plus about $8 for its disk.

## Running it

- **Adding a subscription on the machine:** add it to `providers.json` with
  `"runs_on": "night-vm-<id>"`, then run `setup.sh` with its id, log it in if it is a machine
  login, and run `register-runners.sh`.
- **A login stopped working** (the job's doctor or the run says it was refused): log that user in
  again as in step 3. Nothing else changes.
- **Disk:** every subscription's jobs share the one disk, and a full one fails whichever job
  writes next (on 2026-10-04 a revision of #206 died on `No space left on device`, 66 MB free when
  its job began). What keeps it clear:
  - **`clean.sh`, as each user.** After every job (the runners' job-completed hook,
    `/usr/local/bin/night-vm-job-done.sh`) it deletes that job's files; and after every job and
    every ten minutes (`night-vm-disk.timer`, for every agent user) it deletes what nothing needs:
    the user's package store, Cypress's binary and the npm cache once the user has no job going
    (Devin's and Muse's other lanes share them), what a job that died without its hook left in an
    idle runner's `_work/_temp`, the runner version a self-update replaced (`bin.<version>` and
    `externals.<version>`, about 650 MB a runner, beside the one `bin` links to) and its download,
    the runner's own logs after two days, the user's `/tmp` leftovers after 6 hours, Codex's
    sessions after 7 days, Muse's (a few hundred MB a day) after 8 hours, and Devin's session
    database (about 700 MB a day; the bot never resumes a Devin session) once Devin has no job
    going, with its logs and summaries after 8 hours. It runs as the user whose home it cleans,
    never as root, and the checkout and the logins stay.
  - **Temporary files stay with the job:** on the machine `TMPDIR` is the job's own
    `RUNNER_TEMP/tmp` (`bot-night.yml`), emptied when the job ends.
  - **The system:** the journal is capped at 200 MB (it may otherwise take a tenth of the disk),
    and the timer runs `apt-get clean`.
  - **A model job checks before it starts** (`bot/harness/disk.py`): with under 8 GB free it runs
    `clean.sh` first, from its checkout, and with under 3 GB still free it does no work. Its item is
    not at fault, so it stays queued with nothing counted against it, the subscription backs off
    for a while and the others take the work. A disk that fills up during the work pauses the item
    and keeps what it built.
  - **People hear of it:** every machine job reports its readings, and the deliver job and the
    status loop (every ten minutes) open the issue "Night bot: the machine's disk is filling up"
    at 80% used or under 3 GB free, rewrite it as readings come, and close it at 70% or under. The
    status issue shows the newest reading. To see what fills it:
    `bot/machine/on-machine.sh bot/machine/disk-report.sh`.
  - **A bigger disk:** it was 30 GB until it filled up on 2026-10-04 (254 MB free), and was grown
    to 60 GB in place; then to 120 GB on 2026-10-09, after four Muse jobs building cards at once
    (the first since the Rust rewrite) took it to 98% (#590): each kept about 8.5 GB of Rust
    builds. About 27 GB is fixed: 8 GB of swap, twelve runners' installs at about 0.7 GB each
    (twice that after a self-update until `clean.sh` removes the old version), the agent users'
    Rust toolchains, the system and the CLIs; six jobs' worktrees and builds come on top. To grow it again, raise the volume's
    size (`aws ec2 modify-volume`); `setup.sh` grows the partition and the filesystem into it
    (`growpart`, `resize2fs`), and cloud-init does at the machine's next start. Neither needs a
    restart.
  - `CYPRESS_INSTALL_BINARY=0` keeps `pnpm install` from fetching Cypress's 800 MB binary at all,
    since the bot's checks never run e2e.
  - **Rust:** each agent user's toolchain (about 0.8 GB) and its crate downloads
    (`~/.cargo/registry`, a few hundred MB) stay in its home; a job's `target/` is in its worktree
    under `RUNNER_TEMP`, and goes with the job. The release build of `jackioh` the checks run is
    about 0.3 GB; what grows it is the builds a model runs itself, the tests above all. So a job
    here builds with no incremental cache and no debug info (`CARGO_INCREMENTAL=0`,
    `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`, set in `bot-night.yml`): the
    worktree goes when the job ends, so neither was ever reused, and the cards' tests build in
    0.55 GB instead of 5.2 GB, a third faster. A failing test still names its file and line; only
    a full backtrace loses its line numbers. GitHub's runners and CI build as before.
- **How many at once:** six machine jobs (`machine_parallel`), because a job here runs only the
  light checks. The load is each job's checks, not its model: on 2026-10-02 three jobs running
  the full set had the machine at load average 14 on 2 vCPUs with 1.5 GB swapped. Measured on
  2026-10-03 for one job: `pnpm install` 26 s and 0.55 GB, lint 2 min and 0.8 GB, typecheck
  4.5 min and 1.1 GB, the catalog and rulings checks seconds and 0.13 GB, and `pnpm test` 22.6
  min on one of GitHub's four-vCPU runners. So a job here runs install, typecheck and the light
  checks; lint and the unit tests (`"machine": false` in `.harness/config.json`) run in CI on
  the pull request, and the agents are told to run only the tests for what they changed. Since
  v0.3.0 the checks are Rust and the split is the same: a job here runs the install, `cargo fmt
  --check` and the three `cargo jackioh` checks (catalog, patches, spec), which share one release
  build of the CLI (on 2026-10-07 it took 7.5 minutes on four vCPUs busy with other builds, and
  the engine's crate held about 1.4 GB while it compiled); clippy, the cargo tests and the web's
  unit tests (`"machine": false`) run in CI. The
  Claude accounts run on GitHub's runners, under `max_parallel` (11), apart from these six. The
  Free plan's largest machines are the 2-vCPU `m7i-flex.large` and
  `c7i-flex.large`.
- **Memory** (#317): EC2 records only the CPU (hourly averages above 60% in 27 of the 48 hours
  to 2026-10-05, peaks of 100%), so `setup.sh` installs the CloudWatch agent, which sends
  `mem_used_percent`, `mem_available` and `swap_used_percent` every 60 s to the
  `JackiOh/NightVM` namespace. It needs the instance role to be allowed to send them, once:
  `aws iam attach-role-policy --role-name <the night box's role> --policy-arn
  arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy`, then run `setup.sh` again (it changes
  nothing else). Without it the bot still knows (#312): each model job on the machine reads the
  kernel's memory stall time (`/proc/pressure/memory`), its swapping (`/proc/vmstat`) and the
  memory available (`/proc/meminfo`) as it starts, after each model call and check run, and as
  it ends; the deliver job keeps the last 60 runs' figures, and the status issue says, for the
  last 24 hours, whether the box had memory to spare or ran short (a run stalled on memory for
  5 s or more), with the least memory available and how many runs swapped
  ([`harness/memory.py`](../harness/memory.py)). After a busy day, read it beside CPU and decide
  `machine_parallel`: fewer lanes if the box ran short or swapped, the same if it had memory to
  spare. A bigger box (4 vCPUs) needs the paid plan.
- **Cost:** the machine is billed by the hour while it runs (about $0.096 an hour, so about $70 a
  month if it never stopped) plus its disk (about $4.80 a month). A stopped machine costs only
  the disk. The starter's Lambda calls and its schedule fit in the free tier.
- **The starter** looks only at bot-night runs, through GitHub's public API by repository id, so a
  rename or a transfer does not break it. It leaves alone a job that has waited three hours (its
  runner must be missing), so a broken runner cannot keep the machine running. Its log is the
  CloudWatch group `/aws/lambda/jackioh-night-vm-starter`, kept for seven days. If GitHub's
  rate limit for anonymous calls gets in the way, deploy it again with `STARTER_GITHUB_TOKEN` (a
  fine-grained token that can only read Actions).
