# JackiOh AI ladder (issue #55, phases 1–3)

A self-perpetuating training ladder: an LLM proposes an AI, the pipeline
trains it, a gauntlet gates it, and passing candidates become champions.
Phases 1–3 (this PR) are the repository code: the seeded arena, the audit and
gates, and the LLM adapter plus proposer. Phases 4–5 (the workflows) are
written by a person from the last section below.

## Layout

- `config.yaml` — every threshold (games, win counts, shadowban numbers,
  budgets). Change values here, never in code.
- `DECISIONS.md` — each judgment call the issue left open, with the rejected
  alternative.
- `arena/` — `bridge.py` (stdio client for the Node bridge), `runner.py`
  (seeded games, 50/50 seat alternation, per-decision logs), `types.py`
  (the `Agent` contract).
- `agents/random/` — the reference bot: uniform over `legal` minus resigning
  moves, deterministic in its seed.
- `audit/` — `shadowban.py` (the issue's definition) and `gates.py`
  (promotion needs every applicable gate).
- `llm/` — `client.py` (`LLMClient` plus env configuration) and
  `providers/` (`anthropic`, `openai`, `google`, `mock`). Provider code lives
  only there.
- `proposer/` — context bundle, lenient parsing, strict schema validation,
  retries with error feedback, history entries.
- `prompts/proposer.md` — the provider-neutral proposer prompt.
- `schemas/spec.schema.json` — what a proposal must match.
- `champions/1`, `champions/2`, `hall_of_fame/`, `proposals/`, `history/` —
  filled by the pipeline; `history/proposals.jsonl` records provider, model,
  spec summary, results and failure reasons per generation.

The engine side is `packages/ai/scripts/arena-bridge.ts`: one Node process
speaking JSON lines (`new_game`, `legal`, `observe`, `act`, `result`,
`close`, `deck`, `cards`), holding games in a map. Agents see only `viewFor`
for their seat, never the true state. It is tooling in `scripts/`, not a rule
change: no game rule or card behaviour changed.

## Run the arena locally

Python 3.10+, `pyyaml` and `jsonschema` installed. From `ladder/`:

```
python -m pytest -q            # all ladder tests (spawns the bridge; needs pnpm install)
```

One series in code:

```python
from arena.bridge import Bridge
from arena.runner import play_series
from agents.random.agent import RandomAgent

seeds = [f"gen-1:{i}" for i in range(100)]  # one fresh set per generation, every opponent
with Bridge() as bridge:
    report = play_series(bridge, RandomAgent("cand"), RandomAgent("opp"), seeds)
print(report["wins"], "/", report["games"])
```

## Read results

- A game log holds `seed`, `winner`, `reason`, `forfeit` (if any),
  `decisions` (each: `seat`, `legal_plays`, `played`, `action`, `specialist`),
  `decks`, `built_by_agent` and the replay `hash`.
- `audit_candidate(logs, card_pool, config)` returns `shadowbanned`,
  `unobserved` and per-card `stats` (`opportunities`, `plays`, `use_rate`).
  It counts only the candidate's own decisions: `play_series` stamps each log
  with `candidate_seat`, and stamped logs ignore the other seat's decisions.
- `evaluate_gates(wins_vs_champions, wins_vs_random, candidate_shadowbanned,
  champion_shadowbanned, pool_size, config)` returns `promoted` and the
  `failures` that blocked it, if any.
- `history/proposals.jsonl` has one JSON object per generation: timestamp,
  provider, model, spec summary, results, pass/fail and failure reasons.

Note: this audit's shadowban (a candidate that never plays a card it could)
is not the R186 sweep (`pnpm ai:sweep`, `packages/ai/src/shadowBan.ts`),
which measures cards the shipped AI misplay. Different question, different
list.

## Switch providers

No code change: set repo variables and secrets only.

| Name | Kind | Value |
|---|---|---|
| `LLM_PROVIDER` | variable | `anthropic` \| `openai` \| `google` (`mock` for tests/CI) |
| `LLM_MODEL` | variable | model id passed to the provider |
| `LLM_API_KEY` | secret | provider credential (`mock` needs none) |

A missing or invalid setting fails loudly naming the variable. Provider SDKs
are not used (stdlib HTTP), and transport code stays in `llm/providers/`.

## Adjust the config

Edit `ladder/config.yaml`: game counts, win thresholds, the shadowban
fraction and definition numbers, the training budget, the proposer's retries,
context cap, history depth and token limit, hall-of-fame games, and the
balance-flag streak. `agent_builds_decks: auto` follows the arena logs;
`true`/`false` overrides it.

## What phases 4–5 need from a person

The workflows (`.github/workflows/generation.yml` plus the card-pool-change
workflow), which the bot may not write, and the `LLM_API_KEY` secret.

- **Pipeline stages** (`generation.yml`): `propose` (build bundle, call
  `LLMClient`, write `proposals/gen-N/`), `train` (within
  `train_budget_minutes`; opponents: champions, random bot, self-play),
  `gauntlet` (matrix over champion1/champion2/random, skipping empty slots;
  100 games each, seats 50/50, one fresh seed set per generation for every
  opponent), `audit` (win counts, shadowban list, unobserved cards, specialist
  fire counts, plus non-gating hall-of-fame games; writes `results.json`),
  `promote` (apply gates; on pass rotate champions, commit, publish Release
  `gen-N` with weights and `results.json`), `retro` (`if: always()`; append
  `history/proposals.jsonl`, write the step summary, file issues).
- **Triggers**: `workflow_run` on `generation` completed (self-retrigger),
  `schedule` every 6 hours, `workflow_dispatch`; one `concurrency` group
  `generation` with `cancel-in-progress: false`.
- **Security**: `propose`, `train`, `gauntlet`, `audit` run with
  `contents: read`; only `propose` gets `LLM_API_KEY`, the other three get no
  secrets. `promote` and `retro` get `contents: write` and `issues: write`
  and never import or execute candidate code (they read only `results.json`).
  A missing/invalid provider configuration fails naming the variable.
- **Card-pool workflow**: triggers on card data changes; re-runs the audit
  for current champions against the new pool, updates stored shadowban
  counts, resets the baseline, demotes nobody.
- **Issue filing** (from `retro`, deduplicated by card ID or error
  signature; candidate failures go to history, never issues):
  `pipeline-failure` (stage, error, log excerpt, run URL),
  `bug` (seed, game log, reproduction command),
  `balance-review` (human-only: a card shadowbanned by, or a router trigger
  for, `balance_flag_streak` consecutive champions).
