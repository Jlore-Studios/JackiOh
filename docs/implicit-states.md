# Implicit states

Some of the code's states have no name: several fields read together (an `Option`, a flag, a counter, a
"pending" field beside what it guards) hold them, where one enum would. This file maps them: for each, its
fields, every combination of their values, what moves it between them, and whether each combination can
happen. It was started by #3678. The code audit's fourth part, which extends this map on each run, waits
for a person on branch `claude/pensive-shannon-tomyd9` (`.github/hygiene/code.md` section 4).

How to read and extend it:

- **One table per state, one row per combination.** The columns are `#`, the combination, whether it is
  reachable (and the shortest sequence that reaches it) or not (and why), what it should do (and the
  source: a spec section, a ruling, or the README that is the code's contract, or **unspecified**), and
  what it does (the test that shows it, as `path::module::fn`).
- **Only the real machinery counts.** A combination is reachable when a test reaches it through the
  machinery: the testkit's `scenario()` and `reduce`, the server suite's actor, the run's `run_reduce`.
  A combination that only fields set by hand reach is unreachable, however a test builds it.
- **Versions are not states.** A field that is `None` or a default for a record older than it (a match
  before portraits, an older replay, checkpoint or shipped catalog) is a version. It is listed under
  [Not states](#not-states-versions) and checked to still read.
- **Re-checking.** Each state names the commit it was checked at. `git log <commit>.. -- <its files>`
  lists what has changed since; a row whose code changed is checked again.
- **Representable when serialized.** A row marked so cannot be reached, but a value of the type (a game
  state, a run, a wire frame, a checkpoint or replay read back) can still hold it, which one enum would
  rule out. It is named for a person; no shape is changed here.

## 1. A match actor's seat

**Checked at** `13bd66f47171`, plus #3678's pull request.

**Files:** `crates/server/src/actor/match_actor.rs` (`Core`, `attach`, `on_frame`, `on_socket_gone`,
`detach`, `halt`), `crates/server/src/duos/actor.rs` (the same for a co-op person's seat),
`crates/shared/src/host/duel.rs` and `crates/shared/src/host/duos.rs` (the machine's `stopped`,
`voided`, `connected`), `crates/server/src/actor/guard.rs` (`SeatGuard`'s last answered hello),
`crates/server/src/actor/ws_server.rs` (`Socket`).

**Fields,** per home seat (the seat the account began in; a co-op person's seat a1 or a2):

- `sockets[home]`: none, an open `Socket`, or a closed one the actor has not yet heard close;
- `hello_pending[home]` (P): none, or the socket generation whose Hello is queued and unrun;
- `socket_generation[home]` (G): 0 until the first `attach`, then bumped by each;
- and, read with them, the machine's `stopped` (after `halt`) and, for a 1v1 match, `voided` (R679, which
  sets `stopped` too).

**What moves it:** `attach` (a socket stored, G + 1, a fresh view queued as `Task::Attach`); a `hello`
frame (P := G, a `Task::Hello` queued, unless P = G already, R835001, or the guard drops it, R2686000);
the Hello task running (P := none, only for its own generation); the transport closing
(`on_socket_gone`: the seat empties, a `Task::Disconnect` and its grace follow, §9.5); `detach`; `halt`
(`stop`, the registry's `stop`, `stop_all`'s restart drain, `reap_finished`, R776000's let-go: sockets
taken and closed 1001, `reap_finished`'s with `MATCH_OVER_CLOSE_CODE` (R3694000), `stopped` set); the void (R679); a rebuild or a restart from a checkpoint (a new
actor, G = 0, no socket). `Core` is never serialized, so a JSON round trip does not touch it.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| S1 | live, G = 0, no socket, P none | Reachable: a rebuilt actor, or one restarted from its checkpoint, before any socket attaches; a fresh match before its first socket | The seat is away from the first socket's arrival and keeps a stored grace (§9.5, R744) | `crates/server/tests/actor/recovery.rs::r744_disconnect_grace_for_a_seat_that_is_not_there::r744_a_fresh_matchs_seat_that_never_opens_its_socket_loses_after_the_grace`, `…::r744_a_seat_that_never_comes_back_to_a_rebuilt_actor_loses_when_its_grace_runs_out` |
| S2 | G = 0 with a socket, or with P set | Unreachable: `attach` stores the socket and bumps G under one lock, and a frame reaches `on_frame` only through the handlers `attach` installs after that | — | — |
| S3 | live, open socket, P none | Reachable: `attach` | A fresh full view, never a log replay (§9.5) | `crates/server/tests/actor/match_actor.rs::m6_t4_the_match_actor::two_websocket_clients_complete_a_scripted_game` |
| S4 | live, open socket, P = G | Reachable: a `hello` frame on the seat's socket | Later hellos of this generation are absorbed by the queued one (R835001); one sooner than `WATCH_HELLO_INTERVAL_MS` after the last answered is dropped (R2686000) | `crates/server/tests/actor/match_actor.rs::m6_t4_the_match_actor::r835001_coalesces_queued_hellos_but_never_across_a_reattach`, `crates/server/tests/actor/duos.rs::hello_coalescing::hellos_repeated_while_one_is_queued_coalesce_and_a_newer_socket_always_gets_its_own`, `crates/server/tests/actor/ws_server.rs::r2686000_player_hello_limit::r2686000_match_hellos_are_answered_once_per_interval_actions_continue_and_a_new_generation_answers_at_once`, `crates/server/tests/actor/duos.rs::r2686000_player_hello_limit::r2686000_duos_hellos_are_answered_once_per_interval_actions_continue_and_a_new_generation_answers_at_once` |
| S5 | live, open newer socket, P < G | Reachable: a `hello`, then a reattach before the queue runs | The newer socket's hello is never absorbed by the older one's: both run, beside the attach's view (R835001) | the same R835001 and duos `hello_coalescing` tests (three views on the revived socket) |
| S6 | live, no socket, P none | Reachable: the transport drops | The grace starts and is stored on the match; a reconnect clears it (§9.5) | `crates/server/tests/actor/match_actor.rs::m6_t4_the_match_actor::starts_the_grace_countdown_on_a_drop_and_clears_it_on_reconnect` |
| S7 | live, no socket, P = G | Reachable: a `hello`, then the transport drops before the queue runs | The Hello greets no one; the grace still starts; the next socket is a new generation and its hello is answered (§9.5, R835001, R2686000) | `crates/server/tests/actor/match_actor.rs::seat_states_3678::a_hello_queued_as_its_socket_drops_answers_no_one_and_the_next_socket_is_greeted`, `crates/server/tests/actor/duos.rs::seat_states_3678::a_hello_queued_as_its_duos_socket_drops_answers_no_one_and_the_next_socket_is_greeted` (new) |
| S8 | live, a closed socket the actor has not heard close | Reachable: the socket's outbox overflows (R1441: `Socket::send` marks it closed and leaves the report to the transport), until the transport reports the close | Presence reads it gone at once (R672 reads `is_open`); the grace starts on the report (§9.5) | `crates/server/tests/actor/match_actor.rs::seat_states_3678::a_socket_its_outbox_closed_holds_the_seat_until_its_transport_reports_the_close` (new) |
| S9 | stopped, sockets taken | Reachable: `stop`, `stop_all`'s restart drain, `reap_finished`, R776000's let-go | Every socket closed 1001, so the client reconnects and the registry rebuilds the match (§9.5, R776000; `crates/server/README.md`, the drain: "each live match checkpointed and its actor stopped with 1001"); `reap_finished`'s, a finished match's past its rematch window, with `MATCH_OVER_CLOSE_CODE`, which no client reconnects on (R3694000) | `crates/server/tests/actor/match_actor.rs::r3694000_a_finished_match_lets_its_sockets_go_with_its_window::r3694000_a_finished_match_closes_its_sockets_and_leaves_memory_once_its_window_ends`, `crates/server/tests/actor/ws_server.rs::the_graceful_shutdown::a_sigterm_lets_the_request_in_flight_finish_closes_every_socket_with_1001_stops_the_loops_and_returns`, `crates/server/tests/actor/recovery.rs::r776000_a_server_action_the_store_fails_to_take::r776000_an_actor_behind_its_log_lets_go_and_the_next_socket_folds_the_row_it_missed`, the first asserts of both S10 tests |
| S10 | stopped, then a socket attached to it | Reachable: a handle held across the stop (in production `Registry::attach` holds one between `actor_for` and `attach`, and `stop`, `stop_all` or `reap_finished` can run between the two on another of `#[tokio::main]`'s worker threads) | Closed 1001 and never greeted, as `halt` closes the sockets it held, so the client reconnects to the rebuilt actor (§9.5, R776000, the README's drain line; `DuosActor::attach` already did this) | **Was a mismatch (1v1), fixed in this pull request**: the stopped `MatchActor` kept the socket open, sent it `loadouts`, `view`, `clock` and `portraits`, and wrote the match row's stored clocks (`turnDeadline` set to null), so the person would sit on an actor nothing reaches while, once the other seat reached the rebuilt actor, R744 started their grace there. `crates/server/tests/actor/match_actor.rs::seat_states_3678::a_socket_handed_to_a_stopped_actor_is_closed_going_away_and_never_greeted` (failed before the fix), `crates/server/tests/actor/duos.rs::seat_states_3678::a_socket_handed_to_a_stopped_duos_actor_is_closed_going_away` |
| S11 | voided (and so stopped), then a socket attached to it | Reachable: a handle held across Glitch's void | The socket hears only the void's close, `MATCH_VOIDED_CLOSE_CODE`, never a view (R679); the voided arm goes before the stopped one | `crates/server/tests/actor/glitch.rs::glitchs_void::a_socket_handed_to_the_actor_it_held_across_the_void_hears_only_the_void` (new) |
| S12 | a second socket for one home seat while the first is open | Reachable: a second tab, or another device (a reconnect before the old socket closed is the same, and the client's own replacement of a stale socket lets it go first, so its close is never heard) | The newer socket takes the seat, the older is closed with `SEAT_TAKEN_CLOSE_CODE` (4412), and no grace starts since the seat was never empty (R744; `on_socket_gone`'s comment). The replaced client goes to `elsewhere` (`apps/web/src/game/net.ts`'s and `duos/socket.ts`'s `onclose`): no backoff, no reconnect, and its page says the game is open in another tab or on another device with a Play here button that takes the seat back (R3767000). A watcher's replacement stays 1000 | `crates/server/tests/actor/match_actor.rs::seat_states_3678::r3767000_a_second_socket_for_one_seat_replaces_the_first_with_seat_taken_and_starts_no_grace`, `crates/server/tests/actor/duos.rs::seat_states_3678::r3767000_a_second_socket_for_one_duos_seat_replaces_the_first_with_seat_taken` (new); the web's, `apps/web/src/game/net.test.ts`, `apps/web/src/duos/socket.test.ts`, `apps/web/src/routes/match.test.tsx` and `apps/web/src/duos/DuosRoute.test.tsx`, each under "R3767000" |
| S13 | (co-op) a socket for an AI seat, b1 or b2 | Unreachable from the routes: `DuosActor::seat_of` answers a1 or a2 only. A held handle reaches `attach`'s guard | Team B's seats are the AI's (§13.7) and only a human seat's socket is greeted (§13); the guard closes it 1000 and greets nothing | `crates/server/tests/actor/duos.rs::seat_states_3678::a_socket_handed_to_an_ai_seat_is_closed_and_never_greeted` (new) |

## 2. The engine's paused action

**Checked at** `13bd66f47171`, plus #3678's pull request.

**Files:** `crates/engine/src/work.rs` (read its header first), `crates/engine/src/state.rs` (`pending`,
`work`, `work_cursor`, `mulligan`, `result`), `crates/engine/src/setup.rs` (`open_mulligans`,
`resolve_mulligans`), `crates/engine/src/game_over.rs` (`end_game`),
`crates/engine/src/subsystems/ai_policy.rs` (an owed AI turn), `crates/engine/src/testkit/invariants.rs`
(I7).

**Fields:** `pending` (the one open prompt, §10.1), `work` (the owed continuations, §9.3) and
`work_cursor` (where the running cascade parks next, R113), read with `result` (the game is over, §2.5)
and `mulligan` (the two sealed mulligans, open beside `pending`, never in it, R265).

**What moves it:** a prompt opening (`pending` set) and the scope that asked parking its rest
(`push_work`, `owe`, `park_work`: inserted at `min(cursor, len)`, the cursor set past it); `take_work`
and `drain_work`'s `begin_work_cascade` (cursor 0); `drop_work` and `unpark_work` (the cursor follows
what it loses, clamped to the length); an answer (`pending` cleared, the drain resumes); `end_game`
(`result` set, `pending` and `mulligan` cleared, `work` left as it stands); `open_mulligans` (only while
`pending` is none) and `resolve_mulligans` (`mulligan` cleared before anything it runs can ask); a JSON
round trip, a checkpoint and a replay fold (which rebuild the same values, §10.1).

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| E1 | live, no prompt, no mulligans, `work` empty, cursor 0 | Reachable: any settled game | Nothing owed (§9.3, §10.3, I7) | `crates/cards/tests/cross/paused_sequences.rs::paused_action_states_3678::e1_e3_a_cry_that_asks_owes_the_rest_of_its_play_at_the_cursor_and_survives_json` (new), `crates/engine/tests/rules/invariants.rs::i7_a_settled_game_with_no_prompt_open_owes_nothing_and_the_monitor_is_clean` |
| E2 | prompt open, `work` empty, cursor 0 | Reachable: C #81 The Power to Thrive's `reduce` activation asks for a card of the hand as it resolves (R831009); its answer is the prompt's own continuation, and the activation owes nothing behind it | The answer resumes the prompt's own step; nothing else waits (§10.6, R113) | `crates/cards/tests/cross/paused_sequences.rs::paused_action_states_3678::e2_an_activation_that_asks_as_it_resolves_owes_nothing_behind_its_prompt` (new) |
| E3 | prompt open, `work` owed, cursor = its length | Reachable: Core #7's Cry opens its Discover and the rest of the play is parked behind it; Radiant Core #65's first start-of-turn question parks the rest of the turn start; at turn 0, R224's arrival clause parks the rest of setup | The answer drains the owed items in R113's order (§10.3, §10.5, R113, R117, R122) | `…::paused_action_states_3678::e1_e3_a_cry_that_asks_owes_the_rest_of_its_play_at_the_cursor_and_survives_json`, `…::paused_action_states_3678::e3_e4_a_second_question_of_one_start_of_turn_clause_waits_with_the_turn_start_owed_past_the_cursor` (new), `crates/cards/tests/cross/setup_and_mulligan.rs::r224_setup_waits_for_a_question::r224_r9_r151_r158_r265_the_question_a_replacement_draw_s_arrival_clause_asks_p1_is_not_overwritten_by_p2_s_mulligan_10_1` |
| E4 | prompt open, `work` owed, cursor below its length | Reachable: answer Radiant Core #65's first start-of-turn question. The drain takes the owed turn start back up and resets the cursor (R113); the second question opens from the prompt's own step and parks nothing new, so the turn start waits at index 0 with the cursor at 0. A throwaway random-policy probe of 400 games also met it with the cursor strictly between 0 and the length (a target question inside a draw chain that cast Core #95, four items owed, the cursor at 1) | A park now goes ahead of everything still owed (R113, `work.rs`'s header) | `…::paused_action_states_3678::e3_e4_a_second_question_of_one_start_of_turn_clause_waits_with_the_turn_start_owed_past_the_cursor` (new; it also round-trips this state through JSON) |
| E5 | mulligans open, no prompt, `work` empty, cursor 0 | Reachable: `begin_game` | Both mulligans open at once, `pending` untouched (§2.1 step 3, R265; I7 excuses this state) | `crates/cards/tests/cross/setup_and_mulligan.rs::open_mulligans_3678::e5_the_open_mulligans_hold_no_prompt_in_pending_and_owe_nothing` (new) |
| E5′ | mulligans open with `work` owed | Not shown: `open_mulligans` is setup's last step (`deal_from`), run only after every owed setup item has been taken; no sequence found reaches it. I7's test builds it by hand (`i7_work_owed_behind_an_open_prompt_is_not_orphaned`), which proves the check, not reachability | Owed work waits for the mulligans (I7) | — |
| E6 | over, no prompt, no mulligans, `work` as it stood | Reachable both ways. Empty: Core #68's Cry kills the enemy hero. Owed: p2 concedes (R211) while p1's Core #7 Discover is open, so the play's rest stays owed | Nothing happens after the game is over, every action refused (§2.5, R216); a game over keeps its debt (testkit invariant I7's note: "a game over keeps its debt") | `crates/cards/tests/cross/paused_sequences.rs::paused_action_states_3678::e6_e7_a_game_won_inside_a_cry_closes_every_prompt`, `…::paused_action_states_3678::e6_e7_a_concession_under_an_open_prompt_closes_it_and_leaves_the_rest_of_the_play_owed` (new) |
| E7 | over with a prompt open, or with the mulligans open | Unreachable: `end_game` clears `pending` and `mulligan`, `work::paused` stops every scope once `result` is set, and `reduce` refuses every action after (R216) | — | the two E6 tests assert both closed and every action refused |
| E8 | live, no prompt, no mulligans, `work` owed or cursor ≠ 0 | Unreachable: an action ends by draining `work` (§10.3, `drain_work`) until it is empty or a scope pauses on a prompt or a result, and taking each item resets the cursor. I7 flags it after every action of CI's `cargo jackioh fuzz --seeds 200`, and the 400-game probe never met it. The I7 tests in `invariants.rs` that show it flagged set the fields by hand, so they prove the check, not reachability | Orphaned work (I7) | — |
| E9 | cursor above the length | Unreachable: `place` inserts at `min(cursor, len)` and sets the cursor one past it, `take_work` and `begin_work_cascade` set 0, `drop_work` clamps to the new length, and an owed AI turn (`ai_policy`) resets it to 0 before it appends what was owed | — | — |
| E10 | mulligans open and a prompt open | Unreachable: `open_mulligans` does nothing while `pending` is set, and `resolve_mulligans` clears `mulligan` before the answers it resolves can ask (R224, R265) | — | — |

E7 to E10 are **representable when serialized**: a `GameState` read back from JSON (a checkpoint, a replay
fixture, the wire's state for the tools) can hold them, and one enum for the paused action would rule them
out. No shape is changed here.

## 3. The dungeon run's position

**Checked at** `13bd66f47171`, plus #3678's pull request.

**Files:** `crates/run/src/lib.rs` (`RunState`, `run_create`, `apply`, `fight_won`, `with_max_hp`,
`run_node`, `run_enemy`), `apps/web/src/dungeon/store.ts` (the device keeps a run as its `RunFile`, the
setup, the unlocks and the log, and folds it with `runFold`; it never keeps a `RunState`).

**Fields:** `row` (none before the first step), `col`, `phase` (`Map`, `Fight`, a tile's or a reward's
phase, `Won`, `Dead`) and `hp` against `run_max_hp`.

**What moves it:** `run_create` (row none, col 0, `Map`, HP the max); `Go` (row and col set together, then
the node's phase, R3583000); `FightOver` (a win raises the max and refills HP to it, R3584000; a loss sets
HP 0 and `Dead`); `Abandon` (`Dead`, from any phase, nothing else touched); the tiles' and rewards'
answers (back to `Map`, or `Won` after the final boss's extraction; a max-HP change moves HP with it,
`with_max_hp`, R3584001); `run_fold` (the same state again from the log, R3496000).

Its rows are D1 to D9 (D for the dungeon), never `R<n>`: an R-id reads as a ruling, and `cargo jackioh
spec check` treats one cited in `crates/` as a ruling's citation (CLAUDE.md rule 3).

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| D1 | row none, col 0, `Map`, HP the max | Reachable: `run_create` | Any node of the first row may be stepped onto (§15.5, R3583000) | `crates/run/src/tests.rs::r3583000_go_steps_only_onto_a_node_the_one_stood_on_leads_to` |
| D2 | row none, col 0, `Dead`, HP the max | Reachable: `Abandon` before the first step | The run ends lost, is still scored, and refuses every action (§15.8) | `crates/run/src/tests.rs::abandoning_before_the_first_step_ends_the_run_off_the_map` (new; its one-action log folds back to it) |
| D3 | row none with any other phase, or with col ≠ 0 | Unreachable: only `Go` leaves `Map`, and it sets `row` and `col` before the node's phase; `Won` follows the final boss | — | — |
| D4 | row set, col in its row, a node's phase | Reachable: `Go`, then the node's tiles and fights | The node's own rules (§15.5 to §15.8) | `crates/run/src/walk_tests.rs::walks_play_whole_runs_in_bounds_start_every_fight_and_replay`, `crates/run/src/tests.rs::a_lost_fight_ends_the_run` |
| D5 | col at or past its row's width | Unreachable: `Go` refuses it ("row {row} has no node {col}") | — | — |
| D6 | `Dead`, HP 0 | Reachable: a lost fight | "Any other end drops HP to zero and ends the run" (§15.6) | `crates/run/src/walk_tests.rs::walks_play_whole_runs_in_bounds_start_every_fight_and_replay` (`WALK_LOSS_PERCENT` of its fights are lost), `crates/run/src/tests.rs::a_lost_fight_ends_the_run` |
| D7 | `Dead`, HP above 0, row set | Reachable: `Abandon` in a fight (or on a tile) | Ends lost, scored, refusing every action (§15.8). The HP it keeps is **unspecified**: §15.6 drops HP to zero at "any other end" of a fight, §15.8 does not say what abandoning a fight leaves, and the run keeps the HP it held, where `walk_tests.rs`'s `check_state` holds a dead run to HP 0 (the walk never abandons) | `crates/run/src/tests.rs::abandoning_in_a_fight_ends_the_run_where_it_stands_with_its_hp_left` (new) |
| D8 | alive with HP below 1 or above the max | Unreachable: HP reaches 0 only with `Dead`; a win refills it (R3584000); `with_max_hp` keeps it at the max as the max moves, and an event that would take the max below 1 is refused (R3584001), so outside `Dead` HP is always the max (R3584001: "always the max outside a fight"; a fight's own health lives in its game until the report) | — | the walk's `check_state` after every step; `walk_hp_zero_while_alive_fails` and `walk_hp_past_the_max_fails` prove the check |
| D9 | `Won`, row set, HP the max | Reachable: play to the end | Extract and bank, then the run refuses every action (§15.7, §15.8) | `crates/run/src/tests.rs::a_run_played_to_the_end_wins_and_its_file_folds_back_to_the_same_state` (also its JSON round trip) |

D3 and D5 are **representable when serialized**: a hand-written `RunState` could hold them, and
`fight_won`'s `expect("a fight stands on a node")` would panic on D3's `Fight`, `run_node` on D5. The
device never keeps a `RunState`, only the log it folds, so no run file reaches them.

## Not states (versions)

Each reads an older record; nothing is removed.

- `RunAction`'s `heal` alias, a campfire rest as the device kept it before #3644:
  `crates/run/src/tests.rs::an_old_heal_rest_reads_as_gain_max_hp_and_folds`.
- `FightOver`'s report fields (`damage_dealt`, `healing_done`, `cards_drawn`, `turns`) and
  `RunState.max_hp_bonus` are `serde(default)`: a log kept before the report reads as a report of nothing
  and still folds, and a state written before the bonus reads with none (the device keeps a `RunFile`'s
  log, never a `RunState`):
  `crates/run/src/tests.rs::a_fight_over_kept_before_its_report_and_a_state_before_its_max_hp_bonus_still_read`
  (new).
- The engine's two thread-locals (the shipped catalog a replay folds under, R767000; whether the step in
  flight holds its random pools, R2201001), each set for one call and put back after it (CLAUDE.md
  rule 4).

## For a person

- **The fix (S10).** `MatchActor::attach` now closes a socket handed to a stopped actor with 1001, as
  `DuosActor::attach` did; its test failed before it. Treating S10 as a mismatch rests on §9.5 and
  R776000 ("the next socket … rebuild"), the README's drain line and the co-op actor's parity; if it reads
  as a decision instead, it needs a ruling in a pull request of its own.
- **Unspecified:** D7 (an abandoned run keeps its HP, while a lost fight drops it to 0). S12 (two tabs of
  one account on one seat replace each other on every reconnect) was decided in R3767000.
- **Not shown:** E5′ (mulligans open with work owed).
- **Representable when serialized:** E7 to E10 (`GameState`), D3 and D5 (`RunState`).

### Not mapped yet

- The web's session and live game (`apps/web/src/net/session.ts`, `liveGame.ts`).
- `Game.tsx`'s layers: a drag, an animation and a prompt at once.
