"""The network in integers (#1011; docs/learned-eval.md D7): what ships, and its reference.

    python -m jackioh_ml.quantize --weights <float weights.json> --out <integer weights.json>
                                  [--positions <file> --report <report.json>]

A float network (`export.py`, `"format": 1`) becomes an integer one (`"format": 2`), which every
target computes alike by construction:

- **The first layer** takes the raw dense features, integers as `features` writes them: the fit's
  standardizer is folded into its weights (`W/scale`) and its bias (`b - Σ mean·W/scale`). Its weights
  are i16, scaled by `A·2^s` (`A` = 127, a clipped activation's one), with `s` as large as keeps every
  weight in an i16. An i8 first layer would round most of its weights to a handful of values over 127
  dense inputs of very different spreads; the head's weights are i8, as #1011 asks.
- **Each head layer and the output**: i8 weights scaled by `2^s`, biases at the layer's scale.
- **Arithmetic**: i64 sums, so no feature value can overflow them; after each layer an arithmetic shift
  right by its `s` (a floor, alike in Rust and Python) and a clip to `[0, A]`. The output's sum is
  divided once by `A·2^s` into the correction's logit, and mapped onto the hand eval's points as the
  float network is (`hand + (handB + logit)·handUnit/handA`).

`correction` is that arithmetic in Python integers: the reference the Rust is held to bit for bit
(R1011000). `--report` compares the integer network's held-out log loss with the float one's on
positions (the tolerance #1011 asks the pull request to state).
"""

from __future__ import annotations

import argparse
import base64
import json
import math
import struct
from pathlib import Path

#: The weights file's layout for an integer network.
FORMAT = 2
#: A clipped activation's one, in integers.
ACTIVATION = 127
I16, I8 = 32767, 127


def _pack(fmt: str, values: list[int]) -> str:
    return base64.b64encode(struct.pack("<" + fmt * len(values), *values)).decode("ascii")


def _unpack(fmt: str, text: str) -> list[int]:
    raw = base64.b64decode(text)
    size = struct.calcsize("<" + fmt)
    return list(struct.unpack("<" + fmt * (len(raw) // size), raw))


def _shift_for(largest: float, limit: int, unit: float) -> int:
    """The largest s with `largest·unit·2^s` within `limit` (at most 24)."""
    if largest == 0:
        return 0
    s = math.floor(math.log2(limit / (largest * unit)))
    while largest * unit * 2.0**s > limit:
        s -= 1
    return max(min(s, 24), -24)


def quantize(weights: dict) -> dict:
    """An integer network (`"format": 2`) from a float one (`"format": 1`)."""
    if weights.get("format") != 1:
        raise ValueError("quantize takes a float network (format 1)")
    width = weights["width"]
    mean, scale = weights["mean"], weights["scale"]
    folded = [[w / scale[j] for w in row] for j, row in enumerate(weights["denseIn"])]
    bias = [weights["bias"][i] - sum(mean[j] * folded[j][i] for j in range(len(folded))) for i in range(width)]
    largest = max([abs(w) for row in folded for w in row] + [abs(w) for row in weights["sparseIn"] for w in row] + [0.0])
    s1 = _shift_for(largest, I16, ACTIVATION)
    unit1 = ACTIVATION * 2.0**s1
    dense_q = [round(w * unit1) for row in folded for w in row]
    sparse_q = [round(w * unit1) for row in weights["sparseIn"] for w in row]
    bias_q = [round(b * unit1) for b in bias]

    head = []
    inputs = width
    for layer in weights["head"]:
        flat = [w for row in layer["w"] for w in row]
        s = _shift_for(max([abs(w) for w in flat] + [0.0]), I8, 1.0)
        head.append({"inputs": inputs, "shift": s, "w": _pack("b", [round(w * 2.0**s) for w in flat]),
                     "b": _pack("q", [round(b * ACTIVATION * 2.0**s) for b in layer["b"]])})
        inputs = len(layer["b"])
    out_w = weights["out"]["w"]
    so = _shift_for(max([abs(w) for w in out_w] + [0.0]), I8, 1.0)
    return {
        "format": FORMAT,
        "featuresVersion": weights["featuresVersion"],
        "catalog": weights["catalog"],
        "denseNames": weights["denseNames"],
        "keys": weights["keys"],
        "width": width,
        "activation": ACTIVATION,
        "shift": s1,
        "denseIn": _pack("h", dense_q),
        "sparseIn": _pack("h", sparse_q),
        "bias": _pack("q", bias_q),
        "head": head,
        "out": {"shift": so, "w": _pack("b", [round(w * 2.0**so) for w in out_w]),
                "b": round(weights["out"]["b"] * ACTIVATION * 2.0**so)},
        "handA": weights["handA"],
        "handB": weights["handB"],
        "handUnit": weights["handUnit"],
    }


def unpacked(weights: dict) -> dict:
    """The integer network's arrays as Python lists (once, for `correction`)."""
    return {
        "width": weights["width"],
        "activation": weights["activation"],
        "shift": weights["shift"],
        "index": {key: r for r, key in enumerate(weights["keys"])},
        "denseIn": _unpack("h", weights["denseIn"]),
        "sparseIn": _unpack("h", weights["sparseIn"]),
        "bias": _unpack("q", weights["bias"]),
        "head": [(layer["inputs"], layer["shift"], _unpack("b", layer["w"]), _unpack("q", layer["b"])) for layer in weights["head"]],
        "out": (weights["out"]["shift"], _unpack("b", weights["out"]["w"]), weights["out"]["b"]),
        "dense": len(weights["denseNames"]),
    }


def _clip(x: int, top: int) -> int:
    return 0 if x < 0 else (top if x > top else x)


def correction(net: dict, dense: list[int], sparse: list[list]) -> float:
    """net(x) in integers, in the Rust's order (`learned.rs`, `IntNetwork::correction`)."""
    width, top = net["width"], net["activation"]
    if len(dense) != net["dense"]:
        return 0.0
    acc = list(net["bias"])
    for j, x in enumerate(dense):
        if x:
            base = j * width
            for i in range(width):
                acc[i] += x * net["denseIn"][base + i]
    for key, count in sparse:
        r = net["index"].get(key)
        if r is not None:
            base = r * width
            for i in range(width):
                acc[i] += count * net["sparseIn"][base + i]
    hidden = [_clip(a >> net["shift"], top) for a in acc]
    for inputs, shift, w, b in net["head"]:
        nxt = []
        for k, bias in enumerate(b):
            total = bias
            for i in range(inputs):
                total += w[k * inputs + i] * hidden[i]
            nxt.append(_clip(total >> shift, top))
        hidden = nxt
    shift, w, b = net["out"]
    total = b
    for i, h in enumerate(hidden):
        total += w[i] * h
    return total / (top * 2.0**shift)


def points(weights: dict, net: dict, hand: float, dense: list[int], sparse: list[list]) -> float:
    if weights["handA"] == 0.0:
        return hand
    return hand + (weights["handB"] + correction(net, dense, sparse)) * weights["handUnit"] / weights["handA"]


def report(float_weights: dict, int_weights: dict, positions: Path) -> dict:
    """The integer network against the float one, held-out log loss on `positions` (its held-out
    games, as the fit split them) and the largest difference in their logits."""
    from . import data as D
    from . import export as E

    loaded = D.load([positions])
    _, held = D.split_by_game(loaded)
    net = unpacked(int_weights)
    a, b = float_weights["handA"], float_weights["handB"]
    rows = []
    for r in held:
        dense = [int(x) for x in loaded.dense[r]]
        sparse = [[loaded.keys[k], int(c)] for k, c in zip(loaded.sparse_idx[loaded.sparse_ptr[r] : loaded.sparse_ptr[r + 1]],
                                                             loaded.sparse_val[loaded.sparse_ptr[r] : loaded.sparse_ptr[r + 1]])]
        base = a * loaded.hand[r] / float_weights["handUnit"] + b
        rows.append((base + E.correction(float_weights, dense, sparse), base + correction(net, dense, sparse), float(loaded.outcome[r])))

    def loss(index: int) -> float:
        total = 0.0
        for row in rows:
            p = min(max(1 / (1 + math.exp(-row[index])), 1e-7), 1 - 1e-7)
            total += -(row[2] * math.log(p) + (1 - row[2]) * math.log(1 - p))
        return total / max(len(rows), 1)

    return {"rows": len(rows), "floatLogLoss": loss(0), "intLogLoss": loss(1),
            "maxLogitDifference": max((abs(x - y) for x, y, _ in rows), default=0.0)}


def parity(int_weights: dict, positions: Path, rows: int) -> dict:
    """The parity fixture (R1011000): an integer network and `rows` positions of `positions`, each with
    the correction `correction` computes, which the Rust must give bit for bit natively and in WASM."""
    net = unpacked(int_weights)
    cases = []
    with positions.open(encoding="utf-8") as handle:
        handle.readline()
        for line in handle:
            if len(cases) >= rows:
                break
            row = json.loads(line)
            # As text: the shortest digits that read back as the same double, which Rust's and JS's own
            # parsers read exactly (serde_json's default float parser may miss by a unit in the last place).
            value = correction(net, row["dense"], row["sparse"])
            cases.append({"dense": row["dense"], "sparse": row["sparse"], "expected": repr(value)})
    return {"network": int_weights, "cases": cases}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--weights", type=Path, help="a float network (export.py)")
    parser.add_argument("--random", type=int, metavar="SEED", help="instead of --weights: export.py's seeded random test network over --positions")
    parser.add_argument("--parity", type=Path, help="write the parity fixture (R1011000) over --positions")
    parser.add_argument("--rows", type=int, default=40)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--positions", type=Path, help="positions for --report")
    parser.add_argument("--report", type=Path, help="write the integer network's held-out loss against the float one's")
    args = parser.parse_args()
    if args.random is not None:
        from . import export as E

        float_weights = E.random_network(args.positions, args.random)
    else:
        float_weights = json.loads(args.weights.read_text(encoding="utf-8"))
    int_weights = quantize(float_weights)
    if args.parity:
        args.parity.parent.mkdir(parents=True, exist_ok=True)
        fixture = parity(int_weights, args.positions, args.rows)
        args.parity.write_text(json.dumps(fixture, separators=(",", ":")) + "\n", encoding="utf-8")
        print(f"wrote {args.parity}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(int_weights, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"wrote {args.out}")
    if args.report and args.positions:
        found = report(float_weights, int_weights, args.positions)
        args.report.write_text(json.dumps(found, indent=1), encoding="utf-8")
        print(json.dumps(found))


if __name__ == "__main__":
    main()
