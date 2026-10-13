# Bot calibration: skill, Elo, and the data behind the decks

How the ladder bots' strength is measured rather than guessed (issues #747–#750, part of #636).

## The skill parameter (R747000)

One AI ships, so weaker bots degrade it: `skill` in [0, 1] is the share of decisions that follow
the best AI, the rest SPEC §10.7's random policy. 1.0 is the reference (bit-for-bit the old play);
0.0 is fully random. The roll draws on the caller's seeded stream and knows no cards or sets.

## The skill-to-Elo function

`skill_expected_elo(skill, reference_elo)` (`crates/ai/src/skill.rs`) is the reference's rating
minus a piecewise-linear offset over `SKILL_ELO_OFFSETS`. The table below is provisional until
the arena ladder below measures it:

| skill | expected Elo below reference |
|---|---|
| 1.0 | 0 |
| 0.9 | 60 |
| 0.75 | 150 |
| 0.5 | 320 |
| 0.25 | 500 |
| 0.0 | 720 |

### Measuring the curve (re-run whenever the best AI is updated)

Play each skill against the reference (skill 1.0) at full budget, alternating seats, 30+ games
per skill (the same bar #636 sets for calibration games):

```sh
cargo jackioh arena --a self --b self --skill-b 0.75 --games 40 --seed calibrate-<date> --out /tmp/skill-ladder
```

Score each ladder rung from a's side (wins + draws/2 over finished games), convert with
`elo_delta_from_score`, and paste the offsets into `SKILL_ELO_OFFSETS` by hand (CLAUDE.md rule 9:
measured numbers are pasted, never auto-written). Then set each `BOT_STRENGTHS` skill from the
curve so its seed rating matches the Elo its skill earns, and re-run the focused tests:
`cargo test -p jackioh-ai skill`, `cargo test -p jackioh-server bots::`, `cargo test -p jackioh-tools arena`.

The reference anchors to the top strength's live rating: `bot_ratings` (Glicko, R610) move with
every rated game, so the curve's absolute position self-corrects between calibrations. The
ceiling gate needs no tuning: `choose_bot` never seats a player rated above every bot.

## Farming the data (R749000)

The gather lane (`training/gather.md`, `loop.sh gather`, `jackioh-train@gather` on the box) plays
best-AI versus best-AI random-deck games continuously, one `cargo jackioh stats` series per UTC
day under the named patch, and `stats-import` loads each finished day. Per-card figures (play
rate, win rate in-deck, win rate when played, played delta) come from `card_stats` over those
records (`stats-cards --source=dev`) and over live games (`--source=live`, the default).

The public figures split three ways: merged on display, `?pilot=human` for the people's sides,
`?pilot=ai` for the bots'. Bot decks for higher Elo weight toward high-win-rate cards from these
tables (#750); nothing in the weighting names a card or a set.
