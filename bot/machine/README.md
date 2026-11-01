# The night bot's machine

The night bot's model sessions for the subscriptions that log in here run on this machine, the
night box: one virtual machine on AWS, where each subscription in
[`.harness/providers.json`](../../.harness/providers.json) whose `runs_on` is `night-vm-<id>`
(Codex, agy and Muse) has a Linux user of its own and a GitHub runner of its own per lane.
The Claude accounts log in from secrets, so their jobs run on GitHub's own runners (free for this
public repository, four vCPUs each), and so do `gate`, `plan` and `deliver`, the jobs that hold a
GitHub write token. A second box, the training box (below), ran the two LLM training lanes
outside the harness until #1677 stopped them on 2026-10-25. It is the Muse box now, and each box
runs three of these jobs at most: the Muse box three of Muse's own lanes, the night box gpt's,
agy's and one of Muse's, and Muse fills gpt's or agy's while that one cannot start work. It still
runs `gather` ([The Muse box](#the-muse-box)).

**After the v0.3.0 cutover (#306), run `setup.sh` on the night box again, once**, with the same
ids as before (Building it, step 2). The repository's checks are Rust now, and that run gives every
agent user the Rust toolchain `rust-toolchain.toml` pins and puts it on each runner's `PATH`; until
it runs, a machine job finds no `rustup` or `cargo`, so its install and its Rust checks fail. It
touches neither the logins nor the runners' registrations, and a runner in the middle of a job
picks up the new `PATH` at its next restart (the script says which):

```sh
bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse
```

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus compressed swap in memory and 8 GB of swapfile), the largest the project's Free plan allows: the move to `m7i-flex.xlarge` planned for 2026-10-26 never took (its jobs still read 7.6 GB of memory on 10-27, `reviews/2026-10-27-muse-usage.md`, finding 4); Ubuntu 24.04, 120 GB gp3, tagged `Name=jackioh-night-vm`, in the project's Region (`us-east-2`) |
| Way in | Session Manager only (`aws ssm start-session --target <instance>`): no inbound port, no key pair. The instance role has `AmazonSSMManagedInstanceCore` and nothing else. |
| Users | `agent-<id>` per subscription (`agent-gpt`, `agent-agy`, `agent-muse`): a home only it can read, no `sudo`, no Docker |
| Runners | `~agent-<id>/actions-runner` (and `actions-runner-2` … for a subscription with `lanes` over 1), registered as `night-vm-<id>` (`night-vm-<id>-2` …) with the one label `night-vm-<id>`, each a systemd service under that user. Muse's fill-in runners ([The Muse box](#the-muse-box)) are `~agent-muse/actions-runner-fill` and `-fill-2`, `night-vm-muse-fill` and `night-vm-muse-fill-2`, labelled `night-vm-muse-fill` |
| CLIs | `claude`, `codex`, `agy` and `muse`, installed for every user; Node 24, pnpm (corepack) and Python 3.12 (for the training lanes and the commands models run; the harness no longer needs it) |
| Harness | none installed: each job gets the `harness` binary (`bot/go/`, #467) from the workflows' own action, [`.github/actions/harness`](../../.github/actions/harness/action.yml), which restores it from the Actions cache, or on a miss builds it with the Go that `setup-go` fetches, so `setup.sh` is unchanged |
| Browser | Google Chrome stable (`google-chrome-stable`, from Google's `.deb`, which adds Google's apt repository, so a later `setup.sh` upgrades it), for the UI walkthroughs `scripts/browse.sh` drives (`.claude/skills/ui-walkthrough/`, #816); nothing starts it but an item's UI work |
| Rust | in each user's home: rustup, and the toolchain `rust-toolchain.toml` pins (1.97.0) on the minimal profile with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target; `~/.cargo/bin` is first on every runner's `PATH` |
| Idle stop | a timer powers it off after 30 minutes with no job and no Session Manager session |
| Starter | a Lambda run every five minutes starts it when a job waits for one of its runners |

## Why each subscription is its own user

A machine login (`"login": "machine"`) is a file in a home directory that its CLI refreshes in
place. With one user per subscription, a session can read only its own login and leave things
only in its own home, and its runner, labelled with its id alone, takes only its jobs. A Claude
account's token comes from its GitHub secret, handed to that job alone, and is never written to
the home. At most `machine_parallel` (6) jobs run here at once, each as its own user; Muse has
four lanes (two since #1273): one here,
`night-vm-muse`, and three on the Muse box (`night-vm-muse-b2`, `-2-b2`, `-3-b2`), plus two
fill-in runners here in gpt's and agy's lanes (`night-vm-muse-fill`, `-fill-2`), each under that
box's `agent-muse` and its login (#1677). A lane the harness counts but no
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
| `starter.py` | AWS Lambda | starts the machine when a bot-night job is queued for a `night-vm-*` runner (tested in `bot/tests/test_machine.py`) |
| `deploy-starter.sh` | on your computer | creates or updates the starter, its role and its five-minute schedule |
| `credits.py` | AWS Lambda | opens one issue when the project's credits fall under $20 (tested in `bot/tests/test_machine.py`) |
| `deploy-credits.sh` | on your computer | creates or updates the credits watch, its role and its six-hour schedule |
| `local/` | on a person's Windows PC | the local runner (below): the runner image, the supervisor that lends the PC only while it is idle, its logon task and the logins |

The scripts on your computer need the AWS CLI signed in to the project (`aws login`) and, for the
runners, `gh` signed in as a repository admin. They find the machine by its `Name` tag, or take
`INSTANCE_ID`.

## Building it

1. **The instance.** Launch Ubuntu 24.04 (x86_64) as `m7i-flex.large` with a 120 GB gp3 disk, an
   instance profile holding `AmazonSSMManagedInstanceCore`, a security group with no inbound
   rule, no key pair, shutdown behaviour **stop**, and the tag `Name=jackioh-night-vm`.
2. **Set it up**, with the ids from `providers.json`:

   ```sh
   bot/machine/on-machine.sh bot/machine/setup.sh gpt agy muse
   ```
3. **Log in** each machine subscription, once, as its own user
   (`aws ssm start-session --target <instance>`):

   ```sh
   sudo -iu agent-gpt codex login --device-auth   # Sign in with ChatGPT
   sudo -iu agent-agy agy                         # sign in with Google, then quit
   sudo -iu agent-muse muse login
   ```
4. **Register the runners**, from the repository's root:

   ```sh
   bot/machine/register-runners.sh OWNER/REPO
   ```

   It prints each runner's state; they also appear under Settings → Actions → Runners. With the
   Muse box, name what each box carries instead ([The Muse box](#the-muse-box), step 4): here,
   `bot/machine/register-runners.sh OWNER/REPO gpt agy muse=1 muse:fill`.
5. **Deploy the starter**:

   ```sh
   REPO_ID=$(gh api repos/OWNER/REPO --jq .id) bot/machine/deploy-starter.sh
   ```
6. **Deploy the credits watch**, with a fine-grained token for the repository that can read and
   write its issues and nothing else:

   ```sh
   REPO=OWNER/REPO CREDITS_GITHUB_TOKEN=<token> bot/machine/deploy-credits.sh
   ```

   On the Free plan, AWS closes the project when its credits run out, and both boxes stop with
   it. Every six hours the watch reads the credits left (`aws freetier get-account-plan-state`),
   and once they are under `FLOOR` ($20) it opens the issue "AWS credits are under $20", labelled
   `human`, once: an issue with that title ever opened, open or closed, stops another. Its log
   is the CloudWatch group `/aws/lambda/jackioh-credits-watch`, kept for seven days. Deployed
   without a token, it only logs; set `GITHUB_TOKEN` on the function in the Lambda console, or
   deploy again with one.

## The training box

**Since 2026-10-25 only `jackioh-train@gather` runs here.** #682's audit
(`reviews/2026-10-22-issue-682-subscriptions.md`) found that the two LLM lanes made no fair promotion
in about 427 hours, and that on Muse's plan they spent most of the night bot's week. So
`jackioh-train@improve` and `@unban` are disabled (`disabled; inactive (dead)`), and `setup.sh
--training` keeps their users and checkouts but starts neither again. The AI improves through the
`learn` lane on the owner's PC instead (The local runner, below). Whether this box becomes a second
night box or is stopped was #1677's question: it is the Muse box now ([The Muse box](#the-muse-box)).

The AI training lanes (`training/README.md`, #306 part 39) run on a second, bigger box that
is always on: `jackioh-train@improve`, `jackioh-train@unban` and `jackioh-train@gather`, three systemd
services that run `training/loop.sh <lane>` forever, each as its own user with its own checkout and
Rust (the two training lanes also hold a Muse login and GitHub token, which the gather lane needs
neither of). They run outside the harness: no runner, no GitHub Actions job, no slot of
`machine_parallel`, no `training` label. A training lane opens its own pull requests from `ai/<lane>`,
and CI's `training gate` re-runs its promotion before anything merges; the gather lane opens none.

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus compressed swap in memory and 8 GB of swapfile), Ubuntu 24.04 x86_64, 100 GB gp3 encrypted, tagged `Name=jackioh-train-box`, in the project's Region (`us-east-2`). The plan was an `m7i.xlarge` (4 vCPUs, 16 GB); the project's Free plan refused instance types outside the Free Tier, so the box built on 2026-10-07 (`i-017d1843ea5e68cef`, #427) is an `m7i-flex.large`. The move to `m7i-flex.xlarge` planned for 2026-10-26 never took (finding 4 of `reviews/2026-10-27-muse-usage.md`) |
| Way in | Session Manager only, like the night box: no inbound port, no key pair. It uses the night box's subnet and instance profile (`jackioh-night-vm`: `AmazonSSMManagedInstanceCore`, and `CloudWatchAgentServerPolicy`). IMDSv2 required. |
| Users | `agent-train-improve`, `agent-train-unban` and `agent-train-gather`: a home only each can read, no `sudo`, no Docker |
| In each home | `~/JackiOh`, the lane's checkout (`loop.sh` resets it to `main` every cycle; its `target/` stays, so builds after the first are incremental); Rust (rustup and the toolchain `rust-toolchain.toml` pins, with `rustfmt` and `clippy`); `~/training-out/<lane>/` (the games' records, `attempts.md`, the gate's reports); `~/logs/<lane>.log` (the loop's log) |
| CLIs | `muse` (as on the night box; #1272 moved the lanes to it), `gh` (GitHub's apt repository), `git` (pushing through `gh`'s login), Node 24 and pnpm (corepack), with which the loop and Muse run the web's tests that play the AI before a promotion goes up |
| Services | `jackioh-train@.service`: `User=agent-train-%i`, `training/loop.sh %i` from the lane's checkout, `Restart=always` after `RestartSec=60`, `MUSE_MODEL=muse-spark-1.3-contributor` (the knob for Muse's model) and `MUSE_NO_AUTO_UPDATE=1`, `RAYON_NUM_THREADS=2` (the three lanes' games share the box's vCPUs), `JACKIOH_TRAINING_OUT` the lane's `~/training-out/<lane>`; only `gather` enabled since #1677, so it alone starts at boot |
| Idle stop | none: the box never powers itself off, and no starter wakes it |

Build it once, after the cutover has put `training/loop.sh` on `main` (a lane runs whatever `main`
holds; before that its service restarts every minute and does nothing):

1. **The instance**, as in the table above.
2. **Set it up**: `TAG=jackioh-train-box bot/machine/on-machine.sh bot/machine/setup.sh --training`.
   It installs the CLIs, makes both users with their checkouts and Rust, writes the service and
   starts both lanes; they wait, retrying every minute, until the logins below are in.
3. **Log each lane in once**, as its own user (`aws ssm start-session --target <instance>`): Muse,
   then a fine-grained GitHub token that can write contents and pull requests on this repository
   only, one token per lane:

   ```sh
   sudo -iu agent-train-improve muse login
   sudo -iu agent-train-improve gh auth login --hostname github.com --git-protocol https --with-token < improve-token.txt
   sudo -iu agent-train-unban muse login
   sudo -iu agent-train-unban gh auth login --hostname github.com --git-protocol https --with-token < unban-token.txt
   ```

   `setup.sh` already made `gh` git's credential helper for each user, and gave each a commit name
   of its own (`JackiOh training (<lane>)`); set `git config --global user.name`/`user.email` as
   that user to commit as someone else. Delete the token files after.
4. **Check it**: `systemctl status jackioh-train@improve jackioh-train@unban`, then `sudo reboot`
   and the same again (both come back by themselves). A lane's first cycle builds `jackioh` from
   `main` (minutes), starts Muse, and writes `GameRecord` lines to
   `~agent-train-<lane>/training-out/<lane>/<date>.jsonl`:
   `sudo tail -f ~agent-train-improve/logs/improve.log`. `muse --version` and a one-line
   `muse exec` as each user show whether Muse answers; if it does not, leave the services as they
   are: they retry every minute, which costs nothing, and start working once it does.

Running the lanes:

- **Stop a lane** (its Muse session ends with it): `sudo systemctl stop jackioh-train@improve`;
  `sudo systemctl disable jackioh-train@improve` keeps it stopped across reboots, and `sudo
  systemctl enable --now jackioh-train@improve` starts it again.
- **Watch a lane**: each keeps a pull request open from `ai/<lane>`, a draft while Muse works, with
  one comment per session rewritten every 15 minutes (`training/README.md`, Watching a lane on
  GitHub). To refuse one promotion, close its pull request: the lane starts again from `main` with a
  new draft.
- **Change Muse's model**: edit `MUSE_MODEL` in `setup.sh`'s unit, run `setup.sh --training`
  again, then `sudo systemctl restart jackioh-train@improve jackioh-train@unban` (a running lane
  reads a changed unit only when it restarts). The lanes spend the same Muse plan as the night bot's
  `muse` seat, which its usage readings show (the status issue).
- **Updating** the CLIs or Rust: run `setup.sh --training` again; it changes nothing else and never
  restarts a lane.
- **Disk**: the box cleans itself (`training/loop.sh`'s `free_disk`), since the night box's
  clean-up does not run here. Each cycle starts, with no session of its lane going, by reading the
  disk; under 20 GB free (`TRAIN_DISK_FLOOR_GB`) the lane deletes its own builds but the release
  one (the parent and the gate's), its download caches (cargo's, pnpm's, npm's) and its `/tmp`
  leftovers older than 6 hours, all of which come back by themselves, and says so in its log. The
  lanes' test builds carry no debug info, as on the night box: with it, Muse's `cargo test` runs
  had built 20 GB of `target/debug` a lane in two weeks and left the disk 6 GB free (2026-10-19).
  Muse keeps every session in `~/.local/share/muse/sessions/` (a few hundred MB a day); the loop
  deletes one 8 hours after it ended, as `clean.sh` does on the night box. The records
  (`training-out/`) and logs stay.
- **Cost**: about $0.096 an hour for the instance (on-demand `m7i-flex.large` in `us-east-2`), so
  about $70 a month were it always on (the Muse box stops when idle), plus about $8 for its disk,
  all taken from the Free plan's credits while they last.

## The Muse box

Since #1677 the training box also carries Muse's runners. Two builds at once on one 8 GB box swap
(`reviews/2026-10-19-issue-383-muse.md`, finding 1): a Muse build that shared the box took a median
57 minutes in its build call, against 34 alone, so each box held one Muse runner. Since 2026-10-26
each box runs three of these jobs at most. That was set for `m7i-flex.xlarge` (4 vCPUs, 16 GB), but
both boxes are still `m7i-flex.large` (2 vCPUs, 8 GB), so three building jobs overrun a box's
memory; compressed swap in memory (`setup.sh`, zram) takes the overflow before the swapfile does:

- **The Muse box** holds three of Muse's four lanes of its own (`runs_on` `night-vm-muse`,
  `"lanes": 4`): `night-vm-muse-b2`, `night-vm-muse-2-b2` and `night-vm-muse-3-b2`.
- **The night box** holds the fourth, `night-vm-muse`, and gpt's lane and agy's, which Muse fills
  while that subscription cannot start work: capped, off, outside its hours or backing off
  (`fill_in` in [`providers.json`](../../.harness/providers.json); `plan.FillInFree`). Muse's
  two runners for that carry a label of their own, `night-vm-muse-fill`, which a Muse run takes
  only when the planner puts it in such a lane, once its own four are held (`plan.RunnerFor`;
  the item's record keeps it as `runs_on`). gpt, agy and Muse's runs there share the two lanes:
  a subscription whose lane Muse holds waits for that run to end, even once its own cap resets
  (`plan.FillInFull`), so the night box never runs more than three of them.

So Muse runs six at once at most, and the two boxes six jobs together (`machine_parallel`). A
job for `night-vm-muse` goes to whichever of the four runners with that label is free, so the
runners each box holds are its limit. Muse has no login a secret can carry onto GitHub's runners,
which is why both boxes are AWS's.

| | The night box (`jackioh-night-vm`) | The Muse box (`jackioh-train-box`) |
|---|---|---|
| Instance | `m7i-flex.large` (2 vCPUs, 8 GB) | `m7i-flex.large` (2 vCPUs, 8 GB) |
| Runners | `night-vm-gpt`, `night-vm-agy`, `night-vm-muse` and Muse's `night-vm-muse-fill` and `night-vm-muse-fill-2` | `night-vm-muse-b2`, `night-vm-muse-2-b2` and `night-vm-muse-3-b2` |
| At once | three: gpt's lane and agy's, which Muse fills while either cannot start work, and one of Muse's own | three of Muse's own |
| Also | | `jackioh-train@gather` ([`training/gather.md`](../../training/gather.md)), while the box is on |
| Idle stop | after 30 minutes with no job | the same, since `setup.sh muse` ran on it |
| Starter | `jackioh-night-vm-starter`, for every `night-vm-*` label | `jackioh-muse-box-starter`, for `night-vm-muse` alone |

The status issue draws each box as its slots (`dashboard.BoxSlots` in `bot/go/internal/dashboard/`): the night box's gpt's, agy's and Muse's lanes, and the Muse box's three of Muse's. Change it with the boxes.

Both boxes are one machine to the harness: `machine_parallel` (6) holds their jobs together, and a
runner's `-b2` says which box a run, or its memory reading, came from. The status issue's memory
line gives each box its own verdict.

**Building it**, once, from the repository's root, with the AWS CLI signed in and `gh` as a
repository admin:

1. **Its shutdown behaviour must be stop.** The idle stop powers the box off from inside, and a box
   whose shutdown behaviour is terminate would be deleted. See it, and set it if it is not `stop`:
   ```sh
   aws ec2 describe-instance-attribute --instance-id i-017d1843ea5e68cef --attribute instanceInitiatedShutdownBehavior
   aws ec2 modify-instance-attribute --instance-id i-017d1843ea5e68cef --instance-initiated-shutdown-behavior Value=stop
   ```
2. **Set it up for Muse:** `TAG=jackioh-train-box bot/machine/on-machine.sh bot/machine/setup.sh
   muse`. It adds `agent-muse`, its runner and Rust, the idle stop and the disk clean-up, and
   leaves the `gather` lane as it is.
3. **Log Muse in** as that user, once: `aws ssm start-session --target i-017d1843ea5e68cef`, then
   `sudo -iu agent-muse muse login` (a second login of the same account).
4. **Register the runners**, the night box's first:
   ```sh
   bot/machine/register-runners.sh OWNER/REPO gpt agy muse=1 muse:fill
   TAG=jackioh-train-box BOX=b2 bot/machine/register-runners.sh OWNER/REPO muse=3
   ```
   The first keeps one of the night box's own Muse runners, `night-vm-muse` (it removes any of
   `-2` to `-4` left from before #1677), and registers the fill-in ones,
   `~agent-muse/actions-runner-fill` and `-fill-2`, copies of the unpacked runner under the same
   user and login; `muse:fill=1` would register one. A runner in the middle of a job is left
   registered, and the same command run again once it ends removes it. The second registers
   `night-vm-muse-2-b2` and `night-vm-muse-3-b2` beside `night-vm-muse-b2`.

   A copy takes the directories a self-updated runner's `bin` and `externals` link to
   (`bin.<version>`), never the links: a copied link made the copy run as the first runner, in
   its directory and with its registration ("already configured", 2026-10-26). The script
   removes such a copy, with its service, before any lane registers, and makes it again; and a
   lane whose directory holds another runner's registration (`night-vm-muse-fill`'s, left in
   `~agent-muse/actions-runner`) drops those files and registers under its own name, leaving the
   registration on GitHub to that runner's own lane, which takes it over (`--replace`).
5. **Deploy its starter**:
   ```sh
   TAG=jackioh-train-box STARTER_NAME=jackioh-muse-box-starter STARTER_ROLE=jackioh-muse-box-starter \
     RUNNER_LABELS=night-vm-muse REPO_ID=$(gh api repos/OWNER/REPO --jq .id) bot/machine/deploy-starter.sh
   ```
6. **The night box's starter wakes for every `night-vm-*` label**, its default, since the night
   box holds a `night-vm-muse` runner too. A job for that label queued while both boxes are off
   starts both, and the one that takes no job stops again after 30 idle minutes. A starter
   deployed with `RUNNER_LABELS` goes back to the default when deployed again without it:
   ```sh
   REPO_ID=$(gh api repos/OWNER/REPO --jq .id) bot/machine/deploy-starter.sh
   ```

**Moving the boxes to three jobs each** (2026-10-26), meant for once both instances are
`m7i-flex.xlarge`, which needs AWS's Paid plan (stop each, change its type, start it again;
neither the idle stop nor the starters change for that): run step 4's two commands, in that order, and step 6. Until the night box's fill-in
runners are registered, a Muse run the planner puts in gpt's or agy's lane waits queued for them.

**Updating it:** `TAG=jackioh-train-box bot/machine/on-machine.sh bot/machine/setup.sh muse`. Never
`--training` on it: that takes the idle stop off again, and the box then stays on for good.

**Going back to one box:** take `fill_in` out of Muse's entry in `providers.json` and set its
`lanes` to what the night box should hold, run
`TAG=jackioh-train-box BOX=b2 bot/machine/register-runners.sh OWNER/REPO muse=0`, then
`bot/machine/register-runners.sh OWNER/REPO muse muse:fill=0` on the night box.

## The local runner

A person's own machine runs the learn lane, every job of it (`.github/workflows/learn.yml`,
`training/learn.md`: the self-play export, the retrain, the screen and the gate, the lane's plan and
watch, the import of each export's games and the start of the next run, one run after another,
#1976), the gather workflow's job (`.github/workflows/gather.yml`, while `GATHER_ON_PATCH` is `true`),
and is the default runner of any other self-play export (#1008, `docs/learned-eval.md`
D1; `.github/workflows/selfplay.yml`, `training/README.md`, Where the exports come from), so an export
no longer holds 12 of GitHub's 20 hosted runners while CI waits (#1455); a dispatch of `selfplay.yml`
can still ask for GitHub's. It never touches the training box, which the lanes keep, and it can also
take some of the night box's own jobs while the box is busy or off.

| | |
|---|---|
| Machine | the owner's laptop (jgoetzmann): Windows 11, Intel i7-11800H (8 cores, 16 threads), 32 GB, Docker Desktop on WSL2 (which `%USERPROFILE%\.wslconfig` caps at 8 GB and 8 processors for every container). The owner looks after it |
| Label | `jackioh-selfplay-local`: `learn.yml` names it for every job, and `selfplay.yml` when its `runner` input does (its default since #1455). And `jackioh-selfplay-local-gather`, which `gather.yml`'s one job names (#2134): its entry in `config.json` has `ownSlot`, a slot of its own outside `maxJobs`, so gather runs beside a learn shard instead of waiting behind it. The entry also has `priority`: while both want the CPU, gather's runner takes it first (its cgroup weighs about five times the shard's: `--cpu-shares 8192`, which Docker 29 makes `cpu.weight` 532 against the default 100; on 2026-10-27 gather built at 5.6 CPUs while the shard played at 1.9), and when the Docker VM runs out of memory the kernel kills a training step before a gather one (the runners' `PIPELINE_JOB_OOMSCOREADJ`, 1000 against gather's 200), while both still give way to the owner's own programs (When, below). The supervisor starts its runner only when a gather job is queued (no standby) and retires it a minute after it goes idle. Both are registered to this repository alone |
| How it runs | `bot/machine/local/supervisor.ps1`, started at logon by a scheduled task (`install-task.ps1`). Each runner is one ephemeral container of the image `local/Dockerfile` builds (`jackioh-pc-runner`): it registers with a single-use JIT config, takes one job and is gone. A `docker` or `gh` call that gives no answer in two minutes (`docker info`, 30 seconds) is killed and counts as failed, so a Docker engine that stops answering reads as Docker not running, not as a supervisor stuck in one call |
| When | only while the PC is otherwise idle: CPU under 50% and memory under 24 GB in every sample of the last minute, leaving out the runners' own use and that of the `yieldingContainers` (a long-running container of the owner's that a gate of its own holds to the same rule and throttles while a runner works, so the two never read each other as the person's load) (`config.json`). A runner starts only when the PC is also quiet, CPU under 15% (`startCpuPercentBelow`) in every sample of that minute; one already up stays while it is calm. It keeps one runner Idle for the label while it is calm; when the PC gets busy the idle runner is unregistered at once, and a runner in a job is throttled to half a CPU until it is calm again, so no export loses a shard |
| Limits | one job at a time (`maxJobs`), 6 CPUs and 6 GB a container (`containerCpus`, `containerMemory`), swap off; and gather's slot, one job more at the same size (an entry may set its own `containerCpus` and `containerMemory`; gather's build of `jackioh-server` outgrew 3 GB, #2134). A shard and gather together ask for 12 of WSL2's 8 processors, so the shard runs on what gather leaves; never a GPU (no `--gpus`, `NVIDIA_VISIBLE_DEVICES=void`); a non-root user with `no-new-privileges`, no host folder and no Docker socket |
| Tools | Rust 1.97.0 (`rust-toolchain.toml`), a C++ compiler (`mlua` builds Luau from source), `git`, `jq` and `gh`, Node 24, Python 3.12, and the night box's CLIs. A job installs the rest itself (the learn lane's retrain: uv, which fetches Python 3.13 and torch for the fit; a night box job, the `harness` binary, from `.github/actions/harness` as on the boxes) |
| Heartbeat | every `heartbeatSeconds` (120, `config.json`; 0 turns it off) the supervisor writes `pc.json` to a secret gist of the account `gh` is signed in as: the PC's CPU and memory in use, as a whole and without its runners (the figures that decide whether it lends itself), the size of its memory, whether it is calm (lending itself), and each runner container with the CPUs it uses, whether it is throttled and, while it plays games, how many of how many (`Get-Progress`: the `jackioh selfplay` of a learn shard or the `jackioh stats` of gather it runs, its `--games`, `--from` and `--patch`, and the lines its `--out` holds, one a finished game), which the status issue's progress chart at its top reads. It makes the gist the first time, keeps its id in `pc-gist.txt` beside the scripts and names it to the repository as the variable `JACKIOH_PC_GIST` (`gh variable set`, which needs the admin sign-in the supervisor has anyway). `bot-status.yml` hands that variable to the status loop, whose runners section draws the PC from it, gather's job included. A heartbeat that fails is logged and tried again two minutes on; it never holds a runner up. `gh` needs its `gist` scope, which `gh auth login` gives by default (`gh auth refresh -s gist` adds it) |
| Disk | a container's work directory goes with it, so `target/` never grows between exports; a shard runs the release `jackioh` the export's build job built, so it builds nothing. The learn lane's retrain is the biggest job: about 10 GB while it runs (the export's logs, its positions, two or three builds, the web's install). The image is about 6 GB |
| Memory | the learn lane's retrain peaks at about 4.1 GB of the container's 6 GB on a 100,000-game export (the positions, the six seeds' fit, the AI's tests and the 800-game screen took 59 minutes here: learn run 21, 2026-10-21). Its 6 GB and gather's add up to more than WSL2's 8 GB, which the owner's own containers share too. A cap is a limit, not a reservation: a shard that plays uses well under 1 GB, and the lane nears its cap only in the retrain. Should the VM run out, the training step is the one the kernel kills (`priority`, above); raising `.wslconfig`'s `memory` is the owner's call |

Any workflow could ask for the label, so it is safe only while **"Require approval for all external
contributors"** stays on (Settings → Actions → General), as the night box's runners are (Why each
subscription is its own user).

**The night box's labels.** `config.json` also lists the `night-vm-<id>` subscriptions. A
subscription is served here only once it is logged in on the PC (`.\login.ps1 <id>`, a second login
of the same account, into the volume `jackioh-pc-home-<id>`), and then only when one of its jobs is
queued: a runner here with that label takes it, or the night box does. While the box is stopped the
PC usually takes it first, since the starter looks every five minutes and the supervisor every 20
seconds, and the box stays off. The jobs still count against `machine_parallel`. Since the learn lane
runs one run after another (#1976), the PC is seldom free, and its jobs come first (`config.json`
lists `jackioh-selfplay-local` first): a night-box job queued while it works waits for the box's own
starter, which starts the box in five minutes. To give the night box's jobs the PC first instead, list
their entries before `jackioh-selfplay-local` in the installed `config.json` and restart the task.

**Setting it up**, once, on the PC (PowerShell, Docker Desktop running):

```powershell
docker build -t jackioh-pc-runner:latest bot/machine/local
docker run --rm jackioh-pc-runner bash -c "ls /dev/dxg /dev/nvidia0; nproc; rustc -V"   # no GPU device
copy bot\machine\local\* D:\somewhere\jackioh-pc-runner\    # a folder no checkout moves
cd D:\somewhere\jackioh-pc-runner
.\install-task.ps1          # the supervisor at every logon, started now
```

`gh` must be signed in as a repository admin: the supervisor asks GitHub for each runner's JIT
config. The runner shows Idle under Settings → Actions → Runners as `pc-selfplay-<time>` within a
minute of a quiet PC. The log is `logs\supervisor.log` beside the scripts.

**After a change here**, on the PC: build the image again (the `docker build` above) when the
Dockerfile changed, since a runner started after that uses the new one, and when `config.json` or
`supervisor.ps1` changed, copy them over the installed ones (keep any setting of your own, such as
`yieldingContainers`) and restart the task (`Stop-ScheduledTask` then `Start-ScheduledTask -TaskName
'JackiOh PC runner'`), since the supervisor reads its config once, as it starts. Neither stops a job
that is running: its container keeps going, and the new supervisor finds it. The first supervisor
with the heartbeat makes its gist and sets `JACKIOH_PC_GIST` within two minutes (its log says so);
the status issue shows the PC from the next `bot-status` loop on, which reads the variable as it
starts.

**The learn lane runs only here**, so the PC is its only runner: it should not sleep while it is
plugged in (Settings → System → Power), since a shard in the middle of its work is lost when it does,
and while it is off or busy for a day the lane's jobs fail (`training/learn.md`, Running unattended).

**Stopping and removing it:**

```powershell
.\install-task.ps1 -Remove   # no new runners from now on
.\stop-all.ps1               # unregister and remove every idle runner here (-Force kills one in a job)
docker image rm jackioh-pc-runner:latest
```

GitHub drops an ephemeral runner when it ends, and the supervisor drops any it left offline every
ten minutes, so nothing stays registered once the supervisor is gone and `stop-all.ps1` has run.

## Running it

- **Adding a subscription on the machine:** add it to `providers.json` with
  `"runs_on": "night-vm-<id>"`, then run `setup.sh` with its id, log it in if it is a machine
  login, and run `register-runners.sh`.
- **A login stopped working** (the job's doctor or the run says it was refused): log that user in
  again as in step 3. Nothing else changes.
- **Upgrades never stop a job.** Ubuntu's unattended upgrades run every morning (06:00 to 07:00
  UTC), and on 24.04 needrestart restarts each service whose libraries an apt run replaced, at the
  end of that run. A runner restarted mid-job kills the job ("The runner has received a shutdown
  signal"), so `setup.sh` writes `/etc/needrestart/conf.d/night-vm.conf`, which leaves the runners
  and the training lanes out; they take new libraries at their next restart. Re-registering a
  runner that holds a job (`register-runners.sh`) kills that job the same way, so run it while
  the box is idle.
- **Disk:** every subscription's jobs share the one disk, and a full one fails whichever job
  writes next (on 2026-10-04 a revision of #206 died on `No space left on device`, 66 MB free when
  its job began). What keeps it clear:
  - **`clean.sh`, as each user.** After every job (the runners' job-completed hook,
    `/usr/local/bin/night-vm-job-done.sh`) it deletes that job's files; and after every job and
    every ten minutes (`night-vm-disk.timer`, for every agent user) it deletes what nothing needs:
    the user's package store, Cypress's binary and the npm cache once the user has no job going
    (Muse's other lanes share them), what a job that died without its hook left in an
    idle runner's `_work/_temp`, the runner version a self-update replaced (`bin.<version>` and
    `externals.<version>`, about 650 MB a runner, beside the one `bin` links to) and its download,
    the runner's own logs after two days, the user's `/tmp` leftovers after 6 hours, Codex's
    sessions after 7 days, and Muse's (a few hundred MB a day) after 8 hours. It runs as the
    user whose home it cleans,
    never as root, and the checkout and the logins stay.
  - **Temporary files stay with the job:** on the machine `TMPDIR` is the job's own
    `RUNNER_TEMP/tmp` (`bot-night.yml`), emptied when the job ends.
  - **The system:** the journal is capped at 200 MB (it may otherwise take a tenth of the disk),
    and the timer runs `apt-get clean`.
  - **A model job checks before it starts** (`bot/go/internal/disk/`): with under 8 GB free it runs
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
  Claude accounts run on GitHub's runners, under `max_parallel` (11), apart from these six. Both
  boxes are `m7i-flex.large` (2 vCPUs, 8 GB), the Free plan's largest with `c7i-flex.large`.
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
  last 24 hours, whether each box had memory to spare or ran short (a run stalled on memory for
  5 s or more), with the least memory available and how many runs swapped
  ([`go/internal/memory/`](../go/internal/memory/)). After a busy day, read it beside CPU and decide
  `machine_parallel`: fewer lanes if the box ran short or swapped, the same if it had memory to
  spare. Both boxes have 2 vCPUs and 8 GB.
- **Cost:** the machine is billed by the hour while it runs (about $0.096 an hour for the
  `m7i-flex.large`, so about $70 a month if it never stopped) plus its disk (about $9.60 a
  month for 120 GB of gp3). A stopped machine costs only
  the disk. The starter's Lambda calls and its schedule fit in the free tier.
- **The starter** looks only at bot-night runs, through GitHub's public API by repository id, so a
  rename or a transfer does not break it. It leaves alone a job that has waited three hours (its
  runner must be missing), so a broken runner cannot keep the machine running. Its log is the
  CloudWatch group `/aws/lambda/jackioh-night-vm-starter`, kept for seven days. If GitHub's
  rate limit for anonymous calls gets in the way, deploy it again with `STARTER_GITHUB_TOKEN` (a
  fine-grained token that can only read Actions).
