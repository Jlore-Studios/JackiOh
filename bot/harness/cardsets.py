"""Which set's cards win most, in the status issue's statistics (#148): each set's card win rates
over the last three patches with 1,000 games or more, read once a day from production's public card
statistics (`GET /api/stats/cards`, SPEC §9.11) and kept in the issue between reads.

- **A patch** is a patch or major version with its micros (`v0.4.0`, `v0.4.0b`, …): the window
  every card shares (R1367000). Its figures are the endpoint's at its newest version, found by
  walking back from production's newest version through `previousPatch`.
- **Its games** are its live games once it has the gate's 1,000 of them (the endpoint then counts
  nothing else, R654), else its live and AI development games together (`?source=live` plus
  `?source=dev`, the provisional figures the gate pads with). A patch counts with `MIN_GAMES`.
- **A card's win rate** is R377's in-deck rate: its deck's wins over the games it was in a deck.
  A set's pooled rate is its cards' wins over their games; its mean and median are over the cards
  with the endpoint's `minSample` games or more. Decks mix sets (SPEC §2), so a set's rate says how
  its cards fare in mixed decks, not how a deck of it would.

A count of the statistics reads the figures afresh once `EVERY` has passed since the last good read,
or `RETRY` after a failed one; a failed read keeps the last good section and says when it failed.
Only the status loop reads production (`fetch_json`); with no fetcher, as in a test, the section
the issue holds is kept as it is.
"""

from __future__ import annotations

import json
import math
import re
import statistics
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import timedelta
from typing import Any, Callable, Mapping

from harness.clock import iso, parse_iso, zone
from harness.context import Context

START = "<!-- jackioh-bot:card-sets -->"
END = "<!-- /jackioh-bot:card-sets -->"
HEADING = "### 🃏 Which set's cards win most"
#: Production's server (`render.yaml`), whose public card statistics are read.
API = "https://jackioh-server.onrender.com"
#: How often the figures are read, and how soon a failed read is tried again.
EVERY = timedelta(days=1)
RETRY = timedelta(hours=1)
#: How many patches are read, and the games a patch needs to be one of them.
PATCHES = 3
MIN_GAMES = 1000
#: The most versions, micros included, the walk back reads, so a long run of thin patches costs a
#: bounded number of calls.
WALK = 30
#: A sleeping Render instance takes up to a minute to wake.
TIMEOUT_SECONDS = 120
#: A card this many standard errors from 50% is listed.
OUTLIER_Z = 3.0
#: The endpoint's sample floor (`CARD_STATS_MIN_SAMPLE`), when a response does not say it.
MIN_SAMPLE = 20
#: How much of a failed read's error the section repeats.
ERROR_SHOWN = 160
#: `v0.4.0b` belongs to the patch `v0.4.0`.
VERSION = re.compile(r"^(v\d+\.\d+\.\d+)[a-z]*$")
FAILED = re.compile(r"^> ⚠️ .*\n\n", re.M)

Fetch = Callable[[str], Any]


def fetch_json(url: str) -> Any:
    """`url`'s JSON, the way the status loop reads production."""
    request = urllib.request.Request(url, headers={"Accept": "application/json",
                                                   "User-Agent": "jackioh-bot"})
    with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:
        return json.load(response)


@dataclass
class Card:
    id: str
    name: str
    set: str
    wins: int = 0
    games: int = 0


@dataclass
class Patch:
    #: `v0.4.0`, and the newest of its versions production has shipped, read for its figures.
    name: str
    version: str
    #: Whether it has the gate's live games, so its figures are live games alone.
    live: bool
    games: int
    cards: dict[str, Card] = field(default_factory=dict)


@dataclass
class SetRow:
    name: str
    cards: int
    games: int
    wins: int
    #: The win rate of each of its cards with the sample floor's games.
    rates: list[float]
    by_patch: dict[str, float | None]

    @property
    def pooled(self) -> float | None:
        return self.wins / self.games if self.games else None

    @property
    def mean(self) -> float | None:
        return statistics.fmean(self.rates) if self.rates else None

    @property
    def median(self) -> float | None:
        return statistics.median(self.rates) if self.rates else None

    @property
    def error(self) -> float | None:
        """The pooled rate's standard error, as if every card-game were independent: they are not
        (one game counts every card of both decks), so it is a floor."""
        p = self.pooled
        return math.sqrt(p * (1 - p) / self.games) if p is not None else None


def _url(base: str, version: str | None, source: str) -> str:
    query = {"source": source} | ({"patch": version} if version else {})
    return f"{base}/api/stats/cards?{urllib.parse.urlencode(query)}"


def _add(cards: dict[str, Card], rows: Any) -> None:
    for row in rows or []:
        games = int(row.get("games") or 0)
        rate = row.get("winRate")
        card = cards.setdefault(str(row["id"]), Card(str(row["id"]), str(row.get("name") or row["id"]),
                                                     str(row.get("set") or "?")))
        card.games += games
        card.wins += round(float(rate) * games) if rate is not None else 0


def read(fetch: Fetch, base: str = API) -> tuple[list[Patch], int, str]:
    """The last `PATCHES` patches with `MIN_GAMES` games or more, newest first; the endpoint's
    sample floor; and the oldest version the walk read."""
    found: list[Patch] = []
    seen: set[str] = set()
    version: str | None = None
    min_sample = MIN_SAMPLE
    for _ in range(WALK):
        live = fetch(_url(base, version, "live"))
        version = str(live.get("patch") or "")
        min_sample = int(live.get("minSample") or min_sample)
        match = VERSION.match(version)
        name = match.group(1) if match else version
        if name not in seen:  # the newest version of its patch: its window holds the whole patch
            seen.add(name)
            cleared = bool((live.get("gate") or {}).get("cleared"))
            patch = Patch(name, version, cleared, int(live.get("totalGames") or 0))
            _add(patch.cards, live.get("cards"))
            if not cleared:
                dev = fetch(_url(base, version, "dev"))
                patch.games += int(dev.get("totalGames") or 0)
                _add(patch.cards, dev.get("cards"))
            if patch.games >= MIN_GAMES:
                found.append(patch)
                if len(found) == PATCHES:
                    break
        previous = live.get("previousPatch")
        if not previous:
            break
        version = str(previous)
    return found, min_sample, version or ""


def _total(patches: list[Patch]) -> list[Card]:
    """Each card's wins and games over every patch read."""
    total: dict[str, Card] = {}
    for patch in patches:
        for card in patch.cards.values():
            into = total.setdefault(card.id, Card(card.id, card.name, card.set))
            into.wins += card.wins
            into.games += card.games
    return [card for card in total.values() if card.games]


def rows(patches: list[Patch], min_sample: int) -> list[SetRow]:
    """Each set over every patch read, highest pooled rate first."""
    cards = _total(patches)
    out = []
    for name in sorted({card.set for card in cards}):
        mine = [card for card in cards if card.set == name]
        by_patch = {}
        for patch in patches:
            held = [card for card in patch.cards.values() if card.set == name]
            games = sum(card.games for card in held)
            by_patch[patch.name] = sum(card.wins for card in held) / games if games else None
        out.append(SetRow(name, len(mine), sum(c.games for c in mine), sum(c.wins for c in mine),
                          [c.wins / c.games for c in mine if c.games >= min_sample], by_patch))
    return sorted(out, key=lambda row: -(row.pooled or 0))


def outliers(patches: list[Patch], min_sample: int) -> list[tuple[Card, float]]:
    """The cards `OUTLIER_Z` or more standard errors from 50% over every patch read, furthest
    first."""
    out = []
    for card in _total(patches):
        if card.games >= min_sample:
            z = (card.wins / card.games - 0.5) / math.sqrt(0.25 / card.games)
            if abs(z) >= OUTLIER_Z:
                out.append((card, z))
    return sorted(out, key=lambda pair: -abs(pair[1]))


def _pct(value: float | None) -> str:
    return f"{value * 100:.1f}%" if value is not None else "—"


def _best(table: list[SetRow], key: Callable[[SetRow], float | None]) -> str:
    scored = [(key(row), row.name) for row in table if key(row) is not None]
    return f"**{max(scored)[1]}**" if scored else "—"


def _when(ctx: Context) -> str:
    return ctx.now().astimezone(zone(ctx.cfg.timezone)).strftime("%Y-%m-%d %H:%M")


def draw(ctx: Context, patches: list[Patch], min_sample: int, oldest: str) -> str:
    """The section for figures just read."""
    lines = [START, HEADING, ""]
    if not patches:
        return "\n".join(lines + [
            f"_Read at {_when(ctx)} from production's card statistics (`/api/stats/cards`): no "
            f"patch back to {oldest or 'the first'} has {MIN_GAMES:,} games yet. Read again once "
            "a day._", "", END])
    names = [f"{p.name} ({p.games:,} {'live' if p.live else 'live and AI'} games)" for p in patches]
    listed = ", ".join(names[:-1]) + f" and {names[-1]}" if len(names) > 1 else names[0]
    which = f"the last {len(patches)} patches" if len(patches) > 1 else "the one patch"
    lines += [f"_Read at {_when(ctx)} from production's card statistics (`/api/stats/cards`), once "
              f"a day: {which} with {MIN_GAMES:,} games or more, {listed}._", ""]
    table = rows(patches, min_sample)
    lines += ["| Set | Cards | Card-games | Pooled | Mean | Median | "
              + " | ".join(patch.name for patch in patches) + " |",
              "|---|---:|---:|---:|---:|---:|" + "---:|" * len(patches)]
    for row in table:
        error = f" ±{row.error * 100:.1f}" if row.error is not None else ""
        lines.append(f"| {row.name} | {row.cards} | {row.games:,} | {_pct(row.pooled)}{error} | "
                     f"{_pct(row.mean)} | {_pct(row.median)} | "
                     + " | ".join(_pct(row.by_patch[patch.name]) for patch in patches) + " |")
    lines += ["", f"Highest pooled: {_best(table, lambda row: row.pooled)}; mean: "
              f"{_best(table, lambda row: row.mean)}; median: "
              f"{_best(table, lambda row: row.median)}.", ""]
    far = outliers(patches, min_sample)
    if far:
        lines += [f"<details><summary>Cards {OUTLIER_Z:g} or more standard errors from 50% "
                  f"({len(far)})</summary>", "", "| Card | Set | Win rate | Games | z |",
                  "|---|---|---:|---:|---:|"]
        lines += [f"| {card.name} (`{card.id}`) | {card.set} | {_pct(card.wins / card.games)} | "
                  f"{card.games:,} | {z:+.1f} |" for card, z in far]
        lines += ["", "</details>", ""]
    lines += ["<sub>A card's win rate is its deck's, over the games it was in a deck (R377); a "
              "set's pooled rate is its cards' wins over their games, its mean and median over "
              f"the cards with {min_sample} games or more. Decks mix sets, so this is how a set's "
              "cards fare in mixed decks. ± is one standard error as if every card-game were "
              "independent, which they are not, so sets less than two or three ± apart are within "
              "noise.</sub>", "", END]
    return "\n".join(lines)


def kept(body: str | None) -> str | None:
    """The section a count of the statistics keeps between reads, or None."""
    text = str(body or "")
    start, end = text.find(START), text.find(END)
    if start < 0 or end < start:
        return None
    return text[start:end + len(END)]


def due(ctx: Context, state: Mapping[str, Any]) -> bool:
    last = state.get("card_sets") or {}
    at = parse_iso(last.get("at"))
    return at is None or ctx.now() - at >= (EVERY if last.get("ok") else RETRY)


def _noted(ctx: Context, ok: bool) -> None:
    at = iso(ctx.now())
    ctx.store.update(lambda s: s.update(card_sets={"at": at, "ok": ok}), "card sets")


def section(ctx: Context, state: Mapping[str, Any], old: str | None, fetch: Fetch | None) -> str:
    """The section for a count of the statistics: `old` until a read is due (or with no `fetch`),
    else read afresh. A failed read keeps `old`, or an empty section, and says it failed."""
    if fetch is None or (old is not None and not due(ctx, state)):
        return old or ""
    try:
        text = draw(ctx, *read(fetch))
    except (OSError, ValueError, LookupError, TypeError, AttributeError) as exc:
        _noted(ctx, ok=False)
        failed = (f"> ⚠️ The read at {_when(ctx)} failed ({type(exc).__name__}: "
                  f"{' '.join(str(exc).split())[:ERROR_SHOWN]}); it is tried again in "
                  f"{int(RETRY.total_seconds() // 60)} minutes.")
        base = FAILED.sub("", old or "\n".join([START, HEADING, "", END]))
        return base.replace(HEADING + "\n\n", f"{HEADING}\n\n{failed}\n\n", 1)
    _noted(ctx, ok=True)
    return text
