"""The offline fit and its report (#1009): `python -m jackioh_ml.fit <positions>… --out <dir>`.

It holds 10% of the games out whole (`data.split_by_game`), fits the three models of `models.py` on
the rest, and writes `<out>/report.md` and `<out>/report.json`:

- held-out log loss, AUC and a calibration table, overall and by turn band (1-3, 4-10, 11 on);
- the learning curve: model (c) fitted on 10,000, 30,000 and 100,000 games (as many as there are);
- the run-to-run spread: model (c) fitted again on other seeds;
- the decision #1009 asks for: **go** when (c) beats (a) and (b) on held-out log loss in turns 4-10 by
  more than the spread, **more data** when the curve is still falling at the largest size, **stop**
  otherwise;
- the hand eval's most misjudged cards: on the seat's board in positions the calibrated hand eval
  scores as won that the seat lost, and the reverse.

`<out>/model.pt` keeps the best seed's network (c) with its vocabulary and standardizer, which
`export.py` turns into the Rust weights; `<out>/model-b.pt` is (b), the linear correction alone, in the
same form.
"""

from __future__ import annotations

import argparse
import json
import math
import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

import numpy as np
import torch
from torch import nn

from . import data as D
from .models import Batch, HandModel, LinearModel, NetModel

#: R-free numbers of the fit, named once (CLAUDE.md rule 9's spirit).
HELD_OUT = 0.1
VALIDATION = 0.1
MIN_KEY_COUNT = 20
BATCH = 2048
#: Threads the fit runs on: one, so its sums add in one order and a refit gives the same weights.
FIT_THREADS = 1
#: The decoupled weight decay of the per-card rows (AdamW), and of every other weight of (c).
SPARSE_DECAY = 1e-2
MAX_EPOCHS = 40
PATIENCE = 3
CURVE_GAMES = (10_000, 30_000, 100_000)
SEEDS = (1, 2, 3)
TURN_BANDS = ((1, 3), (4, 10), (11, 10_000))
CALIBRATION_BINS = 10
MISJUDGED = 20
MISJUDGED_MIN_ROWS = 100


@dataclass
class Prepared:
    """A Positions turned into model inputs, under one vocabulary and standardizer."""

    hand: torch.Tensor
    dense: torch.Tensor
    idx: torch.Tensor
    ptr: torch.Tensor
    val: torch.Tensor
    outcome: torch.Tensor
    turn: np.ndarray

    def __len__(self) -> int:
        return len(self.outcome)

    def batch(self, rows: np.ndarray) -> Batch:
        rows_t = torch.as_tensor(rows, dtype=torch.int64)
        starts, ends = self.ptr[rows_t], self.ptr[rows_t + 1]
        lengths = ends - starts
        offsets = torch.zeros(len(rows), dtype=torch.int64)
        if len(rows) > 1:
            offsets[1:] = torch.cumsum(lengths, 0)[:-1]
        total = int(lengths.sum())
        gather = torch.repeat_interleave(starts - offsets, lengths) + torch.arange(total)
        return Batch(
            hand=self.hand[rows_t],
            dense=self.dense[rows_t],
            idx=self.idx[gather],
            offsets=offsets,
            val=self.val[gather],
            outcome=self.outcome[rows_t],
        )


def prepare(positions: D.Positions, vocab: D.Vocabulary, scaler: D.Standardizer) -> Prepared:
    """Inputs for every row of `positions`: keys outside `vocab` dropped, dense standardized."""
    remap = vocab.remap(positions)
    rows_of_entry = np.repeat(np.arange(len(positions)), np.diff(positions.sparse_ptr))
    mapped = remap[positions.sparse_idx] if len(positions.sparse_idx) else np.zeros(0, np.int64)
    keep = mapped >= 0
    kept_rows = rows_of_entry[keep]
    ptr = np.zeros(len(positions) + 1, dtype=np.int64)
    np.cumsum(np.bincount(kept_rows, minlength=len(positions)), out=ptr[1:])
    return Prepared(
        hand=torch.as_tensor(positions.hand, dtype=torch.float32),
        dense=torch.as_tensor(scaler.apply(positions.dense)),
        idx=torch.as_tensor(mapped[keep], dtype=torch.int64),
        ptr=torch.as_tensor(ptr),
        val=torch.as_tensor(positions.sparse_val[keep], dtype=torch.float32),
        outcome=torch.as_tensor(positions.outcome, dtype=torch.float32),
        turn=positions.turn,
    )


def predict(model: nn.Module, data: Prepared, rows: np.ndarray | None = None) -> np.ndarray:
    """The model's probability for each row (all rows by default)."""
    rows = np.arange(len(data)) if rows is None else rows
    out: list[np.ndarray] = []
    model.eval()
    with torch.no_grad():
        for start in range(0, len(rows), 8 * BATCH):
            logit = model(data.batch(rows[start : start + 8 * BATCH]))
            out.append(torch.sigmoid(logit).numpy())
    return np.concatenate(out) if out else np.zeros(0)


def log_loss(p: np.ndarray, y: np.ndarray) -> float:
    p = np.clip(p, 1e-7, 1 - 1e-7)
    return float(np.mean(-(y * np.log(p) + (1 - y) * np.log(1 - p)))) if len(y) else float("nan")


def auc(p: np.ndarray, y: np.ndarray) -> float:
    """The area under the ROC curve over decided games (a draw is neither side)."""
    won, lost = p[y == 1.0], p[y == 0.0]
    if not len(won) or not len(lost):
        return float("nan")
    order = np.argsort(np.concatenate([won, lost]), kind="mergesort")
    ranks = np.empty(len(order))
    ranks[order] = np.arange(1, len(order) + 1)
    # Ties share their mean rank.
    values = np.concatenate([won, lost])[order]
    start = 0
    while start < len(values):
        end = start
        while end + 1 < len(values) and values[end + 1] == values[start]:
            end += 1
        if end > start:
            ranks[order[start : end + 1]] = (start + end + 2) / 2
        start = end + 1
    return float((ranks[: len(won)].sum() - len(won) * (len(won) + 1) / 2) / (len(won) * len(lost)))


def calibration(p: np.ndarray, y: np.ndarray, bins: int = CALIBRATION_BINS) -> list[dict]:
    edges = np.linspace(0, 1, bins + 1)
    table = []
    for k in range(bins):
        mask = (p >= edges[k]) & ((p < edges[k + 1]) if k < bins - 1 else (p <= edges[k + 1]))
        if mask.any():
            table.append({"from": round(float(edges[k]), 2), "to": round(float(edges[k + 1]), 2), "rows": int(mask.sum()),
                          "predicted": round(float(p[mask].mean()), 4), "observed": round(float(y[mask].mean()), 4)})
    return table


def metrics(p: np.ndarray, y: np.ndarray, turn: np.ndarray) -> dict:
    out = {"all": {"rows": int(len(y)), "logLoss": log_loss(p, y), "auc": auc(p, y)}}
    for low, high in TURN_BANDS:
        mask = (turn >= low) & (turn <= high)
        name = f"turns {low}-{high}" if high < 10_000 else f"turns {low}+"
        out[name] = {"rows": int(mask.sum()), "logLoss": log_loss(p[mask], y[mask]), "auc": auc(p[mask], y[mask])}
    return out


def train(
    model: nn.Module,
    data: Prepared,
    train_rows: np.ndarray,
    valid_rows: np.ndarray,
    seed: int,
    lr: float = 3e-3,
    weight_decay: float = 1e-4,
    log: Callable[[str], None] = print,
) -> dict:
    """Adam on the log loss, keeping the epoch with the best validation loss (early stopping)."""
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    # The per-card rows see the fewest games each: they take the strongest penalty.
    sparse = [p for name, p in model.named_parameters() if name.startswith("sparse_in")]
    rest = [p for name, p in model.named_parameters() if not name.startswith("sparse_in")]
    groups = [{"params": rest, "weight_decay": weight_decay}]
    if sparse:
        groups.append({"params": sparse, "weight_decay": SPARSE_DECAY})
    optimizer = torch.optim.AdamW(groups, lr=lr)
    loss_fn = nn.BCEWithLogitsLoss()
    yv = data.outcome[torch.as_tensor(valid_rows)].numpy()
    # Where it starts counts as an epoch: a fit that never beats its start keeps it.
    best, stale, epochs = log_loss(predict(model, data, valid_rows), yv), 0, 0
    best_state = {k: v.detach().clone() for k, v in model.state_dict().items()}
    for epoch in range(MAX_EPOCHS):
        model.train()
        order = rng.permutation(train_rows)
        for start in range(0, len(order), BATCH):
            batch = data.batch(order[start : start + BATCH])
            optimizer.zero_grad()
            loss = loss_fn(model(batch), batch.outcome)
            loss.backward()
            optimizer.step()
        valid = log_loss(predict(model, data, valid_rows), yv)
        epochs = epoch + 1
        if valid < best - 1e-5:
            best, stale = valid, 0
            best_state = {k: v.detach().clone() for k, v in model.state_dict().items()}
        else:
            stale += 1
            if stale >= PATIENCE:
                break
    if best_state is not None:
        model.load_state_dict(best_state)
    log(f"  {type(model).__name__} seed {seed}: {epochs} epochs, validation log loss {best:.5f}")
    return {"epochs": epochs, "validation": best}


def fit_convex(model: nn.Module, data: Prepared, rows: np.ndarray, l2: float = 0.0, log: Callable[[str], None] = print) -> dict:
    """(a) and (b) are logistic regressions, convex: full-batch L-BFGS to convergence, so neither is a
    straw man for (c) to beat."""
    batch = data.batch(rows)
    loss_fn = nn.BCEWithLogitsLoss()
    optimizer = torch.optim.LBFGS(model.parameters(), lr=1.0, max_iter=500, tolerance_grad=1e-9,
                                  tolerance_change=1e-12, history_size=50, line_search_fn="strong_wolfe")
    penalized = [p for name, p in model.named_parameters() if name.endswith("w.weight")]

    def closure() -> torch.Tensor:
        optimizer.zero_grad()
        loss = loss_fn(model(batch), batch.outcome)
        if l2:
            loss = loss + l2 * sum((p**2).sum() for p in penalized)
        loss.backward()
        return loss

    model.train()
    final = float(optimizer.step(closure).detach())
    log(f"  {type(model).__name__}: L-BFGS, training loss {final:.5f}")
    return {"training": final}


#: (b)'s L2 penalties per weight, on standardized features; the one with the best validation loss is kept.
LINEAR_L2 = (1e-5, 1e-4, 1e-3, 1e-2)


def start_from(model: nn.Module, hand: HandModel, linear: "LinearModel | None" = None) -> None:
    """(b) starts where (a) ended, the calibrated hand eval; (c) where (b) ended, when it is given."""
    with torch.no_grad():
        source = linear.hand if linear is not None else hand
        model.hand.a.copy_(source.a)
        model.hand.b.copy_(source.b)
        if linear is not None and hasattr(model, "linear"):
            model.linear.weight.copy_(linear.w.weight)


def misjudged(positions: D.Positions, p_hand: np.ndarray, catalog: dict[str, str]) -> dict:
    """The cards on the seat's own board where the calibrated hand eval is most wrong, by the mean
    residual (outcome less prediction) over the rows holding the card, in standard errors."""
    residual = positions.outcome.astype(np.float64) - p_hand
    entries = np.diff(positions.sparse_ptr)
    row_of = np.repeat(np.arange(len(positions)), entries)
    keys = np.array(positions.keys, dtype=object)
    own_board = np.array([k.startswith("board:own:") for k in positions.keys])
    entry_keys = positions.sparse_idx
    mask = own_board[entry_keys]
    sel_keys, sel_rows = entry_keys[mask], row_of[mask]
    n = np.bincount(sel_keys, minlength=len(keys))
    s = np.bincount(sel_keys, weights=residual[sel_rows], minlength=len(keys))
    s2 = np.bincount(sel_keys, weights=residual[sel_rows] ** 2, minlength=len(keys))
    won_but_lost = np.bincount(sel_keys, weights=((p_hand[sel_rows] > 0.5) & (positions.outcome[sel_rows] == 0)).astype(float), minlength=len(keys))
    lost_but_won = np.bincount(sel_keys, weights=((p_hand[sel_rows] < 0.5) & (positions.outcome[sel_rows] == 1)).astype(float), minlength=len(keys))
    rows = []
    for k in np.flatnonzero(n >= MISJUDGED_MIN_ROWS):
        mean = s[k] / n[k]
        var = max(s2[k] / n[k] - mean**2, 1e-12)
        card = keys[k].split(":", 2)[2]
        rows.append({"card": card, "name": catalog.get(card, ""), "rows": int(n[k]), "meanResidual": round(float(mean), 4),
                     "z": round(float(mean / math.sqrt(var / n[k])), 2), "scoredWonButLost": int(won_but_lost[k]),
                     "scoredLostButWon": int(lost_but_won[k])})
    over = sorted(rows, key=lambda r: r["z"])[:MISJUDGED]
    under = sorted(rows, key=lambda r: -r["z"])[:MISJUDGED]
    return {"overrated": over, "underrated": under}


def load_catalog(root: Path) -> dict[str, str]:
    path = root / "crates" / "cards" / "catalog.json"
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    cards = raw.get("cards", raw) if isinstance(raw, dict) else raw
    names: dict[str, str] = {}
    items = cards.values() if isinstance(cards, dict) else cards
    for card in items:
        if isinstance(card, dict) and "id" in card:
            names[card["id"]] = card.get("name", "")
    return names


def decide(report: dict) -> dict:
    """#1009's go/no-go from the report's numbers."""
    band = "turns 4-10"
    a = report["models"]["a"]["heldOut"][band]["logLoss"]
    b = report["models"]["b"]["heldOut"][band]["logLoss"]
    seeds = [r["heldOut"][band]["logLoss"] for r in report["seeds"]]
    c = float(np.mean(seeds))
    spread = float(np.std(seeds, ddof=1)) if len(seeds) > 1 else 0.0
    gain = min(a, b) - c
    curve = [(p["games"], p["heldOut"][band]["logLoss"]) for p in report["curve"]]
    falling = len(curve) >= 2 and (curve[-2][1] - curve[-1][1]) > max(spread, 1e-4)
    if gain > max(spread, 1e-4):
        verdict = "go"
    elif falling:
        verdict = "more data"
    else:
        verdict = "stop"
    return {"band": band, "a": a, "b": b, "c": c, "spread": spread, "gain": gain, "curveFalling": falling, "verdict": verdict}


def run(paths: list[str], out: Path, root: Path, seeds: tuple[int, ...] = SEEDS, curve: tuple[int, ...] = CURVE_GAMES,
        width: int = 16, head: tuple[int, ...] = (16,), log: Callable[[str], None] = print) -> dict:
    started = time.time()
    # #1012: the same positions and seeds give the same weights. Only deterministic kernels, and one
    # thread for the order of every floating-point sum (a reduction split across threads adds in an
    # order that changes from run to run).
    torch.use_deterministic_algorithms(True)
    torch.set_num_threads(FIT_THREADS)
    torch.manual_seed(0)
    random.seed(0)
    positions = D.load(paths)
    log(f"{len(positions):,} positions from {len(positions.games):,} games, FEATURES_VERSION {positions.features_version}")
    train_all, held = D.split_by_game(positions, HELD_OUT)
    # Validation for early stopping: another share of the games, also whole.
    fit_rows, valid_rows = _validation_split(positions, train_all)
    vocab = D.Vocabulary.build(positions, fit_rows, MIN_KEY_COUNT)
    scaler = D.Standardizer.fit(positions.dense[fit_rows])
    data = prepare(positions, vocab, scaler)
    y_held = positions.outcome[held]
    log(f"  {len(D.games_of(positions, fit_rows)):,} games to fit, {len(D.games_of(positions, valid_rows)):,} to validate, "
        f"{len(D.games_of(positions, held)):,} held out; {len(vocab.keys):,} sparse keys")

    report: dict = {"featuresVersion": positions.features_version, "catalogs": sorted(positions.catalogs),
                    "positions": len(positions), "games": len(positions.games), "heldOutGames": len(D.games_of(positions, held)),
                    "vocabulary": len(vocab.keys), "denseFeatures": len(positions.dense_names), "models": {}}

    hand = HandModel()
    fit_convex(hand, data, fit_rows, log=log)
    p_hand = predict(hand, data, held)
    report["models"]["a"] = {"name": "the calibrated hand eval", "weights": 2, "a": float(hand.a.detach()), "b": float(hand.b.detach()),
                             "heldOut": metrics(p_hand, y_held, positions.turn[held]),
                             "calibration": calibration(p_hand, y_held)}

    linear, linear_l2, linear_valid = None, None, math.inf
    y_valid = positions.outcome[valid_rows]
    for l2 in LINEAR_L2:
        candidate = LinearModel(len(positions.dense_names))
        start_from(candidate, hand)
        fit_convex(candidate, data, fit_rows, l2=l2, log=log)
        valid = log_loss(predict(candidate, data, valid_rows), y_valid)
        if valid < linear_valid:
            linear, linear_l2, linear_valid = candidate, l2, valid
    p_lin = predict(linear, data, held)
    report["models"]["b"] = {"name": "a logistic over the dense features and the hand eval", "l2": linear_l2,
                             "weights": len(positions.dense_names) + 2, "heldOut": metrics(p_lin, y_held, positions.turn[held]),
                             "calibration": calibration(p_lin, y_held)}

    best_model, best_loss = None, math.inf
    report["seeds"] = []
    for seed in seeds:
        torch.manual_seed(seed)  # the network's first weights come from the seed, not the process
        net = NetModel(len(positions.dense_names), len(vocab.keys), width=width, head=head)
        start_from(net, hand, linear)
        fitted = train(net, data, fit_rows, valid_rows, seed=seed, log=log)
        p_net = predict(net, data, held)
        entry = {"seed": seed, "epochs": fitted["epochs"], "heldOut": metrics(p_net, y_held, positions.turn[held])}
        report["seeds"].append(entry)
        if fitted["validation"] < best_loss:
            best_model, best_loss, best_p = net, fitted["validation"], p_net
    report["models"]["c"] = {"name": f"the network: {width} wide, head {list(head)}, skip connection to the hand eval",
                             "weights": best_model.weight_count(), "heldOut": metrics(best_p, y_held, positions.turn[held]),
                             "calibration": calibration(best_p, y_held)}

    report["curve"] = []
    fit_games = len(D.games_of(positions, fit_rows))
    for games in curve:
        if games > fit_games:
            if not report["curve"] or report["curve"][-1]["games"] < fit_games:
                games = fit_games
            else:
                continue
        rows = D.first_games(positions, fit_rows, games)
        # Each size's (b) is fitted on that size's games, so a small fit never starts from the big one's.
        linear_sub = LinearModel(len(positions.dense_names))
        start_from(linear_sub, hand)
        fit_convex(linear_sub, data, rows, l2=linear_l2, log=log)
        torch.manual_seed(seeds[0])
        net = NetModel(len(positions.dense_names), len(vocab.keys), width=width, head=head)
        start_from(net, hand, linear_sub)
        train(net, data, rows, valid_rows, seed=seeds[0], log=log)
        report["curve"].append({"games": games, "heldOut": metrics(predict(net, data, held), y_held, positions.turn[held])})
        if games == fit_games:
            break

    report["misjudged"] = misjudged(positions, predict(hand, data), load_catalog(root))
    report["decision"] = decide(report)
    report["seconds"] = round(time.time() - started, 1)

    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=1), encoding="utf-8")
    (out / "report.md").write_text(render(report), encoding="utf-8")
    def save(model: NetModel, name: str) -> None:
        torch.save({"state": model.state_dict(), "width": width, "head": list(head), "vocab": vocab.keys,
                    "mean": scaler.mean.tolist(), "scale": scaler.scale.tolist(), "denseNames": positions.dense_names,
                    "featuresVersion": positions.features_version, "catalogs": sorted(positions.catalogs)}, out / name)

    save(best_model, "model.pt")
    # (b) as a network whose head is zero: the linear correction alone, which `export.py` reads alike.
    torch.manual_seed(0)
    linear_net = NetModel(len(positions.dense_names), len(vocab.keys), width=width, head=head)
    start_from(linear_net, hand, linear)
    save(linear_net, "model-b.pt")
    log(f"wrote {out / 'report.md'} in {report['seconds']} s: {report['decision']['verdict']}")
    return report


def _validation_split(positions: D.Positions, train_rows: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The training rows split again by game: the games in buckets [100, 200) validate."""
    bucket = np.array([D.game_bucket(seed) for seed in positions.games])
    rows = np.asarray(train_rows)
    valid = (bucket[positions.game[rows]] >= 100) & (bucket[positions.game[rows]] < 100 + int(VALIDATION * 1000))
    return rows[~valid], rows[valid]


def _fmt(x: float) -> str:
    return "n/a" if x != x else f"{x:.4f}"


def render(report: dict) -> str:
    lines = ["# The learned evaluation's offline fit (#1009)", ""]
    lines.append(f"{report['positions']:,} positions from {report['games']:,} games ({report['heldOutGames']:,} held out whole), "
                 f"FEATURES_VERSION {report['featuresVersion']}, catalogs {', '.join(report['catalogs'])}; "
                 f"{report['denseFeatures']} dense features and {report['vocabulary']:,} sparse keys.")
    lines += ["", "## Held-out log loss and AUC", "", "| model | weights | all | turns 1-3 | turns 4-10 | turns 11+ | AUC all | AUC 4-10 |", "|---|---|---|---|---|---|---|---|"]
    for key in ("a", "b", "c"):
        m = report["models"][key]
        h = m["heldOut"]
        lines.append(f"| ({key}) {m['name']} | {m['weights']:,} | {_fmt(h['all']['logLoss'])} | {_fmt(h['turns 1-3']['logLoss'])} | "
                     f"{_fmt(h['turns 4-10']['logLoss'])} | {_fmt(h['turns 11+']['logLoss'])} | {_fmt(h['all']['auc'])} | {_fmt(h['turns 4-10']['auc'])} |")
    lines += ["", "## Run-to-run spread of (c)", "", "| seed | epochs | log loss, turns 4-10 | all |", "|---|---|---|---|"]
    for s in report["seeds"]:
        lines.append(f"| {s['seed']} | {s['epochs']} | {_fmt(s['heldOut']['turns 4-10']['logLoss'])} | {_fmt(s['heldOut']['all']['logLoss'])} |")
    lines += ["", "## Learning curve of (c)", "", "| games fitted | log loss, turns 4-10 | all |", "|---|---|---|"]
    for p in report["curve"]:
        lines.append(f"| {p['games']:,} | {_fmt(p['heldOut']['turns 4-10']['logLoss'])} | {_fmt(p['heldOut']['all']['logLoss'])} |")
    d = report["decision"]
    lines += ["", "## Decision", "",
              f"In {d['band']}: (a) {_fmt(d['a'])}, (b) {_fmt(d['b'])}, (c) {_fmt(d['c'])} (mean of {len(report['seeds'])} seeds, "
              f"spread {_fmt(d['spread'])}). (c) gains {_fmt(d['gain'])} on the better of (a) and (b); the curve is "
              f"{'still falling' if d['curveFalling'] else 'flat'} at its largest size.", "", f"**{d['verdict']}**"]
    for key in ("a", "c"):
        lines += ["", f"## Calibration of ({key}), held out", "", "| predicted | rows | mean predicted | observed |", "|---|---|---|---|"]
        for row in report["models"][key]["calibration"]:
            lines.append(f"| {row['from']:.1f}-{row['to']:.1f} | {row['rows']:,} | {row['predicted']:.3f} | {row['observed']:.3f} |")
    for side, title in (("overrated", "scores as won more than the games bear out"), ("underrated", "scores as lost more than the games bear out")):
        lines += ["", f"## Cards on the seat's board the hand eval {title}", "",
                  "| card | name | rows | mean residual | z | scored won, lost | scored lost, won |", "|---|---|---|---|---|---|---|"]
        for r in report["misjudged"][side]:
            lines.append(f"| {r['card']} | {r['name']} | {r['rows']:,} | {r['meanResidual']:+.3f} | {r['z']:+.1f} | {r['scoredWonButLost']:,} | {r['scoredLostButWon']:,} |")
    lines += ["", f"Fitted in {report['seconds']} s.", ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("positions", nargs="+")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3], help="the repository, for the catalog's names")
    parser.add_argument("--seeds", type=int, nargs="+", default=list(SEEDS))
    parser.add_argument("--curve", type=int, nargs="+", default=list(CURVE_GAMES))
    parser.add_argument("--width", type=int, default=16)
    parser.add_argument("--head", type=int, nargs="*", default=[16])
    args = parser.parse_args()
    run(args.positions, args.out, args.root, tuple(args.seeds), tuple(args.curve), args.width, tuple(args.head))


if __name__ == "__main__":
    main()
