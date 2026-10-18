"""Positions files in, arrays out (#1009).

A positions file is what `cargo jackioh positions` writes (R1006004): a header line naming
`featuresVersion` and `denseNames`, then one row per position. `load` reads any number of them into
one `Positions`: the dense features as a matrix, the sparse ones as CSR (every key a string in
`keys`, each row's key ids in `sparse_idx[sparse_ptr[i]:sparse_ptr[i + 1]]` with their counts), the
hand eval's score, the outcome, the turn and the game each row came from.

The feature length comes from the data, and every file must hold the same `FEATURES_VERSION` and
dense names (D10): a mismatch is refused, never mixed. A row whose dense length differs from its
header's is refused too.

`split_by_game` holds a share of the games out whole, by a hash of the game's seed, so one game's
positions never sit on both sides of the split. `Vocabulary` keeps the sparse keys seen often enough
in the training games, and maps every other key to nothing: a card the weights do not know is valued
by its dense stats alone (D5).
"""

from __future__ import annotations

import hashlib
import json
from array import array
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

import numpy as np

#: Rows of dense features read before they are packed into an array.
CHUNK_ROWS = 20_000


class FeatureMismatch(ValueError):
    """Files or rows that do not share one feature layout."""


@dataclass
class Positions:
    features_version: int
    dense_names: list[str]
    dense: np.ndarray  # float32, rows x len(dense_names)
    hand: np.ndarray  # float64
    outcome: np.ndarray  # float32: 1, 0.5 or 0
    turn: np.ndarray  # int32
    game: np.ndarray  # int64: an index into `games`
    games: list[str]
    keys: list[str]
    sparse_ptr: np.ndarray  # int64, rows + 1
    sparse_idx: np.ndarray  # int64: ids into `keys`
    sparse_val: np.ndarray  # float32: counts
    catalogs: set[str] = field(default_factory=set)

    def __len__(self) -> int:
        return len(self.outcome)

    def take(self, rows: np.ndarray) -> "Positions":
        """The rows `rows` (indices, in order) as a Positions of their own, sharing `keys` and `games`."""
        rows = np.asarray(rows, dtype=np.int64)
        starts, ends = self.sparse_ptr[rows], self.sparse_ptr[rows + 1]
        lengths = ends - starts
        ptr = np.zeros(len(rows) + 1, dtype=np.int64)
        np.cumsum(lengths, out=ptr[1:])
        # Entry j of the new rows is entry j - ptr[row] of its old row, which starts at `starts`.
        gather = np.repeat(starts - ptr[:-1], lengths) + np.arange(ptr[-1], dtype=np.int64)
        return Positions(
            features_version=self.features_version,
            dense_names=self.dense_names,
            dense=self.dense[rows],
            hand=self.hand[rows],
            outcome=self.outcome[rows],
            turn=self.turn[rows],
            game=self.game[rows],
            games=self.games,
            keys=self.keys,
            sparse_ptr=ptr,
            sparse_idx=self.sparse_idx[gather],
            sparse_val=self.sparse_val[gather],
            catalogs=self.catalogs,
        )


def load(paths: Iterable[str | Path], expect_version: int | None = None) -> Positions:
    """Every row of `paths`, in order. `expect_version`, when given, must be the files' version too."""
    version: int | None = None
    names: list[str] | None = None
    key_ids: dict[str, int] = {}
    keys: list[str] = []
    game_ids: dict[str, int] = {}
    games: list[str] = []
    # Compact buffers: a million rows of Python lists would not fit beside the arrays.
    dense_chunks: list[np.ndarray] = []
    dense: list[list[int]] = []
    hand = array("d")
    outcome = array("f")
    turn = array("i")
    game = array("q")
    ptr = array("q", [0])
    idx = array("q")
    val = array("f")
    catalogs: set[str] = set()

    for path in paths:
        path = Path(path)
        with path.open(encoding="utf-8") as handle:
            header_line = handle.readline()
            if not header_line.strip():
                raise FeatureMismatch(f"{path}: empty, no header")
            header = json.loads(header_line)
            if "denseNames" not in header:
                raise FeatureMismatch(f"{path}: the first line is not a positions header")
            file_version = int(header["featuresVersion"])
            if version is None:
                version, names = file_version, list(header["denseNames"])
                if expect_version is not None and version != expect_version:
                    raise FeatureMismatch(f"{path}: FEATURES_VERSION {version}, not the expected {expect_version}")
            elif file_version != version or list(header["denseNames"]) != names:
                raise FeatureMismatch(f"{path}: FEATURES_VERSION {file_version} and its names differ from the first file's ({version})")
            width = len(names)
            for number, line in enumerate(handle, start=2):
                if not line.strip():
                    continue
                row = json.loads(line)
                if int(row["featuresVersion"]) != version:
                    raise FeatureMismatch(f"{path}:{number}: a row of FEATURES_VERSION {row['featuresVersion']} under a header of {version}")
                if len(row["dense"]) != width:
                    raise FeatureMismatch(f"{path}:{number}: {len(row['dense'])} dense features, the header names {width}")
                dense.append(row["dense"])
                if len(dense) >= CHUNK_ROWS:
                    dense_chunks.append(np.asarray(dense, dtype=np.float32))
                    dense = []
                hand.append(float(row["hand"]))
                outcome.append(float(row["outcome"]))
                turn.append(int(row["turn"]))
                seed = row["game"]
                if seed not in game_ids:
                    game_ids[seed] = len(games)
                    games.append(seed)
                game.append(game_ids[seed])
                catalogs.add(row.get("catalog", ""))
                for key, count in row["sparse"]:
                    at = key_ids.get(key)
                    if at is None:
                        at = key_ids[key] = len(keys)
                        keys.append(key)
                    idx.append(at)
                    val.append(float(count))
                ptr.append(len(idx))

    if version is None or names is None:
        raise FeatureMismatch("no positions files")
    if dense:
        dense_chunks.append(np.asarray(dense, dtype=np.float32))
    matrix = np.concatenate(dense_chunks) if dense_chunks else np.zeros((0, len(names)), dtype=np.float32)
    return Positions(
        features_version=version,
        dense_names=names,
        dense=matrix.reshape(len(matrix), len(names)),
        hand=np.frombuffer(hand, dtype=np.float64).copy(),
        outcome=np.frombuffer(outcome, dtype=np.float32).copy(),
        turn=np.frombuffer(turn, dtype=np.int32).copy(),
        game=np.frombuffer(game, dtype=np.int64).copy(),
        games=games,
        keys=keys,
        sparse_ptr=np.frombuffer(ptr, dtype=np.int64).copy(),
        sparse_idx=np.frombuffer(idx, dtype=np.int64).copy(),
        sparse_val=np.frombuffer(val, dtype=np.float32).copy(),
        catalogs=catalogs,
    )


def game_bucket(seed: str, buckets: int = 1000) -> int:
    """A game's place in [0, buckets), from a hash of its seed: the same on every machine and run."""
    return int.from_bytes(hashlib.sha256(seed.encode("utf-8")).digest()[:8], "big") % buckets


def split_by_game(positions: Positions, held_out: float = 0.1) -> tuple[np.ndarray, np.ndarray]:
    """Row indices of the training and held-out games: `held_out` of the games, whole."""
    cut = int(round(held_out * 1000))
    held = np.array([game_bucket(seed) < cut for seed in positions.games], dtype=bool)
    rows_held = held[positions.game]
    return np.flatnonzero(~rows_held), np.flatnonzero(rows_held)


def games_of(positions: Positions, rows: np.ndarray) -> list[int]:
    """The distinct games of `rows`, in the order they first appear."""
    games, first = np.unique(positions.game[rows], return_index=True)
    return [int(g) for g in games[np.argsort(first)]]


def first_games(positions: Positions, rows: np.ndarray, count: int) -> np.ndarray:
    """The rows of the first `count` games among `rows` (by the games' hash order, so a smaller
    subset is always inside a larger one)."""
    order = sorted(games_of(positions, rows), key=lambda g: (game_bucket(positions.games[g], 1 << 30), g))
    keep = np.array(order[:count], dtype=np.int64)
    rows = np.asarray(rows, dtype=np.int64)
    return rows[np.isin(positions.game[rows], keep)]


@dataclass
class Vocabulary:
    """The sparse keys a model has a row for: `keys[i]` is row `i`."""

    keys: list[str]

    @staticmethod
    def build(positions: Positions, rows: np.ndarray, min_count: int) -> "Vocabulary":
        """The keys present in at least `min_count` of `rows`, sorted."""
        subset = positions.take(rows)
        # A row holds each key once (positions writes them tallied), so a key's entries are its rows.
        counts = np.bincount(subset.sparse_idx, minlength=len(positions.keys))
        return Vocabulary(sorted(positions.keys[i] for i in np.flatnonzero(counts >= min_count)))

    def remap(self, positions: Positions) -> np.ndarray:
        """For each of `positions.keys`, its row here, or -1 for a key this vocabulary does not know."""
        at = {key: i for i, key in enumerate(self.keys)}
        return np.array([at.get(key, -1) for key in positions.keys], dtype=np.int64)


@dataclass
class Standardizer:
    """Dense features to zero mean and unit spread, fitted on the training rows. A feature that
    never varies gets a spread of one."""

    mean: np.ndarray
    scale: np.ndarray

    @staticmethod
    def fit(dense: np.ndarray) -> "Standardizer":
        mean = dense.mean(axis=0, dtype=np.float64)
        std = dense.std(axis=0, dtype=np.float64)
        return Standardizer(mean=mean, scale=np.where(std > 1e-9, std, 1.0))

    def apply(self, dense: np.ndarray) -> np.ndarray:
        return ((dense - self.mean) / self.scale).astype(np.float32)
