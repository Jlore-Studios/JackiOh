"""What every command works with: config, a GitHub client, the state store, the trust list."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from typing import Any, Callable

from harness import clock
from harness.config import HALT_PATH, NIGHT_WORKFLOW, TRUST_PATH, Config
from harness.gh import GitHub
from harness.state import StateStore
from harness.trust import Trust


@dataclass
class Context:
    cfg: Config
    gh: Any
    store: StateStore
    trust: Trust
    clock_fn: Callable[[], datetime]
    #: The client for Actions calls (starting a run, re-running CI jobs). In a workflow it holds
    #: the job's own token, which has `actions: write`, so the bot's token needs no `repo` scope.
    actions_gh: Any = None

    @property
    def act(self) -> Any:
        return self.actions_gh if self.actions_gh is not None else self.gh

    def now(self) -> datetime:
        return self.clock_fn()

    @property
    def window(self) -> clock.Window:
        return clock.Window.of(self.cfg.timezone, self.cfg.window_start, self.cfg.window_end)

    def repo_halted(self) -> bool:
        """True when `.harness/HALT` exists on the default branch."""
        text, _ = self.gh.get_file(HALT_PATH, self.cfg.default_branch)
        return text is not None

    def dispatch(self, *, item: int | None = None, force: bool = False, mode: str = "auto") -> None:
        """Start a night run now through workflow_dispatch."""
        inputs = {"item": str(item or ""), "force": "true" if force else "false", "mode": mode}
        self.act.dispatch_workflow(NIGHT_WORKFLOW, self.cfg.default_branch, inputs)


def build(cfg: Config, *, token: str | None = None, gh: Any = None,
          clock_fn: Callable[[], datetime] | None = None) -> Context:
    client = gh if gh is not None else GitHub(
        cfg.repo, cfg.write_token if token is None else token, dry_run=cfg.dry_run
    )
    actions = None
    if gh is None and token is None and cfg.bot_token and cfg.actions_token:
        actions = GitHub(cfg.repo, cfg.actions_token, dry_run=cfg.dry_run)
    return Context(
        cfg=cfg,
        gh=client,
        store=StateStore(client),
        trust=Trust.load(cfg.root / TRUST_PATH),
        clock_fn=clock_fn or (lambda: clock.now(cfg.now_override)),
        actions_gh=actions,
    )
