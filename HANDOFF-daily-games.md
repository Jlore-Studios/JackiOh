# Handoff: daily games (#3727): what's left after the merge

**Status on 2026-11-13.** Daily games are merged into `main` as `293a3273` ([PR #3762](https://github.com/Jlore-Studios/JackiOh/pull/3762), issue [#3727](https://github.com/Jlore-Studios/JackiOh/issues/3727), now closed). Every required check passed, and so did the merge queue's build. The code is finished, but three kinds of work remain:

1. **Configuration (needed before the feature fully works):** three secrets on each Cloudflare Worker. Without them, daily games still play, but no email and no push notification is ever sent.
2. **Screenshots** that the issue asked for. Nobody has taken them yet.
3. **Six known gaps**, none of them blocking. Each should become its own issue.

Background: [`crates/server/README.md`](crates/server/README.md) ("Daily games") covers how it works; the rules are SPEC §9.13 in [`spec/09-architecture.md`](spec/09-architecture.md). Rulings R3727000–R3727008 are in `spec/rulings/`.

---

## 1. Set the three secrets (a person with Cloudflare and Resend access)

| Secret | What it is | If it's unset |
| --- | --- | --- |
| `PLAYER_RESEND_API_KEY` | A Resend API key with **Sending** access for `jackioh.com`. Make a new key; don't reuse the alerts key (`ALERT_RESEND_API_KEY`). | No "your move" email is sent. |
| `VAPID_PRIVATE_KEY` | The Web Push signing key: the raw P-256 private scalar, base64url, no padding. | No push notification is sent. |
| `VAPID_PUBLIC_KEY` | Its public half: the raw 65-byte uncompressed point, base64url, no padding. | No push notification is sent. |

Each Worker (staging and production) needs all three. Use one VAPID pair per Worker, generated once. Do staging first, check it, then do production.

### 1a. Generate a VAPID key pair (once per Worker)

On Linux (`basenc` is part of GNU coreutils):

```bash
openssl ecparam -name prime256v1 -genkey -noout -out vapid.pem
```

```bash
openssl ec -in vapid.pem -outform DER | tail -c +8 | head -c 32 | basenc --base64url | tr -d '=\n'
```

```bash
openssl ec -in vapid.pem -pubout -outform DER | tail -c 65 | basenc --base64url | tr -d '=\n'
```

The second command prints `VAPID_PRIVATE_KEY`; the third prints `VAPID_PUBLIC_KEY`. macOS has no `basenc`, so in each of those two commands replace `basenc --base64url | tr -d '=\n'` with `openssl base64 -A | tr '+/' '-_' | tr -d '='`.

Keep `vapid.pem` somewhere safe, such as a password manager, then delete the local copy. **Don't rotate the pair later without a reason.** A new pair breaks every device's existing subscription, and each player has to turn push on again (see gap 6 below).

### 1b. Make the Resend key

In Resend, create an API key with Sending access, restricted to `jackioh.com`. The emails come from `JackiOh <no-reply@jackioh.com>`, which is `PLAYER_EMAIL_FROM` in `crates/server/src/config.rs`. `jackioh.com` should already be verified in Resend, since the alert emails and sign-in mail use it. Check under Domains.

**Note:** Resend's free plan allows 100 emails a day, and alerts, sign-in mail and now player emails all share that limit (see gap 5).

### 1c. Put the secrets on each Worker

From `apps/edge`, with Cloudflare credentials available to wrangler. Staging:

```bash
cd apps/edge
```

```bash
pnpm exec wrangler secret put PLAYER_RESEND_API_KEY --config ../../wrangler.jsonc --env staging
```

```bash
pnpm exec wrangler secret put VAPID_PRIVATE_KEY --config ../../wrangler.jsonc --env staging
```

```bash
pnpm exec wrangler secret put VAPID_PUBLIC_KEY --config ../../wrangler.jsonc --env staging
```

For production, run the same three commands with `--env=""` in place of `--env staging`, using production's own VAPID pair.

A container reads its secrets only when it starts. After setting them, redeploy: GitHub, then Actions, then **deploy staging**, then **Run workflow**. Production picks them up at its next deploy. This is also in `docs/deploy-cloudflare.md` §3 and `apps/edge/README.md` (Settings).

### 1d. Check it worked (on staging first)

- **Push.** Sign in and open `/daily`. Under Notifications, **"Notify me on this device"** should work. If it says *"Notifications on this device aren't available right now."*, the server has no VAPID pair.
  - Another check: `GET /api/daily/settings` (signed in) returns `vapidPublicKey`, which is `null` while push is off.
  - A pair that doesn't match is logged at boot as `daily.push.keys_refused`.
- **Email.** Use two accounts that are friends. One sends the other a daily challenge from the friends panel (Challenge, then Daily). The other accepts, and moves are played until the game waits on an account that has an email address and **"Email me when it's my move"** on.
  - The email goes out on that move, or at the latest within the hour, when the hourly sweep runs. The server logs `daily.emailed` when one is sent, or `daily.email.failed` with the reason.
  - Resend's dashboard lists the sends.
  - A player gets at most one email every 6 hours (`DAILY_EMAIL_MIN_GAP_HOURS`).
- **The hourly cron.** In Cloudflare, the Worker's Triggers should list two crons: `23 14 * * *` (the existing daily wake-up) and `7 * * * *` (the daily-games sweep). Both deploy from `wrangler.jsonc`, so there's nothing to set by hand.

Staging moves to `main`'s newest green commit every two hours. Production follows once the daily "super" run passes on staging (`docs/deploy-cloudflare.md` §2). So daily games may already be on staging before the secrets are set. That's fine: the game works, it just sends nothing.

---

## 2. Take the screenshots the issue asked for

Issue #3727 asked for screenshots of **the Daily screen (`/daily`)** and **a daily board (`/daily/<matchId>`)** at four sizes:

| Size | Device |
| --- | --- |
| 1440×900 | desktop |
| 1180×820 | iPad landscape |
| 820×1180 | iPad portrait |
| 390×844 | phone |

That's eight screenshots. For the board, use a game with some cards in play, where the clock pill and **"End turn and send"** are visible.

**Easiest way: run the e2e setup locally** (commands from `CLAUDE.md`, E2E). Each command runs in its own terminal, from the repository root.

```bash
pnpm install
```

```bash
E2E=1 cargo run --release -p jackioh-server
```

```bash
pnpm --dir apps/web build:e2e && pnpm --dir apps/web exec vite preview --port 5173 --strictPort
```

```bash
cd e2e && pnpm exec cypress open
```

In Cypress, run spec **`52-daily-challenge.cy.ts`**. It makes two accounts friends, starts a daily game by challenge in the browser, plays moves, and shows the Daily screen and the board. To capture the screenshots, either:

- pause it (Cypress's step-through), resize the browser to each size and take a screenshot; or
- copy spec 52 to a scratch spec and add `cy.viewport(1440, 900); cy.screenshot("daily-1440x900");` (and so on for each size) at the points where the Daily screen and the board are shown. The files land in `e2e/cypress/screenshots/`. Don't commit the scratch spec.

The helpers that set up a game over HTTP (`makeFriends`, `dailyKeepHandAs`, `dailyEndTurnAs`, and others) are in `e2e/support/daily.ts`. `e2e/README.md` explains the e2e server's fixture accounts.

**Or ask a Claude Code session to do it.** The repo has a `ui-walkthrough` skill (`.claude/skills/ui-walkthrough/SKILL.md`) that serves the client and screenshots it in a headless browser. That needs a container that can reach crates.io and the npm registry (see "For future Claude sessions" below).

**Where to post them:** as a comment on [PR #3762](https://github.com/Jlore-Studios/JackiOh/pull/3762). Issue #3727 is closed and locked; if the PR is locked too, use the tracker, [#3726](https://github.com/Jlore-Studios/JackiOh/issues/3726). Look at the phone size especially, since the layout was built for desktop and iPad first.

---

## 3. Six known gaps (not blocking; file each as an issue)

These came out of the code review and were deliberately left out of #3762. File each as its own issue under the tracker #3726. Give each one `difficulty:*` label as `docs/issues-and-patches.md` says (CLAUDE.md rule 11). Add `bot:build` only if you want the night bot to pick it up. Each fix needs a test that fails without it.

### Gap 1. KY's Competition shows a 60-second countdown on the daily board

- **What's wrong:** Ruling R3727003 says a daily game has no 60-second window for KY's Competition (Meditative+ #20); the move's own deadline is the window. But the board still shows the live game's 60-second countdown on those questions. Nothing enforces it, so it only misleads the player.
- **Where:** `apps/web/src/game/Prompt.tsx:821` always calls `useSequenceSecondsLeft` (from `apps/web/src/game/sequenceClock.ts`). The daily board is `apps/web/src/daily/DailyBoard.tsx`, which renders the ordinary `Game.tsx`.
- **Fix:** let `DailyBoard` tell `Game` (and through it `Prompt`) not to show the sequence countdown, for example with a prop, or show the daily clock instead. Add a `DailyBoard.test.tsx` case showing a KY's Competition question with no countdown.
- **Difficulty:** medium.

### Gap 2. An open board or Daily list doesn't refresh when the opponent ends the game on their own move

- **What's wrong:** the board and the list reread the game only when `dailyWaiting` changes. That number, sent on the social socket, counts the games waiting on *you*. If the opponent concedes, or otherwise ends the game while it's their move, that count doesn't change, so your open page shows the game as running until you reload.
- **Where:** `apps/web/src/daily/DailyBoard.tsx:93` and `apps/web/src/daily/DailyRoute.tsx:367`. The count comes from `crates/server/src/social/` (`dailyWaiting` in the social snapshot).
- **Fix, either:**
  - (a) the server's social frame also carries a number that changes whenever any of the player's daily games changes (a version or counter), and the web rereads when it moves; or
  - (b) the web also rereads when the tab regains focus or becomes visible, plus on a slow interval kept as a named constant in `apps/web/src/daily/config.ts` (CLAUDE.md rule 9).

  (a) is cleaner. (b) is web-only.
- **Difficulty:** medium.

### Gap 3. During the mulligan, the other player's automatic "End turn automatically" can make your mulligan answer stale

- **What's wrong:** during the mulligan both players hold the move at once (R3727002). The board sends the player's saved "End turn automatically" setting (`setAutoEndTurn`, R345) as soon as the player holds the move. That's immediately during the mulligan, so it adds a row to the game's log. If the other player sends their keep or mulligan at the same moment, theirs fails as `409 stale`. The board rereads quietly, and that player has to click again.
- **Where:**
  - `apps/web/src/daily/DailyBoard.tsx`, lines 101–137: the held `setAutoEndTurn`, sent once `hasMove` is true.
  - `crates/shared/src/host/daily.rs:112` (`daily_holders`): both seats hold the move during the mulligan.
- **Fix:** in `DailyBoard`, also hold `setAutoEndTurn` until the mulligan is over, then send it as now. A server-side alternative, refusing `setAutoEndTurn` during the mulligan in `daily_allows` (`crates/shared/src/host/daily.rs`), changes a rule and would need a ruling note. Add a `DailyBoard.test.tsx` case.
- **Difficulty:** medium.

### Gap 4. The 20-game cap can be exceeded with simultaneous requests

- **What's wrong:** a player may hold at most `DAILY_MAX_ACTIVE_GAMES` (20) running games, open seeks and open challenges. `has_room` counts them in one database transaction, and the new challenge, seek or game is written in a later one. Several requests sent at the same moment can each see 19 and all succeed.
- **Where:** `crates/server/src/daily/invites.rs:83` (`has_room`), called at lines 180, 254 and 338.
- **Fix:** count and insert in the same transaction while holding a per-player lock, for example `select … from public.profiles where id = $1 for update` or `pg_advisory_xact_lock` in `crates/server/src/db/pg.rs`. Keep the in-memory fake (`fake.rs`) matching and add a case to `crates/server/tests/store/contract.rs`.
- **If it needs a migration** (say a trigger): take the next free number after `0089`, and register it in all of these:
  - `crates/server/src/db/migrate.rs`;
  - `crates/server/tests/store/migrations_pinned.rs` (the test prints the checksum to pin);
  - `crates/server/tests/sql/run.sh`'s two lists.
- **Difficulty:** medium without a migration, hard with one.

### Gap 5. Player emails share Resend's free 100-a-day limit with sign-in mail

- **What's wrong:** alerts, sign-in and password mail, and now "your move" emails all go through Resend on the free plan's 100 emails a day. With enough daily games, player emails could use up the day's limit, and sign-in or password-reset mail would stop until the next day.
- **This is a decision for a person, not mainly a code fix. Options:**
  - upgrade the Resend plan;
  - send player mail from a separate Resend account or subdomain;
  - add a daily cap on player emails in code: a named constant in `crates/server/src/config.rs`, after which the sweep skips email for the rest of the day while push still goes out.
- **Where it's noted:** `docs/deploy-cloudflare.md` §3.

### Gap 6. After a VAPID key change, old push subscriptions are never deleted

- **What's wrong:** when a push service answers 404 or 410, the server deletes that subscription. After the VAPID pair is changed, push services answer **403** to every old subscription instead. Those are logged as `daily.push.failed`, retried every sweep, and never removed.
- **Where:** `crates/server/src/daily/webpush.rs:281` (only 404 and 410 count as gone) and `crates/server/src/daily/notify.rs`, around line 236, where a gone subscription is deleted.
- **Fix:** treat 403 as gone too, or after a few 403s in a row. Add a test next to `a_push_subscription_answered_410_is_deleted` in `crates/server/tests/api/daily.rs`.
- **Difficulty:** medium.
- **Until it's fixed:** don't rotate the VAPID keys.

---

## Good to know

- **Re-agreeing to terms:** this change moved `TERMS_VERSION` because the privacy policy now mentions notification settings and push subscriptions. Every account is asked to agree again at its next visit (R757001). This is expected.
- **Cost:** the new hourly cron wakes each site's container once an hour, even with nobody online. `docs/deploy-cloudflare.md` §6 (Cost) covers it.
- **Nothing else to ship:** the change has no card-patch fragment (`cargo jackioh classify` called it "App"), so there's no `patches ship` step. Ruling paragraphs in `spec/pending/3727.md` are folded into `spec/11-rulings.md` by the daily `spec-fold.yml` job.
- **For future Claude sessions:** this cloud environment couldn't reach `crates.io`, `static.crates.io`, `static.rust-lang.org` or the npm registry, so it couldn't build or test locally and depended on CI alone. If a Claude session will work on these gaps, add those hosts to the environment's allowed network domains first. A session working an issue also reads and leaves a handoff comment on it (CLAUDE.md rule 14, `.claude/skills/handoff/SKILL.md`). The previous session's is on #3727.
