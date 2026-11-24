# Implicit states

Some of the code's states have no name: several fields read together (an `Option`, a flag, a counter, a
"pending" field beside what it guards) hold them, where one enum would. This file maps them: for each, its
fields, every combination of their values, what moves it between them, and whether each combination can
happen. It was started by #3678. The code audit's fourth part (`.github/hygiene/code.md` section 4, #3719)
extends it on each run: #3899's mapped states 4 to 6 and checked states 1 to 3 again; #4114's mapped states 7 to 9 and checked 1 to 6 again; #4291's mapped states 10 to 12 and checked 1, 2, 6 and 8 again; #4550's mapped states 13 to 15 and checked 1 to 3, 6 to 8 and 10 to 12 again.

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

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `4bc32f259f` (#4529) has `attach` take no seat for a socket already closed when it reaches the seat, and start
the grace for one that drops while it is being seated (S15, S16); `0cbf13009a` (#4474) has both actors keep each
seat's view until the game next changes, which moves no row. #4550's pull request changes `duel.rs`'s clock run
(state 13's T4), not `connected` or `stopped`. Checked before at `ae977ece3e`, plus #4291's pull request. Since `b0e998a29b` (`git log b0e998a29b.. -- <its files>`,
`87bd279557` being #4114's own pull request): `14e3fd964e` (#4239) queues a checkpoint after every
`MATCH_CHECKPOINT_MIN_ACTIONS`-th logged row (state 10), `17e34d0815` (#4169) gives the co-op actor its
checkpoint, `094e51ea83` (#4128, R4013000) makes a reused 1v1 nonce its own seat's duplicate, and
`f3f2d6b6bc` (#4212) logs an upgrade that carries its token in the address; none touches the seat's socket,
`hello_pending` or `socket_generation`, so no row moved. Checked before at `b0e998a29b`, plus #4114's pull
request. Since `656a2238acf5` (`git log 656a2238acf5.. -- <its
files>`): `9889adcf9b` (#3896, R3731002) gives a 2v2's team B people the seats b1 and b2 in the co-op actor
(S13, S14), and `9cea15006d` (#3826) has `ws_server.rs` report the upgrade's route health, which moves no
row. Checked before at `656a2238acf5`, plus #3899's pull request; since #3678 (`git log d0f7305317.. -- <its files>`):
#3742 (R3694000) closes a finished match's sockets with `MATCH_OVER_CLOSE_CODE` once its rematch window
ends (S9), #3783 (R3767000) closes a replaced seat socket with `SEAT_TAKEN_CLOSE_CODE` (S12), and #3899
gives a socket handed to an actor the reaper let go the same 4411 (S10).

**Files:** `crates/server/src/actor/match_actor.rs` (`Core`, `attach`, `on_frame`, `on_socket_gone`,
`detach`, `halt`), `crates/server/src/duos/actor.rs` (the same for a co-op person's seat, and in a 2v2 for team B's people's),
`crates/shared/src/host/duel.rs` and `crates/shared/src/host/duos.rs` (the machine's `stopped`,
`voided`, `connected`), `crates/server/src/actor/guard.rs` (`SeatGuard`'s last answered hello),
`crates/server/src/actor/ws_server.rs` (`Socket`).

**Fields,** per home seat (the seat the account began in; a co-op person's seat a1 or a2, and in a 2v2 b1 or b2):

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
taken and closed 1001, `reap_finished`'s with `MATCH_OVER_CLOSE_CODE` (R3694000), `stopped` set, and the
close kept as `halted_with` for a socket handed to the actor later); the void (R679); a rebuild or a restart from a checkpoint (a new
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
| S10 | stopped, then a socket attached to it | Reachable: a handle held across the stop (in production `Registry::attach` holds one between `actor_for` and `attach`, and `stop`, `stop_all` or `reap_finished` can run between the two on another of `#[tokio::main]`'s worker threads) | Closed and never greeted, with the close the stop gave the sockets it held: 1001 after `stop`, `stop_all` or R776000's let-go, so the client reconnects to the rebuilt actor (§9.5, R776000, the README's drain line); `MATCH_OVER_CLOSE_CODE` after `reap_finished`, which no client reconnects on, since a 1001 would send it into R1285000's refusal again and again (R3694000, its Rejected) | #3678 fixed the 1v1 actor greeting such a socket. **#3899: was a mismatch, fixed**: both actors closed it 1001 after the reaper too, so its client reconnected into R1285000's refusal. `halt_with` now keeps its close (`Core.halted_with`, set under the lock that stops the machine) and `attach` gives it to the late socket. `crates/server/tests/actor/match_actor.rs::seat_states_3678::a_socket_handed_to_a_stopped_actor_is_closed_going_away_and_never_greeted`, `…::seat_states_3678::r3694000_a_socket_handed_to_an_actor_the_reaper_let_go_hears_the_match_is_over` (new; failed before the fix with `left: Some(1001) right: Some(4411)`), `crates/server/tests/actor/duos.rs::seat_states_3678::a_socket_handed_to_a_stopped_duos_actor_is_closed_going_away`, `…::seat_states_3678::r3694000_a_socket_handed_to_a_coop_actor_the_reaper_let_go_hears_the_match_is_over` (new; failed the same way) |
| S11 | voided (and so stopped), then a socket attached to it | Reachable: a handle held across Glitch's void | The socket hears only the void's close, `MATCH_VOIDED_CLOSE_CODE`, never a view (R679); the voided arm goes before the stopped one | `crates/server/tests/actor/glitch.rs::glitchs_void::a_socket_handed_to_the_actor_it_held_across_the_void_hears_only_the_void` (new) |
| S12 | a second socket for one home seat while the first is open | Reachable: a second tab, or another device (a reconnect before the old socket closed is the same, and the client's own replacement of a stale socket lets it go first, so its close is never heard) | The newer socket takes the seat, the older is closed with `SEAT_TAKEN_CLOSE_CODE` (4412), and no grace starts since the seat was never empty (R744; `on_socket_gone`'s comment). The replaced client goes to `elsewhere` (`apps/web/src/game/net.ts`'s and `duos/socket.ts`'s `onclose`): no backoff, no reconnect, and its page says the game is open in another tab or on another device with a Play here button that takes the seat back (R3767000). A watcher's replacement stays 1000 | `crates/server/tests/actor/match_actor.rs::seat_states_3678::r3767000_a_second_socket_for_one_seat_replaces_the_first_with_seat_taken_and_starts_no_grace`, `crates/server/tests/actor/duos.rs::seat_states_3678::r3767000_a_second_socket_for_one_duos_seat_replaces_the_first_with_seat_taken` (new); the web's, `apps/web/src/game/net.test.ts`, `apps/web/src/duos/socket.test.ts`, `apps/web/src/routes/match.test.tsx` and `apps/web/src/duos/DuosRoute.test.tsx`, each under "R3767000" |
| S13 | (co-op) a socket for an AI team's seat, b1 or b2 | Unreachable from the routes: `DuosActor::seat_of` (`DuosMatch::seat_of`) answers none for an empty profile, and an AI team's b1 and b2 hold `""` (`duos/store.rs`; `duos/rooms.rs` names them only for a 2v2), so it answers a1 or a2 only. A held handle reaches `attach`'s guard (`DuosTeamB::person`: none for an AI team's seat) | Team B's seats are the AI's (§13.7) and only a human seat's socket is greeted (§13); the guard closes it 1000 and greets nothing | `crates/server/tests/actor/duos.rs::seat_states_3678::a_socket_handed_to_an_ai_seat_is_closed_and_never_greeted` (new) |
| S14 | (2v2) a socket for a team-B person's seat, b1 or b2 | Reachable: a four-account private lobby's 2v2 (R3731002); `seat_of` answers b1 or b2, and S1 to S12 hold for it as for a1 and a2 (the same per-slot arrays) | Greeted with its own seat's view; a drop starts its grace, the AI plays the seat past it, and its return takes it back (R3731002, §13.7) | `crates/server/tests/actor/duos.rs::two_v_two::r3731002_four_sockets_play_a_two_v_two_to_its_end`, `…::two_v_two::r3731002_a_team_b_disconnect_hands_the_seat_to_the_ai_and_its_return_takes_it_back` |
| S15 | a socket already closed when it reaches `attach` | Reachable: the transport closes while the registry finds or rebuilds the actor, before `attach` runs (the tests hand `attach` a socket already closed) | It takes no seat: the seat keeps what it held, no other socket is closed (no 4412), and a running grace keeps running, or an empty seat's absent grace starts as the other seat attaches (`crates/server/README.md`'s attach paragraph; R744, R3767000) | `crates/server/tests/actor/match_actor.rs::seat_states_3678::a_dropped_socket_for_a_seat_with_a_running_grace_leaves_the_grace_running`, `…::seat_states_3678::a_dropped_socket_rebuilt_after_a_restart_takes_no_seat_so_the_other_seats_attach_starts_its_absent_grace`, `…::seat_states_3678::a_dropped_second_socket_for_a_held_seat_closes_nothing_and_the_open_socket_keeps_its_views`, `crates/server/tests/actor/duos.rs::seat_states_3678::a_dropped_socket_takes_no_coop_seat_so_the_grace_runs_and_the_ai_takes_over` |
| S16 | a socket that closes after `attach`'s open check but before its close handler is in, while another socket held the seat | Reachable only as a race: the transport closes between the check and `socket.attach`; no test can order it | **Unspecified**: the new socket takes the seat, so the held one is closed with `SEAT_TAKEN_CLOSE_CODE` and its tab reads `elsewhere`, while the second open check runs `on_socket_gone` and the seat's grace starts with no socket on it (the README's attach paragraph covers only a socket closed before it is seated) | — |

## 2. The engine's paused action

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `65538df6ca` (#4404, R3925000) brings the loop cap: past `RESOLUTION_EVENT_CAP`
events in one interaction card text is off until the next, while owed work still drains (`loop_cap.rs`'s header).
Its count lives on the sink's `Frontier`, never in state, so it adds no field here; E6 and E8 cite it, and E11 and E12
are new. The other ten (`5c99ba4501`, `3ba8d95152`, `77e6ef9230`, `07683ea0a1`, `7c0760e0f5`, `0cd6b9dfb3`,
`a1bc49198d`, `99bf500496`, `d5f68be2ec`, `6b9d92dea2`) touch Dormant cards, Quests, mana, the attack preview and
the testkit's monitor, fields outside this state. Checked before at `ae977ece3e`, plus #4291's pull request. Since `b0e998a29b`: `80a4d31165` (#4057) runs I7 on I14's
copies too (E8); `e73836296f` (#4111, I18), `afb994522b` (#4143, I8), `087a371898` (#4254) and `f034bfcd6c`
(#4255) add or widen other testkit invariants; `70985031f0` (#4122) and `978ccfceda` (#4265) follow renamed
ids through board history, `69a14a320d` (#4194) adds a hero's Burn and `79474b2d38` (#4234) a card's
`unpreventable_destroy`, fields outside this state. #4291's pull request reads every card through one
`every_instance` and the hero ids through one parser, in the same order. None touches `pending`, `work`,
`work_cursor`, `mulligan` or `result`, so no row moved. Checked before at `b0e998a29b`. Since `656a2238acf5`: `72dc211abd` (#3811) has `end_game` also clear each player's
`turn_log.played_before_end` (R3800003), a field outside this state, and `6b457de6f4` (#3859, I16),
`d7851b0b47` (#3938, I17), `7767d9815a` (#3924), `3292f2ea70` (#3973) and `b8093a6f3f` (#4025, I9) add or
refactor testkit invariants; I7 and the state's fields are unchanged, so no row moved. Checked before at
`656a2238acf5`; since #3678 (`git log d0f7305317.. -- <its files>`): `@setResolved` (R3806000,
#3817), the chaos launch watch's resume step (R3570004, #3786), `sink_bottom_starters` in setup
(R3558006, #3786) and the testkit's invariants I16 (#3746) and I17 (#3799); none touches `pending`, `work`,
`work_cursor`, `mulligan` or `result`, and I7 is unchanged, so no row moved.

**Files:** `crates/engine/src/work.rs` (read its header first), `crates/engine/src/state.rs` (`pending`,
`work`, `work_cursor`, `mulligan`, `result`), `crates/engine/src/setup.rs` (`open_mulligans`,
`resolve_mulligans`), `crates/engine/src/game_over.rs` (`end_game`),
`crates/engine/src/subsystems/ai_policy.rs` (an owed AI turn), `crates/engine/src/testkit/invariants.rs`
(I7).

**Fields:** `pending` (the one open prompt, §10.1), `work` (the owed continuations, §9.3) and
`work_cursor` (where the running cascade parks next, R113), read with `result` (the game is over, §2.5)
and `mulligan` (the two sealed mulligans, open beside `pending`, never in it, R265).

**What moves it:** a prompt opening (`pending` set) and the scope that asked parking its rest
(`push_work`, `owe`, `park_work`: inserted at `min(cursor, len)`, the cursor set past it; a Spell set
under Paranoia owes its `@setResolved`, R3806000); `take_work`
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
| E6 | over, no prompt, no mulligans, `work` as it stood | Reachable both ways. Empty: Core #68's Cry kills the enemy hero. Owed: p2 concedes (R211) while p1's Core #7 Discover is open, so the play's rest stays owed | Nothing happens after the game is over, every action refused (§2.5, R216); a game over keeps its debt (testkit invariant I7's note: "a game over keeps its debt") | `crates/cards/tests/cross/paused_sequences.rs::paused_action_states_3678::e6_e7_a_game_won_inside_a_cry_closes_every_prompt`, `…::paused_action_states_3678::e6_e7_a_concession_under_an_open_prompt_closes_it_and_leaves_the_rest_of_the_play_owed` (new), `crates/engine/tests/rules/loop_cap.rs::r3925000_a_hero_killed_in_the_capped_resolution_still_ends_the_game` (a hero killed in a capped resolution) |
| E7 | over with a prompt open, or with the mulligans open | Unreachable: `end_game` clears `pending` and `mulligan`, `work::paused` stops every scope once `result` is set, and `reduce` refuses every action after (R216) | — | the two E6 tests assert both closed and every action refused |
| E8 | live, no prompt, no mulligans, `work` owed or cursor ≠ 0 | Unreachable: an action ends by draining `work` (§10.3, `drain_work`) until it is empty or a scope pauses on a prompt or a result, and taking each item resets the cursor. I7 flags it after every action of CI's `cargo jackioh fuzz --seeds 200`, and the 400-game probe never met it. The I7 tests in `invariants.rs` that show it flagged set the fields by hand, so they prove the check, not reachability. Since #4113 (#4057) I7 also runs on the copy I14 reduces each listed action on (`branch_state_findings`, `orphaned_work_findings`); its test there plants the work in the branch's result (`crates/cards/tests/cross/invariants.rs::i14_a_listed_action_reduce_refuses::i14_branch_reports_orphaned_work_on_accepted_branch_action`), which again proves the check, not reachability. A capped interaction (R3925000) still drains what is owed, its card text off, so it too ends with nothing owed: `crates/engine/tests/rules/loop_cap.rs::r3925000_a_self_recursive_death_loop_stops_at_the_cap` | Orphaned work (I7) | — |
| E9 | cursor above the length | Unreachable: `place` inserts at `min(cursor, len)` and sets the cursor one past it, `take_work` and `begin_work_cascade` set 0, `drop_work` clamps to the new length, and an owed AI turn (`ai_policy`) resets it to 0 before it appends what was owed | — | — |
| E10 | mulligans open and a prompt open | Unreachable: `open_mulligans` does nothing while `pending` is set, and `resolve_mulligans` clears `mulligan` before the answers it resolves can ask (R224, R265) | — | — |
| E11 | live, `work` owed, the loop cap tripped in the interaction | Reachable: p1's End turn parks one `END_OF_TURN_WORK` behind a question its end-of-turn chain asks, and the answer's chain trips the cap (the test's second part; its first trips the cap inside End turn itself) | The owed steps still drain with card text off, so the engine's own step finishes: the turn ends and the next starts, and nothing is left owed (R3925000; `loop_cap.rs`'s header: "a turn's end") | `crates/engine/tests/rules/loop_cap.rs::r3925000_a_turn_end_capped_mid_way_still_ends_the_turn` (new), `crates/engine/tests/rules/pauses.rs::the_work_queue_9_3_r68::r3925000_a_sequence_that_keeps_owing_work_stops_at_the_cap` |
| E12 | a prompt open when the interaction would reach the cap | Reachable: a question opens before the count reaches `RESOLUTION_EVENT_CAP` | A pause, not a trip: the answer is the next interaction and its count starts over (R3925000) | `crates/engine/tests/rules/loop_cap.rs::r3925000_the_count_starts_over_after_each_answer` |

E7 to E10 are **representable when serialized**: a `GameState` read back from JSON (a checkpoint, a replay
fixture, the wire's state for the tools) can hold them, and one enum for the paused action would rule them
out. No shape is changed here.

## 3. The dungeon run's position

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `f1ddc391cf` (#4360) moves the walk's state checks into
`crates/run/src/invariants.rs`'s `check_run_invariants` and adds a random-policy map fuzz with illegal probes; it
moves no row (D5, D7 and D8 cite it). Checked before at `ae977ece3e`, plus #4291's pull request: unchanged since `b0e998a29b` but for #4114's own pull request
(`87bd279557`) and #4291's, which reads the store's string lists with the shared `isStringArray`, the same
test. Checked before at `b0e998a29b`, plus #4114's pull request. Since `656a2238acf5`, `a040a62718` (#4097) has
`run_create` refuse a curse taken twice and a mini-boss's replacement drop roll at its tier floor, which made
`fight_won`'s two roll arms one (#4114 merges them); neither touches `row`, `col`, `phase` or HP, so no row
moved. Checked before at `13bd66f47171`, plus #3678's pull request; unchanged at `656a2238acf5` (`git log
d0f7305317.. -- crates/run/src/lib.rs apps/web/src/dungeon/store.ts` prints nothing).

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
| D5 | col at or past its row's width | Unreachable: `Go` refuses it ("row {row} has no node {col}") | — | `crates/run/src/map_fuzz_tests.rs::map_fuzz_random_policy_with_illegal_probes_stays_in_bounds_and_replays` (its map probe steps past the row's width when every node of the row is reachable, and is refused with the state left as it was) |
| D6 | `Dead`, HP 0 | Reachable: a lost fight | "Any other end drops HP to zero and ends the run" (§15.6) | `crates/run/src/walk_tests.rs::walks_play_whole_runs_in_bounds_start_every_fight_and_replay` (`WALK_LOSS_PERCENT` of its fights are lost), `crates/run/src/tests.rs::a_lost_fight_ends_the_run` |
| D7 | `Dead`, HP above 0, row set | Reachable: `Abandon` in a fight (or on a tile) | Ends lost, scored, refusing every action (§15.8). The HP it keeps is **unspecified**: §15.6 drops HP to zero at "any other end" of a fight, §15.8 does not say what abandoning a fight leaves, and the run keeps the HP it held, where `crates/run/src/invariants.rs`'s `check_run_invariants` holds a dead run to HP 0 (the walks never abandon) | `crates/run/src/tests.rs::abandoning_in_a_fight_ends_the_run_where_it_stands_with_its_hp_left` (new) |
| D8 | alive with HP below 1 or above the max | Unreachable: HP reaches 0 only with `Dead`; a win refills it (R3584000); `with_max_hp` keeps it at the max as the max moves, and an event that would take the max below 1 is refused (R3584001), so outside `Dead` HP is always the max (R3584001: "always the max outside a fight"; a fight's own health lives in its game until the report) | — | `crates/run/src/invariants.rs`'s `check_run_invariants` after every step of the walks and the map fuzz; `walk_hp_zero_while_alive_fails` and `walk_hp_past_the_max_fails` prove the check |
| D9 | `Won`, row set, HP the max | Reachable: play to the end | Extract and bank, then the run refuses every action (§15.7, §15.8) | `crates/run/src/tests.rs::a_run_played_to_the_end_wins_and_its_file_folds_back_to_the_same_state` (also its JSON round trip) |

D3 and D5 are **representable when serialized**: a hand-written `RunState` could hold them, and
`fight_won`'s `expect("a fight stands on a node")` would panic on D3's `Fight`, `run_node` on D5. The
device never keeps a `RunState`, only the log it folds, so no run file reaches them.

## 4. The web's stored session

**Checked at** `0bf5057480`: unchanged since `ae977ece3e` but for #4291's own pull request (`bcdc70af1d`). Checked before at `ae977ece3e`: unchanged since `b0e998a29b` (`git log b0e998a29b.. -- <its files>` prints nothing).
Checked before at `b0e998a29b`: unchanged since `656a2238acf5`, plus #3899's pull request (`git log 656a2238acf5..
-- apps/web/src/net/session.ts apps/web/src/net/storage.ts` prints nothing).

**Files:** `apps/web/src/net/session.ts` (`readSession`, `writeSession`, `clearSession`),
`apps/web/src/net/storage.ts` (`sessionStorageOrNull`, `localStorageOrNull`: a storage that throws on
touch reads as none).

**Fields:** the tab's session T (`sessionStorage["jackioh.session"]`), the copy an older build left in
`localStorage` under the same key (S), the end-to-end fixtures' session E (`localStorage["jackioh.e2e.session"]`,
e2e/README.md A6), and whether the storage throws. A stored value that is not a session (bad JSON, an
empty or missing `accessToken`) reads as none.

**What moves it:** `readSession` (S moves into T and is deleted, unless T is there); `writeSession` (a
sign-in or a renewal: T written, S deleted); `clearSession` (a sign-out: T, S and E deleted, R2125000's
kept reads with them); a reload (T kept); a new tab or a browser closed and opened (no T); a duplicated
tab (a copy of its original's T); a tab still on an older build, which writes S. The pending sign-up and
reset addresses (R192, R193) are records of their own, with their own TTL, and not part of this state.

Its rows are W1 to W10 (W for the web's session), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| W1 | no T, no S, no E | Reachable: a new tab, or a sign-out | No session (R632) | `apps/web/src/net/session.test.ts` › "R632 a sign-out clears the tab's session, any copy in localStorage and the fixtures' session", › "R632 treats anything but a stored access token as no session" |
| W2 | T only | Reachable: a sign-in | T is the session; nothing is written to `localStorage` (R632) | › "R632 writes a sign-in to the tab's sessionStorage and nothing to localStorage" |
| W3 | S only | Reachable: the first read after a deploy, in a tab that has not signed in on this build | S moves into this tab and is deleted from `localStorage` (R632) | › "R632 moves a session an older build left in localStorage into this tab, and deletes it there" |
| W4 | T and S | Reachable: a tab still on an older build writes S beside a tab on this build | T wins and is left alone (R632); S stays until the next sign-in or renewal deletes it | › "R632 prefers the tab's own session to one in localStorage and leaves the tab's alone", › "R632 a sign-in or a renewal deletes the copy an older build left in localStorage" |
| W5 | E only | Reachable: an end-to-end spec seeds it | E is read, and nothing moves (R632: "read only when the tab has no session of its own") | › "R632 still reads the end-to-end fixtures' session from localStorage when the tab has none" |
| W6 | T and E | Reachable: W5, then a sign-in | T wins (R632) | the same test's second half |
| W7 | S and E, no T | Reachable: a spec's fixture beside an older build's copy | S moves in and wins, E stays (R632: S "moves into the first tab that reads it"; E is read only with no session of the tab's own) | › "R632 an older build's session moves into the tab ahead of the fixtures' session, which stays" (new) |
| W8 | the storage throws (a private window, blocked site data) | Reachable: either storage throws on touch, or on `getItem`, `setItem` or `removeItem` | No session; writing and clearing do nothing and never throw (R632; `session.ts`'s header) | › "R632 never throws on blocked storage: there is no session, and writing and clearing do nothing" |
| W9 | S, and the tab's storage refuses the move | Reachable: `setItem` throws (a quota) | S is still this read's session and is not deleted, so the next read can move it | › "R632 keeps reading an older build's session when the tab's storage refuses the move" |
| W10 | a duplicated tab's copy of T | Reachable in a browser only: the browser copies `sessionStorage`, which jsdom cannot | Starts with a copy of its original's session (R632); the module reads it as W2 | — (W2's test) |

## 5. The web's live game

**Checked at** `0bf5057480`, plus #4550's pull request: unchanged since `ae977ece3e` but for #4291's own pull request (`bcdc70af1d`); #4550's moves `/play`'s deck, trio and challenge picks onto `together/challengeChoice.ts`'s (the same outputs for every input), which moves no row. Checked before at `ae977ece3e`: unchanged since `b0e998a29b` but for #4114's own pull request (`87bd279557`). Checked
before at `b0e998a29b`. Since `656a2238acf5`, plus #3899's pull request: `88b1df6f05` (#3902, R3729001)
keeps the moment a tab queued beside its mode, for the menus' queue banner and its Cancel (L14 to L17).

**Files:** `apps/web/src/net/liveGame.ts` (`useQueueFollow`, `useLiveGame`, `rememberQueued`,
`forgetQueued`, `readQueuedSince`), `apps/web/src/routes/play.tsx` (the lobby's own watch, which `/play`
runs instead), `apps/web/src/net/gate.ts` (`callWithRenewal`), `apps/web/src/shell/QueueBanner.tsx` (the
menus' queue banner).

**Fields:** the queue follow's: the mode this tab is queued in (`queued`, `sessionStorage["jackioh.play.queued"]`)
and the moment it queued (`QUEUE_SINCE_KEY`, `sessionStorage["jackioh.play.queuedSince"]`, the banner's clock),
whether the path is `/play`, `found` (a pairing being announced), `endedReads` (R897000's reads in a row
that say the ticket is gone) and `reading` (a read out); the banner's (`useLiveGame`): the account read,
`polled` (the last poll's game, paired with the account read it followed) and `watched` (the game polled).

**What moves it:** `rememberQueued` and `forgetQueued` (`/play`'s Find a match and Leave queue; each writes
or removes the mode and the moment together); the queue banner's Cancel (`dequeue`, then `forgetQueued`; a
refused one keeps both), which also ends `/play`'s wait; a
navigation; a read every `SERIES_POLL_SECONDS` (the follow, R3207001) or `LIVE_GAME_POLL_SECONDS` (the
banner), and at once on `pageWake` (R3205000); a read naming a game (`found`, then a navigation after
`MATCH_FOUND_NAV_DELAY_MS`); a signed-out answer or no session; a reload (the queue is kept, the
counts start from nothing); a duplicated tab (a copy of the queue). Rows L follow a queue, rows B are
the banner's.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| L1 | not queued | Reachable: any tab that never queued | Nothing is read (R765) | `apps/web/src/net/liveGame.test.tsx` › "R765 a tab that is not queued reads nothing" |
| L2 | queued, on `/play` | Reachable: Find a match | The lobby watches for itself; the follow reads nothing (R765) | › "R765 on /play the lobby watches for itself, so the follower reads nothing there" |
| L3 | queued, away, nothing found, `endedReads` 0 | Reachable: queue, then leave `/play` | The account is read every `SERIES_POLL_SECONDS`, in a hidden tab too (R765, R3205000, R3207001) | › "R765 elsewhere it reads the account every SERIES_POLL_SECONDS, announces the pairing, then takes the player there", › "R3205000 the queue follow keeps reading in a hidden tab, for its reads keep the ticket alive (R897000)", › "R3207001 the queue follow keeps SERIES_POLL_SECONDS" |
| L4 | queued, away, `endedReads` 1 | Reachable: one read with `queued: false` | The wait goes on; a live read between two gone ones resets the count (R897000) | › "R897000 one false read, or none at all, keeps the wait" |
| L5 | `endedReads` reaches `QUEUE_ENDED_AFTER_READS` | Reachable: two gone reads in a row | The queue is forgotten and the follow stops (R897000) | › "R897000 a queue the server no longer names twice in a row is forgotten, and the follower stops" |
| L6 | `found` set | Reachable: a read naming a match or a series | "Match found!" for `MATCH_FOUND_NAV_DELAY_MS`, then the game (R765) | the L3 test, › "R765 a Conquest pairing goes to the series screen" |
| L7 | queued, a read naming only a co-op game | Reachable: the account is in a co-op game while queued | No pairing: neither announced nor followed (R1566000) | › "R1566000 a co-op game is no pairing: the queue follow neither announces it nor goes there" |
| L8 | queued, a read that fails | Reachable: the network drops | Ignored and read again (R897000: "a failed read … keeps the wait") | › "R765 a read that fails is ignored and retried; a tab with no session forgets the queue" |
| L9 | queued, no session | Reachable: the tab signed out elsewhere in it | **Unspecified** (R765 does not say); the follow forgets the queue | the same test's second half |
| L10 | a new wait with `endedReads` 1 left by the last | Reachable: one gone read away from `/play`; back on `/play`, leave the queue and join it again; leave `/play` | Each wait counts its own reads in a row (R897000; `/play`'s watch starts each wait from nothing, `play.tsx`) | **#3899: was a mismatch, fixed**: the count outlived its wait, so the new wait's first gone read ended it. The effect now starts each wait at 0. › "R897000 a new wait counts its own reads: a gone read the last wait left never ends it early" (new; failed before the fix: "the new wait's first gone read is its first: expected null to be 'bo1'") |
| L11 | a gone read, a failed read, a gone read | Reachable: as it says | **Unspecified** (R897000 says a failed read keeps the wait, not whether it breaks the row); as `/play`'s watch, it neither counts nor breaks it, so the third read ends the wait | › "R897000 a failed read between two gone reads does not break the row" (new) |
| L12 | queued, a read answered signed out (the renewal found no account) | Reachable: the refresh token is revoked | **Unspecified**; the follow forgets the queue, as with no session | › "R765 a read answered signed out forgets the queue" (new) |
| L13 | `found` and `queued` both set | Unreachable: `forgetQueued` runs just before `setFound`, in the same answer | — | — |
| L14 | queued, a moment kept | Reachable: Find a match | The banner's clock counts from the moment, and a reload keeps it (R3729001) | `apps/web/src/net/liveGame.test.tsx` › "R3729001 the moment the queue began is written with the mode, read back and cleared with it"; `apps/web/src/shell/MenuShell.test.tsx` › "R3729001 the clock counts from when the tab queued and advances every tick" |
| L15 | queued, no moment or a malformed one | Reachable: a tab that queued before #3902 (a version), or a storage that took the mode and refused the moment (`setStored` swallows the throw); malformed only by hand | Counts from when the banner mounted (R3729001) | › "R3729001 a missing or malformed moment reads as none"; `MenuShell.test.tsx` › "R3729001 with no moment kept the clock counts from when the banner first showed" |
| L16 | a moment, not queued | Unreachable through the machinery: `rememberQueued` writes the mode first and `forgetQueued` removes both, so only a storage that refuses the short mode and takes the longer moment, or a hand-set key, leaves one; nothing reads it then (`MenuShell` mounts the banner only while a queue is kept) and the next queue overwrites it | — | — |
| L17 | queued, the banner's Cancel | Reachable: Cancel on any menu screen, `/play` included | The queue is left on the server, then forgotten, and `/play`'s wait ends; a refused Cancel keeps the search (R3729001) | `MenuShell.test.tsx` › "R3728002 R3729001 the queue search shows on every menu screen, /play included, and Cancel dequeues and forgets it", › "R3729001 a mode the server has counted no wait for shows no wait, and a refused Cancel keeps the search on screen"; `apps/web/src/routes/play.test.tsx` › "R3729001 inside the menus' shell the banner shows the search, and Cancel there ends the wait here" |
| B1 | the account not ready | Reachable: loading, anonymous, an error | No banner, no poll (R765) | › "R765 useLiveGame is the account's game, and none for an account that is not ready" |
| B2 | a live game, polled | Reachable: an account read naming a game | Read again every `LIVE_GAME_POLL_SECONDS`, gone once a read says it ended; a co-op game too (R765, R3207001, R1566000) | › "R765 a live game is read again every LIVE_GAME_POLL_SECONDS, and is gone once a read says it ended", › "R1566000 useLiveGame offers a live co-op game, …", › "R3205000 the live-game banner's read pauses in a hidden tab, …" |
| B3 | no live game | Reachable: an account read naming none | Nothing is polled (R765) | › "R765 an account with no live game is not polled" |
| B4 | a poll's answer, then a fresh account read | Reachable: the poll says the game ended, then the account is read again (a sign-in, a renewal) | The fresh account read is shown, never the older poll's (R765; `polled.after`) | › "R765 a fresh account read wins over the last poll's answer" (new) |

## 6. `Game.tsx`'s layers

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `65538df6ca` (#4404) adds `loopCapped` to the runner's bursts,
`6b9d92dea2` (#4307) changes the attack preview's `overflowOnto`, and `4224de56cd` (#4343) rewrites the runner's
internals under the same tests; none touches the selection, the drag or the prompt's fade, so no row moved.
Checked before at `ae977ece3e`. Since `b0e998a29b`: `70985031f0` (#4079, R3864000) has a click on a lit unit a play
tributes, once the play asks only its zone, answer the zone (`onClickTarget`); it is one click of a build,
with no layer beside it, so no row moved. Checked before at `b0e998a29b`. Since `656a2238acf5`, plus #3899's pull request: `ff38380a54` (#3976) and
`e3dbd4c61a` (#3839) pass the heroes their portraits and the prompt a sequence clock, `1b3c343975` (#3910)
words the turn banner for a cut-short turn and the autopilot, `3643877e46` (#3821) words the notice when no
attack is offered, and `fa6d2f2f88` (#3884) adds an animated `armorGained`, one more event a burst may hold
(G2, G3); none moves a row.

**Files:** `apps/web/src/game/Game.tsx` (`interaction`, `selectionKey`, the runner with `shown` against
`view`, `legal`, `lastSent`, `promptOver`), `apps/web/src/game/drag/DragLayer.tsx` (`press`, `flight`,
`landed`, `returning`), `apps/web/src/game/actions.ts` (`highlightFor`, `onClickTarget`, `boardKeyOf`),
`apps/web/src/game/promptOver.ts`, `apps/web/src/game/animations.ts` (the runner).

**Fields:** the selection (`interaction`: idle, an attacker, a play or activation being built) and the
board it was built on (`selectionKey`); the drag layer's press, flight (with the board it was lifted on,
`liftedOn`) and landed card; the runner's burst, with `shown` older than `view` while it plays, and
`legal`, which is empty until the board shows the newest view; the prompt (`shown.pending`, and
`promptOver` while an answered one is behind the entry in flight); `lastSent` (R3060000's last intent and
the view it was sent against); the motion and drag-to-play settings.

**What moves it:** a click; a press, a move past `DRAG_THRESHOLD_PX`, a release, Escape, a right
button, a lost pointer or a window blur; a new view (the same turn, another turn, seat or viewer); the
runner's entries finishing, a drain (a hotseat hand-over, a hidden tab, Reduce motion) or a skip (a set
piece, the opening deal); a prompt opening or answered; a setting changed. Its rows are G1 to G24, never
`R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| G1 | settled: `shown` is `view`, nothing selected | Reachable: any view with nothing to animate | Its moves at once (`Game.tsx`'s header: the moves offered are the newest view's) | `apps/web/src/game/Game.test.tsx` › "offers the moves at once when nothing is animating" |
| G2 | a burst holds `view` back | Reachable: a view whose window adds an animated event | No move of the newest view until the board shows it (the same header) | › "does not enable End turn for a view the runner is still holding back, and does once it drains" |
| G3 | a second view while a burst plays | Reachable: two views in quick succession | Its events queue behind; a copy of the newest view (a resend) adds nothing | › "layer states 3899" › "a second burst queues behind the first and a fresh copy of the newest view adds nothing" (new) |
| G4 | a click selection, then a fresh view of the same turn | Reachable: the clock or the other seat's prompt updates the view | **Unspecified** (#2924 keys the drop on turn, seat and viewer; no spec section says); kept | › "a half-built play or attack is dropped at the turn change (#2924)" › "keeps the selected attacker through view updates within the same turn" |
| G5 | a selection, then a view of another turn | Reachable: the turn clock ends the turn mid-choice | Dropped, as Escape drops it (#2924, `Game.tsx`'s comment, §10.10) | › "drops the selected attacker when the turn changes and keeps it dropped next turn", › "closes a play's own picker when the view moves to the opponent's turn" |
| G5b | a selection, then a burst that ends on another turn | Reachable: the opponent's turn animates up to the viewer's next | Lit nothing while the burst plays (`legal` is empty); back after a same-turn burst; dropped once the board shows the new turn (#2924, keyed on `shown`) | › "layer states 3899" › "an animated turn change keeps a selection through the burst and drops it once the board shows the turn" (new) |
| G6 | a flight, then a fresh view of the same turn | Reachable: as G4, mid-drag | The drag goes on and its drop is the click (R658) | `apps/web/src/test/ux/drag-layer.test.tsx` › "layer states 3899" › "a fresh view of the same turn mid-drag keeps the drag and its release attacks" (new) |
| G7 | a flight, then a view of another turn, seat or viewer | Reachable: the turn clock ends the turn mid-drag; a hotseat hand-over | Its release sends nothing: Game dropped the build (#2924), and the drop is the click on what the board lights now (R658) | **#3899: was a mismatch, fixed**: the release sent the attack it was lifted with. The flight keeps `boardKeyOf` the board it was lifted on, and a release on another board ends the drag. › "a turn that ends mid-drag sends nothing on its release and leaves the board idle" (new; failed before the fix: `onAction` called with the attack) |
| G8 | a flight, then a burst of the same turn | Reachable: lift as the previous action's view arrives | **Unspecified**: the drop reads the targets it was lifted with and sends the attack; a click then lands on nothing, as the board lights nothing while `legal` is held back. R658 says "drag stays two clicks", so this may be a mismatch; changing it would refuse every drop let go while a burst plays | › "a drag released while a burst holds the board still sends its attack, where a click then sends nothing" (new) |
| G9 | a build lifted again from its picks (R658), then another turn; released on nothing, or Escape | Reachable: place a card whose Cry still wants a target, lift the zone, the turn ends | Stays dropped (#2924); a release on nothing or Escape goes back to the build (R658) only on the board it was built on | **#3899: was a mismatch, fixed**: both brought the stale build back, its target picker open, and a click then sent the play. `apps/web/src/test/ux/drag-continue.test.tsx` › "layer states 3899" › "a build lifted again and released after the turn ended stays dropped", › "… and Escape after the turn ended stays dropped" (new; failed before the fix: the `prompt-modal` "Choose a target" was back) |
| G10 | a press while a burst holds the board | Reachable: as it says | Lifts nothing (nothing is lit, `legal` is empty) until the board catches up (R658: a drag lifts what `highlightFor` lights) | `drag-layer.test.tsx` › "layer states 3899" › "a press while a burst holds the board lifts nothing, and lifts once it catches up" (new) |
| G11 | a hand press under an open prompt | Reachable: the engine asks (its `legal` lists only the answers and concede then, R211) | Lifts nothing | › "a hand card pressed under an open prompt lifts nothing" (new) |
| G12 | a prompt that arrives inside a burst | Reachable: an answer's view that asks again, with events | Opens once the board shows it; one tap answers once (R3060000) | `Game.test.tsx` › "layer states 3899" › "a prompt inside a burst opens once the board shows it and one tap answers once" (new) |
| G13 | a second tap while the answered prompt fades (`promptAnswered` in flight) | Reachable: tap, the answer's view arrives, tap again within the fade | **Unspecified**: R3060000 drops a repeat against the view it was sent against, and the answer's own view is newer, so the second tap sends the answer again (the server refuses it as stale) | › "a second tap while the answered prompt fades sends the answer again" (new) |
| G14 | an answered prompt behind the entry in flight | Reachable: the mulligan, then the first turn's banner | Taken down for the turn's banner, which names the turn it announces (#37, `promptOver.ts`) | `apps/web/src/game/turn-start-bleed.test.tsx` › "#37 an answered mulligan does not come back over the first turn", "#37 promptOver" |
| G15 | a flight, then a prompt opens in the same turn | Reachable: the other seat's trap asks the viewer mid-drag | **Unspecified**: the selection is kept (G4), so the drop sends the attack the board lit, as a click does then; the engine refuses it under its prompt | `drag-layer.test.tsx` › "a drag released after a prompt opened in the same turn sends its attack, as a click does then" (new) |
| G16 | Reduce motion on | Reachable: the setting, or the OS preference | Each view shown at once; turning it on mid-burst shows the newest view (B41) | `apps/web/src/test/ux/settings.test.tsx` › "B41 reduce motion drains the animation queue at once, as the OS preference does"; `Game.test.tsx` › "a runner rebuilt for Reduce motion mid-burst replays nothing when motion comes back" |
| G17 | a hidden tab with a burst | Reachable: switch tabs during the opponent's turn | The newest view at once, with its moves (R3497000) | `apps/web/src/game/Game.hiddenDrain.test.tsx` › "R3497000 a hidden online board drains its animation queue" |
| G18 | a set piece holding the board | Reachable: a Mythic's takeover | A tap skips it and reaches no control (R2785000) | `apps/web/src/game/SetPieceBoard.test.tsx` › "R2785000 a set piece on the board" |
| G19 | the opening deal | Reachable: a game's first view, in the mulligan | A tap ends it and goes on to what it was on (#3347) | `apps/web/src/game/Game.opening.test.tsx` › "#3347 the opening deal" |
| G20 | a hotseat hand-over while a burst plays | Reachable: the hotseat's next seat takes the board mid-burst | The burst is drained and nothing replays: views of two seats are not comparable, and replaying the window would show the arriving seat the other's history (`Game.tsx`'s comment; CLAUDE.md rule 7) | `Game.test.tsx` › "layer states 3899" › "a hotseat hand-over mid-burst drains it and replays nothing" (new) |
| G21 | a reconnect's view while a burst plays | Reachable: the socket drops and comes back mid-burst | **Unspecified**: the runner's `drain()` doc names a reconnect, but nothing calls it then; the view is any view, its new events queue (G3) and one already played adds nothing | G3's test (its resend) |
| G22 | the emote menu open, then a selection | Reachable: open the portrait's menu, pick up an attacker | The menu closes: targeting wins (R643) | `apps/web/src/game/emotes.test.tsx` › "R643 selecting an attacker shuts an open menu — targeting wins" |
| G23 | a flight on a replay board | Unreachable: a replay board mounts no drag layer (`replay ? null : <DragLayer …/>`) | — | — |
| G24 | a flight with drag to play off | Unreachable: a press does not arm while the setting is off | — | `drag-layer.test.tsx` › "B39 with drag to play off, a long move starts no drag and a release over a zone sends nothing" |

## 7. The web's match socket

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `c4bc8bd2a8` (#4355, R4250000) starts the backoff over on the
first `view`, not on `onopen`, in both clients (N3, N17, N18). Checked before at `ae977ece3e`: unchanged since `b0e998a29b` but for #4114's own pull request (`87bd279557`). Checked
before at `b0e998a29b`, plus #4114's pull request.

**Files:** `apps/web/src/game/net.ts` (`createMatchClient`, `createBackoff`, `createWakeProbe`, `letGo`),
`apps/web/src/duos/socket.ts` (`createDuosClient`, the same machine for a co-op seat).

**Fields:** the snapshot's `connection` (`connecting`, `open`, `reconnecting`, `refused`, `closed`,
`elsewhere`); the `socket` the client holds (none, or one connecting or open); `stopped` (a deliberate
`close()`); the backoff's `pending` step and its `retry` count; `awaitingFirstView` (a new socket's first
view clears an older error); `refusedToken` (the token a 4401 refused, R1115000); and the wake probe's
timer (R1441).

**What moves it:** `connect()` (the page's mount and its Reconnect and Play here buttons: `stopped`
cleared, a waiting step cut short, a socket opened); `close()` (the page's unmount); `setToken` (the gate's
renewal, R194); a socket's `onopen` (the steps start over); its `onclose`, by code: 1006, 1000 or 1001
reconnect on the backoff, 4401, 4403 and 4404 refuse, 4410 (R679) and 4411 (R3694000) close, 4412
(R3767000) goes `elsewhere`; a socket factory that throws; the backoff's timer; and a page wake (the tab
shown, the network back), whose probe replaces a socket silent past `WS_PING_INTERVAL_SECONDS` that does
not answer. The client is a closure, never serialized; a reload starts it from nothing. Its rows are N1 to
N18 (N for the network), never `R<n>`. The co-op client runs the same machine, and
`apps/web/src/duos/socket.test.ts` holds its tests, the new 1001 one (N3) and N15's among them, each named
as here with "co-op".

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| N1 | `connecting`, a socket not yet open, no step | Reachable: `connect()` | `hello` on open asks for a fresh full view (§9.5); the token rides the handshake, never the address (R1379000) | `apps/web/src/game/net.test.ts` › "sends `hello` on open: §9.5's 'push me a fresh full view'", › "R1379000 the address names the match and never the token, which is offered as a subprotocol" |
| N2 | `open`, a probe waiting | Reachable: a page wake past the silence | Replaced if the probe goes unanswered, kept if it answers (R1441) | › "R1441 a silent socket is replaced when the tab turns visible, and the new socket sends hello", › "R1441 a quiet socket that answers the probe is kept", › "R1441 a socket heard within the ping interval is not probed" |
| N3 | `reconnecting`, no socket, a step waiting | Reachable: a close with 1006, 1000 or 1001 | Reopened after the step, plus up to `RECONNECT_JITTER` of it (R1441); 1001 is the restart drain's close, which the client reconnects on (R3694000, R1115000) | › "reopens after an unexpected close, with a backoff", › "R3767000 a close with 1000 still reconnects", › "R3694000 a close with 1001, a restart's drain, reconnects with a backoff" (new), › "R1441 a reconnect waits its backoff plus up to a quarter more", › "R4250000 a socket refused with 4429 after open climbs the backoff", › "R4250000 a socket closed with 1011 after open climbs the backoff" |
| N4 | `reconnecting` with a transport error, no socket, a step waiting | Reachable: the browser refuses to construct the socket (a bad URL, a blocked scheme) | **Unspecified**: retried on the backoff like a dropped socket, its error shown (`errorCode` `transport`) until a view clears it | › "a socket the browser cannot construct is retried on the backoff, with a transport error" (new) |
| N5 | `reconnecting`, a socket connecting | Reachable: N3's step fires, or `connect()` during N3 | The connection reads `reconnecting` until the socket opens; `connect()` cuts the step short (the match bar's Reconnect now) | › "reopens after an unexpected close, with a backoff", › "connect() during a backoff opens a socket at once and drops the pending retry" |
| N6 | `reconnecting`, no socket, no step | Unreachable: every move into `reconnecting` either opens a socket (`replace`) or schedules a step (`onclose`, a throwing factory), and the step's timer opens one | — | — |
| N7 | `refused` by a 4401, `refusedToken` set | Reachable: the handshake's token is refused | Final for that token; a renewed token opens one more socket that carries it; one renewed before the close arrived opens it at once (R1115000) | › "R1115000 a 4401, then a renewed token, opens one more socket that carries it", › "R1115000 the token the 4401 refused opens no socket", › "R1115000 a 4401 on the socket a renewed token opened is final for that token", › "R1115000 a token renewed before the 4401 arrives opens one more socket at once" |
| N8 | `refused` by a 4403 or 4404, `refusedToken` none | Reachable: not a seat of the match, or no such match | Final: no backoff, no renewal reopens it (R1115000) | › "does not retry a refusal: 4401/4403/4404 are settled answers", › "R1115000 a 4403 or 4404 is not reopened by a renewed token, even after a 4401's retry" |
| N9 | `closed`, `stopped`, no socket, no step | Reachable: `close()` | Nothing reopens it, and a late frame of the old socket is ignored | › "close() stops the reconnect and the socket", › "a view that arrives after close() is ignored" |
| N10 | `closed` by 4410 or 4411, not stopped | Reachable: the match voided (R679) or finished past its rematch window (R3694000) | Final, no error, the last view kept; a wake opens nothing (R679, R3694000) | › "R679 a voided match's close (4410) is final: no reconnect, the last view kept", › "R3694000 a finished match's close (MATCH_OVER_CLOSE_CODE) is final: no reconnect, no error, the last view kept" |
| N11 | `elsewhere`, no socket, no step, no probe | Reachable: a 4412, another tab or device took the seat | No backoff, no socket however long, the last view kept; Play here (`connect()`) opens exactly one socket, with the newest token (R3767000) | › "R3767000 a close with SEAT_TAKEN_CLOSE_CODE leaves the client elsewhere: …", › "R3767000 connect() takes the seat back by opening exactly one socket", › "R3767000 a wake probe waiting when the seat is taken is dropped" (new), › "R3767000 a renewed token while elsewhere opens no socket, and Play here carries it" (new) |
| N12 | `stopped` with a socket, a step or a probe | Unreachable: `close()` cancels the step, disposes the probe and lets the socket go, and `letGo` drops its `onclose`; `open` and `schedule` do nothing while stopped | — | — |
| N13 | `open` with no socket | Unreachable: every path that drops the socket (`onclose`, `replace`, `close()`) sets the connection in the same step | — | — |
| N14 | a socket already closing when the page wakes | Reachable: the browser has begun the close (readyState 2) and the page wakes past the silence | Replaced at once, the old one let go with 1000 "replaced", no probe sent (R1441) | › "R1441 a socket that never opened is replaced at once when the page wakes", › "R1441 a socket already closing when the page wakes is replaced at once" (new) |
| N15 | `refusedToken` set outside `refused` | Was reachable: a 4401, then Reconnect (`connect()`), the server takes that socket, then a 4412 or a 4410; `setToken` with a renewed token then opened a socket from `elsewhere` or onto a gone match | A refusal is its handshake's (R1115000): once a socket opens after it, a renewal reopens nothing. From `elsewhere` only Play here opens a socket (R3767000); a voided match is gone (R679) | **#4114: was a mismatch, fixed**: `open` now clears `refusedToken` with each socket it creates, in both clients. › "R1115000 a socket opened after a 4401 forgets the refusal: a renewal once the seat is taken or the match is gone opens nothing" (new; failed before the fix: "4412: expected [ FakeSocket{ …(9) }, …(2) ] to have a length of 2 but got 3") |
| N16 | a socket and a waiting step at once | Was reachable only through N15 (a renewal opening a socket while a 1006's step waited); unreachable since: `connect()` cancels the step before it opens, and nothing else opens a socket while a step waits | The step's `open` finds the socket and does nothing | N15's test |
| N17 | the backoff past its last step | Reachable: six drops in a row | The last step (`RECONNECT_DELAYS_MS`'s last) repeats; a socket that delivered a view starts the steps over (R1441, R4250000) | › "R1441 the backoff's last step repeats, and a socket that delivered a view starts the steps over", › "R4250000 a socket that delivered a view starts the backoff over" |
| N18 | open, no view yet | Reachable: a socket opens and the server sends another frame (`watchers`) before the view, or refuses it after the upgrade | Still on the backoff: only a delivered view starts the steps over, since every refusal reaches the browser after its open (R4250000) | › "R4250000 a frame before the first view does not start the backoff over" (new), and in `apps/web/src/duos/socket.test.ts` › "R4250000 a co-op frame before the first view does not start the backoff over" (new) |

## 8. A match's registry entry

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `1dcd5b65a6` (#4439) folds a co-op rebuild under the fold's
panic guard, so a panicking fold is an internal error that registers no actor (A6's co-op test); #4550's pull
request reads both registries' window through `in_again_window` (the same outputs). No row moved. Checked before
at `ae977ece3e`, plus #4291's pull request. Since `b0e998a29b`: `17e34d0815` (#4169) checkpoints each
co-op actor before the drain stops it (`DuosRegistry::drain_one`) and has a co-op rebuild fold from it;
`1fd0162fbf` (#4131, R4131000) swaps `app.rs`'s redemption breaker for the operator's pause, a field outside
this state; and `14e3fd964e` (#4239), outside these files, has a live match write a checkpoint every
`MATCH_CHECKPOINT_MIN_ACTIONS`-th row, which a rebuild after a crash or a `stop` now folds from (A3; the
checkpoint itself is state 10). Checked before at `b0e998a29b`, plus #4114's pull request.

**Files:** `crates/server/src/actor/registry.rs` (`start`, `rebuild`, `actor_for`, `attach`, `stop`,
`stop_all`, `drain`, `reap_finished`, `forget`), `crates/server/src/duos/registry.rs` (the same for a
co-op match; since #4169 it has a `stop_all` too, and its drain checkpoints each actor before it stops it,
as a match's does), `crates/server/src/app.rs` (`serve_until`, the restart drain's order).

**Fields,** per match id: `actors[id]` (the live actor in memory, or none); `rebuilding[id]` (a rebuild in
flight, shared by every caller that finds it); the store's row (none, live or finished; a void deletes it,
R679), its log (purged from a finished match after `MATCH_ACTION_RETENTION_DAYS`, R1285000) and its
checkpoint (#1169: the drain's, or since #4224 the one every `MATCH_CHECKPOINT_MIN_ACTIONS`-th row writes as
the match is played; state 10 maps it); and, since #4114, the registry's `draining` mark, set by the restart drain and never
cleared.

**What moves it:** `start` (the row written, the actor inserted, §9.2); `actor_for` (an actor in memory is
returned, else one rebuild folds the row's log, or its tail from the last checkpoint stored, and inserts it,
§9.5); a logged row whose seq is a multiple of `MATCH_CHECKPOINT_MIN_ACTIONS` (the actor's periodic
checkpoint, #4224, which a crash then rebuilds from); `attach` (`actor_for`,
then the seat's socket); `stop` (the actor leaves memory, the log stays); `drain`, the restart drain (the
registry marked draining, then `stop_all`: every actor checkpointed, then stopped with 1001); `stop_all`
alone (the same without the mark, which a test calls to drain and rebuild in one process); `reap_finished` (a finished actor past its rematch
window leaves with `MATCH_OVER_CLOSE_CODE`, R3694000); `forget` (a void's actor leaves without `stop`,
R679); the retention purge; a restart (memory empty, the store as it was). Its rows are A1 to A13 (and
A10b, A10c), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| A1 | no row, no actor | Reachable: a match id that never was, or a voided one | "no such match", 4404, nothing folded (§9.1, R679) | `crates/server/tests/actor/glitch.rs::glitchs_void::r679_closes_both_sockets_with_the_voided_close_code_after_a_last_view_that_shows_the_void` (its late attach) |
| A2 | a finished row whose log was purged, no actor | Reachable: the retention purge, then a socket | Not found, nothing folded or filed (R1285000) | `crates/server/tests/actor/recovery.rs::r1285000_a_finished_match_whose_log_was_purged::r1285000_answers_not_found_on_attach_folds_nothing_and_files_no_report`, `crates/server/tests/actor/ws_server.rs::a_finished_matchs_seats_may_reopen_its_socket::r1285000_a_finished_match_whose_log_was_purged_is_not_found_inside_its_window` |
| A3 | a live row, no actor | Reachable: a restart, a crash, `stop` | The next socket rebuilds it to the same state and view (§9.5): from the last checkpoint stored when it passes (the drain's or, since #4169 and #4224, the periodic one; #1169), else from `(seed, log)` | `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::rebuilds_an_actor_for_a_socket_that_arrives_when_nothing_is_in_memory`, `…::m6_t4_crash_recovery::killing_the_actor_mid_game_and_reconnecting_yields_the_same_view_for_for_both_players`, `crates/server/tests/actor/checkpoints.rs::a_drained_match_rebuilds_from_its_checkpoint_as_a_full_fold_rebuilds_it`, `…::a_live_match_checkpoints_every_min_actions_rows_as_it_is_played_and_a_crash_resumes_there`, `…::a_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded` |
| A4 | a finished row, its log kept, no actor | Reachable: `reap_finished` past the window, or a restart | A seat's socket rebuilds it inside the window; past the window it is refused (R1285000) | `crates/server/tests/actor/ws_server.rs::a_finished_matchs_seats_may_reopen_its_socket::a_seat_reopens_its_socket_on_its_finished_match`, `…::r1285000_a_seat_is_refused_its_finished_matchs_socket_once_the_rematch_window_has_passed`, `crates/server/tests/actor/telemetry.rs::r1442_play_telemetry_through_the_actor::r1442_the_backfill_and_a_rebuild_fold_a_finished_match_after_the_reaper_dropped_its_actor` |
| A5 | a rebuild in flight, a second socket arrives | Reachable: both seats reconnect together | One fold, both sockets on the one actor (`rebuilding`'s doc) | `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::folds_the_log_once_when_both_sockets_arrive_together` |
| A6 | a rebuild that failed | Reachable: the store fails a read, or the fold panics | The socket is refused (1011 for an internal error, `AttachError`), nothing is kept, and the next socket rebuilds | `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::a_rebuild_the_store_fails_is_not_kept_and_the_next_socket_rebuilds` (new), `crates/server/tests/actor/long_fold.rs::a_rebuild_whose_fold_panics_is_an_internal_error_and_registers_no_actor`, `…::a_duos_rebuild_whose_fold_panics_is_an_internal_error_and_registers_no_actor` |
| A7 | a `rebuilding` entry whose creator was dropped | Reachable only if the call that made the entry is cancelled mid-rebuild. Every production caller runs where nothing cancels it (a socket's `on_upgrade` task) except the bot loop's `kick_off_bot_match`, which `stop_loops` aborts in the restart drain, after which the process ends | **Unspecified**: the entry is never removed, so the next caller initialises the cell again and the actor it caches is handed to every later `actor_for` that misses the map, a stopped one after a `stop` | — (for a person) |
| A8 | a live actor in memory | Reachable: `start`, or a rebuild | Every socket for the match reaches it (§9.2: one actor per match) | `crates/server/tests/actor/match_actor.rs::m6_t4_the_match_actor::two_websocket_clients_complete_a_scripted_game` |
| A9 | `start` and a rebuild of the same id at once | Reachable only as a race: a socket for the id between `start`'s row write and its insert | **Unspecified**: whichever inserts second wins the map, and a socket attached to the other plays on an actor no one else reaches (§9.2 asks for one) | — (for a person) |
| A10 | a rebuild in flight across the restart drain | Was reachable: the drain runs while a rebuild reads the store, and the rebuild inserts after it | Every live match checkpointed and stopped with 1001 (`crates/server/README.md`, the drain; `stop_all`'s doc: no grace stored) | **#4114: was a mismatch, fixed**: the rebuilt actor was left live after the drain, never checkpointed nor stopped, so the shutdown's `close_all` closed its socket while it still ran and its `on_socket_gone` stored a grace the drained matches do not. `drain` now marks the registry draining before `stop_all`, and a rebuild that finds the mark as it inserts (under the actors lock) drains its own actor instead (checkpointed, stopped), which `attach` then closes 1001 (S10). `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::a_rebuild_in_flight_across_the_restart_drain_is_stopped_too` (new; failed before the fix: `left: ["match-recovery"] right: []`, the rebuilt actor still live after the drain) |
| A10b | a rebuild begun after the restart drain | Was reachable: `serve_until` runs `stop_accepting`, `stop_loops`, `server.await`, then the drain; axum's graceful shutdown waits on no upgraded socket, so a socket upgraded just before `stop_accepting` whose `authenticate_upgrade` outlasts the rest reaches `attach`, and its rebuild, after the drain | As A10 | **#4114: was a mismatch, fixed** by the same mark. A count bumped by each `stop_all` (this pull request's first fix) caught only a rebuild that read it before the drain; this one read the bumped count and inserted a live actor. `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::a_rebuild_begun_after_the_restart_drain_is_stopped_too` (new; failed without the mark: `left: ["match-recovery"] right: []`) |
| A10c | a match `start`s once the restart drain has begun | Was reachable by the code's order: `stop_all` drains each actor's queue so a result being written lands (its doc), and a series game's result starts the series' next game (`results.rs`'s `resume_series`, `series.rs`'s `start_series_game`), which inserted its actor after `stop_all` had taken the map. The test reaches it through `start` after `drain`, not through a series' last result | As A10: the row is written, so the next process rebuilds it | **#4114: was a mismatch, fixed** by the same mark: `start` drains the actor it made instead of keeping it (`Registry::keep`). `crates/server/tests/actor/recovery.rs::m6_t4_crash_recovery::a_match_started_after_the_restart_drain_is_stopped_too` (new; failed without the mark: `left: ["match-started-late"] right: []`) |
| A11 | a rebuild in flight across `stop`, a reap or `forget` | Reachable only as a race | **Unspecified**: the rebuild inserts what it read, a fold of the store's log (§9.5), so nothing is lost; a voided match's row is gone, so its rebuild answers not found | — |
| A12 | a void whose delete the store failed | Reachable: the store fails `forgetVoided` | The row stays live, so the next socket rebuilds the actor, which folds to the same void and voids it again (`on_void`'s comment, R679) | `crates/server/tests/actor/glitch.rs::glitchs_void::r679_a_void_the_store_failed_to_forget_is_voided_again_by_the_next_socket` (new) |
| A13 | a series game started again under the same id | Reachable: a voided Conquest game | The series plays the same game again, a new actor under the old id (R679) | `crates/server/tests/actor/glitch.rs::glitchs_void::r679_a_voided_conquest_game_never_happened_the_series_plays_the_same_game_again` |

The co-op registry holds A3, A6, A7, A9 and A11 the same way: since #4169 a co-op rebuild folds from the
last checkpoint stored, the drain's or the periodic one, when it passes
(`crates/server/tests/actor/duos_checkpoints.rs::a_drained_coop_match_rebuilds_from_its_checkpoint_as_a_full_fold_rebuilds_it`,
`…::a_two_v_two_match_checkpoints_every_min_actions_rows_as_it_runs_and_a_crash_resumes_there`,
`…::a_coop_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded`,
`…::the_restart_drain_checkpoints_each_long_coop_match_and_passes_over_a_short_one`). **#4114: its A10 and A10b were a mismatch,
fixed**: the drain stopped each co-op actor with `stop` from a `live()` snapshot, so a co-op rebuild that
landed after that loop was left live and could write its log after the store closed (#936, the drain's own
comment). Its A10c was found unreachable: every co-op `start` is a request's (a room's join, the lobby's
start, the co-op rematch in `duos/again.rs`), which `server.await` finishes before the drain; the same mark
guards it anyway. `DuosRegistry::drain` now marks the registry draining and stops every actor in memory, and
`start` or a rebuild that finds the mark stops its own actor (`DuosRegistry::keep`), which `attach` then
closes 1001. Since #4169 each is checkpointed before it is stopped (`DuosRegistry::drain_one`), as a
match's is.
`crates/server/tests/actor/duos.rs::registry_states_4114::a_coop_rebuild_in_flight_across_the_restart_drain_is_stopped_too`
(new; failed without the mark: "left live after the drain"),
`…::registry_states_4114::a_coop_rebuild_begun_after_the_restart_drain_is_stopped_too` (new; failed without
the mark: `left: None right: Some(1001)`, the socket greeted on a live actor) and
`…::registry_states_4114::a_coop_match_started_after_the_restart_drain_is_stopped_too` (new; failed
without the mark: "left live after the drain").

## 9. The practice game kept for a resume

**Checked at** `0bf5057480`: unchanged since `ae977ece3e` but for #4291's own pull request (`bcdc70af1d`). Checked before at `ae977ece3e`: unchanged since `b0e998a29b` but for #4114's own pull request (`87bd279557`). Checked
before at `b0e998a29b`, plus #4114's pull request.

**Files:** `apps/web/src/practice/resume.ts` (R, read as untrusted input), `apps/web/src/practice/saveStore.ts`
(W's IndexedDB store), `apps/web/src/practice/controller.ts` (`begin`, `applySnapshot`),
`apps/web/src/routes/practice.tsx` (the banner, Save and leave, Back), `crates/shared/src/host/practice/host.rs`
(`resume`, `save`: which save may fold).

**Fields:** R, the page's record (`localStorage["jackioh.practice.game"]`: the setup, seed, seat, difficulty
and deck, with `sandbox`, `earning`, `dailyDeal` and `portraits`, and `saved`, R765); W, the worker's save
(IndexedDB: its config, log, AI cursor, catalog and hash, R668; a run fight has a slot of its own); and
whether the game is over.

**What moves it:** a game's start (R written without `saved`, `controller.ts:307`; W after every answer);
the game's end (R cleared, `controller.ts:222`, and W, `host.rs`'s `save`); a resume that fails (R cleared,
`controller.ts:313`); Save and leave, or Back while a resume folds (R written `saved`, `practice.tsx:734`,
`:759`); Leave without saving, New game, Menu, a lesson or Play practice (R cleared); a reload (both kept);
a deploy that moves `CATALOG_VERSION`; a second tab (one R and one W for the device). A run fight touches
neither (R2706000, R3476000). Its rows are K1 to K21 (K for kept), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| K1 | neither | Reachable: a first visit, or a game that ended | The setup, no banner (R668) | `apps/web/src/routes/practice.test.tsx` › "B32 renders with no account, session or server: three difficulty radios on easy and random plus every preset" |
| K2 | R and W of one game, R not saved, after a reload | Reachable: start a game, reload | Picked up at once, folded to the saved hash, the AI at its cursor (R668; R765: a reload "still picks it up"); a URL that starts a game wins and replaces it (R668) | › "R668 with no params that start a game, the game this device left is asked back as resume and shows", › "R765 a reload in the middle of a game, never left through the menu, still picks it up at once (R668)", › "R668 a URL that starts a game wins over the one left, and replaces it"; `apps/web/src/practice/driver.test.ts` › "R668 a new core folds the saved log back to the same state, snapshot and AI stream" |
| K3 | R saved, W of the same game | Reachable: New game or Menu, then Save and leave | Not picked up: the setup shows its banner (R765) | › "R765 Save and leave from New game keeps the game: the setup shows its banner, and so does the next visit, with no resume", › "R765 Save and leave from Menu goes to the main menu, and /practice then shows the banner" |
| K4 | K3, then Resume | Reachable: the banner's Resume | Folds as a reload does; R is written again without `saved` (R765) | › "R765 Resume picks the saved game up as a resume, and a reload after that picks it up at once (R668)" |
| K5 | a resume still folding, then Back | Reachable: Back on the loading screen | Kept as saved, for the banner; Back from a new game's deal gives it up (R765) | › "R765 Back while a resume is still folding keeps the game, saved for the banner; Back from a new game's deal gives it up" |
| K6 | W only | Reachable: Leave without saving (R cleared, nothing clears W), or a localStorage that refuses the write | Nothing offers it again; the next game's save replaces W (R765) | › "R765 Leave without saving gives the game up: no banner now, and none on the next visit" |
| K7 | R only, not saved | Reachable: no IndexedDB, or a database that will not open, then a reload | Refused, and the page shows the setup (R668) | › "R668 a game the worker cannot bring back is forgotten, and the player lands on the setup"; `driver.test.ts` › "R668 a resume of a game the store does not hold, of another game, or from another catalog fails and changes nothing"; `apps/web/src/practice/saveStore.test.ts` › "R668 a malformed row is no save, and a database that will not open is memory only" |
| K8 | R saved only | Reachable: K3, then W lost | The banner shows; a Resume that cannot fold says so and the banner goes (R765) | › "R765 a Resume the worker cannot fold says so on the setup, and the banner is gone" |
| K9 | R and W of two games that differ in seed, seat, difficulty or sandbox | Reachable: two tabs (B starts another game, A answers, B reloads), or a full localStorage that keeps the old R | Refused, the setup (R668: "a save of another game … is refused") | `driver.test.ts` › the K7 refusal test; `crates/shared/tests/practice.rs::r668_a_resume_of_no_save_another_game_or_another_catalog_fails` |
| K9b | R and W of two games with one seed, seat, difficulty and sandbox but another deck | Reachable: two tabs on `?seed=s&difficulty=easy&seat=p1&deck=random` and the same with `deck=preset:<id>`, then a reload with no parameters | Refused: R668 names the page's setup "seed, seat, difficulty, deck", and a save of another game is refused | **Mismatch, named for a person, not fixed**: `host.rs`'s `resume` compares seed, seat, difficulty and sandbox, never the deck, and folds W's own config, so the page resumes the other tab's game under its own setup. No test |
| K10 | W from another catalog | Reachable: a deploy that moves `CATALOG_VERSION` between visits | Refused, the setup (R668, R2882000) | `driver.test.ts` › the K7 refusal test; the `practice.rs` test above |
| K11 | a W that does not fold to its hash | Unreachable: the machine writes W from its own log and hash, and one build folds one way (R187). **Representable when serialized**: an edited IndexedDB row | Refused (R668) | `driver.test.ts` › the K7 refusal test (its tampered and forged saves) |
| K12 | a malformed R | Reachable by hand, or across builds: an older build reading a newer one's R (a portrait or deck kind it does not know) | No game (`resume.ts`'s header) | › `apps/web/src/practice/resume.test.ts` › "R668 reads anything malformed as no game", › "R765 a flag that is not true or false is malformed: no game", › "R3976000 a game is kept with its portraits, and portraits naming an unknown one are malformed" |
| K13 | storage that throws or refuses | Reachable: a private window, a full quota | No game and nothing thrown (R668). **Unspecified**: a full localStorage keeps the previous R, so Save and leave may not mark it saved and a new start may leave the old game's R (K9); a refused IndexedDB write keeps the previous answer's save, which resumes one answer back | `resume.test.ts` › "R668 storage that throws is no game, and a refused write throws nothing"; `routes/practice.test.tsx` › "a localStorage that refuses the write still starts the game" |
| K14 | the game over | Reachable: concede, win or lose (R and W cleared, K1); over with R saved is unreachable in one tab, for the leave question shows only while the game is on | A finished game is not resumed and clears the save (R668) | › "R668 leaving through Menu gives it up too, and a finished game is not kept", › "B33 once the game is over, practice-new-game goes straight back to setup"; `driver.test.ts` › "R668 a lesson is never kept, and a finished game clears the save" |
| K15 | a lesson | Reachable: `?lesson=` or its path | Never kept, and starting one ends the saved game (R668, R765) | `resume.test.ts` › "R668 a lesson is never remembered: writing one forgets the free game"; › "R765 starting a tutorial lesson from the path ends the saved game too" |
| K16 | a sandbox, earning or daily game kept | Reachable: the Sandbox box, a server seed, Play today's deal | Kept as each one; a sandbox resumes only for a page that remembers one (R923000, R1303001, R2882000) | `resume.test.ts` › "R923000 a sandbox game is kept as one, and a sandbox flag that is not true is malformed", › "R1303001 a game on a server seed is kept as one, so its end is claimed after a reload, and a flag that is not true is malformed", › "R2882000 today's deal is kept as one, and a day that is not a string is malformed"; `driver.test.ts` › "R923000 R668 a sandbox game resumes as a sandbox, and only for a page that remembers a sandbox"; › "R1303001 a game on a server seed picked up after a reload is still claimed at its end, on its own seed" |
| K17 | a game kept with its portraits, resumed | Reachable: any free game since R3976000, then a reload | The resumed board wears the portraits it kept (R3976000: "a resumed game keeps them") | › "R3976000 a resumed game wears the portraits it kept" (new) |
| K18 | a run fight | Reachable: the dungeon's fight | Leaves R and the free game's W alone (R2706000, R3476000) | `apps/web/src/practice/controller.test.ts` › "R2706000 a run fight's start goes as given and leaves practice's memory alone", › "R3476000 a run fight's missed resume and start leave practice's memory alone"; `saveStore.test.ts` › "R3476000 the free game and the run fight coexist, and each clear leaves the other" |
| K19 | one tab's end clears the other tab's game | Reachable: tab A plays X; tab B starts Y (R and W are Y's); A concedes, clearing R and W; B's next answer writes W again (K6), so a reload of B loses Y | **Unspecified**: R668 and R765 speak of one game per device | — |
| K20 | a stale banner clears another tab's R | Reachable: A saves and leaves X, both tabs show the banner; B starts Y; A's Resume is refused (W is Y's) and clears Y's R (`controller.ts:313`) | **Unspecified** (R765 drops "a save that will not fold", not another game's) | — |
| K21 | a failing engine or IndexedDB on a reload, with a good R and W | Reachable: the WebAssembly load fails, or the database will not open | **Unspecified**: R668 refuses another game, another catalog or another fold; the page clears R anyway (`controller.ts:313`), and the game is lost though W holds it | › "R668 a game the worker cannot bring back is forgotten, and the player lands on the setup" (its failed resume) |

## 10. A live match's stored checkpoint

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `37774db591` (#4543) reads the Daily list's portraits and
the daily games due through the store, `4bc32f259f` (#4529) seats no closed socket (state 1's S15), `0cbf13009a`
(#4474) caches the seats' views, `73fe3958cd` (#4429) and `0b6703d0cd` (#4416) add store reads for settings and
retention, and `4256b42744` (#4415) has the share put report whether it wrote (state 11's F31); none touches the
checkpoint's put, its queue or `verify`, so no row moved. Checked before at `ae977ece3e`, plus #4291's pull request.

**Files:** `crates/server/src/actor/match_actor.rs` (`Task::Checkpoint`, `checkpoint`, `write_checkpoint`),
`crates/server/src/actor/checkpoint.rs` (`checkpoint_row`, `verify`), `crates/server/src/actor/registry.rs`
(`rebuild`, `stop_all`), `crates/server/src/db/pg.rs` and `crates/server/src/db/fake.rs`
(`matches_put_checkpoint`, `matches_finish`, `put_newer_checkpoint`).

**Fields,** per match: the log's newest seq n; the stored checkpoint's seq c (none, or one row, at most one
per match, #1169); a `Task::Checkpoint` queued (the drain's, or the periodic one every
`MATCH_CHECKPOINT_MIN_ACTIONS`-th written row queues, #4224); the host's finished, voided and stopped marks;
the row's status (live or finished; a void deletes it); and the store's answer to a put (written, kept, or
failed).

**What moves it:** a row written whose seq is a multiple of `MATCH_CHECKPOINT_MIN_ACTIONS` (a periodic
checkpoint queued behind it); the restart drain (`stop_all`: each actor checkpointed, then stopped); `stop`, a
crash or the reaper (none written, but a put already in flight lands, C10′); the match's ending (`matches_finish`
deletes the row) or void (the cascade); a rebuild (`Registry::rebuild` reads the row with the log and folds the
tail from it when `verify` passes, else the whole log, §9.5); the store failing a put. Its rows are C1 to C18 and
C10′ (C for checkpoint), never `R<n>`; K is state 9's.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| C1 | n < `MATCH_CHECKPOINT_MIN_ACTIONS`, none stored | Reachable: a short match, drained | Nothing written: a short log folds whole cheaply (`MATCH_CHECKPOINT_MIN_ACTIONS`'s doc) | `crates/server/tests/actor/checkpoints.rs::a_short_match_writes_no_checkpoint` |
| C2 | the row whose seq is the interval, none stored | Reachable: play 32 rows | One checkpoint at that seq, queued behind the row so no answer waits on it (#4224) | `…::a_live_match_checkpoints_every_min_actions_rows_as_it_is_played_and_a_crash_resumes_there` |
| C3 | a periodic checkpoint over an older one (c < n) | Reachable: 40 rows drained (c = 40), rebuilt, rows 41 to 64 played | Replaced by the newer (only a later seq replaces one, #1169); a crash resumes from it to `fold(..64)`'s hash (§9.5) | `…::a_periodic_checkpoint_replaces_an_older_one_and_a_crash_resumes_from_it` (new) |
| C4 | c < n, no actor | Reachable: drained at 40, rows appended after | The rebuild folds the tail from c to the state a whole fold reaches (§9.5) | `…::a_drained_match_rebuilds_from_its_checkpoint_as_a_full_fold_rebuilds_it` |
| C5 | c = n, rebuilt | Reachable: drained, then rebuilt | The actor carries the log's seq and nonces on, and the next drain moves c up | `…::a_rebuilt_match_carries_its_sequence_on_and_the_next_drain_moves_its_checkpoint_up` |
| C6 | the drain right after a periodic checkpoint at n | Reachable: 32 rows played, `stop_all` | The store keeps the one held (the same seq), `kept` logged | `…::a_drain_right_after_a_periodic_checkpoint_keeps_it` (new) |
| C7 | c > n | Unreachable: a put stores only a row the log holds under its nonce (§9.3: the row before the state), and a live match's log only grows. **Representable when serialized**: an edited row is refused for its seq and the log folds whole | Refused (`verify`) | `…::a_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded` (its "seq" poison) |
| C8 | a periodic checkpoint queued, the match finishes before it runs | Reachable: the 32nd row and a concession submitted together | Nothing written, no `match.checkpoint.*` line (`write_checkpoint` passes over a finished host) | `…::a_match_finishing_while_its_periodic_checkpoint_is_queued_writes_none` (new) |
| C9 | the match finished with a checkpoint stored | Reachable: concede after C2 | The ending deletes it (§9.3, #1169) | `crates/server/tests/store/contract.rs::matches::finishing_a_match_drops_its_checkpoint`; C2's test (its concession) |
| C10 | a periodic checkpoint queued when the actor is stopped | Reachable: the 32nd row's write waits on the store (its lock held) and `stop` lands, so the checkpoint is still queued behind the row | Nothing written, no `match.checkpoint.*` line, and the next rebuild folds the whole log (`checkpoint`'s doc: `stop` writes none; `write_checkpoint` passes over a stopped host) | `crates/server/tests/actor/checkpoints.rs::a_periodic_checkpoint_queued_when_the_actor_is_stopped_writes_none` (new; fails with the stopped check removed) |
| C10′ | a periodic checkpoint in flight when the actor is stopped | Reachable: the 32nd row acked, its checkpoint's put built and waiting on the store's lock, then `stop` | **Unspecified**: `checkpoint`'s doc says `stop` writes none, but `write_checkpoint` checks the stopped mark only before it builds the row, so a put already past it is written (`written` logged); it holds the log's head, and the next rebuild resumes from it to the whole fold's state | `…::a_periodic_checkpoint_in_flight_when_the_actor_is_stopped_is_written` (new) |
| C11 | the match voided with a checkpoint stored | Reachable: a void after C2 | Deleted with the match (R679, the cascade) | `crates/server/tests/store/contract.rs::matches::voiding_a_match_drops_its_checkpoint` |
| C12 | another writer finished the row between the queue and the put | Reachable: the row turned finished as the put runs | The store's live-row guard keeps it out, `kept` logged | `crates/server/tests/actor/checkpoints.rs::a_checkpoint_of_a_match_another_writer_finished_is_kept_out` (new) |
| C13 | a put while the ending is uncommitted (Postgres) | Reachable: `matches_finish`'s transaction open while a put runs in its own | The put stores nothing, so the ended match keeps none (§9.3: "a match that ends drops its checkpoint") | **#4291: was a mismatch, fixed**: Postgres's put read the row live under the ending's open transaction and stored the checkpoint, which the ending's delete had already passed, so the finished match kept it. The put's live-row guard now locks the row (`for share`), waits for the ending and stores nothing. `crates/server/tests/store/contract.rs::matches::a_checkpoint_written_as_its_match_finishes_never_outlives_the_ending` (new, both stores; failed on Postgres before the fix: "postgres store (db/pg.rs): the put waits for the ending and stores nothing") |
| C14 | a put the store fails | Reachable: the store down at the 32nd row | Every row acked, one warn `match.checkpoint.failed`; the next rebuild folds from the last checkpoint stored, here none (`write_checkpoint`'s doc) | `crates/server/tests/actor/checkpoints.rs::a_checkpoint_the_store_fails_is_logged_and_the_rebuild_folds_from_the_last_one_stored` (new) |
| C15 | an older put after a newer one | Unreachable from one actor: its queue writes in seq order, and a rebuilt actor starts at a seq at or past the one stored; the store keeps only a later seq anyway | — | `crates/server/tests/store/contract.rs::matches::checkpoint_round_trips_and_only_a_later_seq_replaces_it` |
| C16 | a stored checkpoint the rebuild refuses, at the log's head | Reachable: one written under other rules (a deploy that moves `RULES_FINGERPRINT`), then a restart drain before any new row | **Unspecified**: the drain's good one has the same seq, so the store keeps the refused one, and the next rebuild refuses it again and folds the whole log until a later row's checkpoint replaces it | `crates/server/tests/actor/checkpoints.rs::a_refused_checkpoint_at_the_logs_head_outlasts_the_drains_good_one` (new) |
| C17 | the body read back (its JSON round trip) | Reachable: every rebuild from a checkpoint | The state, the recorder (R1442) and the bot turns (R639000) carry on as a whole fold's; **unspecified**: a periodic checkpoint keeps the live `clockLeftMs` readings a whole fold has no clock for | `…::a_recorder_read_back_from_a_checkpoint_carries_on_as_replaying_the_whole_log`, `…::bot_turns_carried_on_from_a_checkpoint_equal_the_whole_logs`, `…::every_state_of_a_whole_game_checkpoints_and_passes_its_checks`; C2's test strips `clockLeftMs` to compare |
| C18 | a checkpoint whose match row is gone | Unreachable: `match_checkpoints.match_id` references `matches` `on delete cascade` (`migrations/0042_match_checkpoints.sql`) | — | — |

A co-op or 2v2 match's checkpoint (`crates/server/src/duos/checkpoint.rs`, #4169) holds the same rows through
the same `verify_body` and the same store guards (`put_newer_checkpoint` keeps both fakes' copies one):
`crates/server/tests/actor/duos_checkpoints.rs::a_two_v_two_match_checkpoints_every_min_actions_rows_as_it_runs_and_a_crash_resumes_there`
(C2), `…::a_coop_checkpoint_the_store_fails_to_write_costs_only_the_shortcut` (C14) and
`…::a_coop_match_ending_after_a_checkpoint_rebuild_drops_its_checkpoint` (C9).

## 11. A notification's read mark

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `70a1730b46` (#4367, R4348000) has a block delete the rows
between the two either way and push both counts (F30); `4256b42744` (#4415, R4391000) has a share already held
write no row and tell nobody (F31, and F25's exception); `37774db591` (#4543) reads the Daily list's portraits in
one query and moves no row. Checked before at `ae977ece3e`, plus #4291's pull request.

**Files:** `crates/server/src/notifications/mod.rs` (`notify`, `mark_subject`, `list`, `mark_read`) and
`store.rs`; R4233000's callers (`crates/server/src/daily/games.rs`, `daily/invites.rs`, `social/friends.rs`,
`lobby/mod.rs`, `api/replays.rs`, `social/shares.rs`, `daily/notify.rs`); `apps/web/src/inbox/feed.ts` (`openFeed`), `InboxBell.tsx`,
`NotificationToast.tsx`, `RecentSection.tsx` and `apps/web/src/together/socialPoll.ts` (`onNotificationPush`).

**Fields,** per row: `read_at` (none or a moment); its kind, actor and subject (what an act marks it by); for a
`lobbyInvite`, whether its lobby is open (`Lobby::open_at`); the account's unread count (the social read's
`notificationsUnread`, the bell's dot); and, in a tab, the feed's page and whether the panel is open.

**What moves it:** `notify` (a row written unread, or read for its own actor's ending, R4233000; it replaces the
recipient's unread row of the same kind, actor and subject; a `notification` frame for an unread one); an act's
mark (`mark_subject`: a board read or action, a challenge's accept or decline, a friend request's answer, a lobby's
enter, a replay's read); the panel's opening (`POST /api/notifications/read` up to the newest row, R4039001); a
page read (it deletes a closed lobby's invites); the cap (`NOTIFICATIONS_KEEP`) and the daily sweep's purge
(`NOTIFICATIONS_KEEP_DAYS`); a block (R4348000: the rows between the two deleted); a push to the account's sockets; a reload. Its rows are F1 to F31 (F for feed),
never `R<n>`; N is state 7's.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| F1 | a row written unread | Reachable: any notice to another account | Listed, counted, pushed to every socket of its account alone (R4039000) | `crates/server/tests/actor/social_live.rs::r4039000_frames::r4039000_a_notification_reaches_every_socket_of_its_account_alone`, `crates/server/tests/api/notifications.rs::r4039000_the_social_read_counts_unread` |
| F2 | a row written read | Reachable: the player whose own move or concession ended a daily game | Listed, never counted, no toast or frame (R4233000) | `crates/server/tests/api/daily.rs::r4233000_read_on_open::r4233000_the_senders_own_ended_row_is_written_read`, `crates/server/tests/actor/social_live.rs::r4233000_push::r4233000_a_conceders_own_ended_row_sends_them_no_notification` (new) |
| F3 | a loss on time, the loser's row | Reachable: the move's time runs out | Unread: the server's act, not the loser's (R4233000, R3727000) | `crates/server/tests/api/daily.rs::r4039000_daily_feed::r4039000_time_up_tells_both_on_time` |
| F4 | a loss on time, the winner's row | Reachable: as F3 | Unread (R4233000) | F3's test |
| F5 | a daily game's move and ended rows, then the board read or an action there | Reachable: as it says | Marked read in the read's own transaction, only while unread, pushing the reader's frame (R4233000, R3840000) | `crates/server/tests/api/daily.rs::r4233000_read_on_open::r4233000_a_board_read_marks_its_move_rows_read`, `…::r4233000_an_action_marks_the_same_rows`, `crates/server/tests/actor/social_live.rs::r4233000_push::r4233000_a_board_read_pushes_the_new_count` |
| F6 | a daily challenge's row, then its accept or decline | Reachable | Marked (R4233000) | `crates/server/tests/api/daily.rs::r4233000_read_on_open::r4233000_accepting_or_declining_a_daily_challenge_marks_its_row` |
| F7 | a friend request's row, then its accept, decline or a mutual add | Reachable | Only that sender's row marked (R4233000) | `crates/server/tests/api/friends.rs::r4233000_read_on_open::r4233000_accepting_or_declining_a_friend_request_marks_only_that_senders_row`, `…::r4233000_a_mutual_add_marks_the_first_request_row` |
| F8 | a lobby invite's row, then the lobby entered | Reachable | Marked, still listed (R4233000) | `crates/server/tests/api/lobbies.rs::r4233000_read_on_open::r4233000_entering_a_lobby_marks_its_invite_read` |
| F9 | a shared replay's row, then the friend reads it | Reachable | Marked (R4233000) | `crates/server/tests/api/shares.rs::r4233000_read_on_open::r4233000_a_friends_read_of_a_shared_replay_marks_its_row` |
| F10 | a shared replay's row of a seat of the match, the other seat its sharer, then it reads the replay | Was reachable: `share_replay` lets one seat share with the other, a friend | Marked: "a friend's read of a replay shared with them" (R4233000) | **#4291: was a mismatch, fixed**: `replays.rs` marked only a reader reading as someone else's seat, so a seated friend's read marked nothing. It now marks the caller's own `replayShared` rows on every read. `…::r4233000_a_seated_friends_read_of_a_replay_shared_with_them_marks_its_row` (new; failed before the fix: "left: Number(2) right: 1") |
| F11 | two rows on one replay, from both seats | Reachable: both share with one friend | One read marks both (R4233000 marks by subject) | `…::r4233000_one_read_marks_every_sharers_row_for_the_match` (new) |
| F12 | an accept's `dailyAnswered` row, then the board it links read | Reachable | **Unspecified**: R4233000 names no act that answers an answer; the board read marks only the move and ended rows, so the answer stays unread until the panel opens | `crates/server/tests/api/daily.rs::r4233000_read_on_open::r4233000_opening_the_board_an_accept_links_leaves_its_answer_unread` (new) |
| F13 | a daily challenge cancelled by its sender, or lapsed, unanswered | Reachable: a cancel; the sweep past `DAILY_CHALLENGE_TTL_DAYS` | **Unspecified**: no act of the friend's, so the row keeps counting though there is nothing to answer | `…::r4233000_a_challenge_cancelled_or_lapsed_unanswered_keeps_its_row_unread` (new) |
| F14 | a friend request cancelled, then sent again | Reachable | **Unspecified**: the cancelled one's row keeps counting; the new request is a row of its own (another subject), and answering it marks only that one | `crates/server/tests/api/friends.rs::r4233000_read_on_open::r4233000_a_cancelled_request_keeps_its_row_unread_beside_a_resent_one` (new) |
| F15 | a lobby invite whose lobby is gone, started or expired | Reachable: the host leaves or starts; the lobby lapses | Counted nowhere, even before a page is read; the page deletes it (R4233000) | `crates/server/tests/api/lobbies.rs::r4233000_read_on_open::r4233000_a_dead_lobby_invite_is_not_counted`, `…::r4039000_the_invite_feed::r4039000_an_invite_lists_until_the_host_leaves_or_starts` |
| F16 | an entered invite, then its lobby closes | Reachable | **Unspecified**: R4039001 keeps rows as history, while R4039000 lists an invite only while its lobby is open; the page deletes it | `…::r4233000_read_on_open::r4233000_an_entered_invite_leaves_recent_once_its_lobby_closes` (new) |
| F17 | an invite into a lobby another player filled | Reachable: a third player takes the seat | **Unspecified**: it still counts, though the full lobby reads as gone to the friend (R4233000 leaves out only gone, started or expired) | `…::r4233000_an_invite_to_a_full_lobby_still_counts` (new) |
| F18 | the panel's mark fails | Reachable: the store or network fails the POST | The rows stay unread at the server and the next opening marks them again (`feed.ts`'s comment) | `apps/web/src/inbox/RecentSection.test.tsx` › "R4039001 a mark that fails is sent again at the next opening" (new) |
| F19 | unread rows, the bell's dot | Reachable | The dot shows only with no badge, and the panel's opening clears it (R4039001) | `apps/web/src/inbox/RecentSection.test.tsx` › "R4039001 opening the panel marks the feed read up to the newest row shown, and the bell's dot shows only with no badge", › "R4039001 a badge takes the dot's place, and a page with nothing unread sends no mark" |
| F20 | the panel opened on a page an act already read, an older row unread | Was reachable: an act marks the newest rows (F5 to F9) and an older one stays unread | Every row up to the newest marked: the panel "marks every row up to its newest one read" (R4039001) | **#4291: was a mismatch, fixed**: `openFeed` sent the mark only for an unread row on the page, so the dot stayed lit. It now also marks when the page's `unread` count is not 0. › "R4039001 opening the panel marks an older unread row below a page its acts already read" (new; failed before the fix: "expected [] to have a length of 1 but got +0") |
| F21 | a row pushed while the panel is open | Reachable: a friend acts while the panel shows | The page read again and marked up to its newest (R4039001) | › "R4039001 a row pushed while the panel is open reads the page again and marks it" (new) |
| F22 | a mark, an account with two tabs | Reachable | Both tabs' sockets hear the new count (R4233000) | `crates/server/tests/actor/social_live.rs::r4233000_push::r4233000_a_mark_reaches_every_socket_of_its_account` (new), `…::r4233000_a_mark_pushes_the_new_count` |
| F23 | a pushed row while a board is shown | Reachable | **Unspecified**: the toast is held until the board goes, then shown, though the row may be read by then | `apps/web/src/inbox/NotificationToast.test.tsx` › "R4039001 while a board is shown the toast is held, and it shows once the board goes" |
| F24 | the socket down, or a reload | Reachable | The poll's social read carries the count (R3214000); the panel reads the page afresh | `apps/web/src/together/polling.test.tsx` › "R3214000 after the connection drops the social poll reads at once and resumes its pace" |
| F25 | a second notice on an unread row's kind, actor and subject | Reachable: a daily game's next move | Replaces it, so moves never stack; a read row stays beside the new one (R4039000), except a share already held, which writes no row at all (F31, R4391000) | `crates/server/tests/store/contract.rs::r4039000_notifications::r4039000_add_replaces_an_unread_row_and_keeps_the_newest`, `crates/server/tests/api/daily.rs::r4039000_daily_feed::r4039000_a_second_move_stands_beside_a_read_row` |
| F26 | past the cap, or past the keep days | Reachable | The newest `NOTIFICATIONS_KEEP` kept; the sweep purges older rows, read or not (R4039000) | `crates/server/tests/api/notifications.rs::r4039000_the_cap_keeps_the_newest`, `…::r4039000_the_sweep_purges_rows_past_the_keep_days` |
| F27 | a read row unread again | Unreachable: no write clears `read_at` (`mark_read` and `mark_subject` set it on unread rows only) | — | `crates/server/tests/store/contract.rs::r4039000_notifications::r4233000_mark_subject_marks_only_matching_kind_subject_and_actor` |
| F28 | a row naming a deleted account | Unreachable: a deleted profile takes its rows both ways | — | `…::r4039000_a_deleted_profile_takes_its_rows_both_ways` |
| F29 | a `notification` frame for a read row | Unreachable: `notify` sends a frame only for a row written unread. **Representable when serialized**: the frame carries `read` | — | F2's tests |
| F30 | a block between two accounts with rows between them | Reachable: Ana and Bob each request the other and cancel (each keeps the other's row unread), Cy requests Ana, then Ana blocks Bob | Every row between the two goes, either way and read or not, no third party's row is touched, and each count drops and is pushed (R4348000; unblocking restores nothing) | `crates/server/tests/api/friends.rs::r4039000_friend_feed::r4348000_a_block_deletes_the_notifications_between_the_two_and_unblock_restores_nothing`, `crates/server/tests/store/contract.rs::r4039000_notifications::r4348000_delete_between_removes_only_the_pairs_rows_either_way`, `crates/server/tests/actor/social_live.rs::r4348000_block_push::r4348000_a_block_that_removes_only_notifications_pushes_both_the_new_count` (new; Ana's frame reads 2 then 1, Bob's 1 then 0) |
| F31 | a share of a match with a friend that already holds it | Reachable: share, then share again (the first row read or not) | 204, with no row, no frame and no toast, so the friend's feed and count stay as they were (R4391000) | `crates/server/tests/api/shares.rs::r4391000_share_limits::r4391000_a_share_already_held_answers_204_and_tells_the_friend_nothing_again` |

## 12. The web's social socket

**Checked at** `0bf5057480`, plus #4550's pull request. Since `ae977ece3e` (`git log ae977ece3e.. -- <its files>`, `bcdc70af1d` being #4291's own pull request): `c4bc8bd2a8` (#4355) rewords `createBackoff`'s comment for the
match sockets' R4250000 reset; the social socket still starts the steps over on its `social` frame (V28), so no
row moved. Checked before at `ae977ece3e`, plus #4291's pull request.

**Files:** `apps/web/src/together/socialSocket.ts` (`openSocialSocket`: `open`, `replace`, `afterClose`, `retry`,
`close`), `apps/web/src/together/socialPoll.ts` (`start`, `stop`, `tick`, `onLive`, `onQuiet`, `openSocket`),
`apps/web/src/game/net.ts` (`createBackoff`, `createWakeProbe`, `letGo`, `quietWhileHidden`). `net/social.ts`
is REST only.

**Fields:** the poll's `pollers` (screens mounted that poll), its `socket` (the client, or none), `reads.live`,
its `refusedToken` (a session the REST read refused) and the tab's visibility and quiet (`HIDDEN_QUIET_MS`,
R3576000); the client's own `socket` (none, connecting, open or closing), `openedWith` (the token it was opened
with), `live` (a `social` frame came), `refusedToken` (a 4401 or 4403's), `waiting` (a drop's backoff step), the
backoff's step and the wake probe's timer (R1441); `disposed`.

**What moves it:** the first poller's mount and the last's unmount; the poll's tick (`SOCIAL_POLL_MS`), whose
`retry` opens a socket nothing waits on and replaces one whose token the session no longer holds; a frame
(`social` turns it live and resets the backoff); a close, by code: a 4401 reopens at once with a token renewed
meanwhile and is otherwise, as a 4403 is, final for its token; anything else (1000, 1001, 1006, 1011, 4429) is a
drop on the backoff;
the factory throwing; a page wake (R1441); the tab hidden, quiet and shown (R3576000); the session renewed,
switched or signed out. A reload starts it from nothing. Its rows are V1 to V28 (V for the social socket's
view), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| V1 | two screens polling, one socket | Reachable: the toast and Play together mounted | One socket for the tab; the last to unmount lets it go (`useSocialPoll`'s doc, R3214000) | `apps/web/src/together/polling.test.tsx` › "R3214000 two screens that poll share one socket, which the last to unmount lets go" (new), › "#1831 the toast and Play together share one social read per SOCIAL_POLL_MS" |
| V2 | no poller | Reachable: V1's last unmount | No socket, no read (`stop`) | V1's test |
| V3 | a socket open, not live | Reachable: before its first `social` frame | The poll still reads; an error frame changes nothing (R3214000) | `apps/web/src/together/socialSocket.test.ts` › "R3214000 offers the session's token on /ws/social and is live from its first social frame" |
| V4 | live | Reachable: the first `social` frame | No poll sent; pushes applied (R3214000) | `polling.test.tsx` › "R3214000 while the connection is up no social poll is sent, and a pushed challenge shows" |
| V5 | a poll's answer arriving once live | Reachable: the mount's read in flight as the socket turns live | **Unspecified**: dropped (the pushed read is newer) | › "R3214000 a poll's answer that comes after the socket turned live is dropped" (new) |
| V6 | a drop by 1001 (a restart's drain) or 1011 (a server fault) | Reachable | A drop: the backoff, as 1006 (R1441, R1115000); **unspecified** for 1011 | `socialSocket.test.ts` › "R3214000 a restart's drain (1001) and a server fault (1011) are drops" (new) |
| V7 | a drop while the wake probe waits | Reachable: a wake past the silence, then a close | The probe cancelled; only the drop's step waits (R1441) | › "R3214000 a drop while the wake probe waits cancels it, and a wake while the drop's step waits opens nothing before the step" (new) |
| V8 | a wake past the silence, an open socket | Reachable | Probed with `hello`; kept if it answers, replaced if not (R1441) | › "R3214000 a woken page probes a quiet socket with hello and replaces it if nothing answers" |
| V9 | a wake while a drop's step waits | Reachable | Nothing opens before the step (R1441) | V7's test |
| V10 | a wake with a socket that never opened, or one the browser is closing | Reachable | Replaced at once, no probe (R1441, as N14) | › "R3214000 a socket that never opened, or is already closing, is replaced at once when the page wakes (R1441)" (new) |
| V11 | a 4401 | Reachable: the handshake's token refused | A renewed token reopens at once; the same one waits for another (R1115000) | › "R3214000 a 4401 reopens at once with a renewed token, and a refusal for the same token waits for another" |
| V12 | a 4403 (an inactive account, or too many open tabs) | Reachable: a sixth tab of one account | Final for its token (R1115000); **unspecified**: a sixth tab opens nothing when another tab closes, until its token is renewed | › "R3214000 4429 is a drop and 4403 is final for its token" |
| V13 | a 4429 | Reachable: past the account's budget | A drop on the backoff | V12's test |
| V14 | a 4404 | Unreachable: the social route closes with 4401, 4403, 1008, 1011 or 1001, never 4404 (`social/live.rs`) | — | — |
| V15 | a close with 1008 (a malformed frame) | Unreachable: the client sends only `hello` (`HELLO`, the probe's) | — | — |
| V16 | the session renewed under a live socket | Reachable: the gate's renewal | The next tick replaces it with one carrying the new token (R3214000); **unspecified**: the replacement's drop to the poll reads once more | `polling.test.tsx` › "R3214000 a renewed session's next tick replaces the socket with one that carries the new token" (new); `socialSocket.test.ts` › "R3214000 a frame after the session's token changed is never shown, and the socket opens again for the new one" |
| V17 | signed out under a mounted poll | Was reachable: the session cleared in another tab or by a refusal | The signed-out session's socket "is replaced" (R3214000, `retry`'s doc); with no token none opens | **#4291: was a mismatch, fixed**: `tick` returned on no token before `retry`, so the old session's socket stayed open and live. It now calls `retry` first. `polling.test.tsx` › "R3214000 a tab signed out under a mounted poll lets its socket go at the next tick, and opens none" (new; failed before the fix: "expected null to deeply equal { code: 1000, …(1) }") |
| V18 | no session at all | Reachable: a signed-out tab | No socket opens, whatever closed the last (R3214000) | `socialSocket.test.ts` › "R3214000 4429 is a drop and 4403 is final for its token" (its last part) |
| V19 | a socket the browser cannot construct | Reachable: a blocked scheme | **Unspecified**: retried on the backoff, as the match socket's N4 | › "R3214000 a socket the browser cannot construct is retried on the backoff" (new) |
| V20 | hidden for less than `HIDDEN_QUIET_MS` | Reachable | The socket kept, the count started over when shown (R3576000) | `polling.test.tsx` › "R3576000 a tab that shows again before HIDDEN_QUIET_MS keeps its socket, and the count starts over" |
| V21 | quiet | Reachable: hidden for `HIDDEN_QUIET_MS` | The socket let go, the poll paused; one opens when shown (R3576000, R3205000) | › "R3576000 a tab hidden for HIDDEN_QUIET_MS lets its socket go and reads nothing, and opens one again when it shows" |
| V22 | quiet with a challenge of the tab's own open | Reachable | The poll goes on (R3205000) | › "R3576000 a quiet tab with a challenge of its own open goes on polling for it (R3205000)" |
| V23 | a drop while hidden | Reachable | No read until shown (R3205000) | › "R3214000 a drop while the tab is hidden reads nothing until it shows (R3205000)" |
| V24 | quiet while a drop's step waits | Reachable: a drop just before the quiet | The step goes with the client; no socket until shown (R3576000) | › "R3576000 a tab that goes quiet while a drop's step waits opens no socket until it shows" (new) |
| V25 | a client while no screen polls | Unreachable: `stop` closes it, and only `start` and `onQuiet` (stopped with the poll) open one | — | — |
| V26 | two clients at once | Unreachable: `openSocket` opens none while one is held, and `onQuiet` lets go of the one held before any other opens | — | — |
| V27 | `live` with no socket | Unreachable: every path that drops the socket (`onclose`, `replace`, the probe's `replace`) sets it down in the same step, `close` clears it, and `onQuiet` tells the poll | — | — |
| V28 | the backoff past its last step | Reachable: drops in a row | The last step repeats; a `social` frame starts the steps over (R1441) | › "R3214000 a dropped socket reopens on the match socket's backoff" |

## 13. The match clock

**Checked at** `0bf5057480`, plus #4550's pull request.

**Files:** `crates/shared/src/host/clock.rs` (`MatchClock`: `sync`, `start_grace`, `clear_grace`, `retry`, `fire`,
`stop`, `arm_ceiling`), `crates/shared/src/host/timer.rs` (`Fx`, `Countdown`), `crates/shared/src/host/snapshot.rs`
(`clock_view_for`) and `crates/shared/src/host/duel.rs` (`new`, `on_arm`, `on_expire`, `advance_run`, `resolve`,
`on_attached`, `on_disconnected`, `on_seats_swapped`, `on_stop`).

**Fields:** the clock's `stopped`; the turn clock's `owner`, `turn_key` and `remaining` (with its timer); the
non-active holder's prompt clock (`holder`, with its timer); the mulligan window's `mulligan_open` and
`mulligan_expired` (with its timer and deadline); each seat's `grace[p]`; the question sequence's `sequence_first` and
`sequence_player` (with its timer); `ceiling` and `ceiling_at`; and, read with them, the host's `finished`, `stopped`,
`run` (an expiry's server actions still owed, logged one row at a time) and `stored_grace` (R744's deadline not yet
handed to the clock). Each timer is a `Countdown`'s id in the state; the host that runs the machine keeps the timer.

**What moves it:** `DuelHost::new` (the ceiling armed from the match's start, then a sync: the mulligan window or the
turn clock); `arm` (a sync, and R744's absent grace in a bot's match); a row written (`after_change`'s sync: the turn
key, the prompt's holder, the window, the sequence); a timer firing (`fire` names the expiry, and `on_expire` derives
its server actions and logs them, R79, R268); the log refusing a server row (R776000's `retry`, a retry later); a
socket arriving or leaving (§9.5's grace, R147, R744); a Glitch's swap (R677); the game ending or `stop` (every timer
taken back); a rebuild (a new host: the ceiling from `created_at`, the rest afresh but a stored grace, R744); a JSON
round trip (the timer ids kept, the timers not). Its rows are T1 to T20 (T for timer), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| T1 | not synced: only the ceiling | Reachable: a match row before its host's first sync (`initial_clocks`); `DuelHost::new` syncs at once, so no host holds it past `new` | Only the ceiling stated (§9.5) | `crates/shared/tests/clock.rs::match_clock::gives_a_fresh_match_row_only_the_ceiling_9_5` |
| T2 | the mulligan window open, no prompt | Reachable: `begin_game`, then `new` | One deadline for both seats, no turn or prompt clock under it, an answer moving nothing (R268) | `crates/shared/tests/clock.rs::match_clock::r268_the_mulligan_clock_runs_one_deadline_for_both_seats_at_once_and_no_turn_clock_runs_under_it`, `…::match_clock::r268_one_seat_answering_neither_re_arms_nor_extends_the_mulligan_clock`, `crates/shared/tests/duel.rs::the_opening::r79_a_new_host_arms_the_ceiling_and_the_mulligan_clock_at_once` |
| T3 | the window run out, its timeouts owed (`run` holding them) | Reachable: the window runs out with both seats owing | One `timeout` for each seat still owing, in seat order, each logged before the next; a stop between them dispatches no more; turn 1 starts its turn clock from full (R268) | `crates/shared/tests/duel.rs::the_opening::r268_the_clock_running_out_times_out_every_seat_still_owing_in_seat_order_and_turn_1_starts`, `…::the_opening::r268_a_seat_that_answered_is_not_timed_out_and_its_answer_does_not_move_the_deadline`, `…::reads_and_stops::r268_a_stop_between_the_mulligan_timeouts_dispatches_no_more_of_them` |
| T4 | another clock (a grace) running out while `run` still owes a timeout | Was reachable: p2 drops during the mulligans, and the store is still writing the first seat's timeout when p2's grace runs out | Every seat still owing gets its timeout ("the actor sends a `timeout` for each player who still owes a mulligan", R268), and the other clock runs out again a retry later (R776000) | **#4550: was a mismatch, fixed**: `on_expire` replaced the run, so the second seat's timeout was lost and, the window's clock spent, the match waited in its mulligans. A clock that runs out while a run still owes actions now runs out again `MATCH_SERVER_ACTION_RETRY_MS` later. `crates/shared/tests/duel.rs::reads_and_stops::r268_a_grace_running_out_between_the_mulligan_timeouts_still_times_out_the_second_seat` (new; failed before the fix with `left: []`, `right: ["srv-mulligan-2"]`) |
| T5 | a setup question the first seat holds (turn 0, the prompt the active seat's) | Reachable: a cast-on-draw card in p1's opening deal asks before the mulligans open (§2.4) | **Unspecified**: R268 calls setup nobody's turn, while R79 governs the active player's prompt by the turn clock. The turn clock times it from full and its expiry is a `Turn` timeout | `crates/shared/tests/clock.rs::match_clock::a_setup_question_the_first_seat_holds_is_timed_by_its_turn_clock` (new); p2's, on its prompt clock: `…::match_clock::r268_a_cards_question_during_setup_is_still_timed_by_r79_on_either_side_of_the_window` |
| T6 | the turn clock running, no prompt | Reachable: any turn | Runs out after `TURN_CLOCK_SECONDS`, naming the active player; a `timeout` logged under `srv-turn-<seq>`; the next seat's clock from full (R79, R270) | `crates/shared/tests/clock.rs::match_clock::r79_expires_the_turn_clock_after_turn_clock_seconds_and_names_the_active_player`, `…::match_clock::resets_the_turn_clock_when_the_turn_changes`, `crates/shared/tests/duel.rs::the_turn_clock::r79_the_active_players_clock_running_out_logs_a_timeout_under_a_server_nonce` |
| T7 | a prompt the non-active player holds | Reachable: a trap's question on the opponent's turn | Its own prompt clock; the turn clock paused with what it had left; only the prompt's clock runs out; the active player's prompt stays on the turn clock (R79) | `crates/shared/tests/clock.rs::match_clock::r79_a_prompt_held_by_the_non_active_player_pauses_the_turn_clock_and_only_its_own_clock_expires`, `…::match_clock::does_not_pause_the_turn_clock_for_a_prompt_held_by_the_active_player` |
| T8 | a second prompt for the same non-active holder, with no sync between that has none | Reachable: the holder answers and the answer's resolution asks it again | **Unspecified**: R79 gives such a prompt "its own prompt clock of 30 s"; the clock keys it by holder alone, so the second keeps the first one's deadline | `crates/shared/tests/clock.rs::match_clock::a_second_prompt_for_the_same_non_active_holder_keeps_the_first_ones_deadline` (new) |
| T9 | a server row the log refused | Reachable: the store fails the row | The clock that ran out runs out again a retry later and the action is derived again under the same nonce; whatever would stop that clock stops the retry (R776000) | `crates/shared/tests/duel.rs::the_turn_clock::r776000_a_row_the_log_would_not_take_runs_the_clock_out_again_and_the_same_nonce_is_retried`, `crates/shared/tests/clock.rs::match_clock::r776000_a_retry_runs_an_expired_turn_clock_out_once_more_and_a_new_turn_cancels_it`, `…::match_clock::r776000_a_retried_prompt_mulligan_or_ceiling_clock_runs_out_once_more`, `…::r776000_retries::r776000_a_retry_for_another_seat_or_a_closed_window_re_arms_nothing` |
| T10 | a clock running out while a seat's own row is in flight | Reachable: the active seat's End turn is still being written as its turn clock runs out | **Unspecified**: the timeout finds the log busy and is answered `NotRecorded`, so the clock runs out again a retry later; the row landing ends the turn, which takes the retry back, and the next seat's clock runs from the row | `crates/shared/tests/duel.rs::the_turn_clock::a_clock_running_out_while_a_seats_row_is_in_flight_is_tried_again_and_a_new_turn_cancels_it` (new) |
| T11 | a seat's grace running | Reachable: a socket drops (§9.5), or the first attach finds the other account not yet here (R744) | The clock keeps running; the grace's end is that seat's loss; a second grace keeps the first deadline (R147); a stored one is kept up to a fresh window (R744); coming back inside it ends it (§9.5) | `crates/shared/tests/duel.rs::being_here::s9_5_a_dropped_socket_starts_the_grace_and_the_opponent_is_told`, `…::being_here::s9_5_the_grace_running_out_is_a_loss_for_the_seat_that_left`, `…::being_here::s9_5_returning_inside_the_window_ends_the_grace_and_greets_the_seat_with_a_fresh_view`, `…::being_here::r744_a_grace_stored_before_a_restart_is_kept_up_to_a_fresh_window`, `crates/shared/tests/clock.rs::match_clock::r147_keeps_the_first_deadline_when_a_second_grace_starts_so_a_flapping_socket_cannot_extend_it` |
| T12 | a seat back while the row of its forfeit is in flight, the row then refused | Reachable: the grace runs out, the store is slow on the forfeit's row, the seat attaches, then the store fails the row | **Unspecified**: R776000 runs the clock out again and stops the retry for "the player back inside the grace"; this seat came back after the grace ran out but before the retry. The retry re-arms a grace one retry long and the seat forfeits though it is here, where a seat back after the refusal clears the retried grace and plays on | `crates/shared/tests/duel.rs::being_here::a_seat_back_before_its_failed_forfeit_is_answered_still_forfeits_on_the_retry` (new) |
| T13 | a Glitch swap while an account is away | Reachable: p2 drops, then a Glitch swaps the seats (R677) | **Unspecified**: R677 moves the accounts to the other seats and says nothing of a running grace; the away account's grace restarts from full on the seat it plays now, so the time it was already away is given back | `crates/shared/tests/duel.rs::scripted::r677_a_swap_moves_the_away_accounts_grace_to_the_seat_it_plays_now` |
| T14 | a question sequence open | Reachable: a card's run of questions | One window from its first question, beside the turn clock; a new first question opens another; its expiry answers Stop for its holder only (R1563006, R1184001) | `crates/shared/tests/clock.rs::r1563006_the_question_sequence_clock::r1563006_a_sequence_runs_out_60_seconds_after_its_first_question_whatever_it_answered_since`, `…::r1563006_the_question_sequence_clock::r1563006_it_runs_beside_the_turn_clock_never_instead_of_it`, `…::r1563006_the_question_sequence_clock::r1563006_the_same_first_question_keeps_its_window_and_a_new_one_opens_another`, `crates/shared/tests/duel.rs::the_sequence::r1563006_a_sequence_expiry_answers_stop_for_the_question_holder_only` |
| T15 | a question met mid-sequence (its streak past 0) with no window open | Reachable: a host made after a sequence's first question (a rebuild) meets a later one | **Unspecified**: R1563006 counts the window from the sequence's first question, which the new host never saw; it opens a full window from the question it first meets | `crates/shared/tests/clock.rs::r1563006_the_question_sequence_clock::a_question_met_mid_sequence_with_no_window_opens_a_full_one` (new) |
| T16 | a host made for a finished match whose game has no result | Reachable: a match ended as a ceiling draw after a reduce panicked (R1253000: no row, so the fold has no result), rebuilt for a socket inside its rematch window | **Unspecified**: only `clock.rs`'s comment says it ("Once a match is over it stays over: a late `sync` must never re-arm a deadline"). `new` arms the ceiling and the turn or mulligan clock, which run out and log nothing (`on_expire` returns for a finished host) | `crates/shared/tests/duel.rs::reads_and_stops::r79_a_finished_matchs_clocks_running_out_log_nothing` |
| T17 | a host made from its JSON on another host, then `arm` | Reachable through the boundary (design rule 3: a caller that keeps `DuelHost` as JSON resumes it on another server); no caller in this repository does, since the server rebuilds from the log | `arm`: "clocks brought up to date" (`crates/shared/README.md`'s event table), so that a match can "resume it on another server" (design rule 3) | **Mismatch, named**: the JSON keeps the old host's timer ids, so `sync` finds each clock armed and sets no timer, and no clock runs out on the new host. `crates/shared/tests/duel.rs::the_checkpoint::a_host_made_from_its_json_elsewhere_is_set_no_clock_timer_by_arm` (new; shows what it does) |
| T18 | a rebuilt host (a new host for a match made earlier) | Reachable: a restart or a let-go, then a socket | The ceiling at what is left of it from the match's start (R79), the mulligan or turn clock afresh (`clock.rs`: "A rebuilt host (§9.5) arms a fresh full deadline"), a stored grace kept (R744) | `crates/shared/tests/duel.rs::the_opening::r79_r268_a_rebuilt_host_arms_the_ceiling_left_and_the_mulligans_afresh_and_the_ceiling_draws` (new), `…::being_here::r744_a_grace_stored_before_a_restart_is_kept_up_to_a_fresh_window` |
| T19 | stopped (the game over, or `stop`) | Reachable: the game ends; `stop` | Every timer taken back; a later sync or timer does nothing (R79, §9.5) | `crates/shared/tests/clock.rs::match_clock::stops_everything_once_the_match_is_over`, `…::match_clock::stop_cancels_every_timer_and_is_idempotent`, `…::match_clock::r268_stop_cancels_a_running_mulligan_clock`, `crates/shared/tests/duel.rs::being_here::stop_takes_every_clock_back_and_later_timers_do_nothing` |
| T20 | the ceiling running out | Reachable: `MATCH_CEILING_MINUTES` after the match's start | One `ceilingReached` row stamped with the active seat, the match a draw (R79, R146, R389) | `crates/shared/tests/clock.rs::match_clock::r79_fires_the_ceiling_once_measured_from_started_at`, `…::r389_the_match_ceiling_doubled_with_the_turn_cap_patch_v0_2_0_b4_3::r389_the_clock_arms_the_ceiling_at_the_configured_minutes_from_the_matchs_start`, T18's test (its `srv-ceiling-1` and the draw) |

T17 is also **representable when serialized**: a `DuelHost` read back holds timer ids that name timers no host keeps.

## 14. The rematch book

**Checked at** `0bf5057480`, plus #4550's pull request.

**Files:** `crates/shared/src/host/rematch.rs` (`RematchBook`: `offer`, `seen`, `release`, `sweep`, `live_entry`;
`in_rematch_window`), `crates/shared/src/host/again.rs` (`in_again_window`, the one window check),
`crates/shared/src/host/boundary.rs` (the book over the boundary, which its caller holds) and
`crates/server/src/api/rematch.rs` (`RematchOffers`, `offer_as`, `rematch_seen`, `sweep_rematch_offers`).

**Fields,** per finished match's id: `offers` (seat to stakes), `leans` (seat to its latest offer's pick), `at` (the
latest offer's moment), `match_id` (the game claimed, or none) and `generation`; the book's `next_generation`; read
with the match's `finished_at` and now.

**What moves it:** `offer` inside the window (an entry made or met; equal stakes claim one game) or past it (it
lapses: an unclaimed entry dropped, a claimed one answers its game); `seen` (a stale unclaimed entry dropped as it is
read); `release(generation)` (a failed create); the reaper's `sweep` (after `reap_finished`, in the same tick); a
restart (the server's book is `App`'s, in memory, and is lost); a JSON round trip (the boundary's caller holds it).
Its rows are O1 to O16 (O for offer), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| O1 | no entry | Reachable: no offer yet; after a stale read or a lapse; after a restart | Nobody has offered (R672). **Unspecified** after a restart: the offers made before it are forgotten, and a game already claimed is no longer found by a late read | `crates/shared/tests/rematch.rs::an_entry_whose_latest_offer_is_stale_is_dropped_when_read_but_one_with_a_game_stays` (the empty book after the stale read); no test restarts the server's book |
| O2 | one seat's offer waiting | Reachable: one offer inside the window | It waits, and the other seat is told (R672, R3213000) | `crates/shared/tests/rematch.rs::r672_two_equal_offers_make_exactly_one_game_with_the_leans_of_both_seats` (its first offer) |
| O3 | equal offers: one game claimed | Reachable: both seats offer equal stakes | Exactly one game, from both seats' leans (R672, R1372, R924000) | the same test |
| O4 | a third offer after the claim | Reachable: a seat offers again once the game is made | It meets the game already made and makes no second (R672) | the same test (its third offer) |
| O5 | unequal stakes | Reachable: a rematch beside a double-or-nothing | Both wait; a changed mind meets the other offer (R672) | `crates/shared/tests/rematch.rs::r672_unequal_stakes_wait_and_a_changed_mind_meets_the_other_offer` |
| O6 | a seat offering again before the pair meets | Reachable: p1 offers with a lean, then again with none, then p2 offers | The seat's latest offer stands (R672's changed mind), its lean with it (`RematchEntry.leans`: "its latest offer's pick", R1372) | `crates/shared/tests/rematch.rs::a_seat_that_offers_again_is_held_to_its_latest_lean` (new) |
| O7 | an offer past the window, no entry | Reachable: a late offer; a finished row with no end | It lapses silently, recording nothing (R2685000); a row with no end is outside the window (R1285000) | `crates/shared/tests/rematch.rs::r2685000_an_offer_past_the_finished_matchs_window_lapses_and_records_nothing` |
| O8 | an offer past the window on an unclaimed entry | Reachable: p1 inside the window, p2 past it | No game, and the entry is dropped (R2685000) | `crates/shared/tests/rematch.rs::r2685000_an_old_offer_cannot_meet_a_new_one_but_a_game_made_inside_the_window_keeps_its_id` |
| O9 | an offer past the window on a claimed entry | Reachable: the pair met inside the window, then a seat offers or polls late | It lapses but answers the game made, so the seat still learns where to go (R2685000) | the same test |
| O10 | a read past the window of an unmet offer under the TTL old | Reachable: p1 offers 5 ms after the end; p2 reads just past the window | **Unspecified**: a read counts staleness from the latest offer, where R2685000 counts the offer window from the match's end, so it still shows an offer nothing can meet, until an offer lapses (dropping it) or the read comes a TTL after the offer | `crates/shared/tests/rematch.rs::a_read_past_the_window_still_shows_an_unmet_offer_until_an_offer_lapses` (new) |
| O11 | an entry stale by its latest offer while the match's window is open | Unreachable: every offer is taken at or after `finished_at`, so inside the window no entry's latest offer is more than the TTL old. **Representable when serialized**: a book read back over the boundary can hold one | — | — |
| O12 | an offer exactly the TTL old | Reachable | Still read, dropped a millisecond later; the window is the TTL inclusive (R2685000) | `crates/shared/tests/rematch.rs::r2685000_an_offer_exactly_the_ttl_old_is_still_read_and_the_book_counts_its_entries`, `…::the_rematch_window_runs_from_the_end_to_the_ttl_inclusive_and_never_without_an_end` |
| O13 | a claimed game whose create failed | Reachable: the store fails the new match | That entry's claim cleared, so the seats offer again and meet; another entry's generation clears nothing (`release`'s doc) | `crates/shared/tests/rematch.rs::a_failed_create_clears_the_claim_of_that_entry_only` |
| O14 | a game claimed at the window's last moment, the reaper's sweep as the window ends | Reachable: both offers at `finished_at + REMATCH_OFFER_TTL_MS`, the reaper's tick a millisecond later (`reap_finished` lets the actor go, then the sweep) | The game survives the sweep, so the one status read the result screen makes after its socket closes finds it (R3694000: "a rematch made in the window's last moment being found that way") | `crates/shared/tests/rematch.rs::r3694000_a_game_claimed_at_the_windows_last_moment_survives_the_sweep_at_its_end` (new) |
| O15 | the sweep | Reachable: the reaper's tick | Drops each entry whose latest offer is past the TTL with no live actor, and keeps the rest (`sweep_rematch_offers`' doc) | `crates/shared/tests/rematch.rs::the_sweep_drops_old_entries_with_no_live_host_and_keeps_the_rest` |
| O16 | the book read back from JSON | Reachable: every boundary call (the caller holds the book) | The same book (design rule 3) | `crates/shared/tests/rematch.rs::the_book_reads_back_from_json` |

## 15. The undo hold

**Checked at** `0bf5057480`, plus #4550's pull request.

**Files:** `apps/web/src/undo/hold.ts` (`useUndoHold`: `release`, `send`, `hold`, `undo`, `sendNow` and its page
listeners) and `apps/web/src/undo/UndoToast.tsx`; its callers `apps/web/src/game/deckbuilder/DeckWorkshop.tsx` and
`apps/web/src/together/useFriendActions.ts`.

**Fields:** `current` (the held deletion), `held` (the same, as the state that draws the toast), `handle` (the
window's timer), `repeat` (the double-click guard's timer) and `timerRef` (the timers it runs on).

**What moves it:** `hold` (refused inside the guard; otherwise it sends any deletion held, holds the new one and
starts the window and the guard); the window's end (sent); the guard's end; Undo (restored, nothing sent); `sendNow`;
`pagehide` and the page turning hidden (sent with keepalive); the screen's unmount (sent). Every send and Undo goes
through `release`, which clears every field at once. A reload or a second tab starts from nothing. Its rows are U1 to
U17 (U for undo), never `R<n>`.

| # | Combination | Reachable (sequence) or not (why) | Expected (source) | Does (test) |
|---|---|---|---|---|
| U1 | mounted, nothing held | Reachable: the screen mounts | No toast, nothing sent (R2119000) | every test's start in `apps/web/src/undo/hold.test.tsx`; U13's test unmounts from it |
| U2 | held, inside the guard | Reachable: a deletion | The item off the screen, the toast's Undo offered, nothing sent until `UNDO_WINDOW_MS` (R2119000) | `apps/web/src/undo/hold.test.tsx` › "R2119000 sends nothing until UNDO_WINDOW_MS has passed, then sends once" |
| U3 | a second hold inside the guard | Reachable: a double click | Refused: nothing sent, held or hidden, and the first stays held with its window (R3059000) | › "R3059000 a second hold inside UNDO_REPEAT_MS is refused: it sends nothing and the first stays held" |
| U4 | a third press after the guard ends, a refused one between | Reachable | Taken: a refused press does not extend the guard (R3059000) | › "R3059000 a refused hold does not extend the guard" |
| U5 | Undo after a refused second hold | Reachable | The one deletion held is restored (R3059000) | › "R3059000 Undo after a refused second hold restores the one deletion held" |
| U6 | held, the guard over, a second hold | Reachable: a deletion `UNDO_REPEAT_MS` or more after the first | The first sent at once, the second held with a window of its own, one toast (R2119000, R3059000) | › "R2119000 a second deletion sends the first at once and takes the toast", › "R3059000 a hold at UNDO_REPEAT_MS sends the first and holds the second" |
| U7 | the window ends | Reachable | Sent once, without keepalive, and the toast goes (R2119000) | U2's test |
| U8 | Undo inside the guard | Reachable: Undo at once | Restored and never sent; the guard ends with the hold, so a deletion right after is taken (R2119000, R3059000) | › "R2119000 Undo puts the item back and sends nothing, then or later", › "R3059000 Undo ends the guard, so a deletion right after it is taken" |
| U9 | `sendNow` inside the guard | Reachable: a new deck at the cap right after a deletion | Sent at once; the guard ends with it, so a deletion right after is taken (R3059000: "a send at once (`sendNow`)") | › "R3059000 a send at once ends the guard, so a deletion right after it is taken" (new) |
| U10 | the page turning hidden while held | Reachable: a tab switch | Sent at once with keepalive (R2119000) | › "R2119000 pagehide, or the page going hidden, sends it at once with keepalive" |
| U11 | hidden, then `pagehide` | Reachable: a closing tab fires both | Sent once (R2119000: "is then sent once") | › "R2119000 the page going hidden and then pagehide, as a closing tab fires them, send it once" (new) |
| U12 | leaving the screen while held | Reachable: the screen unmounts | Sent, with no timer left (R2119000) | › "R2119000 leaving the screen sends it" |
| U13 | leaving the screen with nothing held | Reachable: after Undo, or after the window's end | Nothing more sent (R2119000) | › "R2119000 leaving the screen after Undo, or after the window's end, sends nothing more" (new) |
| U14 | a deletion held with no window running | Unreachable: `hold` sets both in one call, `release` clears both, and the window's own end clears `handle` as it sends, which releases | — | — |
| U15 | `held` and `current` apart | Unreachable: every write sets both in the same call | — | — |
| U16 | the guard running with nothing held | Unreachable: `release` clears the guard with the hold, and the guard (`UNDO_REPEAT_MS`) ends before the window (`UNDO_WINDOW_MS`) could | — | — |
| U17 | the window running with nothing held | Unreachable: `release` clears it | — | — |

Not checked: the decks screen and the friends drawer each run their own `useUndoHold`, so if both can be on screen
at once, two deletions are held at once, where R2119000 has "one is held at a time". No test mounts both.

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
- A queue kept before #3902 has no moment (`QUEUE_SINCE_KEY`): the banner counts from when it mounted
  (L15), `apps/web/src/net/liveGame.test.tsx` › "R3729001 a missing or malformed moment reads as none".
- A co-op match's `team_b` (#3896, R3731002) is `serde(default)` on the machine and on the match's options,
  which write it only for a 2v2, so a row older than the 2v2 has the shape every co-op row still has and
  reads back as the AI team (S13): every co-op rebuild reads one, as
  `crates/server/tests/actor/duos.rs::b40_the_rebuild::b40_a_hello_from_an_account_with_no_seat_folds_nothing_and_a_seats_hello_rebuilds_the_match`.
- A practice record kept before R765 has no `saved` and reads as a game the player was in; one kept
  before R924000's picker has no `leanNewest` and reads as the newest set; one kept before R3976000 has no
  `portraits` and is dealt them again (K2, K17): `apps/web/src/practice/resume.test.ts` › "R765 a game kept
  before R765 carries no flag and reads as one the player was in (R668)", › "R924000 a random deck's pick is
  kept, a game kept before the picker reads as it was, and a bad pick is malformed", › "R3976000 a game is
  kept with its portraits, and portraits naming an unknown one are malformed".
- A live match's checkpoint written by an older server, under other rules or another catalog
  (`checkpoint_version`, `rules`, `catalog_version`) is refused and the whole log folded (C16 is the one at the
  log's head):
  `crates/server/tests/actor/checkpoints.rs::a_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded`.
- The engine's two thread-locals (the shipped catalog a replay folds under, R767000; whether the step in
  flight holds its random pools, R2201001), each set for one call and put back after it (CLAUDE.md
  rule 4).

## For a person

- **The fixes.** S10 (#3678's, and #3899's for the reaper's close), L10, G7 and G9, each with a test
  that failed before it. S10 rests on R3694000's rule and its Rejected; L10 on R897000's "in a row" and
  `/play`'s own count; G7 and G9 on #2924's drop at the turn change and R658's "drag stays two clicks".
  If any reads as a decision instead, it needs a ruling in a pull request of its own. #4114's: N15 (both
  socket clients forget a 4401's refusal once a socket opens after it, so a renewal never opens a socket
  from `elsewhere` or onto a gone match; R1115000's "a refusal is its handshake's", R3767000, R679) and
  A10, A10b and A10c, in both registries (a match that `start` or a rebuild makes once the restart
  drain has begun, a rebuild in flight across it or begun after it among them, is checkpointed and
  stopped as the drain's are; the server README's drain line and `stop_all`'s doc). The co-op A10c has
  no production path found; the mark guards it anyway.
- **Mismatches named, not fixed:** K9b (a practice save of another deck on the same seed, seat,
  difficulty and sandbox resumes, where R668 names the deck in the page's setup: `host.rs`'s `resume`
  never compares it; comparing it must first say how an older record's deck, normalised on read, compares).
- **Unspecified:** D7 (an abandoned run keeps its HP, while a lost fight drops it to 0); L9 and L12 (a
  queued tab with no session, or answered signed out, forgets its queue); L11 (a failed read neither
  counts nor breaks R897000's row); G4 and G15 (a selection, and a drag, kept when a prompt opens in the
  same turn, so a stale attack is sent under the prompt); G8 (a drag let go while a burst plays sends
  what a click then could not: R658 may call it a mismatch); G13 (a second tap while the answered prompt
  fades sends the answer again); G21 (a reconnect's view does not drain the runner, though `drain()`'s
  doc says it does). S12 (two tabs of one account on one seat) was decided in R3767000. #4114's: N4 (a
  socket the browser cannot construct is retried on the backoff with a transport error); the Reconnect
  button after a 4401 offers the refused token again (`connect()`); a co-op client with no token at all
  is refused and a later `setToken` opens nothing, which only a client made without `useDuosMatch` meets
  (the hook gives the token before it connects); after a 4401, `setToken` keeps the refusal's error in
  the 1v1 client until a view clears it and clears it at once in the co-op one; A7 (a rebuild whose
  caller is cancelled leaves its cell behind, and the cell hands its actor, a stopped one after a `stop`,
  to later callers; only the bot loop's abort in the drain cancels one); A9 (`start` and a rebuild of one
  id at once leave two actors); A11 (a rebuild across `stop`, a reap or `forget` inserts what it folded);
  K13 (a full localStorage keeps the previous record; a refused IndexedDB write resumes one answer back);
  K19 to K21 (two tabs, and a failing engine or IndexedDB, clear another game's record or a good one).
- **#4291's fixes,** each with a test that failed before it: V17 (a tab signed out under a mounted poll kept the
  old session's social socket open and live: R3214000's "is replaced", `retry`'s doc), F20 (the panel's opening
  sent no mark when the page's rows were read but an older one was not: R4039001's "marks every row up to its
  newest one read") and F10 (a seat of the match the other seat shared a replay with read it without marking its
  row: R4233000's "a friend's read of a replay shared with them") and C13 (a Postgres checkpoint put while the
  match's ending was uncommitted outlived the ending: §9.3's "a match that ends drops its checkpoint"). If any
  reads as a decision instead, it needs a ruling in a pull request of its own.
- **#4291's unspecified:** C16 (a refused checkpoint at the log's head outlasts the drain's good one), C17 (a
  periodic checkpoint keeps the live `clockLeftMs` readings), F12 (an accept's answer stays unread when its board
  is read), F13 and F14 (a challenge or friend request cancelled or lapsed keeps counting), F16 (an entered invite
  leaves Recent once its lobby closes), F17 (an invite into a full lobby still counts), F23 (a held toast shows
  once the board goes, read or not), V5 (a poll's answer after live is dropped), V6's 1011, V12 (a sixth tab's
  4403 waits for a renewed token), V16 (a renewal's replacement reads once more), V19 (a socket the browser
  cannot construct is retried on the backoff), C10′ (a periodic checkpoint whose put is in flight when `stop`
  lands is written, though `checkpoint`'s doc says `stop` writes none).
- **#4550's fix,** with a test that failed before it: T4 (a grace running out while the first mulligan timeout's
  row was in flight replaced the run, so the second seat's timeout was lost and the match waited in its mulligans:
  R268's "a `timeout` for each player who still owes a mulligan", R776000's retry). If it reads as a decision
  instead, it needs a ruling in a pull request of its own.
- **#4550's mismatch named, not fixed:** T17 (a `DuelHost` made from its JSON on another host gets no clock timer
  from `arm`, where the shared README's `arm` row says "clocks brought up to date" and design rule 3 has a match
  resume on another server; a fix changes what `arm` emits for every match the server arms).
- **#4550's unspecified:** S16 (a socket that closes as it is seated sends the held tab to `elsewhere` with nobody on
  the seat), T5 (the first seat's setup question runs on its turn clock), T8 (a second prompt for the same holder
  keeps the first one's deadline), T10 (a clock running out with the seat's own row in flight is retried), T12 (a
  seat back while its forfeit's row is in flight still forfeits on the retry, where one back after the refusal
  does not), T13 (a swap gives the away account a fresh grace), T15 (a host that meets a sequence mid-way opens a
  full window), T16 (a host made for a finished match with no result arms clocks that log nothing), O1 (a restart
  forgets every rematch offer and claim) and O10 (a read past the window still shows an offer nothing can meet).
- **#4550's not checked:** the decks screen and the friends drawer each run their own `useUndoHold` (§15), so two
  deletions may be held at once, where R2119000 has "one is held at a time"; no test mounts both.
- **Not shown:** E5′ (mulligans open with work owed); W10 (a duplicated tab: jsdom cannot copy a tab).
- **Representable when serialized:** C7 (a checkpoint ahead of its log) and a checkpoint row on a finished
  match (the row's shape allows both; the store's guards and `verify` keep them out), F29 (a `notification` frame
  carries `read`, which a pushed row always has false), and `NotificationRow`'s `kind` beside a free `subject`
  and `payload` (nothing in the row ties a kind to its subject's shape); E7 to E10 (`GameState`), D3 and D5 (`RunState`), K11 (a practice
  save edited in IndexedDB), and a practice record with `run`, `aiBanned` or `lastBoard` (`resume.ts`'s
  `isConfig` checks no extra keys; with `run`, a resume would read the run fight's slot); T17 (a `DuelHost` read back keeps timer ids no host holds) and O11
  (a `RematchBook` read back with an entry stale inside its window).
