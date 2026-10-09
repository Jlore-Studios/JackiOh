# History: `docs/architecture.md`

Text moved word for word out of `docs/architecture.md` by #603, under the heading it sat beneath there. That heading is still in `docs/architecture.md`, with a line pointing here.

## 11. Decisions this document made that SPEC §9 does not state

All nine are now rulings in SPEC §11, **R104 to R112**. Each is cited by number at the point in the
source that implements it (`SPEC §11 Rnnn` in the SQL and in `config.rs`), so the comment is the
cross-reference between the code and the table. A choice that is local robustness rather than a rule —
an input-shape check, a `max_uses >= 1` constraint, a nullable `display_name` — is marked "no R-row"
instead, so an unnumbered marker never reads as an unrecorded gap.

| Row | Topic | Decision, and where it is implemented |
| --- | --- | --- |
| R104 | The code alphabet as a literal | `23456789ABCDEFGHJKLMNPQRSTUVWXYZ` — uppercase alphanumerics minus `0`, `1`, `I`, `O`. Exactly 32 symbols, so 16 characters are exactly 80 bits and a 6-character room code is 30 bits. SPEC's exclusion of lowercase `l` is satisfied by normalising input to upper case. This is the only 32-symbol set matching SPEC's exclusions: dropping `0`, `1`, `I`, `O` **and** `L` from the 36 alphanumerics leaves 31. |
| R105 | Catalog version format | A short opaque string stamped on every `public.cards` row and mirrored in `app.settings`; `core-1` for the Core set. SPEC requires a version and a rejection but never says what one looks like. |
| R106 | Circuit-breaker threshold and window | 100 system-wide failed redemptions in 600 seconds disables redemption and alerts. SPEC §9.4 requires "a threshold in a window" and names neither. |
| R107 | Constant-time failure floor | Every redemption response is padded to a fixed floor (250 ms) so the three failure kinds are indistinguishable in time. SPEC requires "identical time"; BUILD M6-T1 tests within 5 ms over 50 samples; SQL alone cannot deliver it. |
| R108 | Sweeper and reaper cadence | Matchmaker sweep every 3 seconds (SPEC §9.5 says "a sweeper every few seconds"); the reaper polls every 30 seconds against a 60-minute ceiling. |
| R109 | Rate limits for action flooding | Per-match actions per second and per-account API requests per minute. SPEC §9.8 requires both limits and names no numbers. |
| R110 | Room-code reuse | A room code is unique among matches that are not `over`, so codes are reusable once a match ends. SPEC says codes are 6 characters and nothing about their lifetime. |
| R111 | Launch grant quantity | One copy of every non-token card, which with `MAX_COPIES = 1` and 3 decks of 20 is exactly enough for a legal loadout, and keeps the ledger shape scarcity will need later. Granted by a trigger on the `pending → active` transition, idempotent by skipping cards that already carry a `launch` grant. |
| R112 | A match the reaper resolves, not the actor | `reap_stuck_matches` (`crates/server/src/api/results.rs`) finishes a stuck match itself rather than flagging it for a server that may be the crashed component. The consequence: a ceiling draw resolved by the reaper records `turns = 0` (the turn counter lives only in the actor's in-memory `GameState`) and leaves both ratings unchanged, where a draw resolved by a live actor applies the real rating update. SPEC §9.5 requires the reaper and says nothing about either value. |

## 12. Verifying the schema

The migrations are not taken on faith. `sh crates/server/tests/sql/run.sh` needs nothing but Docker:
it starts a throwaway Postgres, applies `crates/server/tests/sql/00_supabase_stub.sql` (stand-ins for the
Supabase-managed pieces the migrations reference — the `anon`, `authenticated` and `service_role`
roles, `auth.users` and `auth.uid()`; a real project supplies all of it), applies 0001–0006, saves a
loadout the old way (`03b_legacy_loadout_seed.sql`), applies 0007–0018 over it, and then asserts:

| File | What it proves |
| --- | --- |
| `01_schema_invariants.sql` | 20 tables in `public`, **every one with RLS enabled**; `loadout_card_unique` is on `(profile_id, card_id)` and refuses a cross-deck duplicate inserted by raw SQL (BUILD M6-T3); no `SECURITY DEFINER` function in `public`; no non-SELECT policy and no INSERT/UPDATE/DELETE privilege for `anon` or `authenticated` anywhere; the `auth.users` trigger creates a `pending` profile; the six-step redemption returns `email_unverified`, and one identical `invalid_code` for both a missing and a revoked code; success flips the profile to `active` and the activation trigger grants every non-token card to both `collection` and `collection_grants`; `collection_grants` refuses an UPDATE; a stale catalog version raises `update required`; the `cards` tag check admits all nine catalog tags, Jlockeed included, alone and together, and refuses an unknown one (R278). |
| `02_rls_as_client.sql` | Acting as the `authenticated` role inside a transaction (so `SET LOCAL` really takes effect): a profile sees exactly its own `profiles`, `collection`, `collection_grants`, `loadouts` and `loadout_deck_cards` rows and **zero** of the other profile's; `invite_codes`, `code_attempts`, `matches` and `match_actions` are refused outright; every client write — `collection` insert, `profiles` update, `loadout_deck_cards` insert — is refused, as are `app.redeem_invite_code` and `app.save_loadout`. It also sees exactly its own `decks` and `trios`, none of `series`, and cannot write any of them or call `app.upsert_deck` or `app.upsert_trio`; and exactly its own `tutorial_progress` row, which it cannot insert, update or delete, nor call `app.merge_tutorial_progress` (R320); and none of `game_records` (R376). This is §3's trust boundary, executed. |
| `03_match_lifecycle.sql` | `save_loadout` naming the rule it failed; `create_room` → `join_room` (own room refused, a live room refused a second joiner, both players marked in-match, the ceiling stamped on join); `append_match_action` assigning `seq` and returning the **original** seq for a replayed nonce without a second row (BUILD M6-T4); a server action with no author; `live_matches()` returning what a restarting server would fold; `end_match` writing one `results` row, moving both ratings, clearing both `current_match_id`, and staying idempotent on a second call; the room code reusable once the match is `over`; one queued ticket per profile; `claim_ticket_pair` returning true once and **false** to the second matcher (BUILD M7-T3's race test); the reaper turning a match past its ceiling into a `match-ceiling` draw and clearing both players. |
| `04_decks_and_series.sql` | The loadout 03b saved came out of 0007 as three named decks and a trio named "My trio", with the loadout rows untouched (R254); `app.upsert_deck` saves a draft, updates it in place, holds the cap under a lock on the profile and refuses another profile's id (R250); a trio names only its profile's own, distinct decks, and deleting a deck empties its slots (R252); a ticket carries its mode and exactly a Best-of-3 ticket a trio (R257), as a room does (R264); a series is a server-only row written by compare-and-set and found by its next match and by any of its games (R263). |
| `05_tutorial_progress.sql` | `app.merge_tutorial_progress` makes a profile's row on its first write, sorted in code-point order; unions the lessons, so a stale or empty write removes none; keeps the strictly newer Hide/Show choice (an older one and a tie keep what is stored, a write with no choice leaves it); refuses a union past the caller's cap or `app.settings.tutorial_lessons_max` and writes nothing; refuses a malformed id, a half choice, an unknown profile and a pending one; and the table itself refuses a half choice and a null lesson (R320). |
| `08_game_records.sql` | `game_records` holds one record per id, refuses a source other than `live` and `dev`, refuses a `source`, `mode` or `patch` column that disagrees with the record it files, so a filter on the columns cannot read a record as something it is not, and keeps `dev:` ids for development records alone (R376, R378). The client's refusal is `02`'s, with the other server-only tables. |
| `09_catalog_growth.sql` | Stamping a new catalog version grants its new cards to every active account once, through the ledger, and a second stamp or a reseed grants nothing (R481). |
| `10_last_boards.sql` | `last_boards` holds one board per profile and kind, replaced by the server and gone with its profile, refusing an unknown kind or a malformed entry; a match keeps the boards it started with whatever the profiles' rows do later; and no client role may read or write a last board (R417, R565). |

Each SPEC §11 row this schema implements is proved under a `### Rnnn: … ###` heading, which is how
`cargo jackioh spec check` credits an SQL file a ruling note lists in its `proven_in` (REVIEW B4). **That heading form is the signal; a bare
mention in prose is not.** The database-provable rows:

| Row | Heading | What it asserts |
| --- | --- | --- |
| R104 | `03` | Six malformed room codes (`I`, `O`, `0`, `1`, lower case, wrong length) each refused by the `matches.room_code` format check. |
| R105 | `03`, and `01` CHECK 15 | A stale catalog version is refused at save and at queue, with the message `update required`, from both `app.assert_catalog_version` and `app.save_loadout`. |
| R106 | `03` | An open breaker returns `circuit_open`, still logs the attempt, consumes no use and activates nobody; the untouched code redeems once the breaker closes. |
| R110 | `03` | A room code is reusable once its match is `over`. |
| R111 | `03` | A second launch grant leaves quantities at 1, adds no grant row and still owns no token. |
| R112 | `03` | The reaper's `results` row carries `turns = 0` and identical before/after ratings. |
| R250 | `04` | `app.upsert_deck` saves a draft, updates it in place, caps creates and refuses a foreign id. |
| R252 | `04` | A trio names its profile's own distinct decks, and deleting a deck empties its slots. |
| R254 | `04` | Every loadout became three named decks and "My trio", and the loadout stayed. |
| R257 | `04` | A ticket carries its mode, and exactly a Best-of-3 ticket carries a trio. |
| R263 | `04` | A series is a server-only row, written by compare-and-set, found by its next match and by any game. |
| R264 | `04` | A room keeps its mode, and exactly a Best-of-3 room keeps a trio. |
| R278 | `01` CHECK 18 | `cards_tags_check` admits every catalog tag, Jlockeed included, and refuses an unknown one, so `seed-catalog` can write #13 and #14. |
| R320 | `02`, `05` | A client reads only its own `tutorial_progress` row and writes none of it; `app.merge_tutorial_progress` only grows a row: the union of the lessons, the strictly newer choice, and the cap. |
| R376 | `08` | One record per game, its filter columns always equal to the record's own. |

**R107**, **R108** and **R109** are `config.rs` values with no database behaviour to assert, so they
get no heading; they are proved at the server level by BUILD M6-T1 (the 5 ms timing test), M7-T1 and
M7-T3. Those three ids appear in `03`'s header comment as explicit exclusions, so an id search that
keys on any occurrence rather than on the heading form would misread them as proved.

One finding worth keeping: `app.profile_is_active()` is `language plpgsql`, not `language sql`, because
a `language sql` body is parsed at creation time and it reads `public.profiles`, which the same
migration creates further down. The shared helpers stay together at the top of 0001 where the other
migrations look for them, and plpgsql defers the name resolution.

A second: `authenticated` holds `USAGE` on schema `app` and `EXECUTE` on exactly two helpers,
`app.current_profile_id()` and `app.profile_is_active()`. That is not a loosening — an RLS policy
expression is evaluated with the querying role's privileges, so without it every own-row read fails
with "permission denied for schema app". Schema `USAGE` conveys nothing by itself, the Data API does
not expose `app`, and `02_rls_as_client.sql` asserts that the privileged functions stay unreachable.

## 13. File map

Files this milestone owns, as the Rust server lays them out since v0.3.0. "M6-T*/M7-T*" marks what is
designed here and delivered by that task; `crates/server/README.md` has the module contract.

```
crates/server/
  Cargo.toml                       the server's dependencies (axum, tokio, sqlx, …) and the engine crates
  Dockerfile                       the image render.yaml builds: the one binary, CA certificates, tini
  .env.example                     the server-only half of §8
  migrations/
    0001_profiles_and_invites.sql   app schema, profiles, invite_codes, code_attempts,
                                    app.redeem_invite_code (the six steps of §9.4)
    0002_collection.sql             cards, collection, collection_grants, app.grant_cards,
                                    the launch grant on activation
    0003_loadouts.sql               loadouts, loadout_decks, loadout_deck_cards,
                                    loadout_card_unique (L4), app.save_loadout
    0004_matches.sql                tickets, matches, match_actions, results,
                                    app.join_room, app.claim_ticket_pair, app.end_match
    0005_service_role_reads_auth_users.sql, 0006_redeem_ip_lock.sql
    0007_decks_and_trios.sql        decks, trios, app.upsert_deck, app.upsert_trio, and every
                                    loadout turned into three decks and a trio (R250–R254)
    0008_queue_modes.sql            tickets.mode and frozen_trio, matches.room_mode and room_trio (R257, R264)
    0009_series.sql                 series: the Best-of-3 row, server-only (R259–R263)
    0010_jlockeed_tag.sql           cards_tags_check re-added with the Jlockeed tag (R278)
    0011_tutorial_progress.sql      tutorial_progress, app.merge_tutorial_progress (R320)
    0014_game_records.sql           game_records: the card statistics (R376)
    0015_classic_sets_tags.sql      cards_tags_check re-added with Book, Pancake and AI (patch v0.2.0)
    0016_catalog_growth_grants.sql  a new catalog version grants its new cards (R481)
    0017_last_boards.sql            last_boards, matches.p1_last_board / p2_last_board (R417, R565)
    0018_player_settings.sql        player_settings, app.merge_player_settings (R633, R634)
    …                               through 0027, each listed in §10 step 4
  src/
    main.rs                        the binary: serve (default), release, migrate, seed-catalog, mint-code,
                                   seed-accounts, season-start, stats-cards, stats-import
    app.rs                         the route table, boot, the background loops
    config.rs                      R79's values and the code alphabet, as named constants (BUILD §2)
    env.rs                         the environment, refusing to boot naming every missing variable
    auth.rs                        JWKS verification (Supabase) and E2E mode's fixture auth   M6-T1
    api/                           codes, collection, decks, game_records, queue, results, series,
                                   series_rules, settings, tutorial, …: one handler per route
    actor/                         match_actor.rs, protocol.rs, clock.rs, registry.rs, rooms.rs,
                                   ws_server.rs, engine.rs (the one path to the engine)   M6-T4, M7-T1
    db/
      store.rs                     the Db and Tx enums: one method per store operation
      pg.rs                        those methods over Postgres (sqlx)
      fake.rs                      the same over an in-memory store, for cargo test and E2E mode
      migrate.rs                   applies migrations/*.sql in order, ledger in app.migrations
    cli/
      seed_catalog.rs              the compiled-in catalog -> public.cards
      mint_code.rs                 `mint-code`: one invite code, plaintext to stdout once
      card_stats.rs                `stats-cards`: card win rates off game_records (R377, R378)
      import_dev_records.rs        `stats-import`: an AI development run's records -> game_records (R378)
      seed_accounts.rs, season_start.rs
    ranked/                        Glicko-2, the ladder, seasons (R603–R612)
  tests/
    server.rs                      the cargo suite: api/, actor/, store/ on the fake store
    sql/run.sh                     Docker-only schema validation (§12)
    sql/00_supabase_stub.sql       stand-in for auth.users, auth.uid(), the three roles
    sql/01_schema_invariants.sql   RLS everywhere, the L4 index, the redemption steps
    sql/02_rls_as_client.sql       the trust boundary, executed as `authenticated`
    sql/03_match_lifecycle.sql     room code -> log -> result -> reaper
    sql/04_decks_and_series.sql    decks, trios, queue modes and the series, as the server drives them
    sql/05_tutorial_progress.sql   the tutorial's grow-only merge (R320)
    sql/08_game_records.sql        the game records' checks (R376)
    db/run.sh                      the store contract against a throwaway Postgres
    deploy/rehearse.sh             Render's deploy rehearsed: the image, a non-superuser migration, a production boot
docs/
  architecture.md                  this file
```
