"""Shared setup: a Config for tests and a Context around a FakeGitHub."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from harness import config as config_mod
from harness.context import Context
from harness.state import StateStore
from harness.trust import Trust

from tests.fakes import FakeGitHub

ROOT = config_mod.REPO_ROOT
# 2026-09-30 03:00 UTC is 22:00 CDT on the 29th: inside the night window.
NIGHT = datetime(2026, 9, 30, 3, 0, tzinfo=timezone.utc)
# 2026-09-29 17:00 UTC is 12:00 CDT: outside it.
DAY = datetime(2026, 9, 29, 17, 0, tzinfo=timezone.utc)

#: Only the first Claude account's secret is set, as before there were several subscriptions;
#: tests of the others set `HARNESS_SECRETS_SET` themselves.
TEST_ENV = {"GITHUB_RUN_ID": "777", "GITHUB_REPOSITORY": "jgoetzmann/JackiOh",
            "HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN"}


def raw_config() -> dict[str, Any]:
    return json.loads((ROOT / config_mod.CONFIG_PATH).read_text(encoding="utf-8"))


def make_config(root: Path | None = None, env: dict[str, str] | None = None, **overrides: Any):
    raw = raw_config()
    raw.update(overrides)
    return config_mod.parse(raw, root or ROOT, {**TEST_ENV, **(env or {})})


class Clock:
    def __init__(self, at: datetime = NIGHT) -> None:
        self.at = at

    def __call__(self) -> datetime:
        return self.at


def make_ctx(gh: FakeGitHub | None = None, *, at: datetime = NIGHT, cfg=None,
             trust_text: str | None = None) -> Context:
    gh = gh or FakeGitHub()
    cfg = cfg or make_config()
    trust = Trust.parse(trust_text) if trust_text is not None else Trust.load(ROOT / config_mod.TRUST_PATH)
    return Context(cfg=cfg, gh=gh, store=StateStore(gh), trust=trust, clock_fn=Clock(at))
