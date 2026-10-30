// Package split is bot/harness/split.py.
//
// A split (#60): an issue broken into sub-issues, each small enough for one run.
//
// Squishy's `split` and `split-bot` modes run one model session that reads the issue and the code and
// answers with the tree as data, in a `<!-- split: {...} -->` header (`work.Worker._split`). The model
// writes no issue itself: the deliver job, which holds the token and runs no model, checks the tree
// here and creates the sub-issues (`deliver.Deliverer._split_out`): GitHub sub-issues of the parent,
// with GitHub's own "blocked by" links between them, each queued for whoever builds it (Squishy for
// `split`, the night bot for `split-bot`). Each one's plan goes into its description as the
// **Plan** section (`issueplan.py`), which both bots build from without planning again.
//
// The same session closes a tree out: when every sub-issue has closed, the parent is split again
// with its sub-issues listed, and the answer is `"done": true` when the parent's end state holds on
// `main`, or the sub-issues still missing.
//
// Both bots' `fullsend` mode (#505) splits the same way, into parts that own their files
// (`FULLSEND_NOTES`), but each part lands on the parent's own branch (`onto`) instead of `main`,
// with no pull request, checks or review of its own (`work.Worker._part`, `deliver.Deliverer.
// _land_part`); once every part has closed, the parent's reconcile builds that branch into one pull
// request into `main` (`prompts/reconcile.md`).
package split

import (
	"fmt"
	"regexp"
	"slices"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// MaxIssues is the most sub-issues one split may open: a tree bigger than this is a project to
// split in stages.
var MaxIssues int = 20

// TitleChars = 200
var TitleChars int = 200

// BodyChars = 20_000
var BodyChars int = 20000

// PlanChars = 30_000
var PlanChars int = 30000

var keyPattern = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9_.-]{0,40}$`)

// FullsendNotes is what a fullsend split is told beyond a split's rules (#505,
// `prompts/split.md`'s `$fullsend`): the parts land on one branch and are reconciled there, as
// `.claude/skills/fullsend/` shatters a build into slices (its Phase 1). `{branch}` is filled
// with str.format.
var FullsendNotes string = "## This is a fullsend split\n" +
	"\n" +
	"A person asked for this issue to be built with **fullsend** (`.claude/skills/fullsend/SKILL.md`),\n" +
	"and the rules below change for it in four ways.\n" +
	"\n" +
	"- **One branch, not `main`.** Each part lands on `{branch}`, the issue's own branch, not on\n" +
	"  `main`, with no pull request, checks or review of its own. A part need not pass the checks on\n" +
	"  its own: once every part has closed, one run reconciles the branch, makes every check green and\n" +
	"  opens one pull request into `main`.\n" +
	"- **Vertical slices that own their files.** Cut the issue as the skill's Phase 1 shatters a build:\n" +
	"  each part owns its files outright, top to bottom, and no file is in two parts, so the parts with\n" +
	"  no `blocked_by` are built side by side. Name each part's files in its body and its plan.\n" +
	"- **`blocked_by` only where one part reads another's code.** A part that only needs another to\n" +
	"  exist (a type, a function it calls) can follow the interface the issue and your plan fix, and\n" +
	"  needs no link.\n" +
	"- **No \"make it green\" part.** A part may test its own code, but the tests that need several\n" +
	"  parts, and every fix that makes the checks pass, are the reconcile's: a part that only fixes the\n" +
	"  others would wait for all of them.\n"

// PartNote is what a part's builder reads first (#505, `work.Worker._first_prompt`). `{parent}`
// and `{onto}` are filled with str.format.
var PartNote string = "This is part of fullsend tree #{parent}. Your change lands on `{onto}`, which holds " +
	"the parts built before yours, not on `main`, and the harness runs no checks and no " +
	"review on it. Change only the files your plan names. A check that is red because " +
	"another part is missing is expected: a later run reconciles the parts and makes " +
	"every check green.\n\n"

var (
	fence = regexp.MustCompile("(?s)```json[" + py.SpaceClass + "]*\n(.*?)\n```")
	// Python's `\d` is any Unicode decimal digit.
	issueRef  = regexp.MustCompile(`^#?(\p{Nd}{1,7})$`)
	numberRef = regexp.MustCompile(`^#?\p{Nd}+$`)
)

// SubIssue is split.SubIssue.
type SubIssue struct {
	Key        string
	Title      string
	Body       string
	Plan       string   // default: ""
	Difficulty string   // default: ""
	Labels     []string // default: ()
	// BlockedBy are the keys of the sub-issues of this split that must close first, and
	// BlockedByIssues the numbers of issues already open that must (a close-out may build on
	// what is there).
	BlockedBy       []string // default: ()
	BlockedByIssues []int    // default: ()
}

func (x *SubIssue) ToDict() *pyjson.Object {
	return pyjson.New("key", x.Key, "title", x.Title, "body", x.Body, "plan", x.Plan,
		"difficulty", x.Difficulty, "labels", pyjson.ListOf(x.Labels),
		"blocked_by", pyjson.ListOf(x.BlockedBy),
		"blocked_by_issues", pyjson.ListOf(x.BlockedByIssues))
}

// Tree is split.Tree.
type Tree struct {
	Done     bool
	Summary  string
	Issues   []*SubIssue // default: field(default_factory=list)
	Problems []string    // default: field(default_factory=list): what the tree could not be read as, one line each; the answer is refused when non-empty
}

func (x *Tree) OK() bool {
	return len(x.Problems) == 0 && (x.Done || len(x.Issues) > 0)
}

func (x *Tree) ToDict() *pyjson.Object {
	issues := []any{}
	for _, i := range x.Issues {
		issues = append(issues, i.ToDict())
	}
	return pyjson.New("done", x.Done, "summary", x.Summary, "issues", issues,
		"problems", pyjson.ListOf(x.Problems))
}

func raw(text string) any {
	value, _ := verdicts.Header(text, "split")
	if value != nil {
		return value
	}
	found := fence.FindAllStringSubmatch(text, -1)
	for i := len(found) - 1; i >= 0; i-- {
		if v, err := pyjson.LoadsString(found[i][1]); err == nil {
			return v
		}
	}
	return nil
}

func ints(raw []any) []int {
	found := []int{}
	for _, entry := range raw {
		if match := issueRef.FindStringSubmatch(py.Strip(py.Str(entry))); match != nil {
			found = append(found, decimal(match[1]))
		}
	}
	return found
}

// Difficulties is parse's and from_dict's default `difficulties`.
var Difficulties = []string{"easy", "medium", "hard"}

// Parse is the tree a split session answered with. Problems say what is wrong with it; a tree
// with any is not used, so a half-read answer never opens half a project.
func Parse(text string, difficulties []string /* = ("easy", "medium", "hard") */) *Tree {
	return FromDict(raw(text), difficulties)
}

// FromDict is a tree from its JSON: the model's answer, or the work job's result, which the
// deliver job checks again rather than trust.
func FromDict(raw any, difficulties []string /* = ("easy", "medium", "hard") */) *Tree {
	obj, ok := raw.(*pyjson.Object)
	if !ok || obj == nil {
		return &Tree{false, "", []*SubIssue{}, []string{"no `<!-- split: {...} -->` header with a JSON object"}}
	}
	done := obj.Truthy("done")
	summary := py.SliceTo(py.Strip(py.OrStr(obj.Get("summary"), "")), 4000)
	var entries []any
	if v := obj.Get("issues"); pyjson.Truthy(v) {
		list, ok := v.([]any)
		if !ok {
			return &Tree{done, summary, []*SubIssue{}, []string{"`issues` is not a list"}}
		}
		entries = list
	}
	tree := &Tree{done, summary, []*SubIssue{}, []string{}}
	if len(entries) > MaxIssues {
		tree.Problems = append(tree.Problems, fmt.Sprintf("%d sub-issues; at most %d at once", len(entries), MaxIssues))
		entries = entries[:MaxIssues]
	}
	seen := map[string]bool{}
	for i, e := range entries {
		where := fmt.Sprintf("issues[%d]", i)
		entry, ok := e.(*pyjson.Object)
		if !ok {
			tree.Problems = append(tree.Problems, where+" is not an object")
			continue
		}
		key := py.Strip(py.OrStr(entry.Get("key"), fmt.Sprintf("s%d", i+1)))
		if !keyPattern.MatchString(key) || seen[key] {
			tree.Problems = append(tree.Problems, fmt.Sprintf("%s: key %s is not a short unique name", where, py.Repr(key)))
			continue
		}
		seen[key] = true
		title := py.SliceTo(strings.Join(py.Split(py.OrStr(entry.Get("title"), "")), " "), TitleChars)
		body := py.SliceTo(py.Strip(py.OrStr(entry.Get("body"), "")), BodyChars)
		if title == "" || body == "" {
			tree.Problems = append(tree.Problems, fmt.Sprintf("%s (%s): needs a title and a body", where, key))
			continue
		}
		difficulty := py.Lower(py.Strip(py.OrStr(entry.Get("difficulty"), "")))
		if difficulty != "" && !slices.Contains(difficulties, difficulty) {
			difficulty = ""
		}
		labels := []string{}
		for _, label := range py.Each(entry.Get("labels")) {
			if l := py.Strip(py.Str(label)); l != "" && !slices.Contains(labels, l) {
				labels = append(labels, l)
			}
		}
		if len(labels) > 6 {
			labels = labels[:6]
		}
		blockedBy := []string{}
		var numbered []any
		for _, k := range py.Each(entry.Get("blocked_by")) {
			if s, ok := k.(string); ok && !numberRef.MatchString(py.Strip(s)) {
				blockedBy = append(blockedBy, py.Strip(s))
			}
		}
		for _, k := range py.Each(entry.Get("blocked_by")) {
			if numberRef.MatchString(py.Strip(py.Str(k))) {
				numbered = append(numbered, k)
			}
		}
		numbered = append(numbered, py.Each(entry.Get("blocked_by_issues"))...)
		tree.Issues = append(tree.Issues, &SubIssue{
			Key: key, Title: title, Body: body,
			Plan:       py.SliceTo(py.Strip(py.OrStr(entry.Get("plan"), "")), PlanChars),
			Difficulty: difficulty, Labels: labels,
			BlockedBy:       blockedBy,
			BlockedByIssues: ints(numbered),
		})
	}
	keys := map[string]bool{}
	for _, issue := range tree.Issues {
		keys[issue.Key] = true
	}
	for _, issue := range tree.Issues {
		var unknown []string
		for _, k := range issue.BlockedBy {
			if !keys[k] {
				unknown = append(unknown, k)
			}
		}
		if len(unknown) > 0 {
			tree.Problems = append(tree.Problems, fmt.Sprintf("%s: blocked by unknown key(s) %s", issue.Key, strings.Join(unknown, ", ")))
		}
	}
	if len(tree.Problems) == 0 && cycle(tree.Issues) {
		tree.Problems = append(tree.Problems, "the blocked-by links go round in a circle")
	}
	if done && len(tree.Issues) > 0 {
		tree.Problems = append(tree.Problems, "`done` is true but it lists sub-issues to open")
	}
	if !done && len(tree.Issues) == 0 && len(tree.Problems) == 0 {
		tree.Problems = append(tree.Problems, "no sub-issues, and `done` is not true")
	}
	return tree
}

func cycle(issues []*SubIssue) bool {
	after := map[string][]string{}
	var order []string
	for _, issue := range issues {
		if _, ok := after[issue.Key]; !ok {
			order = append(order, issue.Key)
		}
		after[issue.Key] = issue.BlockedBy
	}
	state := map[string]int{}
	var visit func(key string) bool
	visit = func(key string) bool {
		switch state[key] {
		case 1:
			return true
		case 2:
			return false
		}
		state[key] = 1
		for _, k := range after[key] {
			if visit(k) {
				return true
			}
		}
		state[key] = 2
		return false
	}
	for _, key := range order {
		if visit(key) {
			return true
		}
	}
	return false
}

// Ordered is the sub-issues with every one after those it waits for (the tree has no cycle), in
// the model's order otherwise: issues are numbered in the order they are opened, so the queue's
// oldest-first order follows the dependencies.
func Ordered(issues []*SubIssue) []*SubIssue {
	placed := []*SubIssue{}
	done := map[string]bool{}
	rest := append([]*SubIssue(nil), issues...)
	for len(rest) > 0 {
		found := -1
		for i, issue := range rest {
			ready := true
			for _, k := range issue.BlockedBy {
				if !done[k] {
					ready = false
					break
				}
			}
			if ready {
				found = i
				break
			}
		}
		if found < 0 { // a cycle `parse` should have refused: keep the rest as given
			placed = append(placed, rest...)
			break
		}
		placed = append(placed, rest[found])
		done[rest[found].Key] = true
		rest = append(rest[:found:found], rest[found+1:]...)
	}
	return placed
}

// Body is a sub-issue's description, its plan aside (`issueplan.with_plan` adds that). The issues
// it waits for are named in a "Blocked by" line too, which the queue reads (`queue.waits_for`) even
// if GitHub's own link could not be made. A fullsend part (#505) says where it lands.
func Body(issue *SubIssue, parent int, who string, waits []int, onto string /* = "" */) string {
	blocked := ""
	if len(waits) > 0 {
		refs := make([]string, len(waits))
		for i, n := range waits {
			refs[i] = fmt.Sprintf("#%d", n)
		}
		blocked = fmt.Sprintf("\n\nBlocked by %s.", strings.Join(refs, ", "))
	}
	lands := ""
	if onto != "" {
		lands = fmt.Sprintf("\n\nIts build lands on `%s`, not `main` (fullsend).", onto)
	}
	return fmt.Sprintf("%s%s%s\n\nPart of #%d, split by %s.", issue.Body, blocked, lands, parent, who)
}

// Made is one sub-issue a split opened and its number: the Python's `(SubIssue, int)` pair.
type Made struct {
	Issue  *SubIssue
	Number int
}

// Checklist is the comment on the parent after a split: the tree, in build order, and how it goes
// on.
func Checklist(parent int, made []Made, builder string, summary string, link string, onto string /* = "" */) string {
	lines := []string{fmt.Sprintf("Split #%d into %d sub-issue(s) (%s), queued for %s in "+
		"this order; each waits for the ones it is blocked by:", parent, len(made), link, builder), ""}
	numbers := map[string]int{}
	for _, m := range made {
		numbers[m.Issue.Key] = m.Number
	}
	for _, m := range made {
		var waits []string
		for _, k := range m.Issue.BlockedBy {
			if n, ok := numbers[k]; ok {
				waits = append(waits, fmt.Sprintf("#%d", n))
			}
		}
		for _, n := range m.Issue.BlockedByIssues {
			waits = append(waits, fmt.Sprintf("#%d", n))
		}
		after := ""
		if len(waits) > 0 {
			after = fmt.Sprintf(" (after %s)", strings.Join(waits, ", "))
		}
		lines = append(lines, fmt.Sprintf("- [ ] #%d%s", m.Number, after))
	}
	if onto != "" {
		lines = append(lines, "", fmt.Sprintf("This is a fullsend tree: each part lands on `%s`, not on `main`, with "+
			"no pull request of its own. Once every part has closed, one run reconciles them "+
			"on that branch, makes every check green and opens one pull request into `main`.", onto))
	}
	if summary != "" {
		lines = append(lines, "", summary)
	}
	return strings.Join(lines, "\n")
}

// decimal is int() of a run of Unicode decimal digits.
func decimal(s string) int {
	n := 0
	for _, r := range s {
		n = n*10 + decimalValue(r)
	}
	return n
}

// decimalValue is the digit a Unicode decimal digit (category Nd) stands for: every Nd range
// runs in whole sets of ten from a zero.
func decimalValue(r rune) int {
	for _, rg := range unicode.Nd.R16 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10
		}
	}
	for _, rg := range unicode.Nd.R32 {
		if lo, hi := rune(rg.Lo), rune(rg.Hi); r >= lo && r <= hi {
			return int(r-lo) / int(rg.Stride) % 10
		}
	}
	return 0
}
