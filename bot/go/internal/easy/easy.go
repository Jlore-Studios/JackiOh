// Package easy is bot/harness/easy.py.
//
// The easy rule: what `difficulty:easy` means, so that what a weak model may build is what it
// finishes (#317, "The easy rule").
//
// The weak tier may build only easy items. On this repository its model could not resolve the
// conflicts in SPEC §11 and the rulings index (every pull request appended the next R number there;
// since v0.3.0 a ruling is a note in `spec/rulings/`, listed in the generated `spec/INDEX.md`), could
// not hold a big change together, could not plan, ran no tests where it worked, and could not judge
// what it could not see. Its clean merges were all small (7–15 files, under about 500 lines).
//
// The rule is in the planner's prompt (`RULE`, `bot/prompts/plan.md`), and the harness holds the
// rating to it twice, trusting nothing the model said: deliver checks the plan's "Files to touch"
// table before the item is queued (`plan_breach`), and the real change before a weak builder's work
// is pushed (`change_breach`). Either breach makes the item medium, and so does a plan or change
// close to a limit (borderline easy is medium, #695) or a rating whose `why` admits doubt
// (`uncertain_why`). The limits are settings (`easy` in `.harness/config.json`, `config.EasyRule`).
package easy

import (
	"fmt"
	"regexp"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// BorderlineFiles is how close to the file limit still counts as easy: a plan or change within
// this many files of `max_files` is borderline, so medium (#695). Only applied when the limit is
// large enough that clear-easy work still fits under it.
var BorderlineFiles int = 2

// BorderlineLines is how close to the line limit still counts as easy, for a built change
// (`change_breach`): a change within this many lines of `max_lines` is borderline, so medium
// (#695).
var BorderlineLines int = 100

// uncertain is _UNCERTAIN: words in a planner's rating `why` that admit doubt: torn between two
// means the higher (#695). Each `\b` sits beside a letter, so it is the start or end of the text
// or a character that is not a word character.
var uncertain = regexp.MustCompile(
	`(?i)(?:^|` + py.ReNotW + `)(borderline|torn|unsure|uncertain|not sure|close call|close to the (?:limit|line)` +
		`|could go either|either way|debatable|judg?ment call|on the fence|lean(?:s|ing)?)(?:$|` + py.ReNotW + `)`)

var wordChar = regexp.MustCompile(`^` + py.ReW)

// UncertainWhy is whether a planner's rating `why` admits doubt about easy (deliver then rates
// medium: borderline easy is medium). A confident `why` ("one file, one test") is untouched.
func UncertainWhy(why string) bool {
	return uncertain.MatchString(why)
}

// Rule is the rule as the planner reads it. `$max_files` and `$max_lines` are filled from the
// settings.
var Rule = "" +
	"An item is **easy** only if every line below holds. If any line fails, or you cannot\n" +
	"tell, it is **medium** at least.\n" +
	"\n" +
	"1. **Small, with room to spare.** At most $max_files files changed in all, the tests it adds\n" +
	"   included, and at most $max_lines lines added plus removed (generated files such as\n" +
	"   `apps/web/src/wire/generated/` aside). Close to either limit is **medium**, not easy.\n" +
	"2. **One place.** All of it inside one crate or app, except a test of the same feature in that\n" +
	"   crate's `tests/`.\n" +
	"3. **None of the conflict hot spots:** `SPEC.md` and the spec's notes in `spec/` (no new ruling,\n" +
	"   no ruling cited or renumbered, nothing `spec/INDEX.md` lists), `BUILD.md`.\n" +
	"4. **None of the shared surfaces:** `crates/engine/src/wire/events.rs`, `crates/engine/src/script.rs`,\n" +
	"   `crates/engine/src/state.rs`, `crates/engine/src/effects/mod.rs`.\n" +
	"5. **No rules or data work:** nothing in `crates/engine/src/` or `crates/ai/src/`, no change to\n" +
	"   `crates/cards/catalog.json`, `crates/cards/patches/` or the catalog version, no card script\n" +
	"   (`crates/cards/src/scripts/`), no database migration (`crates/server/migrations/`).\n" +
	"6. **No review-only or forbidden path** (every `package.json` and `Cargo.toml`, the lockfiles,\n" +
	"   `rust-toolchain.toml`, the build and test configs, `scripts/`, `vercel.json`, `render.yaml`;\n" +
	"   `.github/`, `.harness/`, `bot/`), and no new dependency.\n" +
	"7. **Checked by a unit test** in the same crate or app, which the plan names; no Cypress or e2e\n" +
	"   spec change, and no judging by eye (animation, layout, sound or art tuning).\n" +
	"8. **No decisions left:** the plan names each file, what changes in it, and the test that proves\n" +
	"   it. If the builder would choose between two designs, it is not easy.\n" +
	"9. **Stands alone:** blocked by no open issue, and no open pull request touches the same files.\n" +
	"\n" +
	"Never easy: a patch that changes cards or rules, anything that adds an R number, engine or AI\n" +
	"work, visual polish.\n" +
	"\n" +
	"- **medium:** anything that fails the easy rule and is not hard: a new ruling, a card or catalog\n" +
	"  change, a shared surface, several crates or apps, e2e work, visual polish.\n" +
	"- **hard:** engine rules or the resolution loop (`crates/engine/src/`), the AI's search\n" +
	"  (`crates/ai/src/`), design across crates, a database migration, or about 40 files or more.\n" +
	"\n" +
	"When torn between two, choose the higher. A borderline easy is **medium**: any doubt on\n" +
	"any line above rates **medium** at least."

// tableHeading is _TABLE_HEADING, `(?im)^\s*(?:#+\s*)?(?:\d+\.\s*)?\**\s*files to touch\b`,
// without its `\b`, which tableHeadingEnd checks: the heading's end is used, so the boundary
// cannot be a character the match takes.
var tableHeading = regexp.MustCompile(`(?im)^` + py.ReS + `*(?:#+` + py.ReS + `*)?(?:` + py.DigitClass + `+\.` +
	py.ReS + `*)?\**` + py.ReS + `*files to touch`)

var pathCell = regexp.MustCompile("`([^`]+)`")

// tableHeadingEnd is _TABLE_HEADING.search(text).end(), or -1 when there is no heading. A match
// starts at a line, so a later one never starts inside an earlier one's "files to touch": the
// first that a word character does not follow is Python's.
func tableHeadingEnd(text string) int {
	for _, m := range tableHeading.FindAllStringIndex(text, -1) {
		if !wordChar.MatchString(text[m[1]:]) {
			return m[1]
		}
	}
	return -1
}

// onlyChars is set(s) <= set(chars).
func onlyChars(s, chars string) bool {
	for _, r := range s {
		if !strings.ContainsRune(chars, r) {
			return false
		}
	}
	return true
}

// PlanPaths is the paths in the first column of the plan's "Files to touch" table, in order.
//
// A row's first cell may hold its path in backticks or bare; a header row, the separator row and
// a cell that is not a path (no `/` or `.`) are skipped.
func PlanPaths(plan string) []string {
	text := plan
	end := tableHeadingEnd(text)
	if end < 0 {
		return []string{}
	}
	found := []string{}
	started := false
	for _, line := range py.Splitlines(text[end:]) {
		row := strings.TrimSpace(line)
		if !strings.HasPrefix(row, "|") {
			if started {
				break
			}
			continue
		}
		started = true
		cells := strings.Split(strings.Trim(row, "|"), "|")
		first := strings.TrimSpace(cells[0])
		if first == "" || onlyChars(first, "-: ") {
			continue
		}
		path := first
		if quoted := pathCell.FindStringSubmatch(first); quoted != nil {
			path = quoted[1]
		}
		path = strings.TrimSpace(path)
		if !strings.Contains(path, "/") && !strings.Contains(path, ".") {
			continue // the header row ("Path", "File"), or a cell that names no file
		}
		if !slices.Contains(found, path) {
			found = append(found, path)
		}
	}
	return found
}

func offLimits(paths []string, patterns []string) []string {
	out := []string{}
	for _, path := range paths {
		if git.Matches(path, patterns) {
			out = append(out, path)
		}
	}
	return out
}

// quoted is ', '.join(f'`{p}`' for p in paths[:5]).
func quoted(paths []string) string {
	out := []string{}
	for _, p := range paths[:min(len(paths), 5)] {
		out = append(out, "`"+p+"`")
	}
	return strings.Join(out, ", ")
}

// PlanBreach is why a plan rated easy breaks the rule (lines 1, 3–6, from its "Files to touch"
// table), or "" when it does not. A plan with no such table cannot show it holds, so it breaks
// line 8. A plan within `BORDERLINE_FILES` of the file limit is borderline easy, so medium
// (#695).
func PlanBreach(plan string, rule *config.EasyRule, reviewPaths []string, forbiddenPaths []string) string {
	paths := PlanPaths(plan)
	if len(paths) == 0 {
		return "line 8: the plan has no **Files to touch** table, so nothing shows it is small"
	}
	if len(paths) > rule.MaxFiles {
		return fmt.Sprintf("line 1: the plan touches %d files, over %d", len(paths), rule.MaxFiles)
	}
	if rule.MaxFiles > BorderlineFiles*2 && len(paths) > rule.MaxFiles-BorderlineFiles {
		return fmt.Sprintf("line 1: the plan touches %d files, close to the limit of "+
			"%d, so borderline easy is medium", len(paths), rule.MaxFiles)
	}
	if bad := offLimits(paths, rule.OffLimits); len(bad) > 0 {
		return "lines 3–5: the plan touches " + quoted(bad)
	}
	if bad := offLimits(paths, append(append([]string{}, reviewPaths...), forbiddenPaths...)); len(bad) > 0 {
		return "line 6: the plan touches " + quoted(bad)
	}
	return ""
}

// ChangeBreach is why a change built for an easy item breaks the rule (lines 1 and 3–5, from what
// it really changed: lines per path, `Git.numstat`), or "" when it does not. A change within
// `BORDERLINE_FILES` files or `BORDERLINE_LINES` lines of a limit is borderline easy, so medium
// (#695).
func ChangeBreach(counts *pyjson.Object, rule *config.EasyRule) string {
	paths := []string{}
	for _, p := range counts.Keys() {
		if !git.Matches(p, rule.Generated) {
			paths = append(paths, p)
		}
	}
	if len(paths) > rule.MaxFiles {
		return fmt.Sprintf("line 1: it changes %d files, over %d", len(paths), rule.MaxFiles)
	}
	lines := 0
	for _, p := range paths {
		lines += counts.Int(p)
	}
	if lines > rule.MaxLines {
		return fmt.Sprintf("line 1: it changes %d lines, over %d", lines, rule.MaxLines)
	}
	if rule.MaxFiles > BorderlineFiles*2 && len(paths) > rule.MaxFiles-BorderlineFiles {
		return fmt.Sprintf("line 1: it changes %d files, close to the limit of "+
			"%d, so borderline easy is medium", len(paths), rule.MaxFiles)
	}
	if rule.MaxLines > BorderlineLines*2 && lines > rule.MaxLines-BorderlineLines {
		return fmt.Sprintf("line 1: it changes %d lines, close to the limit of "+
			"%d, so borderline easy is medium", lines, rule.MaxLines)
	}
	if bad := offLimits(paths, rule.OffLimits); len(bad) > 0 {
		return "lines 3–5: it changes " + quoted(bad)
	}
	return ""
}
