"""The review rule: which approvals a commit needs before it ships. The deliver job counts the
votes against it (`deliver.py`), and the router uses it to pick a review run's reviewer
(`plan.review_seat`), so the two cannot disagree.

A commit ships when no model's rejection of it stands and its lane's final audit approved
it (#725): one strong model's approval (Opus) for a medium or hard item, one medium
model's for an easy item, whose scope the easy rule hard-limits. A strong approval ships
an easy item too. Approvals count per review, never per family: each review run, and each
run's own adversarial reviewer, casts one. A weak approval never ships anything; it is a
finding round, not a vote.

A commit that met the rule is recorded as cleared. When `main` then leaves it with conflicts,
the clearance carries to the revision that resolves them (`carried`), so no review run is
needed again: the revision started from the cleared commit, changed nothing but the files the
merge left conflicted, and its own run's adversarial reviewer (medium or strong) approved it
(`deliver._carry`). A revision that changed anything more goes back under the rule above.
"""

from __future__ import annotations

from collections import Counter
from typing import Any, Callable, Iterable, Mapping

#: The rule in one line, for prompts, comments and the status issue.
SUMMARY = "its lane's final audit: a strong model's approval, or a medium model's for an easy item"


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
    """Whether approvals at these tiers (one per review) let an item of `difficulty` ship:
    one strong approval for a medium or hard item, one medium (or strong) approval for an
    easy item (#725: the Medium audit ships Easy). A weak approval never ships anything."""
    count = Counter(tiers)
    if difficulty == "easy":
        return count["strong"] >= 1 or count["medium"] >= 1
    return count["strong"] >= 1


def carried(votes: Mapping[str, Any]) -> bool:
    """The head carries a cleared commit's clearance across a conflict resolution, and its own
    run's reviewer approved it: that one approval is enough."""
    return bool(votes.get("carried")) and bool(votes.get("approvals"))


def helps(tiers: Iterable[str], tier: str, difficulty: str) -> bool:
    """Whether one more approval at `tier` brings the head closer to the rule. A strong one
    always does; a medium one only where it ships (an easy item's final audit); a weak one
    never does (#725: weak self-review rounds are findings, and no weak review runs go out)."""
    if tier == "strong":
        return True
    if tier == "medium":
        return difficulty == "easy"
    return False


def reachable(tiers: Iterable[str], available: Iterable[str], difficulty: str) -> bool:
    """Whether reviewers of the `available` tiers could still take the head to the rule: a
    strong one always can, and a medium one can for an easy item (#725)."""
    tiers = list(tiers)
    if met(tiers, difficulty):
        return True
    have = set(available)
    if "strong" in have:
        return True
    return difficulty == "easy" and "medium" in have


def missing(tiers: Iterable[str], difficulty: str) -> str:
    """What the head still waits for, in words: a strong model's final audit, or a medium
    model's for an easy item (#725)."""
    if difficulty == "easy":
        return "a medium model's final audit (sooner) or a strong model's (Opus)"
    return "one strong model's final audit (Opus)"


def who(approved: Iterable[tuple[str, str]]) -> str:
    """"`gemini` (medium), `muse` (medium) twice": the approvals so far, a family's repeats
    together."""
    count = Counter(approved)
    times = {1: "", 2: " twice"}
    return ", ".join(f"`{family}` ({tier}){times.get(n, f' {n} times')}"
                     for (family, tier), n in count.items())
