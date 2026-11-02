package queue

import (
	"fmt"
	"sort"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
)

// PRIORITY_TIERS: the pickup tiers by label (#90). A thread with none of them is in tier 2.
var PriorityTiers = map[string]int{config.LabelPriorityHigh: 0, config.LabelPriorityMedium: 1,
	config.LabelPriorityLow: 3}

// NO_PRIORITY
var NoPriority int = 2

// PRIORITY_NAMES
var PriorityNames = map[int]string{0: "high", 1: "medium", NoPriority: "none", 3: "low"}

// The thread's pickup tier, the lowest picked first: `priority:high` 0, `priority:medium`
// 1, none 2, `priority:low` 3. The highest label wins; names match whatever their case, and any
// other `priority:*` label counts as none.
func PriorityTier(names map[string]bool) int {
	tier := NoPriority
	found := false
	for name := range names {
		if t, ok := PriorityTiers[py.Lower(name)]; ok && (!found || t < tier) {
			tier, found = t, true
		}
	}
	return tier
}

// Candidate is queue.Candidate. Build one with NewCandidate, which sets its defaults.
type Candidate struct {
	Number int
	Kind   string // "build" | "revise" | "review" (a review run on a bot PR)
	Title  string
	Forced bool
	// QueuedAt is queued_at.
	QueuedAt string
	// `easy`, `medium` or `hard` (`difficulty_of`): which models may plan, build and review it.
	Difficulty string // default: DEFAULT_DIFFICULTY
	// For a review: the family of the model that built the change.
	Builder string // default: ""
	// Its pickup tier (`priority_tier`), which comes before everything but `forced`.
	Priority int // default: NO_PRIORITY
	// For a build: a planning session already wrote its plan (`plan.py` plans it first if not).
	Planned bool // default: False
	// The tier of the model that wrote that plan ("" if none, or if not recorded).
	PlanTier string // default: ""
	// The subscription that wrote that plan, so a medium or hard build stays with it
	// (one lane, one owner, #725).
	PlannedBy string // default: ""
	// For a review: the families whose approval the head already has (for the reviewer's prompt).
	Approved []string // default: ()
	// For a review: the tier of each approval the head has, one per review (`review_rule`).
	ApprovalTiers []string // default: ()
	// A pull request the bot opened (`bot:pr`): one a person opened never gets a review run, so
	// its revision needs a reviewer in the same run.
	BotPR bool // default: False
	// Someone has rated how hard it is (`rated`); an unrated build is rated when it is planned.
	Rated bool // default: True
	// A revision queued because `main` left the pull request with conflicts: only a builder
	// with its own reviewer in the run takes it, never a weak one (#317 part 2).
	Conflict bool // default: False
	// For a review: the weakest tier a person asked to review it (`/harness review strong`).
	ReviewFloor string // default: ""
	// For a build: how it is built (`mode_of`, #60): "" for a plain build, or `oneshot`,
	// `split`, `split-bot` or `fullsend`.
	Mode string // default: ""
	// For a build: a fullsend part (#505, `is_part`), which lands on its tree's branch with no
	// checks or review, so a medium subscription may build it from its plan (`PART_FLOOR`).
	Part bool // default: False
	// For a revision: the tier of the final audit it answers (#725), so the auditor's tier
	// fixes and pushes it ("" when no audit has asked for changes yet).
	AuditTier string // default: ""
	// Review verdict rounds already used on the item in the current owner's hands (#725).
	RoundsUsed int // default: 0
	// The owner tier that used them ("" when none); a new owner starts its own count.
	RoundTier string // default: ""
}

// NewCandidate is Candidate(number, kind, title, forced, queued_at) with its defaults.
func NewCandidate(number int, kind string, title string, forced bool, queuedAt string) *Candidate {
	return &Candidate{Number: number, Kind: kind, Title: title, Forced: forced, QueuedAt: queuedAt,
		Difficulty: config.DefaultDifficulty, Priority: NoPriority, Approved: []string{},
		ApprovalTiers: []string{}, Rated: true}
}

// The last commit of a bot pull request that met the review rule, as deliver recorded it
// (`{"sha", "at", "by", ...}`), or `{}`. It is keyed by commit: a head that moved since is not
// cleared, whatever the record says.
func Cleared(record *pyjson.Object) *pyjson.Object {
	if found := record.Obj("cleared"); found.Truthy("sha") {
		return found
	}
	return pyjson.New()
}

// A revision queued because `main` left a cleared change with conflicts.
func Carries(record *pyjson.Object) bool {
	return record.Get("source") == "conflict" && Cleared(record).Len() > 0
}

// Whether a build is planned, and by what tier: the bot's own record of its planning run,
// or else a Plan section someone put in the issue's description (`issueplan.START` … `END`),
// which counts as a strong plan: a person, or a session they ran, wrote it on purpose, and the
// bot must not plan over it. A mode (#60) plans for itself: a split is the planning, and a
// one-shot writes its own spec first, so neither waits for a plan.
func PlanOf(record *pyjson.Object, thread *pyjson.Object, kind string, mode string /* = "" */) *pyjson.Object {
	if kind == "build" && mode != "" {
		return pyjson.New("planned", true, "plan_tier", "strong", "planned_by", "")
	}
	if record.Truthy("planned_at") {
		return pyjson.New("planned", true, "plan_tier", py.OrStr(record.Get("planned_tier"), ""),
			"planned_by", py.OrStr(record.Get("planned_by_id"), ""))
	}
	if kind == "build" && issueplan.PlanOf(py.OrStr(thread.Get("body"), "")) != "" {
		return pyjson.New("planned", true, "plan_tier", "strong", "planned_by", "")
	}
	return pyjson.New("planned", false, "plan_tier", "", "planned_by", "")
}

// The weakest tier whose plan this item builds from (`config.PLAN_FLOOR`): medium for an
// easy or medium item, strong for a hard one, and medium to rate and plan an item nobody has
// rated or planned yet. One planned but never rated (a plan from before ratings, or a planner
// that gave none) still needs a plan from its difficulty's floor.
func PlanFloor(candidate *Candidate) string {
	if !candidate.Rated && !candidate.Planned {
		return config.UnratedPlanFloor
	}
	return config.PlanFloor[candidate.Difficulty]
}

// Its plan was written by a model of at least its plan floor (#317 part 6). A plan from
// before planners' tiers were recorded, or one a person put in the description, counts as
// strong.
func PlanMeets(candidate *Candidate) bool {
	tier := candidate.PlanTier
	if tier == "" {
		tier = "strong"
	}
	return candidate.Planned && providers.TierAtLeast(tier, PlanFloor(candidate))
}

// The Needs plan stage: a build with no plan yet, or one whose plan came from a model
// weaker than its difficulty's plan floor (a medium plan for a hard item). A weak builder,
// which cannot plan, builds only from a plan that meets its floor.
func NeedsPlan(candidate *Candidate) bool {
	return candidate.Kind == "build" && !PlanMeets(candidate)
}

// KIND_ORDER: the order of urgency after forced items, the priority tier and the difficulty
// (`pairs` in plan.py puts harder items first, since only the stronger models can take them).
var KindOrder = map[string]int{"review": 0, "revise": 1, "build": 2}

// blank is `[^\S\n]`: whitespace other than a newline.
func blank(r rune) bool { return r != '\n' && py.IsSpace(r) }

func skipBlank(rs []rune, i int) int {
	for i < len(rs) && blank(rs[i]) {
		i++
	}
	return i
}

// issueAt is `#(\d+)` at rs[i]: where it ends, and the number.
func issueAt(rs []rune, i int) (int, int, bool) {
	if i >= len(rs) || rs[i] != '#' {
		return 0, 0, false
	}
	j := i + 1
	for j < len(rs) && unicode.IsDigit(rs[j]) {
		j++
	}
	if j == i+1 {
		return 0, 0, false
	}
	return j, py.Atoi(string(rs[i+1 : j])), true
}

// blockedWords are _BLOCKED_BY's alternatives, in its order: `blocked by|depends on|do not start
// until|waits (?:on|for)`.
var blockedWords = []string{"blocked by", "depends on", "do not start until", "waits on", "waits for"}

// blockedAt is _BLOCKED_BY matched at rs[p], a line of a description that names an issue that
// must close first: "Blocked by #125", "Depends on #12 and #14", "Do not start until #125 has
// merged". The pattern, case-insensitive:
//
//	\b(?:blocked by|depends on|do not start until|waits (?:on|for))[^\S\n]*:?[^\S\n]*
//	(#\d+(?:[^\S\n]*(?:,|\band\b|&)?[^\S\n]*#\d+)*)
//
// It returns where the match ends and the numbers its group 1 names (`re.findall(r"#(\d+)")`).
func blockedAt(rs []rune, p int) (int, []int, bool) {
	if !py.Boundary(rs, p) {
		return 0, nil, false
	}
	for _, word := range blockedWords {
		i, ok := py.FoldAt(rs, p, word)
		if !ok {
			continue
		}
		i = skipBlank(rs, i)
		if i < len(rs) && rs[i] == ':' {
			i++
		}
		i = skipBlank(rs, i)
		end, n, ok := issueAt(rs, i)
		if !ok {
			continue
		}
		numbers := []int{n}
		for {
			j := skipBlank(rs, end)
			switch {
			case j < len(rs) && (rs[j] == ',' || rs[j] == '&'):
				j++
			case py.Boundary(rs, j):
				if k, ok := py.FoldAt(rs, j, "and"); ok && py.Boundary(rs, k) {
					j = k
				}
			}
			j = skipBlank(rs, j)
			next, n, ok := issueAt(rs, j)
			if !ok {
				break
			}
			numbers = append(numbers, n)
			end = next
		}
		return end, numbers, true
	}
	return 0, nil, false
}

// The issues a description says must close first, its Plan section left out (a planner's
// notes are not the task's).
func NamedBlockers(body string) map[int]bool {
	return blockersIn(issueplan.WithoutPlan(body))
}

// blockersIn is the numbers of every _BLOCKED_BY match in `text` (`finditer`).
func blockersIn(text string) map[int]bool {
	found := map[int]bool{}
	rs := []rune(text)
	for i := 0; i < len(rs); {
		end, numbers, ok := blockedAt(rs, i)
		if !ok {
			i++
			continue
		}
		for _, n := range numbers {
			found[n] = true
		}
		i = end
	}
	return found
}

// partMatch is `_PART.match(title)`, a multi-part patch's part (docs/issues-and-patches.md):
// "Patch (part 2 of 4): …", or an older "Patch v0.2.X (part 2 of 4): …". n is the order the
// parts merge in, and every part of a patch carries the same m. The pattern, case-insensitive:
// `^(.*?)\s*\(part (\d+) of (\d+)\)`.
func partMatch(title string) (string, int, int, bool) {
	rs := []rune(title)
	for e := 0; e <= len(rs); e++ {
		if e > 0 && rs[e-1] == '\n' {
			break // `.` stops at a newline
		}
		i := e
		for i < len(rs) && py.IsSpace(rs[i]) {
			i++
		}
		i, ok := py.FoldAt(rs, i, "(part ")
		if !ok {
			continue
		}
		j := i
		for j < len(rs) && unicode.IsDigit(rs[j]) {
			j++
		}
		if j == i {
			continue
		}
		n := py.Atoi(string(rs[i:j]))
		k, ok := py.FoldAt(rs, j, " of ")
		if !ok {
			continue
		}
		l := k
		for l < len(rs) && unicode.IsDigit(rs[l]) {
			l++
		}
		if l == k || l >= len(rs) || rs[l] != ')' {
			continue
		}
		return string(rs[:e]), n, py.Atoi(string(rs[k:l])), true
	}
	return "", 0, 0, false
}

// `((the patch's name, m), n)` for a part's title, or None (ok false). Several patches can
// share a name such as "Patch v0.2.X", so the parts of one patch are those with its name and its
// m.
func PartOf(title string) (name string, total int, n int, ok bool) {
	prefix, n, total, ok := partMatch(py.Strip(title))
	if !ok {
		return "", 0, 0, false
	}
	return strings.Join(py.Split(py.Lower(prefix)), " "), total, n, true
}

// The issue a thread is a sub-issue of, from GitHub's own link, or 0. (`_PARENT`: the link as
// GitHub writes it on the issue, `…/issues/<n>`, searched with `/issues/(\d+)$`.)
func ParentOf(thread *pyjson.Object) int {
	rs := []rune(py.OrStr(thread.Get("parent_issue_url"), ""))
	for i := range rs {
		start, ok := literalAt(rs, i, "/issues/")
		if !ok {
			continue
		}
		j := start
		for j < len(rs) && unicode.IsDigit(rs[j]) {
			j++
		}
		// `$` is the end, or just before a newline that ends the text.
		if j > start && (j == len(rs) || (j == len(rs)-1 && rs[j] == '\n')) {
			return py.Atoi(string(rs[start:j]))
		}
	}
	return 0
}

func literalAt(rs []rune, i int, lit string) (int, bool) {
	for _, c := range lit {
		if i >= len(rs) || rs[i] != c {
			return 0, false
		}
		i++
	}
	return i, true
}

// PatchKey is the patch a part belongs to: `("parent", n)` for the parent it is a sub-issue of,
// or else the name its title gives (Parent 0).
type PatchKey struct {
	Parent int
	Name   string
}

// PatchPart is `((the patch, m), n)`.
type PatchPart struct {
	Patch PatchKey
	Total int
	N     int
}

// `((the patch, m), n)` for a part, or None (nil) (#1370): the patch is the parent it is a
// sub-issue of, which every part has, so two patches' parts never mix when their titles both
// open `Patch (part 2 of 4)`. A part with no parent link falls back to its title's name.
func PatchOf(thread *pyjson.Object) *PatchPart {
	name, total, n, ok := PartOf(py.OrStr(thread.Get("title"), ""))
	if !ok {
		return nil
	}
	key := PatchKey{Name: name}
	if parent := ParentOf(thread); parent != 0 {
		key = PatchKey{Parent: parent}
	}
	return &PatchPart{Patch: key, Total: total, N: n}
}

// The issues among `threads` that wait for another to close first, with what they wait for:
// the issues a "Blocked by #n" line names, GitHub's own blocked-by links, and the earlier parts
// of the same patch, the sub-issues of the same parent (`patch_of`; a part builds only once
// every part before it has closed). Only open ones
// count. GitHub is asked about open issues only when some thread names one or is a part.
func WaitsFor(ctx *hctx.Context, threads []*pyjson.Object) map[int][]int {
	named := map[int]map[int]bool{}
	for _, t := range threads {
		named[numberOf(t)] = NamedBlockers(py.OrStr(t.Get("body"), ""))
	}
	parts := map[int]*PatchPart{}
	var partOrder []int // the dict's order
	for _, t := range threads {
		n := numberOf(t)
		if _, seen := parts[n]; !seen {
			partOrder = append(partOrder, n)
		}
		parts[n] = PatchOf(t)
	}
	anyNamed, anyPart := false, false
	for _, blockers := range named {
		anyNamed = anyNamed || len(blockers) > 0
	}
	for _, part := range parts {
		anyPart = anyPart || part != nil
	}
	openNumbers := map[int]bool{}
	earlier := map[int][]int{}
	if anyNamed || anyPart {
		var everything []*pyjson.Object
		if _, ok := herr.Catch[*herr.GitHubError](func() {
			everything = ctx.GH.ListIssues("", "open", 500, "")
		}); ok {
			everything = nil
		}
		for _, t := range everything {
			openNumbers[numberOf(t)] = true
		}
		for _, number := range partOrder {
			part := parts[number]
			if part == nil {
				continue
			}
			before := []int{}
			for _, t := range everything {
				if t.Has("pull_request") {
					continue
				}
				if other := PatchOf(t); other != nil && other.Patch == part.Patch &&
					other.Total == part.Total && other.N < part.N {
					before = append(before, numberOf(t))
				}
			}
			sort.Ints(before)
			earlier[number] = before
		}
	}
	waiting := map[int][]int{}
	for _, thread := range threads {
		number := numberOf(thread)
		blockers := map[int]bool{}
		for n := range named[number] {
			if openNumbers[n] && n != number {
				blockers[n] = true
			}
		}
		for _, n := range earlier[number] {
			blockers[n] = true
		}
		if summary := thread.Obj("issue_dependencies_summary"); summary != nil &&
			py.MustInt(py.Or(summary.Get("blocked_by"), 0)) > 0 {
			herr.Catch[*herr.GitHubError](func() {
				for _, b := range ctx.GH.BlockedBy(number) {
					if b.Get("state") == "open" {
						blockers[numberOf(b)] = true
					}
				}
			})
		}
		if len(blockers) > 0 {
			waiting[number] = sortedKeys(blockers)
		}
	}
	return waiting
}

func sortedKeys(set map[int]bool) []int {
	out := make([]int, 0, len(set))
	for n := range set {
		out = append(out, n)
	}
	sort.Ints(out)
	return out
}

// Queued threads: forced requests first, then by priority tier, then second reviews,
// revisions, and the oldest builds.
//
// Read by label, so no number of open threads hides one, and a label changed since the last
// plan counts at this one. Either queue label queues either kind of thread: an issue builds
// and a pull request revises. A queue label is the request, so a thread that failed before and
// was labelled again is taken again. A thread labelled `human` or `human required` is left
// out, whatever model would take it, and so are a fullsend issue whose parts are still open
// (`open_parts`, #505),
// forced or not, and a build that waits for another issue (`waits_for`) unless it was forced,
// each with a line in `skipped` when the caller keeps one, and in `held_back` (number -> why)
// when the caller keeps that (the status issue's queue).
func Candidates(ctx *hctx.Context, s *pyjson.Object, skipped *[]string /* = None */, heldBack map[int]string /* = None */) []*Candidate {
	found := map[int]*Candidate{}
	var order []int            // found's insertion order
	human := map[int]string{}  // number -> the label that keeps it from the bot
	others := map[int]string{} // number -> the other bot's name
	parted := map[int][]int{}  // number -> its open parts
	var builds []*pyjson.Object
	labelWorking, labelPR := config.LabelWorking, config.LabelPR
	labelCross := config.LabelCross
	for _, label := range []string{config.LabelBuild, config.LabelRevise, labelCross} {
		for _, thread := range ctx.GH.ListIssues(label, "open", 300, "") {
			number := numberOf(thread)
			names := LabelNames(thread)
			_, isFound := found[number]
			_, isHuman := human[number]
			_, isOther := others[number]
			_, isParted := parted[number]
			if names[labelWorking] || isFound || isHuman || isOther || isParted {
				continue
			}
			isPR := thread.Has("pull_request")
			if label == labelCross && !(isPR && names[labelPR]) {
				continue // a second review is for the bot's own pull requests only
			}
			if IsHuman(names) {
				if lowered(names)[config.LabelHuman] {
					human[number] = config.LabelHuman
				} else {
					human[number] = config.LabelHumanRequired
				}
				continue
			}
			if other := Owner(thread); other != nil {
				others[number] = other.Name
				continue
			}
			record := recordOf(s, number)
			kind := "build"
			if label == labelCross {
				kind = "review"
			} else if isPR {
				kind = "revise"
			}
			mode := ""
			if kind == "build" {
				mode = ModeOf(names)
			}
			if kind == "build" && IsFullsend(mode, record) {
				var parts []int
				if _, ok := herr.Catch[*herr.GitHubError](func() { parts = OpenParts(ctx, number) }); ok {
					parts = []int{number} // unread: wait for the next pickup rather than race them
				}
				if len(parts) > 0 {
					parted[number] = parts
					continue
				}
			}
			votes := record.Obj("votes")
			if votes == nil {
				votes = pyjson.New()
			}
			queuedAt := record.Get("queued_at")
			if !pyjson.Truthy(queuedAt) {
				queuedAt = thread.Get("created_at")
			}
			c := NewCandidate(number, kind, py.Str(thread.GetOr("title", "")), record.Truthy("forced"),
				py.OrStr(queuedAt, ""))
			// A fullsend's split and its reconcile are a strong model's (#505).
			if mode == "fullsend" {
				c.Difficulty = "hard"
			} else {
				c.Difficulty = DifficultyOf(names, CarriedDifficulty(record))
			}
			c.Builder = py.OrStr(votes.Get("builder"), "")
			c.Priority = PriorityTier(names)
			plan := PlanOf(record, thread, kind, mode)
			c.Planned, c.PlanTier, c.PlannedBy = plan.Bool("planned"), plan.Str("plan_tier"), plan.Str("planned_by")
			for _, family := range pyjson.List(votes.Get("approvals")) {
				c.Approved = append(c.Approved, py.Str(family))
			}
			for _, approval := range reviewrule.Approvals(votes, ctx.Cfg.Pool.FamilyTier) {
				c.ApprovalTiers = append(c.ApprovalTiers, approval[1])
			}
			c.BotPR = names[labelPR]
			c.Rated = kind != "build" || Rated(names, record)
			c.Conflict = kind == "revise" && record.Get("source") == "conflict"
			if kind == "review" {
				c.ReviewFloor = py.OrStr(record.Get("review_floor"), "")
			}
			c.Mode = mode
			c.Part = kind == "build" && mode == "" && IsPart(record)
			if kind == "revise" {
				c.AuditTier = py.OrStr(record.Get("audit_tier"), "")
			}
			if kind == "build" || kind == "revise" {
				c.RoundsUsed = py.MustInt(py.Or(record.Get("verdict_rounds"), 0))
				c.RoundTier = py.OrStr(record.Get("round_tier"), "")
			}
			found[number] = c
			order = append(order, number)
			if kind == "build" && !c.Forced {
				builds = append(builds, thread)
			}
		}
	}
	waiting := map[int][]int{}
	if len(builds) > 0 {
		waiting = WaitsFor(ctx, builds)
	}
	for number := range waiting {
		delete(found, number)
	}
	if heldBack != nil {
		for number, label := range human {
			heldBack[number] = fmt.Sprintf("labelled `%s`", label)
		}
		for number, name := range others {
			heldBack[number] = fmt.Sprintf("it is %s's", name)
		}
		for number, parts := range parted {
			heldBack[number] = fmt.Sprintf("its fullsend parts %s are still open", refs(parts))
		}
		for number, blockers := range waiting {
			heldBack[number] = fmt.Sprintf("waits for %s to close", refs(blockers))
		}
	}
	if skipped != nil {
		for _, number := range sortedNumbers(human) {
			*skipped = append(*skipped, fmt.Sprintf("#%d skipped: labelled `%s`, so no model takes it, "+
				"whatever its tier", number, human[number]))
		}
		for _, number := range sortedNumbers(others) {
			*skipped = append(*skipped, fmt.Sprintf("#%d skipped: it is %s's", number, others[number]))
		}
		for _, number := range sortedNumbers(parted) {
			*skipped = append(*skipped, fmt.Sprintf("#%d skipped: its fullsend parts %s are still open",
				number, refs(parted[number])))
		}
		for _, number := range sortedNumbers(waiting) {
			*skipped = append(*skipped, fmt.Sprintf("#%d skipped: it waits for %s to close first",
				number, refs(waiting[number])))
		}
	}
	queue := []*Candidate{}
	for _, number := range order {
		if c, ok := found[number]; ok {
			queue = append(queue, c)
		}
	}
	sort.SliceStable(queue, func(i, j int) bool { return candidateLess(queue[i], queue[j]) })
	return queue
}

// candidateLess is the key `(not c.forced, c.priority, KIND_ORDER[c.kind], c.queued_at,
// c.number)`.
func candidateLess(a, b *Candidate) bool {
	if a.Forced != b.Forced {
		return a.Forced
	}
	if a.Priority != b.Priority {
		return a.Priority < b.Priority
	}
	if KindOrder[a.Kind] != KindOrder[b.Kind] {
		return KindOrder[a.Kind] < KindOrder[b.Kind]
	}
	if a.QueuedAt != b.QueuedAt {
		return a.QueuedAt < b.QueuedAt
	}
	return a.Number < b.Number
}

func sortedNumbers[V any](m map[int]V) []int {
	out := make([]int, 0, len(m))
	for n := range m {
		out = append(out, n)
	}
	sort.Ints(out)
	return out
}
