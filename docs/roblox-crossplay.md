# Roblox crossplay through daily games: the design

2026-11-13 · issue #3735, part of #3726, after #3727 (daily games) and #2898 (the Roblox version)

> **Status: a design, not the spec.** SPEC is still the only source of rules (CLAUDE.md). Nothing here is
> built: each build issue ([RC11](#rc11-build-issues)) writes its own §9.13 text and rulings when it is
> built, and where this note differs from §9.13 today, that build issue changes the spec. Each answer is
> marked as the **Owners' answer** where the owners gave one, or **Suggested** where they have not. When
> this was written they had answered none, so every answer is a suggestion, and the choices only they can
> make are questions A to F in [RC12](#rc12-questions-for-the-owners). The facts about Roblox come from
> Roblox's documentation as the builder knew it. Nobody read that documentation again for this note, so
> each such fact is marked _to confirm_, and each build issue checks the ones it relies on. A number
> marked _strawman_ is to be tuned when it is built.

## Contents

- [RC0. The idea in one page](#rc0-the-idea-in-one-page)
- [RC1. #2898's constraint](#rc1-2898s-constraint)
- [RC2. Q1, how a Roblox server proves who it is](#rc2-q1-how-a-roblox-server-proves-who-it-is)
- [RC3. Q2, whom it plays for](#rc3-q2-whom-it-plays-for)
- [RC4. Q3, decks, collections and pay](#rc4-q3-decks-collections-and-pay)
- [RC5. Q4, catalog versions](#rc5-q4-catalog-versions)
- [RC6. Q5, pairing and friends](#rc6-q5-pairing-and-friends)
- [RC7. Q6, notifications](#rc7-q6-notifications)
- [RC8. Q7, limits](#rc8-q7-limits)
- [RC9. Q8, platform rules](#rc9-q8-platform-rules)
- [RC10. Hidden information](#rc10-hidden-information)
- [RC11. Build issues](#rc11-build-issues)
- [RC12. Questions for the owners](#rc12-questions-for-the-owners)

## RC0. The idea in one page

> Once there is a Roblox client (#2898), crossplay between it and the web happens through **daily games**
> (#3727). A Roblox server and our server never host a live match together.

That is the owner's direction of 2026-11-11, as issue #3735 gives it.

- **Live games stay apart.** A Roblox player plays live games with Roblox players, on the Roblox game's own
  servers, as #2898 says. A Roblox server and our server never host a live match together.
- **Daily games cross over.** A daily game (§9.13) is played one move at a time over plain HTTPS. Each
  action is one request with a JSON answer. The answer is the seat's read: its `view_for`, its legal
  actions, the deadline, who holds the move and the catalog version. No socket, cookie or engine is needed
  on the client's side. A retried action applies once: a nonce already logged applies nothing.
- **So a Roblox game's server is one more client.** It calls the same daily routes the web calls
  ([`crates/server/README.md`](../crates/server/README.md), "Daily games"):
  - `GET /api/daily`: the player's games, challenges and seeks, and how many games wait on them;
  - `POST /api/daily/challenges`, `POST /api/daily/challenges/:id/accept`,
    `POST /api/daily/challenges/:id/decline` and `DELETE /api/daily/challenges/:id`;
  - `POST /api/daily/seeks` and `DELETE /api/daily/seeks/:id`;
  - `GET /api/daily/games/:matchId` and `POST /api/daily/games/:matchId/actions`.
- **The rules stay ours.** The Roblox server sends intent and draws the read it gets back. It never decides
  a rule and never sees the other seat's hidden cards (CLAUDE.md rule 7; [RC10](#rc10-hidden-information)).

How a turn goes:

1. A player joins the Roblox game. Its server sends `GET /api/daily` with the universe's key and the
   player's Roblox user id ([RC2](#rc2-q1-how-a-roblox-server-proves-who-it-is)).
2. Our server finds the JackiOh account linked to that Roblox user
   ([RC3](#rc3-q2-whom-it-plays-for)) and answers as that account.
3. The Roblox server shows the list and the boards to that player alone.
4. The player moves. The Roblox server posts the action, and our server answers with the new read. The
   opponent on the web is told "your move" by email or push, as today. A player on Roblox is told through
   Roblox ([RC7](#rc7-q6-notifications)).

## RC1. #2898's constraint

#2898 says:

> No cross-platform play: Roblox players play Roblox players, and the Roblox game doesn't call our server

The new wording:

> No live cross-platform play: Roblox players play live games with Roblox players only, on the Roblox
> game's own servers. Daily games cross platforms: a Roblox game's server reads and plays its players'
> daily games through JackiOh's API (docs/roblox-crossplay.md), so for daily games the Roblox game calls
> our server.

#2898 is locked, so a person pastes the new wording into it. Until then, this note is the record. The rest
of #2898 stands: a Roblox player's collection lives in DataStores, and Roblox's friends replace ours in
live play on Roblox.

## RC2. Q1, how a Roblox server proves who it is

**Suggested (the builder's; the owners have not answered):** one key per Roblox universe, sent as a bearer
token, and accepted on the daily routes only.

- **One key per universe and environment.** A universe is a Roblox experience with its places. The
  production universe calls jackioh.com, and a test universe calls staging.jackioh.com. Staging is behind
  Cloudflare Access ([`docs/deploy-cloudflare.md`](deploy-cloudflare.md)), so the test universe also sends
  an Access service token's pair. A person makes that token, as they made the deploy check's.
- **What the key is.** 32 random bytes in base64url, after a fixed prefix (`rbxu_`, suggested) that tells
  the server it is a universe key and not a Supabase token before it checks anything. Our server keeps only
  the key's SHA-256, next to the universe id, in a server secret. The suggested name is
  `ROBLOX_UNIVERSE_KEYS`. It holds a list, so a new key and the old one both work while the key rotates.
  The server compares hashes in constant time. It never holds the key itself, so a leak of its secrets
  leaks no key.
- **Where the Roblox side keeps it.** In the experience's Secrets store in Creator Hub, restricted to our
  domain, and read by `HttpService:GetSecret` in a server script _(to confirm)_. The key never goes in a
  script's source, a place file, a DataStore or anything sent to a Roblox client. A `Secret` cannot be
  read as a string in Luau. It can only be added to a request's header or URL, for its own domain _(to
  confirm)_. So a script that went bad can use the key only against our server, and cannot copy it out.
- **What each request carries.** `Authorization: Bearer rbxu_…` and a `Roblox-User-Id` header naming the
  Roblox player it acts for. Only a game server can send HTTP requests, never a Roblox client _(to
  confirm)_.
- **What the key may do.** Only these routes accept it:
  - every daily route of [RC0](#rc0-the-idea-in-one-page): the list, the four challenge routes, the two
    seek routes, a game's read and its actions;
  - `GET /api/friends`, read only, so the Roblox board can pick a friend to challenge
    ([RC6](#rc6-q5-pairing-and-friends)).

  Every other route refuses it with 403. That includes `/api/daily/settings`, `/api/daily/push`, the
  unsubscribe routes, `/api/account`, decks, chat, the social routes and every socket. `GET /api/catalog`
  and `GET /api/catalog/:version` need no key at all.
- **As whom it runs.** As the linked account ([RC3](#rc3-q2-whom-it-plays-for)), under the `Active` gate,
  meeting every check a web request of that account meets: its rate limit (R109), friends and blocks
  (R1689000, R3727007), `daily_full` (R3727004) and the rest.
- *Rejected:*
  - HMAC request signatures. A script cannot read a `Secret`, so it cannot sign with one.
  - A token for each player, held by the Roblox server. Players would have to sign in to our site from
    inside Roblox, which has no browser for it, and their refresh tokens would sit in DataStores we do not
    control.
  - Asking Roblox, on each request, whether the player is in that server. A player's privacy settings can
    hide it _(to confirm)_, and it costs one more call per request.
  - One key for every environment. A leak on staging would then reach production.
- **Risk.** A leaked key, or a script in the experience that went bad, can act for every linked player of
  that universe in their daily games. It cannot reach their account, decks, settings or anything outside
  daily games, and its requests stay inside [RC8](#rc8-q7-limits)'s budgets. To rotate a key, a person adds
  the new hash, updates the experience's Secret, then removes the old hash.

## RC3. Q2, whom it plays for

**Suggested (the builder's; the owners have not answered):** the JackiOh account linked to that Roblox
user by Sign in with Roblox (R3147000–R3147003), and no other.

- **The mapping.** Sign in with Roblox adds an identity of the provider `custom:roblox` to the account
  (R3147000). Its `sub` claim is the Roblox user id. Supabase keeps it in `auth.identities`, with the id as
  `provider_id` _(to confirm for a custom provider)_. The server reads nothing of `auth.identities` today,
  and none of the identity's display data (R3507000). Build issue 1 gives the server a table of its own,
  filled from that one provider's rows as they change. So a link or an unlink on `/account` (R3025000)
  counts at the next request. A Roblox account can be linked to one JackiOh account at most: a link to a
  Roblox account that another JackiOh account holds is refused (R3025000). So each Roblox user maps to at
  most one account.
- **No link.** A Roblox user with no linked account gets 404, and the Roblox game shows no daily games for
  them. What it tells them is question F ([RC9](#rc9-q8-platform-rules)).
- **Consent.** Linking Roblox is the player's consent for the Roblox game to play their daily games.
  Unlinking withdraws it. `/account` says so next to the Roblox row (R3507000's row), in build issue 1.
- **The account's state.** A pending or banned account is refused, since the routes are `Active`. An
  account that owes the terms in force (R757001) keeps playing and reading its running games, as a change
  of terms never interrupts play on the web. A new challenge, acceptance or seek from the key is refused
  with 403 `consent_owed` until the player agrees on the web, since the Roblox game has no consent screen.
  - *Rejected:* refusing every request while terms are owed, which would stop games already running and
    lose them on time; and refusing nothing, which would let a player start new games for good without
    agreeing to the terms in force.
- **2a. A Roblox player with no JackiOh account.** **Suggested:** no account is made in the first cut.
  Crossplay plays only for an active JackiOh account linked to that Roblox user. So §9.4's invite code,
  verified address, age check and terms hold unchanged.
  - *Rejected for now:* an account made on first play. It would need a profile with no address (R3147001
    asks for one before the invite code), a way past the invite code, the age check and the terms asked
    inside Roblox, and a way to delete it from Roblox. It would hold a username and its daily games, and
    whatever question C gives it. That is question A, and build issue 5 if the owners choose it.
- **2b. Age.** **Suggested:** Roblox's age handling does not stand in for R757000. Every account that
  crossplay serves passed R757000's check and agreed to R757001's terms on the web, so nothing changes. The
  server reads no age signal from Roblox. A Roblox player under `MIN_ACCOUNT_AGE_YEARS` cannot hold a
  JackiOh account, so their Roblox game shows them no daily games. Whether a Roblox signal could ever
  replace R757000 is for the owners with legal advice (`docs/compliance.md` §2). That is question B.

## RC4. Q3, decks, collections and pay

**Suggested (the builder's; the owners have not answered):** a game started from Roblox is All Random only
(R258), so neither side needs the other's collection.

- **Starting a game.** From a universe key, a challenge, an acceptance or a seek whose `mode` is `"bo1"`
  is refused with 422 `bad_mode`. All Random deals both decks from the match seed and needs no collection
  or saved deck. Its "More cards from" pick (R1372) is a field of the request, so the Roblox board can
  offer it too. The Roblox board shows a Best of 1 challenge with no Accept button.
- **Playing a game.** The key may play any of the account's running daily games, including a Best of 1
  game the player started on the web. A running game's decks are frozen in its match, and playing it needs
  only its read, never the collection.
- *Rejected:*
  - Best of 1 with the account's JackiOh decks. The server could check them today. But the Roblox board
    would need the account's collection and decks, which are routes outside RC2's list, and a deck screen
    of its own. It can come later.
  - Best of 1 with the Roblox player's DataStore collection. The server cannot check §9.4's L5 (copies
    never exceed the quantity owned) against a collection it does not hold.
- **Pay.** **Suggested:** a crossplay game pays the JackiOh account as §9.13 already says (Jlore, XP,
  quests and proficiency, as an unranked game; R604), with no server change. The Roblox game grants nothing
  in DataStores for it. That is question C.

## RC5. Q4, catalog versions

**Suggested (the builder's; the owners have not answered):** the Roblox build carries one catalog version,
the server's when the experience was published, and never more than one.

- **New games start on the server's version.** Every daily game starts on the server's catalog version
  (§9.4, R388). A universe request that starts a game (a challenge, an acceptance or a seek) names the
  version its build carries. Today the daily routes take no version, so build issue 3 adds the field. If
  the version is not the server's, the request is refused with 409 `update_required`, as every door into a
  match refuses a stale catalog (R1638000).
- **Running games keep theirs.** A running game folds under the version it started on (R767000, R3727005).
  The Roblox board draws an older game with its own catalog and shows the older-catalog notice, as the
  web's board does. `GET /api/catalog/:version` is there if the board ever wants that version's own text.
  A game on a version newer than the build's waits on Roblox until the experience is republished, and plays
  on the web meanwhile. That is a game the player started on the web after a patch, or a seek made from
  Roblox before a patch that paired after it. Holding a board back is drawing, not a rule, so CLAUDE.md
  rule 7 holds.
- **The experience is republished at each card patch.** Between a patch going live and the republish, a
  Roblox player starts no new daily game, and their running games go on. That is #2898's build: it ships
  each patch's cards, and their art as Roblox assets, to the experience.
- *Rejected:* the issue's idea of starting a game on an older version the Roblox build carries. No door
  into a game starts one on a version other than the server's, and old versions would stay open to new
  games for good.

## RC6. Q5, pairing and friends

- **The queue.** **Suggested (the builder's; the owners have not answered):** one daily queue, mixed by
  default, with no per-player choice in the first cut. The player base is small, and a daily game shows
  only a username and the board. The read names no platform, so neither side is told where the other
  plays. That is question D.
  - *Rejected:* separate queues for each platform, which would split a small queue; a choice on each seek,
    which needs a column and a choice in both games' screens and can come later; and a platform badge on
    the opponent, which tells the other player something about the account its owner did not choose to
    show.
- **Friends.** **Suggested:** a Roblox friend is not a JackiOh friend. A daily challenge from Roblox goes to
  a JackiOh friend through the existing route. The Roblox board reads the friends list
  (`GET /api/friends`) to pick one. Adding and removing friends, and blocking, stay on the web. #2898's note
  that Roblox's friends replace ours holds for live play on Roblox only. That is question E.
  - *Rejected:* making a Roblox friendship a JackiOh one. It would make a friendship neither player asked
    for here, and §9.4 makes a friend only by a request one player sends by exact username and the other
    accepts.
- **Presence.** **Suggested:** a universe request is not a presence heartbeat (R3210000), and it keeps no
  live queue ticket alive (R897000). A player on Roblox cannot take a live challenge from a web friend, so
  showing them online would offer a game they cannot play.

## RC7. Q6, notifications

**Suggested (the builder's; the owners have not answered):** a third channel, Roblox's experience
notifications through Open Cloud, beside email and push.

- **When it is due.** `jackioh_shared::notify::decide_notify` (`crates/shared/src/notify.rs`) gains a
  third channel, `roblox`. It falls due by the rules push and email follow: a game newly waiting on the
  account, or a reminder `DAILY_REMINDER_HOURS` before a deadline. It goes only to an account linked to
  Roblox (RC3).
- **A gap like email's.** At most one per `DAILY_ROBLOX_MIN_GAP_HOURS`, suggested 6 _(strawman)_, in
  `crates/shared/src/config.rs`. Roblox limits how many notifications an experience may send each user
  _(to confirm)_. A message the gap holds back goes at the first look after it, as an email does, so the
  gap loses none.
- **How it is sent.** Through Open Cloud's user notifications API, to the linked Roblox user, with an Open
  Cloud API key the server holds _(to confirm)_. It is a server secret, and a person makes the key in
  Creator Hub with only the notification permission. The server's Open Cloud client follows no redirect
  and calls Roblox's host alone, as the push client keeps to `PUSH_SERVICE_HOSTS` (§9.8).
- **What it says.** A fixed template made in Creator Hub, "It's your move in N daily games", with the count
  as its one parameter if templates take one, else "It's your move in a daily game" _(to confirm)_. It
  carries no username, so nothing in it needs filtering.
- **Who gets it.** Roblox sends an experience's notifications only to a player who opted in through Roblox
  _(to confirm)_. Our side adds an on/off switch for it on the web's Daily screen, as email has one. The
  universe key cannot change settings (RC2), so the Roblox side's only switch is Roblox's own.
- **What it stores.** A last-sent column for the channel, and the switch, on `daily_notify`. Migration
  `0089_daily_games.sql` holds `last_email_at` and `last_push_at` there today.
- *Rejected:* a notification sent by the Roblox game itself. It has no server running while no player is
  in it, so it would miss the very player who is away.

## RC8. Q7, limits

- **Roblox's side.** HttpService allows 500 requests a minute for each game server _(to confirm)_.
- **When the Roblox server reads.** **Suggested (the builder's; the owners have not answered):** it reads a
  player's `GET /api/daily` when they join, when they open the Daily board and after each of their moves.
  Otherwise it reads it at most every 15 minutes while they are in that server. That interval is a Roblox
  number, set in #2898's build. Nothing is batched in the first cut. A full server of 50 players then sends
  about 3 background requests a minute, far under its 500.
- **Our side.** **Suggested:** `API_REQUESTS_PER_MINUTE` (300) still holds for each account (R109). Each
  universe gets a budget of its own on top, suggested `ROBLOX_UNIVERSE_REQUESTS_PER_MINUTE` of 1,200
  _(strawman)_, in `crates/server/src/config.rs`. It counts every request the key sends, one for a Roblox
  user with no link included. A request over either budget gets 429 `rate_limited` with its wait (R192). A
  universe refused too often raises `api.rejections.unusual` under its own key, as R1717000 does for an
  account or an address.
- **Cost and the sleeping server.** The server sleeps after 5 minutes with no request
  ([`apps/edge/README.md`](../apps/edge/README.md)), and every read from Roblox wakes it. That is why the
  background interval is 15 minutes and not shorter. The first request after a sleep waits for the boot, up
  to `START_WAIT_MS` (60 seconds). The Roblox side must wait that long for it, and whether HttpService
  allows such a wait is _to confirm_.
- *Rejected:* a read every minute, which would keep the server awake, and billed, whenever anyone is in the
  Roblox game; and one shared budget for all of Roblox's traffic with no per-account limit, which would let
  one player's requests spend everyone's.

## RC9. Q8, platform rules

**Suggested (the builder's; the owners have not answered)** for each:

- **Filtering.** Every text a player made, coming from our server, passes TextService filtering for the
  Roblox player who sees it, before they see it _(to confirm which call)_. In a daily game that means only
  usernames: the opponent's, and each friend's in the friends list. All Random decks have no names. Card
  names and text are the game's own content, not a player's _(to confirm that Roblox does not ask to filter
  them)_.
- **A Roblox player's own username.** Their JackiOh username is what web players see, and it passes R1433's
  check like any other. Roblox's display name is never sent to our server.
- **Chat.** Chat does not cross platforms. Daily games have no chat and no emotes (§9.13), and the key may
  not call the chat routes (RC2).
- **Bots.** The daily queue seats no ladder bot (§9.13, R1452), so a crossplay opponent is always a person.
  R3103000's labelling of bots on a Roblox server never comes up, and Roblox's rules on bots do not apply.
- **Links off Roblox.** Roblox's rules limit what an experience may say about other sites _(to confirm)_.
  In the first cut, the Roblox game names no website. A Roblox player with no linked account is told only
  that daily games need a linked JackiOh account. Whether the game may name jackioh.com is question F. It
  decides how a Roblox player with no linked account ever learns that they can link one.

## RC10. Hidden information

CLAUDE.md rule 7 holds with the Roblox server as the client.

- **It sends intent and draws the read.** It decides no rule. Our server checks every action, and the legal
  actions it sends are what the board offers.
- **Each view goes to its own player.** The Roblox server holds a player's read only to draw it for that
  player. It sends the read to that player's client alone, over a RemoteEvent fired to that player, never
  to all players and never on a part in the world that others can see. When two linked players in the same
  Roblox server play each other, the server holds both seats' reads, and each goes only to its own player.
- **It stores no view.** No read goes into a DataStore or a MemoryStore. A player's list and boards are
  read again from our server.
- **The key stays on the server.** The key never reaches a Roblox client
  ([RC2](#rc2-q1-how-a-roblox-server-proves-who-it-is)).

## RC11. Build issues

Each is a sub-issue of tracker #3726, opened by a person, since the night bot cannot open issues. Each
writes its own §9.13 text, rulings and tests when it is built.

| # | Title | Difficulty | Scope | After | A person's part |
| --- | --- | --- | --- | --- | --- |
| 1 | `Patch: the server links a Roblox user id to its JackiOh account` | `difficulty:hard` (a migration) | A table the server owns, filled from the `auth.identities` rows of the provider `custom:roblox` and kept in step as a link or an unlink changes them; a store method and its test in `tests/store/contract.rs`; the SQL tests; the consent sentence beside `/account`'s Roblox row (RC3) | — | None |
| 2 | `Patch: a Roblox universe key that plays its players' daily games` | `difficulty:hard` (it sets the security boundary for every daily route) | RC2's key, header, route list and 403 elsewhere; RC3's `consent_owed`; RC4's `bad_mode`; RC6's rule that a universe request is not presence; RC8's per-universe budget and its alert | 1 | Make each universe's key, set `ROBLOX_UNIVERSE_KEYS` on each environment's Worker and the key in the experience's Secrets; the Access service token for the test universe |
| 3 | `Patch: crossplay in the daily queue: the Roblox build's catalog version and a mixed queue` | `difficulty:medium`, or `difficulty:hard` if the owners choose question D's option 2 (a migration) | RC5's catalog version on the universe key's new-game requests and its 409 `update_required`; RC6's queue as the owners answer D | 2 | None |
| 4 | `Patch: "your move" on Roblox through Open Cloud notifications` | `difficulty:hard` (the shared crate and the server must agree, and a migration) | RC7: the `roblox` channel in `decide_notify`, `DAILY_ROBLOX_MIN_GAP_HOURS`, the Open Cloud client, the `daily_notify` columns, the Daily screen's switch | 1 | Make the Open Cloud API key and set it on each environment's Worker; make the notification template in Creator Hub |
| 5 | `Patch: Roblox-only accounts`, only if the owners choose it for question A | `difficulty:hard` | RC3's 2a as the owners answer A, B and F | 1, 2 | As the answers need |

The Roblox side (its Daily board, its reads and their interval, its filtering, its RemoteEvents and its
Secret) is #2898's build.

## RC12. Questions for the owners

Each is a choice only the owners can make. **How to answer:** a comment on #3735 with the letter and the
option's number, or your own words; a follow-up updates this note.

### A. Roblox-only accounts

A Roblox player with no JackiOh account cannot play daily games under this note. Making one for them asks
what that account holds and how it meets the invite code, the age check and the terms.

1. None: a Roblox player links an active JackiOh account first. Nothing about accounts changes.
2. An account made on first play, with no economy: it holds a username and its daily games, and earns
   nothing. It still needs a way past the invite code, the age check and the terms inside Roblox.
3. An account made on first play with the invite code waived for Roblox players: the fullest reach, and a
   second way in that skips the invite gate.

**My suggestion:** 1, because it changes no account rule and needs no answer to B, and option 2 or 3 can
follow as build issue 5.

### B. Age

R757000 asks for the month and year of birth at sign-up on the web. Roblox has its own age handling.

1. R757000 on the web only: every crossplay account passed it there.
2. Ask the month and year inside Roblox, under R757000's rules, for a Roblox-only account (A's 2 or 3).
3. Trust Roblox's own age checks in place of R757000.

**My suggestion:** 1, because it needs no legal review and relies on no Roblox signal. 3 is a question
for legal advice (`docs/compliance.md` §2).

### C. Pay

A crossplay game pays the JackiOh account today, as every daily game does.

1. The JackiOh account only: Jlore, XP, quests and proficiency, as §9.13 says. No change.
2. Nothing: a crossplay game pays nobody. It keeps the two economies apart, and the game feels worth less.
3. DataStores too: the Roblox game also grants its own currency for it. Paid twice, and the two economies
   start to mix.

**My suggestion:** 1, because it needs no change and keeps each platform's economy its own.

### D. The queue

1. Mixed: one daily queue for everyone, with no choice.
2. Each player chooses, on each seek, to pair with anyone or with their own platform only: a column and a
   choice in both screens.
3. Separate: a queue for each platform, which splits a small queue.

**My suggestion:** 1, because the player base is small and a daily game shows only a username and the
board.

### E. Friends

1. JackiOh friends only: a Roblox player challenges their JackiOh friends; Roblox's friends stay Roblox's.
2. Roblox friends who have both linked JackiOh accounts become JackiOh friends when one asks and the other
   accepts in the Roblox game.

**My suggestion:** 1, because §9.4's friend requests already work and option 2 needs a new way to send
one.

### F. Links off Roblox

1. The Roblox game names no website, and serves only players who have already linked.
2. The Roblox game names jackioh.com, to link an account there, if Roblox's rules allow it.

**My suggestion:** 1 until someone has read Roblox's current rules on off-platform links. Option 2 is the
only way a Roblox player with no linked account learns they can link one, so it matters most while A
stays 1.
