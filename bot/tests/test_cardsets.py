"""Which set's cards win most, in the status issue's statistics (bot/harness/cardsets.py, #148):
the last three patches with 1,000 games or more, each read at its newest version from production's
`/api/stats/cards`, each set's pooled, mean and median card win rate over them, read once a day and
kept in the issue between reads."""

from __future__ import annotations

import unittest
import urllib.error
import urllib.parse
from datetime import timedelta

from harness import cardsets, dashboard, stats

from tests.support import DAY, make_ctx
from tests.test_stats import world


def payload(version, previous, *, cleared, games, cards):
    """One `/api/stats/cards` response: `cards` as (id, name, set, wins, games)."""
    return {"patch": version, "previousPatch": previous, "gate": {"cleared": cleared},
            "minSample": 20, "totalGames": games,
            "cards": [{"id": i, "name": n, "set": s, "games": g,
                       "winRate": (w / g if g else None)} for i, n, s, w, g in cards]}


# Production's history, newest first: (version, live, dev). A cleared patch's dev figures are its
# live ones, as the endpoint gives them.
CORE = ("core-001", "Stockpile", "Core")
CLASSIC = ("classic-001", "Palantir", "Classic")
HISTORY = [
    ("v0.4.2b", payload("v0.4.2b", "v0.4.2", cleared=True, games=1200,
                        cards=[(*CORE, 700, 1200), (*CLASSIC, 540, 1200)]), None),
    ("v0.4.2", payload("v0.4.2", "v0.4.1", cleared=True, games=1000,
                       cards=[(*CORE, 1, 1)]), None),
    ("v0.4.1", payload("v0.4.1", "v0.4.0c", cleared=False, games=300,
                       cards=[(*CORE, 150, 300)]),
     payload("v0.4.1", "v0.4.0c", cleared=False, games=400, cards=[(*CORE, 200, 400)])),
    ("v0.4.0c", payload("v0.4.0c", "v0.4.0", cleared=False, games=200,
                        cards=[(*CORE, 100, 200), (*CLASSIC, 90, 200)]),
     payload("v0.4.0c", "v0.4.0", cleared=False, games=900,
             cards=[(*CORE, 450, 900), (*CLASSIC, 460, 900), ("classic-002", "Rare", "Classic", 5,
                                                              10)])),
    ("v0.4.0", payload("v0.4.0", "v0.3.5", cleared=False, games=1, cards=[]), None),
    ("v0.3.5", payload("v0.3.5", "v0.3.4", cleared=True, games=1500,
                       cards=[(*CORE, 750, 1500), (*CLASSIC, 760, 1500)]), None),
    ("v0.3.4", payload("v0.3.4", None, cleared=True, games=5000, cards=[]), None),
]


class Production:
    """A fake `/api/stats/cards`, recording what it was asked."""

    def __init__(self, history=HISTORY):
        self.by = {version: (live, dev) for version, live, dev in history}
        self.newest = history[0][0]
        self.asked: list[tuple[str, str]] = []

    def __call__(self, url):
        query = dict(urllib.parse.parse_qsl(urllib.parse.urlsplit(url).query))
        version, source = query.get("patch", self.newest), query["source"]
        self.asked.append((version, source))
        live, dev = self.by[version]
        return dev if source == "dev" and dev is not None else live


class ReadTests(unittest.TestCase):

    def test_the_last_three_patches_with_1000_games_each_read_at_its_newest_version(self):
        api = Production()
        patches, min_sample, oldest = cardsets.read(api)
        # v0.4.2 at v0.4.2b (its micro), v0.4.1 skipped at 700 games, v0.4.0 at v0.4.0c with its
        # AI games (200 live + 900), v0.3.5; the walk stops there.
        self.assertEqual([(p.name, p.version, p.live, p.games) for p in patches],
                         [("v0.4.2", "v0.4.2b", True, 1200), ("v0.4.0", "v0.4.0c", False, 1100),
                          ("v0.3.5", "v0.3.5", True, 1500)])
        self.assertEqual((min_sample, oldest), (20, "v0.3.5"))
        # A cleared patch is read once (live alone); an uncleared one adds its AI games; an older
        # version of a patch already read is only walked through.
        self.assertEqual(api.asked, [("v0.4.2b", "live"), ("v0.4.2", "live"), ("v0.4.1", "live"),
                                     ("v0.4.1", "dev"), ("v0.4.0c", "live"), ("v0.4.0c", "dev"),
                                     ("v0.4.0", "live"), ("v0.3.5", "live")])
        self.assertEqual((patches[1].cards["core-001"].wins, patches[1].cards["core-001"].games),
                         (550, 1100))

    def test_sets_pool_every_card_and_take_mean_and_median_over_the_sampled_ones(self):
        patches, min_sample, _ = cardsets.read(Production())
        rows = {row.name: row for row in cardsets.rows(patches, min_sample)}
        self.assertEqual(list(rows), ["Core", "Classic"])  # highest pooled first
        core, classic = rows["Core"], rows["Classic"]
        self.assertEqual((core.cards, core.games, core.wins), (1, 3800, 2000))
        # classic-002's 10 games count in the pool but not in the mean or median (under 20).
        self.assertEqual((classic.cards, classic.games, classic.wins), (2, 3810, 1855))
        self.assertEqual(classic.rates, [1850 / 3800])
        self.assertAlmostEqual(core.by_patch["v0.4.2"], 700 / 1200)
        self.assertAlmostEqual(classic.by_patch["v0.4.0"], 555 / 1110)

    def test_the_section_names_the_highest_set_and_the_cards_far_from_half(self):
        ctx = make_ctx(at=DAY)
        text = cardsets.draw(ctx, *cardsets.read(Production()))
        self.assertTrue(text.startswith(cardsets.START) and text.endswith(cardsets.END))
        self.assertIn("v0.4.2 (1,200 live games), v0.4.0 (1,100 live and AI games) and v0.3.5 "
                      "(1,500 live games)", text)
        self.assertIn("| Set | Cards | Card-games | Pooled | Mean | Median | v0.4.2 | v0.4.0 | "
                      "v0.3.5 |", text)
        self.assertIn("| Core | 1 | 3,800 | 52.6% ±0.8 | 52.6% | 52.6% | 58.3% | 50.0% | 50.0% |",
                      text)
        self.assertIn("Highest pooled: **Core**; mean: **Core**; median: **Core**.", text)
        # Stockpile, 2000 of 3800, is 3.2 standard errors over half.
        self.assertIn("| Stockpile (`core-001`) | Core | 52.6% | 3,800 | +3.2 |", text)

    def test_no_patch_with_enough_games_says_so(self):
        thin = [("v0.1.0", payload("v0.1.0", None, cleared=False, games=10, cards=[]),
                 payload("v0.1.0", None, cleared=False, games=10, cards=[]))]
        text = cardsets.draw(make_ctx(at=DAY), *cardsets.read(Production(thin)))
        self.assertIn("no patch back to v0.1.0 has 1,000 games yet", text)
        alone = [("v0.3.5", payload("v0.3.5", None, cleared=True, games=1500,
                                    cards=[(*CORE, 750, 1500)]), None)]
        one = cardsets.draw(make_ctx(at=DAY), *cardsets.read(Production(alone)))
        self.assertIn("once a day: the one patch with 1,000 games or more, v0.3.5 (1,500 live "
                      "games)._", one)


class OnceADayTests(unittest.TestCase):

    def test_read_once_a_day_and_a_failed_read_keeps_the_last_with_a_note(self):
        ctx = make_ctx(at=DAY)
        api = Production()
        first = cardsets.section(ctx, ctx.store.load(), None, api)
        self.assertIn("Highest pooled", first)
        self.assertEqual(ctx.store.load()["card_sets"]["ok"], True)
        # Within the day the section is kept, unread.
        soon = make_ctx(ctx.gh, at=DAY + timedelta(hours=23))
        self.assertEqual(cardsets.section(soon, soon.store.load(), first, api), first)
        self.assertEqual(len(api.asked), 8)
        # No fetcher (a test, Squishy's loop): kept as it is.
        self.assertEqual(cardsets.section(soon, soon.store.load(), first, None), first)

        def down(url):
            raise urllib.error.URLError("timed out")
        later = make_ctx(ctx.gh, at=DAY + timedelta(days=1))
        failed = cardsets.section(later, later.store.load(), first, down)
        self.assertIn("> ⚠️ The read at 2026-09-30 12:00 failed (URLError: <urlopen error timed "
                      "out>); it is tried again in 60 minutes.", failed)
        self.assertIn("Highest pooled", failed)  # the last good figures stay
        # A second failure an hour on replaces the note, never stacks it.
        hour = make_ctx(ctx.gh, at=DAY + timedelta(days=1, hours=1))
        again = cardsets.section(hour, hour.store.load(), failed, down)
        self.assertEqual(again.count("> ⚠️"), 1)
        self.assertEqual(again.replace("13:00", "12:00"), failed)
        # A good read clears it.
        back = make_ctx(ctx.gh, at=DAY + timedelta(days=1, hours=2))
        self.assertNotIn("⚠️", cardsets.section(back, back.store.load(), again, api))
        # A first read that fails still draws the section, saying so.
        bare = cardsets.section(make_ctx(at=DAY), {}, None, down)
        self.assertEqual(cardsets.kept(bare), bare)
        self.assertIn("failed (URLError", bare)

    def test_the_status_issue_carries_it_in_the_statistics(self):
        gh, ctx = world()
        api = Production()
        dashboard.update(ctx, stats=True, read_cards=api)
        [number] = [n for n, t in gh.threads.items() if t["title"] == dashboard.TITLE]
        body = gh.threads[number]["body"]
        section = cardsets.kept(body)
        self.assertIn(section, stats.kept(body))
        self.assertLess(body.index("### The last two weeks"), body.index(cardsets.START))
        self.assertLess(body.index(cardsets.END), body.index("### Per window"))
        # The statistics are counted again in 51 minutes; the section is kept, not read again.
        asked = len(api.asked)
        later = make_ctx(gh, at=DAY + timedelta(minutes=51))
        dashboard.update(later, stats=True, read_cards=api)
        self.assertEqual(len(api.asked), asked)
        self.assertEqual(cardsets.kept(gh.threads[number]["body"]), section)
        # A day on, the next count reads it afresh.
        tomorrow = make_ctx(gh, at=DAY + timedelta(days=1))
        dashboard.update(tomorrow, stats=True, read_cards=api)
        self.assertEqual(len(api.asked), 2 * asked)
        self.assertIn("Read at 2026-09-30 12:00", gh.threads[number]["body"])


if __name__ == "__main__":
    unittest.main()
