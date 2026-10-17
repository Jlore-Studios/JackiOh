"""The three models #1009 compares (docs/learned-eval.md D4, D5).

Each predicts the logit of the seat's outcome (1 a win, 0 a loss, a draw half of each) from a batch:

- (a) `HandModel`: the hand eval calibrated to a probability, `a·hand + b`.
- (b) `LinearModel`: a logistic over the dense features with the hand eval among them, the shape of
  session s15's linear fit (`training/improve.md`), which matched the hand eval and added nothing.
- (c) `NetModel`: D5's network with D4's skip connection, `a·hand + b + net(x)`. Its first layer is
  `width` wide over the standardized dense features and one learned row per sparse key (an
  `EmbeddingBag` summed with each key's count: NNUE's accumulator), through a clipped ReLU (0 to 1, as
  NNUE clips it, so part 6's integers can carry it), then a small dense head. Its output layer starts
  at zero and `a`, `b` at model (a)'s fit, so it starts as the calibrated hand eval and learns only
  what the hand eval misses.
"""

from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import nn


@dataclass
class Batch:
    hand: torch.Tensor  # float32 [n]
    dense: torch.Tensor  # float32 [n, d], standardized
    idx: torch.Tensor  # int64 [entries]: vocabulary rows
    offsets: torch.Tensor  # int64 [n]: where each row's entries start in `idx`
    val: torch.Tensor  # float32 [entries]: each entry's count
    outcome: torch.Tensor  # float32 [n]


#: The hand eval's points are large; the models read them in hundreds, so `a` starts near one.
HAND_UNIT = 100.0


class HandModel(nn.Module):
    """(a): a·hand + b."""

    def __init__(self) -> None:
        super().__init__()
        self.a = nn.Parameter(torch.tensor(0.5))
        self.b = nn.Parameter(torch.tensor(0.0))

    def forward(self, batch: Batch) -> torch.Tensor:
        return self.a * batch.hand / HAND_UNIT + self.b


class LinearModel(nn.Module):
    """(b): a·hand + w·dense + b."""

    def __init__(self, dense: int) -> None:
        super().__init__()
        self.hand = HandModel()
        self.w = nn.Linear(dense, 1, bias=False)
        nn.init.zeros_(self.w.weight)

    def forward(self, batch: Batch) -> torch.Tensor:
        return self.hand(batch) + self.w(batch.dense).squeeze(-1)


def clipped_relu(x: torch.Tensor) -> torch.Tensor:
    """ReLU clipped at one, as NNUE clips it."""
    return torch.clamp(x, 0.0, 1.0)


class NetModel(nn.Module):
    """(c): a·hand + b + head(clip(W·dense + Σ E[key]·count + c))."""

    def __init__(self, dense: int, vocab: int, width: int = 16, head: tuple[int, ...] = (16,)) -> None:
        super().__init__()
        self.hand = HandModel()
        self.width = width
        self.dense_in = nn.Linear(dense, width)
        self.sparse_in = nn.EmbeddingBag(max(vocab, 1), width, mode="sum")
        nn.init.normal_(self.sparse_in.weight, std=0.01)
        nn.init.normal_(self.dense_in.weight, std=0.05)
        nn.init.constant_(self.dense_in.bias, 0.5)
        layers: list[nn.Module] = []
        size = width
        for out in head:
            layers.append(nn.Linear(size, out))
            size = out
        self.head = nn.ModuleList(layers)
        self.out = nn.Linear(size, 1)
        nn.init.zeros_(self.out.weight)
        nn.init.zeros_(self.out.bias)

    def correction(self, batch: Batch) -> torch.Tensor:
        """net(x): the logit the network adds to the calibrated hand eval."""
        first = self.dense_in(batch.dense) + self.sparse_in(batch.idx, batch.offsets, per_sample_weights=batch.val)
        h = clipped_relu(first)
        for layer in self.head:
            h = clipped_relu(layer(h))
        return self.out(h).squeeze(-1)

    def forward(self, batch: Batch) -> torch.Tensor:
        return self.hand(batch) + self.correction(batch)

    def weight_count(self) -> int:
        return sum(p.numel() for p in self.parameters())
