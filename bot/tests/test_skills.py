"""The skills under .claude/skills/ hold instructions and nothing that runs or grants (#839).

Squishy may change them (`.squishy/config.json`, `!.claude/skills/`), so a skill must not be a way
for a bot to give itself more: no frontmatter key that grants tools, adds hooks or picks a shell or
model, and no file but Markdown and a licence. This suite is in `bot/`, which no bot may change."""

from __future__ import annotations

import unittest

from harness import config

SKILLS = config.REPO_ROOT / ".claude" / "skills"
#: The frontmatter keys a skill may carry: what it is called, when it applies, and its licence.
#: Not `allowed-tools`, `disallowed-tools`, `hooks`, `shell`, `model`, `effort`, `context` or
#: `agent`: those change what a session may do or what runs it.
ALLOWED_KEYS = {"name", "description", "when_to_use", "argument-hint", "arguments",
                "disable-model-invocation", "user-invocable", "paths", "license", "metadata",
                "compatibility"}
ALLOWED_FILES = (".md", "LICENSE")


def frontmatter_keys(text: str) -> set[str]:
    """The top-level keys between the opening `---` lines."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return set()
    keys = set()
    for line in lines[1:]:
        if line.strip() == "---":
            return keys
        if line and not line[0].isspace() and ":" in line:
            keys.add(line.split(":", 1)[0].strip())
    raise AssertionError("frontmatter never closes")


class SkillTests(unittest.TestCase):
    def test_every_skill_carries_only_allowed_keys(self):
        skills = sorted(SKILLS.glob("*/SKILL.md"))
        self.assertTrue(skills)
        for path in skills:
            keys = frontmatter_keys(path.read_text(encoding="utf-8"))
            self.assertIn("description", keys, path)
            self.assertLessEqual(keys, ALLOWED_KEYS, f"{path}: {sorted(keys - ALLOWED_KEYS)}")

    def test_a_skill_holds_only_markdown_and_a_licence(self):
        for path in SKILLS.rglob("*"):
            if path.is_file():
                self.assertTrue(path.name.endswith(ALLOWED_FILES), path)

    def test_the_reader_sees_keys_that_grant(self):
        text = "---\nname: x\ndescription: >\n  y: z\nallowed-tools: Bash\n---\nbody: 1\n"
        self.assertEqual(frontmatter_keys(text), {"name", "description", "allowed-tools"})


if __name__ == "__main__":
    unittest.main()
