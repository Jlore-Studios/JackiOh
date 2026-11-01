// test_skills.py: the skills under .claude/skills/ hold instructions and nothing that runs or
// grants (#839).
//
// Squishy may change them (`.squishy/config.json`, `!.claude/skills/`), so a skill must not be a
// way for a bot to give itself more: no frontmatter key that grants tools, adds hooks or picks a
// shell or model, and no file but Markdown and a licence. This suite is in `bot/`, which no bot
// may change.
package tests

import (
	"io/fs"
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"testing"
	"unicode"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// skillsDir is SKILLS = config.REPO_ROOT / ".claude" / "skills".
var skillsDir = filepath.Join(config.RepoRoot, ".claude", "skills")

// skillsAllowedKeys is ALLOWED_KEYS: the frontmatter keys a skill may carry: what it is called,
// when it applies, and its licence. Not `allowed-tools`, `disallowed-tools`, `hooks`, `shell`,
// `model`, `effort`, `context` or `agent`: those change what a session may do or what runs it.
var skillsAllowedKeys = setOf("name", "description", "when_to_use", "argument-hint", "arguments",
	"disable-model-invocation", "user-invocable", "paths", "license", "metadata",
	"compatibility")

// skillsAllowedFiles is ALLOWED_FILES.
var skillsAllowedFiles = []string{".md", "LICENSE"}

// skillsFrontmatterKeys is frontmatter_keys(): the top-level keys between the opening `---`
// lines. It fails the test where the Python raises AssertionError.
func skillsFrontmatterKeys(t *testing.T, text string) map[string]bool {
	t.Helper()
	lines := py.Splitlines(text)
	if len(lines) == 0 || strings.TrimSpace(lines[0]) != "---" {
		return map[string]bool{}
	}
	keys := map[string]bool{}
	for _, line := range lines[1:] {
		if strings.TrimSpace(line) == "---" {
			return keys
		}
		if first, _ := utf8.DecodeRuneInString(line); line != "" && !unicode.IsSpace(first) &&
			strings.Contains(line, ":") {
			keys[strings.TrimSpace(strings.SplitN(line, ":", 2)[0])] = true
		}
	}
	t.Fatal("frontmatter never closes")
	return nil
}

func TestSkills_EverySkillCarriesOnlyAllowedKeys(t *testing.T) {
	skills, err := filepath.Glob(filepath.Join(skillsDir, "*", "SKILL.md"))
	if err != nil {
		t.Fatal(err)
	}
	sort.Strings(skills)
	assertTrue(t, len(skills) > 0)
	for _, path := range skills {
		keys := skillsFrontmatterKeys(t, readText(t, path))
		assertTrue(t, keys["description"], path)
		extra := []string{}
		for key := range keys {
			if !skillsAllowedKeys[key] {
				extra = append(extra, key)
			}
		}
		sort.Strings(extra)
		assertTrue(t, len(extra) == 0, path, extra)
	}
}

func TestSkills_ASkillHoldsOnlyMarkdownAndALicence(t *testing.T) {
	err := filepath.WalkDir(skillsDir, func(path string, _ fs.DirEntry, err error) error {
		if err != nil || path == skillsDir {
			return err
		}
		if isFile(path) {
			assertTrue(t, slices.ContainsFunc(skillsAllowedFiles, func(end string) bool {
				return strings.HasSuffix(filepath.Base(path), end)
			}), path)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

func TestSkills_TheReaderSeesKeysThatGrant(t *testing.T) {
	text := "---\nname: x\ndescription: >\n  y: z\nallowed-tools: Bash\n---\nbody: 1\n"
	assertEqual(t, skillsFrontmatterKeys(t, text), setOf("name", "description", "allowed-tools"))
}
