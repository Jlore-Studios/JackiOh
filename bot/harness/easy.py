"""The easy rule: what `difficulty:easy` means, so that what Devin may build is what Devin finishes
(#317, "The easy rule").

Devin, the weak tier, may build only easy items. On this repository it could not resolve the
conflicts in SPEC §11 and the rulings index (every pull request appends the next R number there),
could not hold a big change together, cannot plan, runs no tests where it works, and cannot judge
what it cannot see. Its clean merges were all small (7–15 files, under about 500 lines).

The rule is in the planner's prompt (`RULE`, `bot/prompts/plan.md`), and the harness holds the
rating to it twice, trusting nothing the model said: deliver checks the plan's "Files to touch"
table before the item is queued (`plan_breach`), and the real change before a weak builder's work
is pushed (`change_breach`). Either breach makes the item medium. The limits are settings
(`easy` in `.harness/config.json`, `config.EasyRule`).
"""

from __future__ import annotations

import re
from typing import Any, Iterable

from harness.git import matches

#: The rule as the planner reads it. `$max_files` and `$max_lines` are filled from the settings.
RULE = """An item is **easy** only if every line below holds. If any line fails, or you cannot
tell, it is **medium** at least.

1. **Small.** At most $max_files files changed in all, the tests it adds included, and at most
   $max_lines lines added plus removed (generated files such as
   `packages/cards/src/scripts/_generated.ts` aside).
2. **One place.** All of it inside one package or app, except a test of the same feature in that
   package's `test/`.
3. **None of the conflict hot spots:** `SPEC.md` (no new §11 row, no ruling cited or renumbered),
   `BUILD.md`, `packages/engine/test/rulings.test.ts`.
4. **None of the shared surfaces:** `packages/shared/src/events.ts`, `packages/engine/src/script.ts`,
   `packages/engine/src/state.ts`, `packages/engine/src/effects/index.ts`.
5. **No rules or data work:** nothing in `packages/engine/src/` or `packages/ai/src/`, no change to
   `packages/cards/catalog.json`, `packages/cards/patches/` or `CATALOG_VERSION`, no card script,
   no database migration.
6. **No review-only or forbidden path** (every `package.json`, the lockfile, the build and test
   configs, `scripts/`, `vercel.json`, `render.yaml`; `.github/`, `.harness/`, `bot/`), and no new
   dependency.
7. **Checked by a unit test** in the same package, which the plan names; no Cypress or e2e spec
   change, and no judging by eye (animation, layout, sound or art tuning).
8. **No decisions left:** the plan names each file, what changes in it, and the test that proves
   it. If the builder would choose between two designs, it is not easy.
9. **Stands alone:** blocked by no open issue, and no open pull request touches the same files.

Never easy: a patch that changes cards or rules, anything that adds an R number, engine or AI
work, visual polish.

- **medium:** anything that fails the easy rule and is not hard: a new ruling, a card or catalog
  change, a shared surface, several packages, e2e work, visual polish.
- **hard:** engine rules or the resolution loop (`packages/engine/src/`), the AI's search
  (`packages/ai/src/`), design across packages, a database migration, or about 40 files or more.

When torn between two, choose the higher."""

_TABLE_HEADING = re.compile(r"(?im)^\s*(?:#+\s*)?(?:\d+\.\s*)?\**\s*files to touch\b")
_PATH = re.compile(r"`([^`]+)`")


def plan_paths(plan: str) -> list[str]:
    """The paths in the first column of the plan's "Files to touch" table, in order.

    A row's first cell may hold its path in backticks or bare; a header row, the separator row and
    a cell that is not a path (no `/` or `.`) are skipped."""
    text = str(plan or "")
    heading = _TABLE_HEADING.search(text)
    if heading is None:
        return []
    found: list[str] = []
    started = False
    for line in text[heading.end():].splitlines():
        row = line.strip()
        if not row.startswith("|"):
            if started:
                break
            continue
        started = True
        cells = [cell.strip() for cell in row.strip("|").split("|")]
        first = cells[0] if cells else ""
        if not first or set(first) <= set("-: "):
            continue
        quoted = _PATH.findall(first)
        path = (quoted[0] if quoted else first).strip()
        if "/" not in path and "." not in path:
            continue  # the header row ("Path", "File"), or a cell that names no file
        if path not in found:
            found.append(path)
    return found


def _off_limits(paths: Iterable[str], patterns: Iterable[str]) -> list[str]:
    return [path for path in paths if matches(path, patterns)]


def plan_breach(plan: str, rule: Any, review_paths: Iterable[str],
                forbidden_paths: Iterable[str]) -> str:
    """Why a plan rated easy breaks the rule (lines 1, 3–6, from its "Files to touch" table), or
    "" when it does not. A plan with no such table cannot show it holds, so it breaks line 8."""
    paths = plan_paths(plan)
    if not paths:
        return "line 8: the plan has no **Files to touch** table, so nothing shows it is small"
    if len(paths) > rule.max_files:
        return f"line 1: the plan touches {len(paths)} files, over {rule.max_files}"
    bad = _off_limits(paths, rule.off_limits)
    if bad:
        return f"lines 3–5: the plan touches {', '.join(f'`{p}`' for p in bad[:5])}"
    bad = _off_limits(paths, [*review_paths, *forbidden_paths])
    if bad:
        return f"line 6: the plan touches {', '.join(f'`{p}`' for p in bad[:5])}"
    return ""


def change_breach(counts: dict[str, int], rule: Any) -> str:
    """Why a change built for an easy item breaks the rule (lines 1 and 3–5, from what it really
    changed: lines per path, `Git.numstat`), or "" when it does not."""
    paths = [p for p in counts if not matches(p, rule.generated)]
    if len(paths) > rule.max_files:
        return f"line 1: it changes {len(paths)} files, over {rule.max_files}"
    lines = sum(counts[p] for p in paths)
    if lines > rule.max_lines:
        return f"line 1: it changes {lines} lines, over {rule.max_lines}"
    bad = _off_limits(paths, rule.off_limits)
    if bad:
        return f"lines 3–5: it changes {', '.join(f'`{p}`' for p in bad[:5])}"
    return ""
