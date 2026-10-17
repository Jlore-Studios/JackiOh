"""The network into the Rust search (#1010): `crates/ai/src/learned/weights.json`, which
`crates/ai/build.rs` compiles in as `LEARNED`, and the reference that proves the Rust reads it right.

    python -m jackioh_ml.export --model out/<fit>/model.pt --out crates/ai/src/learned/weights.json
    python -m jackioh_ml.export --null --positions <file> --out crates/ai/src/learned/weights.json
    python -m jackioh_ml.export --fixture <weights.json> --positions <file> --rows 40 --out <fixture.json>
    python -m jackioh_ml.export --random 1010 --positions <file> --rows 40 --out <fixture.json>

The weights record `featuresVersion`, the catalogs the fit read, the dense names, the standardizer,
the id-to-row table (`keys`, sorted) and the skip connection (`handA`, `handB`, `handUnit`), so the
Rust maps the network's logit back onto the hand eval's points: `hand + (handB + net(x))·handUnit/handA`.

`reference` is that arithmetic in plain Python floats (IEEE doubles), in the Rust's loop order, so
the two agree to the last bit. `--fixture` writes a network and positions with the outputs
`reference` computes for them, which `crates/ai/tests/ai/learned.rs` holds the Rust to (R1010001).
`--null` writes the network whose correction is zero everywhere: `Learned` is then `Hand`.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

#: The weights file's own layout; bump it when its fields change.
FORMAT = 1


def _rows(tensor) -> list[list[float]]:
    return [[float(x) for x in row] for row in tensor.tolist()]


def from_model(path: Path) -> dict:
    """weights.json's object from a fit's `model.pt`."""
    import torch

    saved = torch.load(path, map_location="cpu", weights_only=False)
    state = saved["state"]
    width = int(saved["width"])
    keys: list[str] = list(saved["vocab"])
    order = sorted(range(len(keys)), key=lambda i: keys[i])
    sparse = state["sparse_in.weight"].double()
    head = []
    at = 0
    while f"head.{at}.weight" in state:
        head.append({"w": _rows(state[f"head.{at}.weight"].double()), "b": [float(x) for x in state[f"head.{at}.bias"].double()]})
        at += 1
    return {
        "format": FORMAT,
        "featuresVersion": int(saved["featuresVersion"]),
        "catalog": ",".join(saved.get("catalogs", [])),
        "denseNames": list(saved["denseNames"]),
        "mean": [float(x) for x in saved["mean"]],
        "scale": [float(x) for x in saved["scale"]],
        "keys": [keys[i] for i in order],
        "width": width,
        # torch's Linear is out x in; the Rust reads one row of `width` per input.
        "denseIn": _rows(state["dense_in.weight"].double().T),
        "sparseIn": [[float(x) for x in sparse[i]] for i in order] if keys else [],
        "bias": [float(x) for x in state["dense_in.bias"].double()],
        "head": head,
        "out": {"w": [float(x) for x in state["out.weight"].double()[0]], "b": float(state["out.bias"].double()[0])},
        "handA": float(state["hand.a"]),
        "handB": float(state["hand.b"]),
        "handUnit": 100.0,
    }


def null(dense_names: list[str], features_version: int, catalog: str) -> dict:
    """The network that changes nothing: one hidden unit, every weight zero, handB zero."""
    return {
        "format": FORMAT,
        "featuresVersion": features_version,
        "catalog": catalog,
        "denseNames": dense_names,
        "mean": [0.0] * len(dense_names),
        "scale": [1.0] * len(dense_names),
        "keys": [],
        "width": 1,
        "denseIn": [[0.0] for _ in dense_names],
        "sparseIn": [],
        "bias": [0.0],
        "head": [],
        "out": {"w": [0.0], "b": 0.0},
        "handA": 1.0,
        "handB": 0.0,
        "handUnit": 100.0,
    }


def random_network(positions: Path, seed: int, width: int = 4, head: tuple[int, ...] = (3,), keys: int = 40) -> dict:
    """A test network with weights big enough to clip: the standardizer of `positions`, its `keys` most
    common sparse keys, and every weight drawn from a seeded normal. Only the fixture uses it."""
    import random
    from collections import Counter

    from . import data as D

    rng = random.Random(seed)
    loaded = D.load([positions])
    scaler = D.Standardizer.fit(loaded.dense)
    counts = Counter(loaded.keys[i] for i in loaded.sparse_idx)
    chosen = sorted(key for key, _ in counts.most_common(keys))

    def draws(n: int, std: float) -> list[float]:
        return [rng.gauss(0.0, std) for _ in range(n)]

    layers, inputs = [], width
    for out in head:
        layers.append({"w": [draws(inputs, 0.8) for _ in range(out)], "b": draws(out, 0.3)})
        inputs = out
    return {
        "format": FORMAT,
        "featuresVersion": loaded.features_version,
        "catalog": ",".join(sorted(loaded.catalogs)),
        "denseNames": loaded.dense_names,
        "mean": [float(x) for x in scaler.mean],
        "scale": [float(x) for x in scaler.scale],
        "keys": chosen,
        "width": width,
        "denseIn": [draws(width, 0.15) for _ in loaded.dense_names],
        "sparseIn": [draws(width, 0.4) for _ in chosen],
        "bias": draws(width, 0.4),
        "head": layers,
        "out": {"w": draws(inputs, 1.0), "b": rng.gauss(0.0, 0.2)},
        "handA": 0.8 + rng.random(),
        "handB": rng.gauss(0.0, 0.1),
        "handUnit": 100.0,
    }


def _clip(x: float) -> float:
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return x


def correction(weights: dict, dense: list[int], sparse: list[list]) -> float:
    """net(x), in the Rust's order (`learned.rs`, `Network::correction`)."""
    width = weights["width"]
    if len(dense) != len(weights["mean"]):
        return 0.0
    hidden = [float(b) for b in weights["bias"]]
    for j, value in enumerate(dense):
        x = (float(value) - weights["mean"][j]) / weights["scale"][j]
        row = weights["denseIn"][j]
        for i in range(width):
            hidden[i] += x * row[i]
    index = {key: r for r, key in enumerate(weights["keys"])}
    for key, count in sparse:
        r = index.get(key)
        if r is not None:
            row = weights["sparseIn"][r]
            for i in range(width):
                hidden[i] += float(count) * row[i]
    hidden = [_clip(h) for h in hidden]
    for layer in weights["head"]:
        nxt = []
        for k, b in enumerate(layer["b"]):
            out = float(b)
            for w, h in zip(layer["w"][k], hidden):
                out += w * h
            nxt.append(_clip(out))
        hidden = nxt
    out = float(weights["out"]["b"])
    for w, h in zip(weights["out"]["w"], hidden):
        out += w * h
    return out


def points(weights: dict, hand: float, dense: list[int], sparse: list[list]) -> float:
    """The position's value in the hand eval's points (`Network::points`)."""
    if weights["handA"] == 0.0:
        return hand
    return hand + (weights["handB"] + correction(weights, dense, sparse)) * weights["handUnit"] / weights["handA"]


def fixture(weights: dict, positions: Path, rows: int) -> dict:
    """`weights` and the first `rows` positions of `positions`, each with what `reference` computes."""
    cases = []
    with positions.open(encoding="utf-8") as handle:
        handle.readline()
        for line in handle:
            if len(cases) >= rows:
                break
            row = json.loads(line)
            cases.append({"dense": row["dense"], "sparse": row["sparse"], "hand": row["hand"],
                          "correction": correction(weights, row["dense"], row["sparse"]),
                          "points": points(weights, row["hand"], row["dense"], row["sparse"])})
    return {"network": weights, "cases": cases}


def _header(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.loads(handle.readline())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--model", type=Path, help="a fit's model.pt")
    parser.add_argument("--null", action="store_true", help="the network that changes nothing")
    parser.add_argument("--positions", type=Path, help="a positions file: its header (--null) or its rows (--fixture)")
    parser.add_argument("--fixture", type=Path, help="weights.json to compute a test fixture from")
    parser.add_argument("--rows", type=int, default=40)
    parser.add_argument("--random", type=int, metavar="SEED", help="with --fixture: a seeded random test network over --positions, not a file")
    args = parser.parse_args()
    if args.model:
        out = from_model(args.model)
    elif args.null:
        header = _header(args.positions)
        out = null(header["denseNames"], header["featuresVersion"], header.get("buildCatalog", ""))
    elif args.random is not None:
        out = fixture(random_network(args.positions, args.random), args.positions, args.rows)
    elif args.fixture:
        out = fixture(json.loads(args.fixture.read_text(encoding="utf-8")), args.positions, args.rows)
    else:
        parser.error("name --model, --null, --random or --fixture")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=None, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
