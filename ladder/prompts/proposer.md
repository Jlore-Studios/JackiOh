# Proposer prompt (provider-neutral)

You design one contender for the JackiOh AI ladder: an agent that plays the
card game through the `Agent` contract below. A training stage will train it
within its budget, then a gauntlet plays 100 games each against the current
champions and the random bot, with seats alternated 50/50. Promotion needs
wins against every champion, wins against the random bot, and fewer
shadowbanned cards (cards it almost never plays when it could) than the
reigning champion.

## Agent interface

```python
class Agent:
    def act(self, obs: Observation, legal: list[Action]) -> Action: ...
```

- `obs` is the engine's `viewFor` for your seat: you see only what that seat
  may know. You never see hidden cards.
- `legal` is computed by the engine. Returning an action outside it forfeits
  the game.
- Be deterministic given a seed: construct all randomness from the seed you
  are given.

## Proposal types

- **RL agents**: a trained policy behind `act`. Pick PPO with action masking,
  DQN, or MCTS + value net, or write a variant, and say which.
- **Multi-context agents**: a generalist plus N specialists, with a router
  choosing which one acts. Declare each router trigger as conditions on the
  observation: `card_on_board: <card_id>`, `own_life_below: N`,
  `card_in_hand: <card_id>`.

## Reply format

Reply with exactly two parts, in this order:

1. The `spec.yaml` matching the schema given in the context (YAML; you may
   wrap it in a ```yaml fence and surround it with prose, which is stripped).
2. One code block per file in `files`, each delimited as:

```
--- FILE: <path> ---
<code>
--- END FILE ---
```

Every `path` in `files` needs exactly one block. Code fences around the blocks
are acceptable.

An invalid spec is sent back with its error for another try, up to the retry
limit, so match the schema exactly.
