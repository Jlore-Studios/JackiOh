"""The policy prior's offline fit (#1171): `python -m jackioh_ml.policy_fit <positions>… --out <dir>`.

A positions file written with `cargo jackioh positions --policy` labels each decision row (R1171000):
the keys of every move the AI considered there, in the heuristic order, and which one it played.
This fits one weight per key, so that a move's score is the sum of its keys' weights, as
`crates/ai/src/policy.rs` scores it (R1171001), by a multinomial logit of the move played among the
moves that are not End Turn: the beam always keeps End Turn and the prior orders only the others.

It holds 10% of the games out whole (`data.game_bucket`, as `data.split_by_game` does), fits on the
first `MAX_GAMES` of the rest, and writes three files under `--out`:

- `report.json` and `report.md`: on the held-out decisions, where the played move sits in the
  heuristic order and in the prior's (`top1`, `within6` and `within20`, the beam's prefixes, and
  `meanRank`), the prior's log loss beside a uniform choice's, and the decision: **go** when the
  prior puts the played move in the first six more often than the heuristic order does. That is a
  precondition only: the moves played are the heuristic's own choices, and the screen decides.
- `weights.json`: the table `crates/ai/build.rs` compiles in, its keys sorted, one weight per key.
"""

from __future__ import annotations

import argparse
import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable

import numpy as np
import torch

from . import data as D

#: The share of games held out whole.
HELD_OUT = 0.1
#: A key gets a weight when it is one of at least this many training candidates.
MIN_KEY_COUNT = 20
#: The L2 penalty per weight.
L2 = 1e-4
#: The most training games the fit reads (by the games' hash order, so a smaller set is inside a larger).
MAX_GAMES = 20_000
#: Threads the fit runs on: one, so its sums add in one order and a refit gives the same weights.
FIT_THREADS = 1
#: The prefixes the beam expands: `AI_BUDGET`'s branching (deeper lines) and root branching.
PREFIXES = (6, 20)
#: End Turn's keys: it has no card and no rank.
END_TURN = ["kind:endTurn"]


@dataclass
class Decisions:
    """The labelled decisions of positions files, as CSR: decision `i`'s candidates are
    `cand_ptr[i]:cand_ptr[i + 1]`, candidate `c`'s keys `key_idx[key_ptr[c]:key_ptr[c + 1]]`."""

    keys_version: int
    catalog: str
    games: list[str]
    keys: list[str]
    game: np.ndarray  # int64 per decision: an index into `games`
    chosen: np.ndarray  # int64 per decision: the played candidate's place among its own
    cand_ptr: np.ndarray  # int64, decisions + 1
    key_ptr: np.ndarray  # int64, candidates + 1
    key_idx: np.ndarray  # int64: ids into `keys`
    end_turn: np.ndarray  # bool per candidate
    skipped: int

    def __len__(self) -> int:
        return len(self.chosen)


def load_decisions(paths: Iterable[str | Path]) -> Decisions:
    """Every labelled decision of `paths`, in order. A row with no label, or whose move was none of
    its candidates, is counted in `skipped`."""
    version: int | None = None
    catalog = ""
    key_ids: dict[str, int] = {}
    keys: list[str] = []
    game_ids: dict[str, int] = {}
    games: list[str] = []
    game: list[int] = []
    chosen: list[int] = []
    cand_ptr = [0]
    key_ptr = [0]
    key_idx: list[int] = []
    end_turn: list[bool] = []
    skipped = 0
    for path in paths:
        path = Path(path)
        with path.open(encoding="utf-8") as handle:
            header = json.loads(handle.readline() or "{}")
            if "policyKeysVersion" not in header:
                raise ValueError(f"{path}: no policy labels (write it with `jackioh positions --policy`)")
            file_version = int(header["policyKeysVersion"])
            if version is None:
                version, catalog = file_version, str(header.get("buildCatalog", ""))
            elif file_version != version:
                raise ValueError(f"{path}: POLICY_KEYS_VERSION {file_version}, not the first file's {version}")
            for line in handle:
                if not line.strip():
                    continue
                row = json.loads(line)
                label = row.get("policy")
                if not label or label.get("chosen") is None:
                    skipped += 1
                    continue
                seed = row["game"]
                if seed not in game_ids:
                    game_ids[seed] = len(games)
                    games.append(seed)
                game.append(game_ids[seed])
                chosen.append(int(label["chosen"]))
                for candidate in label["candidates"]:
                    for key in candidate:
                        at = key_ids.get(key)
                        if at is None:
                            at = key_ids[key] = len(keys)
                            keys.append(key)
                        key_idx.append(at)
                    key_ptr.append(len(key_idx))
                    end_turn.append(candidate == END_TURN)
                cand_ptr.append(len(end_turn))
    if version is None:
        raise ValueError("no positions files")
    return Decisions(
        keys_version=version,
        catalog=catalog,
        games=games,
        keys=keys,
        game=np.array(game, dtype=np.int64),
        chosen=np.array(chosen, dtype=np.int64),
        cand_ptr=np.array(cand_ptr, dtype=np.int64),
        key_ptr=np.array(key_ptr, dtype=np.int64),
        key_idx=np.array(key_idx, dtype=np.int64),
        end_turn=np.array(end_turn, dtype=bool),
        skipped=skipped,
    )


def split(dec: Decisions) -> tuple[np.ndarray, np.ndarray]:
    """The training and held-out decisions: `HELD_OUT` of the games held out whole, and the first
    `MAX_GAMES` of the others to fit on."""
    cut = int(round(HELD_OUT * 1000))
    held_game = np.array([D.game_bucket(seed) < cut for seed in dec.games], dtype=bool)
    others = [g for g in range(len(dec.games)) if not held_game[g]]
    kept = sorted(others, key=lambda g: (D.game_bucket(dec.games[g], 1 << 30), g))[:MAX_GAMES]
    train = np.flatnonzero(np.isin(dec.game, np.array(kept, dtype=np.int64)))
    held = np.flatnonzero(held_game[dec.game]) if len(dec) else np.zeros(0, dtype=np.int64)
    return train, held


def _ranges(ptr: np.ndarray, items: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The entries `ptr[i]:ptr[i + 1]` of each of `items`, concatenated, and each entry's item's place
    in `items`."""
    items = np.asarray(items, dtype=np.int64)
    starts, lengths = ptr[items], ptr[items + 1] - ptr[items]
    offsets = np.zeros(len(items) + 1, dtype=np.int64)
    np.cumsum(lengths, out=offsets[1:])
    entries = np.repeat(starts - offsets[:-1], lengths) + np.arange(offsets[-1], dtype=np.int64)
    return entries, np.repeat(np.arange(len(items), dtype=np.int64), lengths)


@dataclass
class Choices:
    """The decisions whose played move is not End Turn, over their moves that are not End Turn: move
    `m` is candidate `cand[m]` of decision `owner[m]` (a place in `rows`), `local[m]` its place among
    that decision's moves, and `played[i]` decision `i`'s played move."""

    rows: np.ndarray
    cand: np.ndarray
    owner: np.ndarray
    local: np.ndarray
    played: np.ndarray
    end_turn: int


def choices(dec: Decisions, rows: np.ndarray) -> Choices:
    rows = np.sort(np.asarray(rows, dtype=np.int64))
    played_cand = dec.cand_ptr[rows] + dec.chosen[rows]
    moved = ~dec.end_turn[played_cand]
    rows, played_cand = rows[moved], played_cand[moved]
    cand, owner = _ranges(dec.cand_ptr, rows)
    keep = ~dec.end_turn[cand]
    cand, owner = cand[keep], owner[keep]
    # Each move's place among its decision's moves: its index less where its decision's moves start.
    first = np.searchsorted(owner, np.arange(len(rows)), side="left")
    local = np.arange(len(cand), dtype=np.int64) - first[owner]
    played = np.searchsorted(cand, played_cand)
    return Choices(rows=rows, cand=cand, owner=owner, local=local, played=played,
                   end_turn=int((~moved).sum()))


def vocabulary(dec: Decisions, rows: np.ndarray) -> list[str]:
    """The keys of at least `MIN_KEY_COUNT` of the moves the fit reads in `rows` (`choices`), sorted (a
    move holds each key once)."""
    entries, _ = _ranges(dec.key_ptr, choices(dec, rows).cand)
    counts = np.bincount(dec.key_idx[entries], minlength=len(dec.keys))
    return sorted(dec.keys[i] for i in np.flatnonzero(counts >= MIN_KEY_COUNT))


def _scores(dec: Decisions, ch: Choices, weight_of: np.ndarray) -> np.ndarray:
    """Each move's score: the sum of its keys' weights, `weight_of` one per key of `dec.keys`."""
    entries, move = _ranges(dec.key_ptr, ch.cand)
    return np.bincount(move, weights=weight_of[dec.key_idx[entries]], minlength=len(ch.cand))


def fit(dec: Decisions, rows: np.ndarray, vocab: list[str]) -> np.ndarray:
    """One weight per key of `vocab`: the multinomial logit of the played move among its decision's
    moves, with `L2`, by full-batch L-BFGS to convergence (`fit.fit_convex`'s settings)."""
    torch.set_num_threads(FIT_THREADS)
    ch = choices(dec, rows)
    if not vocab or not len(ch.rows):
        return np.zeros(len(vocab))
    at = {key: i for i, key in enumerate(vocab)}
    remap = np.array([at.get(key, -1) for key in dec.keys], dtype=np.int64)
    entries, move = _ranges(dec.key_ptr, ch.cand)
    mapped = remap[dec.key_idx[entries]]
    known = mapped >= 0
    entry_key = torch.as_tensor(mapped[known])
    entry_move = torch.as_tensor(move[known])
    owner = torch.as_tensor(ch.owner)
    played = torch.as_tensor(ch.played)
    decisions, moves = len(ch.rows), len(ch.cand)

    w = torch.zeros(len(vocab), dtype=torch.float64, requires_grad=True)
    optimizer = torch.optim.LBFGS([w], lr=1.0, max_iter=500, tolerance_grad=1e-9, tolerance_change=1e-12,
                                  history_size=50, line_search_fn="strong_wolfe")

    def closure() -> torch.Tensor:
        optimizer.zero_grad()
        s = torch.zeros(moves, dtype=torch.float64).index_add(0, entry_move, w[entry_key])
        # Each decision's log-sum-exp, shifted by its largest score so no exp overflows.
        top = torch.full((decisions,), -torch.inf, dtype=torch.float64).scatter_reduce(
            0, owner, s.detach(), reduce="amax", include_self=True)
        total = torch.zeros(decisions, dtype=torch.float64).index_add(0, owner, torch.exp(s - top[owner]))
        loss = (top + torch.log(total) - s[played]).mean() + L2 * (w**2).sum()
        loss.backward()
        return loss

    optimizer.step(closure)
    weights = w.detach().numpy().copy()
    if not np.all(np.isfinite(weights)):
        raise ValueError("the fit did not converge to finite weights")
    return weights


def _weight_of(dec: Decisions, prior: dict[str, float] | None) -> np.ndarray:
    prior = prior or {}
    return np.array([prior.get(key, 0.0) for key in dec.keys], dtype=np.float64)


def ranks(dec: Decisions, rows: np.ndarray, prior: dict[str, float] | None = None) -> dict:
    """Where the played move sits among its decision's moves that are not End Turn, in their own
    (heuristic) order or, given `prior`, sorted by it best first with ties in their own order, as
    `prior_order` sorts them. Decisions that ended the turn are counted in `endTurn`."""
    ch = choices(dec, rows)
    out = {"decisions": int(len(ch.rows)), "endTurn": ch.end_turn, "top1": 0.0, "within6": 0.0, "within20": 0.0,
           "meanRank": 0.0}
    if not len(ch.rows):
        return out
    s = _scores(dec, ch, _weight_of(dec, prior))
    mine = ch.played[ch.owner]
    # A move ahead of the played one in a stable sort, best first: a higher score, or an equal one earlier.
    ahead = (s > s[mine]) | ((s == s[mine]) & (ch.local < ch.local[mine]))
    position = np.bincount(ch.owner, weights=ahead.astype(np.float64), minlength=len(ch.rows))
    out.update({"top1": float(np.mean(position < 1)), "within6": float(np.mean(position < PREFIXES[0])),
                "within20": float(np.mean(position < PREFIXES[1])), "meanRank": float(np.mean(position) + 1)})
    return out


def log_losses(dec: Decisions, rows: np.ndarray, prior: dict[str, float]) -> tuple[float, float]:
    """The prior's log loss of the played move, and a uniform choice's, over the decisions `ranks` reads."""
    ch = choices(dec, rows)
    if not len(ch.rows):
        return 0.0, 0.0
    s = _scores(dec, ch, _weight_of(dec, prior))
    top = np.full(len(ch.rows), -np.inf)
    np.maximum.at(top, ch.owner, s)
    total = np.bincount(ch.owner, weights=np.exp(s - top[ch.owner]), minlength=len(ch.rows))
    loss = float(np.mean(top + np.log(total) - s[ch.played]))
    uniform = float(np.mean(np.log(np.bincount(ch.owner, minlength=len(ch.rows)))))
    return loss, uniform


def run(paths: list[str], out: Path, log: Callable[[str], None] = print) -> dict:
    started = time.time()
    torch.use_deterministic_algorithms(True)
    torch.set_num_threads(FIT_THREADS)
    dec = load_decisions(paths)
    train, held = split(dec)

    def games(rows: np.ndarray) -> int:
        return len(np.unique(dec.game[rows]))

    log(f"{len(dec):,} decisions from {len(dec.games):,} games ({dec.skipped:,} skipped), "
        f"POLICY_KEYS_VERSION {dec.keys_version}; {games(train):,} games to fit, {games(held):,} held out")
    vocab = vocabulary(dec, train)
    weights = fit(dec, train, vocab)
    prior = dict(zip(vocab, weights.tolist()))
    heuristic = ranks(dec, held)
    learned = ranks(dec, held, prior)
    learned["logLoss"], learned["uniformLogLoss"] = log_losses(dec, held, prior)
    gain = learned["within6"] - heuristic["within6"]
    report = {"keysVersion": dec.keys_version, "catalog": dec.catalog,
              "games": {"train": games(train), "heldOut": games(held)}, "skipped": dec.skipped, "keys": len(vocab),
              "heuristic": heuristic, "prior": learned, "decision": {"gain": gain, "go": gain > 0},
              "seconds": round(time.time() - started, 1)}

    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=1), encoding="utf-8")
    (out / "report.md").write_text(render(report), encoding="utf-8")
    table = {"keysVersion": dec.keys_version, "catalog": dec.catalog, "keys": vocab, "weights": weights.tolist()}
    (out / "weights.json").write_text(json.dumps(table, separators=(",", ":")) + "\n", encoding="utf-8")
    log(f"wrote {out / 'report.md'} in {report['seconds']} s: {'go' if gain > 0 else 'stop'} (within 6 {gain:+.4f})")
    return report


def render(report: dict) -> str:
    h, p, d = report["heuristic"], report["prior"], report["decision"]
    lines = ["# The policy prior's offline fit", "",
             f"{h['decisions']:,} held-out decisions that played a move ({h['endTurn']:,} more ended the turn) from "
             f"{report['games']['heldOut']:,} games held out whole; fitted on {report['games']['train']:,} games, "
             f"{report['keys']:,} keys, POLICY_KEYS_VERSION {report['keysVersion']}, catalog {report['catalog']}; "
             f"rows skipped for want of a label: {report['skipped']:,}.", "",
             "| order | top 1 | within 6 | within 20 | mean rank |", "|---|---|---|---|---|"]
    for name, m in (("heuristic", h), ("prior", p)):
        lines.append(f"| {name} | {m['top1']:.4f} | {m['within6']:.4f} | {m['within20']:.4f} | {m['meanRank']:.2f} |")
    lines += ["", f"The prior's log loss of the move played: {p['logLoss']:.4f}, against {p['uniformLogLoss']:.4f} for a "
              "uniform choice.", "",
              f"Within 6, the prior gains {d['gain']:+.4f} on the heuristic order: **{'go' if d['go'] else 'stop'}**. "
              "The moves played are the heuristic's own choices, so this is a precondition only: the screen decides.",
              "", f"Fitted in {report['seconds']} s.", ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("positions", nargs="+")
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    run(args.positions, args.out)


if __name__ == "__main__":
    main()
