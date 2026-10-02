"""The exceptions the harness raises on purpose."""

from __future__ import annotations


class HarnessError(Exception):
    """Base class for every error the harness raises on purpose."""


class ConfigError(HarnessError):
    """The committed config or the environment is unusable."""


class GitHubError(HarnessError):
    """A GitHub request failed. `status` is the HTTP status, or 0 when none arrived."""

    def __init__(self, message: str, status: int = 0, body: object = None) -> None:
        super().__init__(message)
        self.status = status
        self.body = body


class GitError(HarnessError):
    """A git command failed."""


class StateConflict(HarnessError):
    """The state file changed under an update more times than the retry allows."""


class LoginError(HarnessError):
    """A provider's secret could not be turned into a login its CLI reads."""
