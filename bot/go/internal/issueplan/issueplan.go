// Package issueplan is bot/harness/issueplan.py.
//
// An item's plan, kept in its issue's description (the Needs plan stage).
//
// A strong model plans every item before it is built (`plan.py`), and the plan goes into a marked
// **Plan** section at the end of the issue's description, so a person sees it where the task is and
// can correct it there: the builder starts from that section, whatever it says by then. The section
// carries no bot marker (`config.MARKER`), which would make the whole description read as the bot's.
package issueplan

import (
	"fmt"
	"regexp"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// Start = "<!-- jackioh-bot:plan -->"
var Start string = "<!-- jackioh-bot:plan -->"

// End = "<!-- /jackioh-bot:plan -->"
var End string = "<!-- /jackioh-bot:plan -->"

// BodyLimit: GitHub refuses a description over 65,536 characters; the plan gives way, never the
// task.
var BodyLimit int = 65000

// PlanWords is what the plan prompt asks a planner to stay under (`bot/prompts/plan.md`, #208).
var PlanWords int = 2500

// PlanChars is how much of a planner's answer becomes the plan: its beginning, where the goal and
// the first steps are, never its end alone (`work.py`), and how much a comment carrying a plan the
// description could not take shows (`deliver.py`). Room over PLAN_WORDS, so a plan that runs a
// little long is kept whole, and well under BODY_LIMIT and `threads.MAX_CHARS`, so the task and
// its comments keep room beside it. #307's plan was cut at the old 20,000.
var PlanChars int = 30000

var (
	sectionPattern = regexp.QuoteMeta(Start) + `.*?(?:` + regexp.QuoteMeta(End) + `|\z)`
	section        = regexp.MustCompile(`(?s)` + sectionPattern)
	around         = regexp.MustCompile(`(?s)[` + py.SpaceClass + `]*` + sectionPattern + `[` + py.SpaceClass + `]*`)
	header         = regexp.MustCompile(`\A[` + py.SpaceClass + `]*## Plan[` + py.SpaceClass + `]*\n+(?:_[^\n]*_[` + py.SpaceClass + `]*\n+)?`)
)

// WithoutPlan is the description with its Plan section taken out.
func WithoutPlan(body string) string {
	return py.Strip(around.ReplaceAllLiteralString(body, "\n\n"))
}

// PlanOf is the Plan section's text (a person's edits included), without its heading, or "".
func PlanOf(body string) string {
	match := section.FindString(body)
	if match == "" {
		return ""
	}
	text := match[len(Start):]
	text = strings.TrimSuffix(text, End)
	return py.Strip(header.ReplaceAllLiteralString(text, ""))
}

// WithPlan is the description with `plan` as its Plan section, in place of any earlier one.
func WithPlan(body string, plan string, who string, link string /* = "" */) string {
	task := WithoutPlan(body)
	at := ""
	if link != "" {
		at = fmt.Sprintf(" (%s)", link)
	}
	note := fmt.Sprintf("_Written by %s%s before anyone built this. The night "+
		"bot's builder starts from this section: edit it here to change the plan._", who, at)
	head := fmt.Sprintf("%s\n\n%s\n## Plan\n\n%s\n\n", task, Start, note)
	room := max(0, BodyLimit-py.Len(head)-py.Len(End)-40)
	text := py.Strip(plan)
	if py.Len(text) > room {
		text = strings.TrimRightFunc(py.SliceTo(text, room), py.IsSpace) + "\n\n…(cut to fit the description)"
	}
	return strings.TrimLeft(fmt.Sprintf("%s%s\n%s\n", head, text, End), "\n")
}

// tableHeading is _TABLE_HEADING, `(?im)^\s*(?:#+\s*)?(?:\d+\.\s*)?\**\s*files to touch\b`,
// without its `\b`, which tableHeadingEnd checks: the heading's end is used, so the boundary
// cannot be a character the match takes.
var tableHeading = regexp.MustCompile(`(?im)^` + py.ReS + `*(?:#+` + py.ReS + `*)?(?:` + py.DigitClass + `+\.` +
	py.ReS + `*)?\**` + py.ReS + `*files to touch`)

var pathCell = regexp.MustCompile("`([^`]+)`")

var wordChar = regexp.MustCompile(`^` + py.ReW)

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

// FilesToTouch is the paths in the first column of the plan's **Files to touch** table, in order
// (`bot/prompts/plan.md` asks for it), which `human.PlanNeeds` reads. It was `easy.plan_paths`
// until #2748 retired the easy rule that also read it.
//
// A row's first cell may hold its path in backticks or bare; a header row, the separator row and
// a cell that is not a path (no `/` or `.`) are skipped.
func FilesToTouch(plan string) []string {
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
