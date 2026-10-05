# Ladder decisions (issue #55)

One line per judgment call the issue did not settle, with the alternative rejected.

- Python + a long-lived Node bridge, because the engine is TypeScript; one process per run (not per game or decision) or the gauntlet would take hours.
- `legal` handed to agents is the engine's raw `legalActions`; a forfeit is judged against exactly what was sent. Rejected: filtering first, which would let agents play actions they never saw.
- The random bot skips `concede`/`offerDraw`/`answerDraw` (the engine's `AI_SKIPPED_ACTIONS`, SPEC §10.7, R84) and draws uniformly over the rest with no end-turn bias. Rejected: uniform over raw `legal`, which resigns coin-flip turns (`{endTurn, concede}`) and is no baseline.
- An agent whose `act` raises forfeits (`reason: threw`); only bridge failures are arena crashes. Rejected: treating candidate-code crashes as pipeline failures, which would file issues for bad proposals instead of history entries.
- Decks are dealt by the arena from seed-derived streams (`built_by_agent: false`); `agent_builds_decks: auto` reads that flag. Rejected: letting agents draft in phase 1, which the interface does not need yet.
- Providers speak their REST APIs over the standard library; no provider SDK is installed or imported. Rejected: SDK dependencies, which add install weight for three interchangeable HTTP calls. Confinement still holds: transport code lives only in `ladder/llm/providers/`.
- `max_retries: 3` means 1 try plus 3 retries (4 attempts total), then the last error raises.
- Over `max_context_chars`, the oldest history entries drop first, then the card list trims to history-referenced plus flagged cards; interface, schema and champions always survive. Rejected: truncating the bundle blindly, which could cut the schema the model must match.
- `spec.schema.json` forbids unknown fields (`additionalProperties: false`): parse leniently, validate strictly, as the issue orders it.
- A use rate exactly at `max_use_rate` is not shadowbanned (the definition bans strictly below it).
- Multi-context specialists are logged from the agent's `last_specialist` attribute read after each `act`; agents without one log `null`. Rejected: a wider return type, which would break the single-contract interface.
- `ladder/` is outside the pnpm workspace and `pnpm test`: Python 3.10+ with `pyyaml` and `jsonschema`, tests via `python -m pytest` from `ladder/`. Rejected: folding Python into the root install, which would slow every JS-only workflow.
- The audit counts only the candidate seat's decisions (`candidate_seat` stamp from `play_series`), because a game log holds both seats and the opponent's habits are not the candidate's. Rejected: caller-side pre-filtering, which every future caller would have to remember.
- DefIds resolve off the view's `instanceId` key (hand and graveyard, the zones `play` actions name), never a bare `id`, which card views do not carry.
