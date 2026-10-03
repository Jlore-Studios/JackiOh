# The night bot's machine

Every model session of the night bot runs here: one virtual machine on AWS, where each
subscription in [`.harness/providers.json`](../../.harness/providers.json) whose `runs_on` is
`night-vm-<id>` has a Linux user of its own and a GitHub runner of its own. `gate`, `plan` and
`deliver`, the jobs that hold a GitHub write token, stay on GitHub's runners.

| | |
|---|---|
| Instance | EC2 `m7i-flex.large` (2 vCPUs, 8 GB, plus 8 GB swap), Ubuntu 24.04, 30 GB gp3, tagged `Name=jackioh-night-vm`, in the project's Region (`us-east-2`) |
| Way in | Session Manager only (`aws ssm start-session --target <instance>`): no inbound port, no key pair. The instance role has `AmazonSSMManagedInstanceCore` and nothing else. |
| Users | `agent-<id>` per subscription (`agent-claude-1` … `agent-devin`): a home only it can read, no `sudo`, no Docker |
| Runners | `~agent-<id>/actions-runner`, registered as `night-vm-<id>` with that one label, a systemd service under that user |
| CLIs | `claude`, `codex`, `agy`, `muse` and `devin`, installed for every user; Node 24, pnpm (corepack) and Python 3.12 |
| Idle stop | a timer powers it off after 30 minutes with no job and no Session Manager session |
| Starter | a Lambda run every five minutes starts it when a job waits for one of its runners |

## Why each subscription is its own user

A machine login (`"login": "machine"`) is a file in a home directory that its CLI refreshes in
place. With one user per subscription, a session can read only its own login and leave things
only in its own home, and its runner, labelled with its id alone, takes only its jobs. A Claude
account's token comes from its GitHub secret, handed to that job alone, and is never written to
the home. Jobs can run three at once (`max_parallel`), each as its own user.

Every repository workflow could ask for these labels, so the repository makes outside
contributors' pull requests wait for approval before any workflow runs (Settings → Actions →
"Require approval for all external contributors"). Keep it that way.

## The files

| File | Runs | What it does |
|---|---|---|
| `setup.sh` | on the machine, as root | everything above except the logins and the registration; idempotent, run it again to update the CLIs or add a subscription |
| `on-machine.sh` | on your computer | runs a local script on the machine through Session Manager, starting the machine first if it is stopped |
| `register-runners.sh` | on your computer | registers each `night-vm-*` subscription's runner with GitHub and starts it as a service |
| `starter.py` | AWS Lambda | starts the machine when a bot-night job is queued for a `night-vm-*` runner (tested in `bot/tests/test_machine.py`) |
| `deploy-starter.sh` | on your computer | creates or updates the starter, its role and its five-minute schedule |

The scripts on your computer need the AWS CLI signed in to the project (`aws login`) and, for the
runners, `gh` signed in as a repository admin. They find the machine by its `Name` tag, or take
`INSTANCE_ID`.

## Building it

1. **The instance.** Launch Ubuntu 24.04 (x86_64) as `m7i-flex.large` with a 30 GB gp3 disk, an
   instance profile holding `AmazonSSMManagedInstanceCore`, a security group with no inbound
   rule, no key pair, shutdown behaviour **stop**, and the tag `Name=jackioh-night-vm`.
2. **Set it up**, with the ids from `providers.json`:

   ```sh
   bot/machine/on-machine.sh bot/machine/setup.sh claude-1 claude-2 claude-3 claude-4 gpt agy muse
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
- **Disk:** each job's files and the user's package store are deleted when the job ends
  (`/usr/local/bin/night-vm-job-done.sh`, the runners' job-completed hook); the checkout and the
  logins stay.
- **How many at once:** three (`max_parallel`) is as many as this machine holds. Measured on
  2026-10-02 with three jobs running (a Claude build in typecheck and the web tests, Muse and
  agy): load average 14 on 2 vCPUs, 6.1 of 7.8 GB in use and 1.5 GB swapped, about 4 GB for the
  Claude job alone. More jobs need a bigger machine, and the Free plan's largest are the
  2-vCPU `m7i-flex.large` and `c7i-flex.large`.
- **Cost:** the machine is billed by the hour while it runs (about $0.096 an hour, so about $70 a
  month if it never stopped) plus its disk (about $2.40 a month). A stopped machine costs only
  the disk. The starter's Lambda calls and its schedule fit in the free tier.
- **The starter** looks only at bot-night runs, through GitHub's public API by repository id, so a
  rename or a transfer does not break it. It leaves alone a job that has waited three hours (its
  runner must be missing), so a broken runner cannot keep the machine running. Its log is the
  CloudWatch group `/aws/lambda/jackioh-night-vm-starter`, kept for seven days. If GitHub's
  rate limit for anonymous calls gets in the way, deploy it again with `STARTER_GITHUB_TOKEN` (a
  fine-grained token that can only read Actions).
