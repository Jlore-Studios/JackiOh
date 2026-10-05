# The night bot's machine

The night bot's model sessions for the subscriptions that log in here run on this machine: one
virtual machine on AWS, where each subscription in
[`.harness/providers.json`](../../.harness/providers.json) whose `runs_on` is `night-vm-<id>`
(Codex, agy, Muse and Devin) has a Linux user of its own and a GitHub runner of its own per lane.
The Claude accounts log in from secrets, so their jobs run on GitHub's own runners (free for this
public repository, four vCPUs each), and so do `gate`, `plan` and `deliver`, the jobs that hold a
GitHub write token.

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus 8 GB swap), Ubuntu 24.04, 60 GB gp3, tagged `Name=jackioh-night-vm`, in the project's Region (`us-east-2`) |
| Way in | Session Manager only (`aws ssm start-session --target <instance>`): no inbound port, no key pair. The instance role has `AmazonSSMManagedInstanceCore` and nothing else. |
| Users | `agent-<id>` per subscription (`agent-gpt`, `agent-agy`, `agent-muse`, `agent-devin`): a home only it can read, no `sudo`, no Docker |
| Runners | `~agent-<id>/actions-runner` (and `actions-runner-2` … for a subscription with `lanes` over 1), registered as `night-vm-<id>` (`night-vm-<id>-2` …) with the one label `night-vm-<id>`, each a systemd service under that user |
| CLIs | `claude`, `codex`, `agy`, `muse` and `devin`, installed for every user; Node 24, pnpm (corepack) and Python 3.12 |
| Idle stop | a timer powers it off after 30 minutes with no job and no Session Manager session |
| Starter | a Lambda run every five minutes starts it when a job waits for one of its runners |

## Why each subscription is its own user

A machine login (`"login": "machine"`) is a file in a home directory that its CLI refreshes in
place. With one user per subscription, a session can read only its own login and leave things
only in its own home, and its runner, labelled with its id alone, takes only its jobs. A Claude
account's token comes from its GitHub secret, handed to that job alone, and is never written to
the home. At most `machine_parallel` (6) jobs run here at once, each as its own user; Devin has
`"lanes": 6`, so Devin can fill the machine on its six runners when the others are idle, and Muse
has two (`night-vm-muse` and `night-vm-muse-2`, both under `agent-muse` and its one login).

Every repository workflow could ask for these labels, so the repository makes outside
contributors' pull requests wait for approval before any workflow runs (Settings → Actions →
"Require approval for all external contributors"). Keep it that way.

## The files

| File | Runs | What it does |
|---|---|---|
| `setup.sh` | on the machine, as root | everything above except the logins and the registration; idempotent, run it again to update the CLIs or add a subscription |
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

1. **The instance.** Launch Ubuntu 24.04 (x86_64) as `m7i-flex.large` with a 60 GB gp3 disk, an
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
    to 60 GB in place. About 22 GB is fixed: 8 GB of swap, ten runners' installs at about 0.7 GB
    each (twice that after a self-update until `clean.sh` removes the old version), the system and
    the CLIs; six jobs' worktrees and installs come on top. To grow it again, raise the volume's
    size (`aws ec2 modify-volume`); `setup.sh` grows the partition and the filesystem into it
    (`growpart`, `resize2fs`), and cloud-init does at the machine's next start. Neither needs a
    restart.
  - `CYPRESS_INSTALL_BINARY=0` keeps `pnpm install` from fetching Cypress's 800 MB binary at all,
    since the bot's checks never run e2e.
- **How many at once:** six machine jobs (`machine_parallel`), because a job here runs only the
  light checks. The load is each job's checks, not its model: on 2026-10-02 three jobs running
  the full set had the machine at load average 14 on 2 vCPUs with 1.5 GB swapped. Measured on
  2026-10-03 for one job: `pnpm install` 26 s and 0.55 GB, lint 2 min and 0.8 GB, typecheck
  4.5 min and 1.1 GB, the catalog and rulings checks seconds and 0.13 GB, and `pnpm test` 22.6
  min on one of GitHub's four-vCPU runners. So a job here runs install, typecheck and the light
  checks; lint and the unit tests (`"machine": false` in `.harness/config.json`) run in CI on
  the pull request, and the agents are told to run only the tests for what they changed. The
  Claude accounts run on GitHub's runners, so `max_parallel` is 10: six here and up to four
  Claude jobs there. The Free plan's largest machines are the 2-vCPU `m7i-flex.large` and
  `c7i-flex.large`.
- **Cost:** the machine is billed by the hour while it runs (about $0.096 an hour, so about $70 a
  month if it never stopped) plus its disk (about $4.80 a month). A stopped machine costs only
  the disk. The starter's Lambda calls and its schedule fit in the free tier.
- **The starter** looks only at bot-night runs, through GitHub's public API by repository id, so a
  rename or a transfer does not break it. It leaves alone a job that has waited three hours (its
  runner must be missing), so a broken runner cannot keep the machine running. Its log is the
  CloudWatch group `/aws/lambda/jackioh-night-vm-starter`, kept for seven days. If GitHub's
  rate limit for anonymous calls gets in the way, deploy it again with `STARTER_GITHUB_TOKEN` (a
  fine-grained token that can only read Actions).
