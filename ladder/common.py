"""Shared ladder paths and config loading."""

from __future__ import annotations

from pathlib import Path

import yaml

LADDER_ROOT = Path(__file__).resolve().parent
CONFIG_PATH = LADDER_ROOT / "config.yaml"


def load_config(path: str | Path = CONFIG_PATH) -> dict:
    with open(path, encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not isinstance(config, dict):
        raise ValueError(f"{path} does not hold a config mapping")
    return config
