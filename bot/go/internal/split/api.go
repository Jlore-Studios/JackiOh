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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MAX_ISSUES = 20
var MaxIssues int = 20

// TITLE_CHARS = 200
var TitleChars int = 200

// BODY_CHARS = 20_000
var BodyChars int = 20000

// PLAN_CHARS = 30_000
var PlanChars int = 30000

// FULLSEND_NOTES = """## This is a fullsend split A person asked for this issue to be built with **fullsend** (`.claude/skills/fullsend/SKILL.md`), and the rules
var FullsendNotes string = "## This is a fullsend split\n\nA person asked for this issue to be built with **fullsend** (`.claude/skills/fullsend/SKILL.md`),\nand the rules below change for it in four ways.\n\n- **One branch, not `main`.** Each part lands on `{branch}`, the issue's own branch, not on\n `main`, with no pull request, checks or review of its own. A part need not pass the checks on\n its own: once every part has closed, one run reconciles the branch, makes every check green and\n opens one pull request into `main`.\n- **Vertical slices that own their files.** Cut the issue as the skill's Phase 1 shatters a build:\n each part owns its files outright, top to bottom, and no file is in two parts, so the parts with\n no `blocked_by` are built side by side. Name each part's files in its body and its plan.\n- **`blocked_by` only where one part reads another's code.** A part that only needs another to\n exist (a type, a function it calls) can follow the interface the issue and your plan fix, and\n needs no link.\n- **No \"make it green\" part.** A part may test its own code, but the tests that need several\n parts, and every fix that makes the checks pass, are the reconcile's: a part that only fixes the\n others would wait for all of them.\n"

// PART_NOTE = ("This is part of fullsend tree #{parent}. Your change lands on `{onto}`, which holds " "the parts built before yours, not on `main`, and the harnes
var PartNote string = "This is part of fullsend tree #{parent}. Your change lands on `{onto}`, which holds the parts built before yours, not on `main`, and the harness runs no checks and no review on it. Change only the files your plan names. A check that is red because another part is missing is expected: a later run reconciles the parts and makes every check green.\n\n"

// SubIssue is split.SubIssue.

type SubIssue struct {
	Key             string
	Title           string
	Body            string
	Plan            string   // default: ""
	Difficulty      string   // default: ""
	Labels          []string // default: ()
	BlockedBy       []string // default: ()
	BlockedByIssues []int    // default: ()
}

func (x *SubIssue) ToDict() *pyjson.Object {
	panic("unported: split.SubIssue.to_dict")
}

// Tree is split.Tree.

type Tree struct {
	Done     bool
	Summary  string
	Issues   []*SubIssue // default: field(default_factory=list)
	Problems []string    // default: field(default_factory=list)
}

func (x *Tree) OK() bool {
	panic("unported: split.Tree.ok")
}

func (x *Tree) ToDict() *pyjson.Object {
	panic("unported: split.Tree.to_dict")
}

// The tree a split session answered with. Problems say what is wrong with it; a tree with
// any is not used, so a half-read answer never opens half a project.

func Parse(text string, difficulties []string /* = ("easy", "medium", "hard") */) *Tree {
	panic("unported: split.parse")
}

// A tree from its JSON: the model's answer, or the work job's result, which the deliver job
// checks again rather than trust.

func FromDict(raw any, difficulties []string /* = ("easy", "medium", "hard") */) *Tree {
	panic("unported: split.from_dict")
}

// The sub-issues with every one after those it waits for (the tree has no cycle), in the
// model's order otherwise: issues are numbered in the order they are opened, so the queue's
// oldest-first order follows the dependencies.

func Ordered(issues []*SubIssue) []*SubIssue {
	panic("unported: split.ordered")
}

// A sub-issue's description, its plan aside (`issueplan.with_plan` adds that). The issues it
// waits for are named in a "Blocked by" line too, which the queue reads (`queue.waits_for`) even
// if GitHub's own link could not be made. A fullsend part (#505) says where it lands.

func Body(issue *SubIssue, parent int, who string, waits []int, onto string /* = "" */) string {
	panic("unported: split.body")
}

// The comment on the parent after a split: the tree, in build order, and how it goes on.

func Checklist(parent int, made [][]any, builder string, summary string, link string, onto string /* = "" */) string {
	panic("unported: split.checklist")
}
