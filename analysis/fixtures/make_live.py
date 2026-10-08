"""Relabel a development run's records as live-shaped ones, for the notebooks' fixtures.

    python3 analysis/fixtures/make_live.py dev.jsonl > analysis/fixtures/live-40.jsonl

The games stay as they were played. What changes is what no development run has: the source is `live`, the ids
are `live:<n>` (a live id is a match id), the first LIVE_OLD_GAMES games are filed under the patch before the
newest and the rest under the newest, the mode alternates bo1 and bo3, and the seats are two humans in a bo1 and a
human against the AI in a bo3. Not a live game of any player: a file that has the live games' shape, so the notebooks' live
columns, patch comparisons and pilot splits have something to run on.
"""

import json
import sys

LIVE_OLD_GAMES = 16
OLD_PATCH = "v0.3.0"
NEW_PATCH = "v0.3.1"


def relabel(number: int, record: dict) -> dict:
    record["id"] = f"live:{number}"
    record["source"] = "live"
    record["patch"] = OLD_PATCH if number <= LIVE_OLD_GAMES else NEW_PATCH
    record["mode"] = "bo1" if number % 2 else "bo3"
    record["pilots"] = {"p1": "human", "p2": "human" if number % 2 else "ai"}
    return record


def main(path: str) -> None:
    with open(path, encoding="utf-8") as lines:
        for number, line in enumerate(filter(str.strip, lines), start=1):
            print(json.dumps(relabel(number, json.loads(line)), separators=(",", ":")))


if __name__ == "__main__":
    main(sys.argv[1])
