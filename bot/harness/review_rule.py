"""The review rule: which approvals a commit needs before it ships. The deliver job counts the
votes against it (`deliver.py`), and the router uses it to pick a review run's reviewer
(`plan.review_seat`), so the two cannot disagree.

A commit ships when no model's rejection of it stands and it has

- one strong model's approval (Opus), or
- two medium models' approvals, the same model twice included (two Muse reviews count), or
- for an easy item only, one weak model's approval (Devin, Sonnet) and one medium model's.

That holds whatever the difficulty: a hard item ships on two medium approvals too. Approvals
count per review, never per family: each review run, and each run's own adversarial reviewer,
casts one. A weak approval counts only toward an easy item's weak-and-medium pair.
"""

from __future__ import annotations

from collections import Counter
from typing import Any, Callable, Iterable, Mapping

#: The rule in one line, for prompts, comments and the status issue.
SUMMARY = ("one strong model, or two medium ones (the same model may give both), or for an easy "
           "item one weak and one medium one")


def approvals(votes: Mapping[str, Any], family_tier: Callable[[str], str]) -> list[tuple[str, str]]:
    """Each approval of the head as (family, tier), one per review. A record kept before reviews
    were counted one by one holds one per approving family, at its recorded tier or else its
    family's strongest (`family_tier`)."""
    reviews = votes.get("reviews")
    if isinstance(reviews, list):
        return [(str(r.get("family") or ""),
                 str(r.get("tier") or family_tier(str(r.get("family") or ""))))
                for r in reviews if isinstance(r, Mapping)]
    tiers = votes.get("tiers") if isinstance(votes.get("tiers"), Mapping) else {}
    return [(str(f), str(tiers.get(f) or family_tier(str(f))))
            for f in votes.get("approvals") or []]


def met(tiers: Iterable[str], difficulty: str) -> bool:
    """Whether approvals at these tiers (one per review) let an item of `difficulty` ship."""
    count = Counter(tiers)
    return (count["strong"] >= 1 or count["medium"] >= 2
            or (difficulty == "easy" and count["medium"] >= 1 and count["weak"] >= 1))


def helps(tiers: Iterable[str], tier: str, difficulty: str) -> bool:
    """Whether one more approval at `tier` brings the head closer to the rule. A strong or medium
    one always does; a weak one only for an easy item that has no weak approval yet."""
    if tier != "weak":
        return True
    return difficulty == "easy" and "weak" not in set(tiers)


def reachable(tiers: Iterable[str], available: Iterable[str], difficulty: str) -> bool:
    """Whether reviewers of the `available` tiers could still take the head to the rule: any
    strong or medium one can (a medium model may review twice), a weak one only for an easy item
    that already has a medium approval."""
    tiers, available = list(tiers), set(available)
    if met(tiers, difficulty) or available & {"strong", "medium"}:
        return True
    return "weak" in available and met([*tiers, "weak"], difficulty)


def missing(tiers: Iterable[str], difficulty: str) -> str:
    """What the head still waits for, in words."""
    count = Counter(tiers)
    if difficulty == "easy":
        if count["medium"]:
            return ("one more review, by a strong, medium or weak model (the same medium model "
                    "may review it again)")
        if count["weak"]:
            return "a strong or medium model's review"
        return ("one strong model's review (Opus), two medium ones (the same model may give both), "
                "or a weak and a medium one")
    if count["medium"]:
        return "a strong or medium model's review (the same medium model may review it again)"
    return "one strong model's review (Opus), or two medium ones (the same model may give both)"


def who(approved: Iterable[tuple[str, str]]) -> str:
    """"`gemini` (medium), `muse` (medium) twice": the approvals so far, a family's repeats
    together."""
    count = Counter(approved)
    times = {1: "", 2: " twice"}
    return ", ".join(f"`{family}` ({tier}){times.get(n, f' {n} times')}"
                     for (family, tier), n in count.items())
