"""docs/repository-secrets.md lists every secret and variable the workflows read, and which workflows
read each, so no session asks a person for a secret that is already set (#3882). A workflow that names
a secret or variable the page does not list fails here, and so does a row no workflow reads that does
not say so."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

#: The repository's root, two directories above this file (bot/tests/).
REPO_ROOT = Path(__file__).resolve().parents[2]
PAGE = REPO_ROOT / "docs" / "repository-secrets.md"
GITHUB = REPO_ROOT / ".github"

#: GitHub's own token, which nobody sets.
BUILT_IN = {"GITHUB_TOKEN"}
#: What a row of a secret no workflow reads says in its last cell.
NO_WORKFLOW = "no workflow"

#: `secrets.NAME`, `vars.NAME`, and a matrix's `secret: NAME` that a step reads as `secrets[matrix.secret]`.
READS = (
    re.compile(r"\bsecrets\.([A-Z][A-Z0-9_]*)"),
    re.compile(r"\bvars\.([A-Z][A-Z0-9_]*)"),
    re.compile(r"^\s*secret:\s*([A-Z][A-Z0-9_]*)\s*$", re.M),
)


def workflow_reads():
    """Each name the files under .github/ read, to the workflows (file names, no `.yml`) reading it."""
    readers: dict[str, set[str]] = {}
    for path in sorted(GITHUB.rglob("*.yml")) + sorted(GITHUB.rglob("*.yaml")):
        text = path.read_text(encoding="utf-8")
        for pattern in READS:
            for name in pattern.findall(text):
                if name not in BUILT_IN:
                    readers.setdefault(name, set()).add(path.stem)
    return readers


def names_in(cell: str):
    """The names a row's first cell gives: one, or a range `A_2` to `A_8`."""
    names = re.findall(r"`([A-Z][A-Z0-9_]*)`", cell)
    if " to " in cell and len(names) == 2:
        first, last = (re.fullmatch(r"(.*?)(\d+)", name) for name in names)
        assert first and last and first.group(1) == last.group(1), f"a range of one stem: {cell}"
        return [f"{first.group(1)}{n}" for n in range(int(first.group(2)), int(last.group(2)) + 1)]
    return names


def page_rows():
    """Each name the page's tables list, to its row's last cell."""
    rows: dict[str, str] = {}
    for line in PAGE.read_text(encoding="utf-8").splitlines():
        if not line.startswith("| `"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        for name in names_in(cells[0]):
            assert name not in rows, f"{name} is listed twice"
            rows[name] = cells[-1]
    return rows


def listed_readers(cell: str):
    """The workflows a row's last cell names: comma-separated, each before any parenthesis."""
    return {re.split(r"\s*\(", part.strip())[0] for part in cell.split(",") if part.strip()}


class RepositorySecretsTest(unittest.TestCase):
    def test_every_secret_and_variable_a_workflow_reads_is_listed(self):
        rows = page_rows()
        missing = sorted(set(workflow_reads()) - set(rows))
        self.assertEqual(missing, [], "add a row for each to docs/repository-secrets.md")

    def test_each_row_names_the_workflows_that_read_it(self):
        reads = workflow_reads()
        for name, cell in page_rows().items():
            with self.subTest(name=name):
                if cell.startswith(NO_WORKFLOW):
                    self.assertNotIn(name, reads, f"{name} is read by {sorted(reads.get(name, ()))}")
                else:
                    self.assertEqual(listed_readers(cell), reads.get(name, set()))


if __name__ == "__main__":
    unittest.main()
