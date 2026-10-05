"""Mock provider: a fixed valid proposal. Tests and CI use it, so the pipeline
is testable without credentials (and the mock needs no API key)."""

from __future__ import annotations

PROVIDER_NAME = "mock"

MOCK_SPEC_YAML = """name: mock-contender
description: Fixed valid proposal for tests and CI.
agent:
  kind: rl
  algorithm: ppo_masked
files:
  - path: agent.py
    purpose: Seeded heuristic behind the Agent contract.
training:
  opponents: [random, self-play]
  budget_minutes: 10
"""

MOCK_AGENT_PY = '''"""Mock contender: plays the first legal action. Deterministic, no learning."""

from arena.types import Action, Observation


class MockAgent:
    def __init__(self, seed="mock"):
        self.last_specialist = None

    def act(self, obs: Observation, legal: list[Action]) -> Action:
        return legal[0]
'''

MOCK_PROPOSAL = f"""```yaml
{MOCK_SPEC_YAML}```

--- FILE: agent.py ---
{MOCK_AGENT_PY}--- END FILE ---
"""


def complete(model: str, api_key: str, system: str, messages: list[dict], max_tokens: int) -> str:
    _ = (model, api_key, system, messages, max_tokens)
    return MOCK_PROPOSAL
