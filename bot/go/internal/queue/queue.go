// Package queue is bot/harness/queue.py.
//
// The work queue, kept as labels on issues and pull requests plus a record in the state file.
//
// `bot:build` on an issue and `bot:revise` on a pull request mean queued; `bot:cross-review` on a
// bot pull request means it waits for a review run; a live label (`bot:building`, `bot:reviewing`
// and the rest, `config.LiveLabels`; `bot:working` before #2748) means a run holds it;
// `bot:blocked` means it waits for a person. Only one `bot:` state label is on a thread at a time,
// apart from `bot:pr` and `bot:pr-open`, which say what a thread is, and `bot:merging`, which says
// its auto-merge is on.
//
// The prefix is the running bot's (`config.LABEL_PREFIX`): Squishy's queue is `squishy:build` and the
// rest (#60). Beside its queue label, a mode label (`squishy:oneshot`, `squishy:split`,
// `squishy:split-bot`, and either bot's `fullsend`, #505) says how a queued issue is built
// (`mode_of`). A thread that is another bot's
// (`owner`: its labels on it, or assigned to it) is left alone, so the two never take the same issue.
package queue

import (
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/asks"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

func cfgOthers(v any) []*identity.Other {
	if v == nil {
		return nil
	}
	return v.([]*identity.Other)
}

// modeLabel is one entry of config.MODE_LABELS, `{f"{LABEL_PREFIX}{mode}": mode for mode in
// MODES}`, which this module reads in order.
type modeLabel struct{ label, mode string }

func modeLabels() []modeLabel {
	var out []modeLabel
	for _, mode := range config.Modes {
		out = append(out, modeLabel{config.LabelPrefix + mode, mode})
	}
	return out
}

// difficultyLabel is config.DIFFICULTY_LABELS (`{f"difficulty:{name}": name}`) read for a
// lowercased label name, a retired one as what it became (`difficulty:easy` is medium, #2748).
func difficultyLabel(lowered string) (string, bool) {
	return config.LabelDifficulty(lowered)
}

// stateItem is state.item.
func stateItem(s *pyjson.Object, number int) *pyjson.Object { return state.Item(s, number) }

// recordOf is `state["items"].get(str(number), {})`.
func recordOf(s *pyjson.Object, number int) *pyjson.Object {
	if record := s.Obj("items").Obj(strconv.Itoa(number)); record != nil {
		return record
	}
	return pyjson.New()
}

// numberOf is `int(thread["number"])`, a KeyError when it has none.
func numberOf(thread *pyjson.Object) int {
	v, ok := thread.Lookup("number")
	if !ok {
		panic(fmt.Errorf("KeyError: 'number'"))
	}
	return py.MustInt(py.Or(v, 0))
}

// lowered is `{name.lower() for name in names}`.
func lowered(names map[string]bool) map[string]bool {
	out := make(map[string]bool, len(names))
	for name := range names {
		out[py.Lower(name)] = true
	}
	return out
}

// STATE_LABELS: the queue's, every label that says a run holds the thread (`config.HeldLabels`:
// the live labels, and the retired `bot:working`, so a run delivered now takes an old one off
// too), `bot:blocked`, and `bot:waiting-turn` (#2948): an approved draft waiting for a pull request
// ahead of it leaves that state whenever the thread goes into any other.
var StateLabels = slices.Concat([]string{config.LabelBuild, config.LabelRevise, config.LabelCross},
	config.HeldLabels, []string{config.LabelBlocked, config.LabelWaitingTurn})

// LabelNames is queue.label_names.
func LabelNames(thread *pyjson.Object) map[string]bool {
	names := map[string]bool{}
	for _, label := range thread.List("labels") {
		if o, ok := label.(*pyjson.Object); ok {
			names[py.Str(o.Get("name"))] = true
		}
	}
	return names
}

// Leave exactly `wanted` (or none) of the queue's state labels on the thread. A pull request
// back in the queue is no longer ready for a person to merge, so `ready for merge` comes off. A
// thread that leaves the queue (a run takes it, it is blocked, stopped or done) waits for no owner
// any more, so `bot:waiting` comes off unless it stays queued (`bot:build`, `bot:revise`).
func SetStateLabel(ctx *hctx.Context, number int, current map[string]bool, wanted string) {
	for _, name := range StateLabels {
		if current[name] && name != wanted {
			ctx.GH.RemoveLabel(number, name)
		}
	}
	if current[config.LabelWaiting] && wanted != config.LabelBuild && wanted != config.LabelRevise {
		ctx.GH.RemoveLabel(number, config.LabelWaiting)
	}
	if wanted != "" && current[config.LabelReady] {
		ctx.GH.RemoveLabel(number, config.LabelReady)
	}
	if wanted != "" && !current[wanted] {
		ctx.GH.AddLabels(number, []string{wanted})
	}
}

// LiveLabelOn is the label that says a run holds the thread (`config.HeldLabels`), the first
// of them on it, or "" for none.
func LiveLabelOn(names map[string]bool) string {
	for _, label := range config.HeldLabels {
		if names[label] {
			return label
		}
	}
	return ""
}

// Merging marks a bot pull request whose auto-merge the bot just turned on (`bot:merging`,
// #2748). `current` is its labels.
func Merging(ctx *hctx.Context, number int, current map[string]bool) {
	if !current[config.LabelMerging] {
		ctx.GH.AddLabels(number, []string{config.LabelMerging})
	}
}

// NotMerging takes `bot:merging` off a pull request whose auto-merge went off, or that merged or
// closed. `current` is its labels.
func NotMerging(ctx *hctx.Context, number int, current map[string]bool) {
	if current[config.LabelMerging] {
		ctx.GH.RemoveLabel(number, config.LabelMerging)
	}
}

// SetWaiting puts `bot:waiting` on an item held for its owner (#2748, section 1), or takes it off
// (`waiting` false): the hold (`plan.SyncHolds`) and a handoff (`plan.Handoff`) call it. No other
// subscription takes an item while it waits (`plan.Owning`, `plan.Pairs`). `current` is its
// labels.
func SetWaiting(ctx *hctx.Context, number int, current map[string]bool, waiting bool) {
	if waiting && !current[config.LabelWaiting] {
		ctx.GH.AddLabels(number, []string{config.LabelWaiting})
	} else if !waiting && current[config.LabelWaiting] {
		ctx.GH.RemoveLabel(number, config.LabelWaiting)
	}
}

// BranchForIssue is queue.branch_for_issue.
func BranchForIssue(number int) string {
	return fmt.Sprintf("%sissue-%d", config.BranchPrefix, number)
}

// Where a revision of pull request `number` that was cut off keeps its unfinished work
// (#317 part 3): never the pull request's own branch, which would start CI on it.
func WipBranch(number int) string {
	return fmt.Sprintf("%swip/%d", config.BranchPrefix, number)
}

// OpenPullForBranch is queue.open_pull_for_branch.
func OpenPullForBranch(ctx *hctx.Context, branch string) *pyjson.Object {
	pulls := ctx.GH.ListPulls("open", branch, 200)
	if len(pulls) > 0 {
		return pulls[0]
	}
	return nil
}

func when(ctx *hctx.Context, s *pyjson.Object) string {
	return providers.WhenFree(ctx.Cfg.Pool, s, ctx.Now(), ctx.Cfg.Timezone, ctx.Cfg.Secrets)
}

func haltNote(ctx *hctx.Context, s *pyjson.Object) string {
	if s.Truthy("halted") {
		return fmt.Sprintf(" The bot is halted, though: `%s start` resumes it.", config.Slash)
	}
	return ""
}

// Labelled `human` (whatever its case): people do it, and the bot leaves it alone. So is one
// labelled `human required` (#793): the bot found it needs what only a person can do and
// handed it to people (`deliver.Deliverer._needs_person`), or a person said so. And so, for every
// queue path, is one labelled `tracking` (#2748): a tracker or a status issue, which the bot may
// comment on and rewrite its own of, but never queues, plans, builds or splits.
func IsHuman(names map[string]bool) bool {
	low := lowered(names)
	return low[config.LabelHuman] || low[config.LabelHumanRequired] || low[config.LabelTracking]
}

// IsTracking is: labelled `tracking` (whatever its case, #2748).
func IsTracking(names map[string]bool) bool {
	return lowered(names)[config.LabelTracking]
}

// KeptBy is the label that keeps the thread from the bot (`IsHuman`): `human`, `human required`
// or `tracking`, in that order, or "" for none.
func KeptBy(names map[string]bool) string {
	low := lowered(names)
	for _, label := range []string{config.LabelHuman, config.LabelHumanRequired, config.LabelTracking} {
		if low[label] {
			return label
		}
	}
	return ""
}

// OWNING: what follows another bot's prefix on a label that makes the thread that bot's (#60): its
// queue and state labels, its pull request, and its modes and trees. A `planned` or `stuck` label
// left from an old run does not.
var Owning = slices.Concat([]string{"build", "revise", "cross-review"}, config.LiveRuns,
	[]string{"working", "blocked", "pr-open", "pr", "merging", "waiting", "waiting-turn", "needs-plan",
		"oneshot", "split", "split-bot", "fullsend", "tree"})

// The other bot whose thread this is (`config.OTHERS`), or None (nil): one of its owning labels
// is on it, or it is assigned to that bot's account.
func Owner(thread *pyjson.Object) *identity.Other {
	names := lowered(LabelNames(thread))
	assignees := map[string]bool{}
	for _, a := range thread.List("assignees") {
		if o, ok := a.(*pyjson.Object); ok {
			assignees[py.Lower(py.OrStr(o.Get("login"), ""))] = true
		}
	}
	for _, other := range cfgOthers(config.Others) {
		owns := slices.ContainsFunc(Owning, func(suffix string) bool {
			return names[other.LabelPrefix+suffix]
		})
		if owns || assignees[py.Lower(other.Login)] {
			return other
		}
	}
	return nil
}

// OwnerReply is queue.owner_reply.
func OwnerReply(number int, other *identity.Other) string {
	return fmt.Sprintf("#%d is %s's (@%s): it carries %s's labels or is "+
		"assigned to it, so I leave it alone. Take its `%s…` labels off "+
		"(and unassign @%s) to hand it to me.", number, other.Name, other.Login, other.Name,
		other.LabelPrefix, other.Login)
}

// How a queued issue is built (#60): the first of the bot's modes whose label is on it
// (`oneshot`, `split`, `split-bot`, `fullsend`), or "" for a plain build.
func ModeOf(names map[string]bool) string {
	low := lowered(names)
	for _, ml := range modeLabels() {
		if low[ml.label] {
			return ml.mode
		}
	}
	return ""
}

// Leave exactly `mode`'s label (or none, for "") of the mode labels on the issue; the labels
// it has after.
func SetMode(ctx *hctx.Context, number int, names map[string]bool, mode string) map[string]bool {
	labels := modeLabels()
	wanted := map[string]bool{}
	isMode := map[string]bool{}
	for _, ml := range labels {
		isMode[ml.label] = true
		if ml.mode == mode {
			wanted[ml.label] = true
		}
	}
	for _, ml := range labels {
		if names[ml.label] && !wanted[ml.label] {
			ctx.GH.RemoveLabel(number, ml.label)
		}
	}
	var missing []string
	for label := range wanted {
		if !names[label] {
			missing = append(missing, label)
		}
	}
	if len(missing) > 0 {
		sort.Strings(missing)
		ctx.GH.AddLabels(number, missing)
	}
	after := map[string]bool{}
	for name := range names {
		if !isMode[name] {
			after[name] = true
		}
	}
	for label := range wanted {
		after[label] = true
	}
	return after
}

// MODE_WORDS: how each mode says what it will do, for the reply that queues it.
var ModeWords = map[string]string{"": "build it", "oneshot": "build it in one run with fullsend",
	"split":     "split it into sub-issues I build",
	"split-bot": "split it into sub-issues the night bot builds",
	"fullsend": "split it into parts that land on one branch, then reconcile them into " +
		"one pull request"}

// A fullsend issue (#505): it is queued in that mode (`mode_of`), or its record says its
// tree was split for fullsend. Its own branch is the one its parts land on.
func IsFullsend(mode string, record *pyjson.Object) bool {
	tree := record.Obj("tree")
	return mode == "fullsend" || tree.Get("mode") == "fullsend"
}

// A fullsend part (#505): its record says it lands on its tree's branch, as deliver wrote
// when it opened the part, or `adopt` when it took in an issue someone had opened.
func IsPart(record *pyjson.Object) bool {
	parent := py.MustInt(py.Or(record.Get("part_of"), 0))
	return parent != 0 && record.Get("onto") == BranchForIssue(parent)
}

// PartBuilder is the subscription whose run built fullsend part `record` (#2748, section 2), the
// one its tree's reconcile may send it back to: its `built_by`, which deliver writes when the part
// lands (`deliver.LandPart`), or, for one landed before that was kept, its owner then (the
// subscription that took its build up). "" for none.
func PartBuilder(record *pyjson.Object) string {
	if builder := py.OrStr(record.Get("built_by"), ""); builder != "" {
		return builder
	}
	return py.OrStr(record.Get("owner"), "")
}

func refs(numbers []int) string {
	listed := make([]string, len(numbers))
	for i, n := range numbers {
		listed[i] = fmt.Sprintf("#%d", n)
	}
	return strings.Join(listed, ", ")
}

// PartsReply is queue.parts_reply.
func PartsReply(number int, parts []int) string {
	return fmt.Sprintf("#%d's parts are still open (%s), and they land on its branch, "+
		"`%s`; I queue its reconcile myself when they have all "+
		"closed.", number, refs(parts), BranchForIssue(number))
}

// The reply to a request for a thread `is_human` keeps from the bot.
func HumanReply(number int, names map[string]bool /* = frozenset() */) string {
	low := lowered(names)
	if KeptBy(names) == config.LabelTracking {
		return fmt.Sprintf("#%d is labelled `%s`: a tracker or status issue, which I may comment on "+
			"but never queue, plan or build. Take that label off to hand it to me.", number,
			config.LabelTracking)
	}
	if low[config.LabelHumanRequired] && !low[config.LabelHuman] {
		return fmt.Sprintf("#%d is labelled `%s`: it needs something only a "+
			"person can do (my comment on it says what), so it is people's and I leave it "+
			"alone. Take that label off to hand it back to me.", number, config.LabelHumanRequired)
	}
	return fmt.Sprintf("#%d is labelled `%s`: people do it, so I leave it alone. Take that "+
		"label off to hand it to me.", number, config.LabelHuman)
}

// What `difficulty:hard` on the thread changes about who takes it (`human` refuses earlier).
func labelNote(names map[string]bool) string {
	if DifficultyOf(names, "") == "hard" {
		return " It is labelled `difficulty:hard`, so only Opus plans, builds and reviews it."
	}
	return ""
}

// hardest is `max(found, key=DIFFICULTIES.index, default=fallback)`.
func hardest(found []string, fallback string) string {
	best := -1
	for _, d := range found {
		best = max(best, slices.Index(config.Difficulties, d))
	}
	if best < 0 {
		return fallback
	}
	return config.Difficulties[best]
}

func labelled(names map[string]bool) []string {
	var found []string
	for name := range names {
		if d, ok := difficultyLabel(py.Lower(name)); ok {
			found = append(found, d)
		}
	}
	return found
}

// The thread's difficulty: the hardest `difficulty:*` label on it (whatever its case) or
// `carried` (what its issue had, kept on a bot PR's record, or a floor the bot raised it to),
// and `medium` with neither.
func DifficultyOf(names map[string]bool, carried string /* = "" */) string {
	found := labelled(names)
	if d := config.AsDifficulty(carried); d != "" {
		found = append(found, d)
	}
	return hardest(found, config.DefaultDifficulty)
}

// The hardest `difficulty:*` label on the thread, or "" for none.
func LabelledDifficulty(names map[string]bool) string {
	return hardest(labelled(names), "")
}

// What the state file carries for a thread's difficulty: a bot pull request's issue's
// (`difficulty`), and a floor the bot raised it to (`difficulty_floor`: the easy rule broken,
// before #2748 retired it), the harder of the two; a retired difficulty reads as what it became.
func CarriedDifficulty(record *pyjson.Object) string {
	var found []string
	for _, key := range []string{"difficulty", "difficulty_floor"} {
		if d := config.AsDifficulty(py.OrStr(record.Get(key), "")); d != "" {
			found = append(found, d)
		}
	}
	return hardest(found, "")
}

// Who set the item's difficulty label: `bot` when its record's rating (`difficulty_by`) names
// the label it carries, `person` for any other label, "" for none (#317 part 8). The bot never
// replaces a person's label.
func RatingSource(names map[string]bool, record *pyjson.Object) string {
	label := LabelledDifficulty(names)
	by := record.Obj("difficulty_by")
	// A rating from before #2748 may say easy, which reads as medium, as its relabelled label does.
	if label != "" && config.AsDifficulty(py.OrStr(by.Get("difficulty"), "")) == label {
		return "bot"
	}
	if label != "" {
		return "person"
	}
	return ""
}

// Someone has said how hard it is: a `difficulty:*` label, the bot's own rating
// (`difficulty_by`), or a difficulty its record carries. An unrated item counts as medium for
// routing, and a medium or strong model rates it when it plans it (#317 part 8).
func Rated(names map[string]bool, record *pyjson.Object) bool {
	return LabelledDifficulty(names) != "" || record.Truthy("difficulty_by") ||
		CarriedDifficulty(record) != ""
}

// Queue an issue for building. Returns the reply line. `ask` is the comment that asked, kept
// on the record that ends up queued so later stages can react to it (`asks`). `mode` is how to
// build it (#60): one of the bot's modes, "" for a plain build (its mode labels come off), or
// None (nil) to leave its mode labels as they are. A fullsend issue whose parts are still open is
// not queued at all (#505): the sweep queues its reconcile once they have all closed.
func QueueBuild(ctx *hctx.Context, number int, by string, force bool /* = False */, labelPresent bool /* = False */, ask *asks.Ask /* = None */, mode *string /* = None */) string {
	issue := ctx.GH.GetIssue(number)
	if issue.Has("pull_request") {
		if mode != nil && *mode != "" {
			return fmt.Sprintf("`%s` works on an issue; on a pull request, ask for a revision.", *mode)
		}
		return QueueRevise(ctx, number, by, force, "request", labelPresent, nil, ask)
	}
	if issue.Get("state") != "open" {
		return fmt.Sprintf("#%d is closed, so there is nothing to build. Reopen it first.", number)
	}
	names := LabelNames(issue)
	if IsHuman(names) {
		return HumanReply(number, names)
	}
	labelBuild := config.LabelBuild
	if other := Owner(issue); other != nil {
		if labelPresent && names[labelBuild] {
			ctx.GH.RemoveLabel(number, labelBuild)
		}
		return OwnerReply(number, other)
	}
	modes := config.Modes
	if mode != nil && *mode != "" && !slices.Contains(modes, *mode) {
		mine := ""
		if len(modes) > 0 {
			quoted := make([]string, len(modes))
			for i, m := range modes {
				quoted[i] = "`" + m + "`"
			}
			mine = fmt.Sprintf(" Mine: %s.", strings.Join(quoted, ", "))
		}
		return fmt.Sprintf("I have no `%s` mode.", *mode) + mine
	}
	if mode != nil && *mode != "" && names[config.LabelPROpen] &&
		OpenPullForBranch(ctx, BranchForIssue(number)) != nil {
		return fmt.Sprintf("#%d has a pull request of mine open already, so I will not %s"+
			" over it: `%s rebuild` closes it and builds again, or comment on the pull "+
			"request to change it.", number, ModeWords[*mode], config.Slash)
	}
	asked := ModeOf(names)
	if mode != nil {
		asked = *mode
	}
	// Only a fullsend issue or a tree can have parts still landing (#505).
	record := pyjson.New()
	if asked == "fullsend" || names[config.LabelTree] {
		record = recordOf(ctx.Store.Load(), number)
	}
	if IsFullsend(asked, record) {
		var children []*pyjson.Object
		if exc, ok := herr.Catch[*herr.GitHubError](func() {
			children = ctx.GH.ListSubIssues(number)
		}); ok {
			return fmt.Sprintf("#%d's parts could not be read (%s), so I queued nothing.", number,
				py.SliceTo(exc.Error(), 200))
		}
		var parts []int
		for _, child := range children {
			if child.Get("state") == "open" {
				parts = append(parts, numberOf(child))
			}
		}
		if len(parts) > 0 && !IsFullsend("", record) {
			return Adopt(ctx, number, names, children, by) // someone opened its parts already
		}
		if len(parts) > 0 {
			if labelPresent && names[labelBuild] {
				ctx.GH.RemoveLabel(number, labelBuild)
			}
			return PartsReply(number, parts)
		}
	}
	if asked == "" {
		// A plain build of a parent with open sub-issues is a fullsend tree's (#2748, section 2).
		if record.Len() == 0 {
			record = recordOf(ctx.Store.Load(), number)
		}
		children, unread := QueuedParent(ctx, issue, record)
		if unread != "" {
			return fmt.Sprintf("#%d's sub-issues could not be read (%s), so I queued nothing.", number, unread)
		}
		if len(children) > 0 {
			return AutoAdopt(ctx, number, names, children, by)
		}
	}
	if mode != nil && len(modes) > 0 {
		names = SetMode(ctx, number, names, *mode)
	}
	words, ok := ModeWords[ModeOf(names)]
	if !ok {
		words = "build it"
	}
	if ModeOf(names) == "fullsend" && IsFullsend("", record) {
		words = "reconcile its parts into one pull request" // split already (#505)
	}
	if labelSuggestion := config.LabelSuggestion; names[labelSuggestion] {
		// A person queued one of my suggestions, which is their yes to it (#664). Its record
		// remembers it was one, so if it comes to nothing it is logged as such (#797).
		ctx.GH.RemoveLabel(number, labelSuggestion)
		ctx.Store.Update(func(s *pyjson.Object) any {
			stateItem(s, number).Set("suggestion", true)
			return nil
		}, fmt.Sprintf("suggestion #%d approved", number))
	}
	if config.IsLive(names) {
		pending(ctx, number, by, ask)
		return fmt.Sprintf("I am working on #%d right now. When this run ends I go round once more "+
			"with your comment.", number)
	}
	if names[config.LabelPROpen] {
		if pull := OpenPullForBranch(ctx, BranchForIssue(number)); pull != nil {
			SetStateLabel(ctx, number, names, "")
			return QueueRevise(ctx, numberOf(pull), by, force, "request", false, nil, ask)
		}
	}
	SetStateLabel(ctx, number, names, labelBuild)
	s := ctx.Store.Update(func(s *pyjson.Object) any {
		queued(s, number, "build", by, force, ctx, ask)
		return nil
	}, fmt.Sprintf("queue #%d", number))
	if force {
		return startNow(ctx, number, "build", fmt.Sprintf("Queued #%d", number)) + haltNote(ctx, s) +
			labelNote(names)
	}
	return fmt.Sprintf("Queued #%d; I will %s %s.%s%s", number, words, when(ctx, s), haltNote(ctx, s),
		labelNote(names))
}

// `fullsend` on an issue whose sub-issues someone opened already, a tracker split by hand:
// they are its parts, with no split (#505). Its branch is made at `main`'s head; each open
// sub-issue the bot may build gets the record that sends its build there (`is_part`), and is
// queued if it was not; and the issue becomes a tree, whose reconcile the sweep queues once
// every sub-issue has closed (`sweep._trees`). One labelled `human` or the other bot's, one a
// run holds, or one with a pull request of mine open finishes on `main` as it is, and the
// reconcile waits for it to close too.
func Adopt(ctx *hctx.Context, number int, names map[string]bool, children []*pyjson.Object, by string) string {
	onto := BranchForIssue(number)
	unread := ""
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		if ctx.GH.BranchSHA(onto) == "" {
			sha := ctx.GH.BranchSHA(ctx.Cfg.DefaultBranch)
			if sha == "" {
				unread = fmt.Sprintf("`%s` could not be read, so I adopted nothing.",
					ctx.Cfg.DefaultBranch)
				return
			}
			ctx.GH.CreateBranch(onto, sha)
		}
	}); ok {
		return fmt.Sprintf("The branch #%d's parts would land on, `%s`, could not be made "+
			"(%s), so I adopted nothing.", number, onto, py.SliceTo(exc.Error(), 200))
	}
	if unread != "" {
		return unread
	}
	labelPROpen := config.LabelPROpen
	var adopted []*pyjson.Object
	var kept []string
	for _, child := range children {
		if child.Get("state") != "open" {
			continue
		}
		childNames := LabelNames(child)
		other := Owner(child)
		why := ""
		switch {
		case lowered(childNames)[config.LabelHuman]:
			why = "labelled `human`"
		case IsHuman(childNames):
			why = fmt.Sprintf("labelled `%s`", KeptBy(childNames))
		case other != nil:
			why = fmt.Sprintf("%s's", other.Name)
		case config.IsLive(childNames):
			why = "a run holds it now"
		case childNames[labelPROpen]:
			why = "a pull request of mine is open"
		}
		if why != "" {
			kept = append(kept, fmt.Sprintf("#%s (%s)", py.Str(child.Get("number")), why))
		} else {
			adopted = append(adopted, child)
		}
	}

	change := func(s *pyjson.Object) any {
		for _, child := range adopted {
			stateItem(s, numberOf(child)).Set("onto", onto).Set("part_of", number)
		}
		all := []any{}
		for _, c := range children {
			all = append(all, numberOf(c))
		}
		stateItem(s, number).Set("tree", pyjson.New(
			"mode", "fullsend", "children", all,
			"at", clock.ISO(ctx.Now()), "onto", onto, "adopted", true)).Set("closeout", false)
		return nil
	}
	ctx.Store.Update(change, fmt.Sprintf("adopt #%d's sub-issues as its parts", number))
	for _, child := range adopted {
		childNames := LabelNames(child)
		if !slices.ContainsFunc(StateLabels, func(label string) bool { return childNames[label] }) {
			QueueBuild(ctx, numberOf(child), by, false, false, nil, nil)
		}
	}
	names = SetMode(ctx, number, names, "")
	SetStateLabel(ctx, number, names, "")
	if names[config.LabelNeedsPlan] {
		ctx.GH.RemoveLabel(number, config.LabelNeedsPlan) // a tree's parts are planned, not it
	}
	if labelTree := config.LabelTree; !names[labelTree] {
		ctx.GH.AddLabels(number, []string{labelTree})
	}
	var listedParts []string
	for _, child := range adopted {
		listedParts = append(listedParts, "#"+py.Str(child.Get("number")))
	}
	listed := strings.Join(listedParts, ", ")
	if listed == "" {
		listed = "none of them"
	}
	branch := ctx.Cfg.DefaultBranch
	said := fmt.Sprintf("#%d's sub-issues are its fullsend parts now, with no split: %s land on "+
		"`%s`, made at `%s`'s head, each as soon as it is built, "+
		"with no checks, review or pull request of its own. A strong model plans each one "+
		"first, and a medium subscription (Muse first) may build it from that plan, whatever "+
		"its difficulty. Once every sub-issue has closed I queue the reconcile: a strong model "+
		"merges the parts on `%s`, makes every check green and opens one pull request "+
		"into `%s`.", number, listed, onto, branch, onto, branch)
	if len(kept) > 0 {
		said += fmt.Sprintf(" These finish on `%s` as they are, and the reconcile "+
			"waits for them to close too: %s.", branch, strings.Join(kept, ", "))
	}
	return said
}

// QueuedParent is the sub-issues of `issue`, queued for a plain build (`record` its state), when
// this bot adopts it as a fullsend tree by itself (#2748, section 2; `fullsend_adopt`): one of
// them is open, and `issue` is an open issue the bot may build that is no tree already, no part
// or sub-issue of another (trees never nest: a sub-issue is built as it is asked), and has no
// pull request of mine open and no run holding it. None (nil) for any other issue, and `unread`
// when its sub-issues could not be read. A `human` or `tracking` issue, and the other bot's
// (Squishy's), are never adopted. The request paths (`QueueBuild`) and the claim of a queued
// build (`plan.Claim`, which a label an issue form or triage put on reaches without an event) ask
// it; only the claim's question costs a call per plain build, and GitHub's `sub_issues_summary`
// spares it for an issue with no sub-issue at all.
func QueuedParent(ctx *hctx.Context, issue *pyjson.Object, record *pyjson.Object) ([]*pyjson.Object, string) {
	names := LabelNames(issue)
	if !ctx.Cfg.FullsendAdopt || !slices.Contains(config.Modes, "fullsend") || issue.Has("pull_request") ||
		issue.Get("state") != "open" || IsHuman(names) || Owner(issue) != nil || config.IsLive(names) ||
		names[config.LabelTree] || names[config.LabelPROpen] || IsFullsend("", record) || IsPart(record) ||
		ParentOf(issue) != 0 {
		return nil, ""
	}
	if summary := issue.Obj("sub_issues_summary"); summary != nil && py.MustInt(py.Or(summary.Get("total"), 0)) == 0 {
		return nil, ""
	}
	var children []*pyjson.Object
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		children = ctx.GH.ListSubIssues(numberOf(issue))
	}); ok {
		return nil, py.SliceTo(exc.Error(), 200)
	}
	if !slices.ContainsFunc(children, func(child *pyjson.Object) bool { return child.Get("state") == "open" }) {
		return nil, ""
	}
	return children, ""
}

// AutoAdopt is `Adopt` for a queued parent (`QueuedParent`, #2748): its open sub-issues become its
// fullsend parts as `fullsend` would make them, and the reply says it was done unasked and how to
// stop it.
func AutoAdopt(ctx *hctx.Context, number int, names map[string]bool, children []*pyjson.Object, by string) string {
	return fmt.Sprintf("#%d was queued with open sub-issues, so I adopted it as a fullsend tree "+
		"by itself (#2748), as `%s fullsend` would. ", number, config.Slash) +
		Adopt(ctx, number, names, children, by) +
		fmt.Sprintf(" To stop it, comment `%s stop` here: it and its parts leave the queue. A "+
			"queued parent with open sub-issues is always built this way; to have one built as a "+
			"single issue, remove its sub-issues before you queue it.", config.Slash)
}

// The squash commit's title: the pull request's, as GitHub writes it for a pull request of
// several commits. Without it a one-commit pull request lands on `main` under the bot's own
// commit message ("bot: build pass 1 for #85").
func MergeTitle(pull *pyjson.Object, method string) string {
	title := py.Strip(py.OrStr(pull.Get("title"), ""))
	if method != "squash" || title == "" {
		return ""
	}
	return fmt.Sprintf("%s (#%s)", title, py.Str(pull.Get("number")))
}

// Keep a pull request from merging: auto-merge off, and out of `main`'s merge queue, and
// `bot:merging` off (#2748). A queued pull request has no auto-merge left to turn off (REST
// `auto_merge` reads null once GitHub put it in the queue), so only the dequeue stops it; a push
// to its branch takes it out as well.
func HoldMerge(ctx *hctx.Context, pull *pyjson.Object) {
	if number := py.MustInt(py.Or(pull.Get("number"), 0)); number != 0 {
		NotMerging(ctx, number, LabelNames(pull))
	}
	node := pull.Get("node_id")
	if !pyjson.Truthy(node) {
		return
	}
	var actions []func(string)
	if pull.Truthy("auto_merge") {
		actions = append(actions, ctx.GH.DisableAutoMerge)
	}
	for _, action := range append(actions, ctx.GH.Dequeue) {
		herr.Catch[*herr.GitHubError](func() { action(py.Str(node)) }) // nothing to turn off, or not in the queue
	}
}

// Put a bot pull request back in `main`'s merge queue when the queue took its cleared head
// out and no failed build of it says why, so `events.on_queue_ci` never heard: it conflicted
// with a pull request ahead of it that then failed, or its checks ran past the queue's timeout.
// Only when the queue's removal is the last thing that touched its auto-merge and no person
// did it, and once for each commit `main` is at, so one that conflicts again waits for `main`
// to move. The housekeeping pass calls it. A note of what it did, or "".
func RequeueCleared(ctx *hctx.Context, pull *pyjson.Object, record *pyjson.Object) string {
	head := py.OrStr(pull.Obj("head").Get("sha"), "")
	node := py.OrStr(pull.Get("node_id"), "")
	names := LabelNames(pull)
	if pull.Truthy("auto_merge") || pull.Get("state") != "open" || node == "" ||
		pull.Get("mergeable_state") == "dirty" || Cleared(record).Get("sha") != head ||
		names[config.LabelReady] || names[config.LabelPersonMerges] ||
		record.Truthy("stop_requested") {
		return ""
	}
	removal := ctx.GH.QueueRemoval(node)
	if removal == nil || (removal["type"] == "User" &&
		py.Lower(removal["actor"]) != py.Lower(ctx.Cfg.BotLogin)) {
		return ""
	}
	main := ctx.GH.BranchSHA(ctx.Cfg.DefaultBranch)
	tried := record.Obj("requeued").Copy()
	if tried.Get(head) == main {
		return ""
	}
	ctx.GH.EnableAutoMerge(node, ctx.Cfg.MergeMethod, head, MergeTitle(pull, ctx.Cfg.MergeMethod))
	number := numberOf(pull)
	Merging(ctx, number, names)
	tried.Set(head, main)
	ctx.Store.Update(func(s *pyjson.Object) any {
		stateItem(s, number).Set("requeued", tried)
		return nil
	}, fmt.Sprintf("requeue #%d", number))
	reason := removal["reason"]
	if reason == "" {
		reason = "no reason"
	}
	return fmt.Sprintf("#%d: back in the merge queue (taken out: %s)", number, reason)
}

// CIRerunsLeft is how many more times CI may run again with no commit on the head a CI fix is for
// (the record's `ci_sha`), out of `allowed` (`ci_reruns`, #2948): a CI fix whose failing check
// passes locally re-runs it (`deliver`), and each re-run is counted in `ci_reruns`, by head.
func CIRerunsLeft(record *pyjson.Object, allowed int) int {
	sha := py.OrStr(record.Get("ci_sha"), "")
	return max(0, allowed-py.MustInt(py.Or(record.Obj("ci_reruns").Get(sha), 0)))
}

// Queue a pull request for a revision. Returns the reply line.
func QueueRevise(ctx *hctx.Context, number int, by string, force bool /* = False */, source string /* = "request" */, labelPresent bool /* = False */, extra *pyjson.Object /* = None */, ask *asks.Ask /* = None */) string {
	pull := ctx.GH.GetPull(number)
	if pull.Get("state") != "open" {
		return fmt.Sprintf("#%d is not open, so I will not change it.", number)
	}
	if IsHuman(LabelNames(pull)) {
		return HumanReply(number, LabelNames(pull))
	}
	headRepo := pull.Obj("head").Obj("repo").Get("full_name")
	if headRepo != ctx.Cfg.Repo {
		return fmt.Sprintf("#%d comes from a fork; I can only push to branches in %s.", number,
			ctx.Cfg.Repo)
	}
	names := LabelNames(pull)
	waits := OwnReviewWaits(recordOf(ctx.Store.Load(), number))
	if config.IsLive(names) && !waits {
		pending(ctx, number, by, ask)
		return fmt.Sprintf("I am revising #%d right now. When this run ends I go round once more "+
			"with your comment.", number)
	}
	if waits {
		// Its own review run has not started (#2748): the revision goes first, and chains an own
		// review run of its own.
		ctx.Store.Update(func(s *pyjson.Object) any {
			stateItem(s, number).Pop(OwnReviewKey)
			return nil
		}, fmt.Sprintf("own review #%d gives way", number))
	}
	SetStateLabel(ctx, number, names, config.LabelRevise)
	botPR := names[config.LabelPR]
	if botPR {
		HoldMerge(ctx, pull)
	}
	change := func(s *pyjson.Object) any {
		queued(s, number, "revise", by, force, ctx, ask)
		record := stateItem(s, number)
		record.Set("source", source)
		if extra != nil {
			record.Update(extra)
		}
		return nil
	}
	s := ctx.Store.Update(change, fmt.Sprintf("queue revise #%d", number))
	held := ""
	if botPR {
		held = " Auto-merge is off until the revision lands."
	}
	if force {
		return startNow(ctx, number, "revise", fmt.Sprintf("Queued a revision of #%d", number)) +
			held + haltNote(ctx, s) + labelNote(names)
	}
	return fmt.Sprintf("Queued a revision of #%d; I will do it %s.%s%s%s", number, when(ctx, s), held,
		haltNote(ctx, s), labelNote(names))
}

// Queue a review run of a bot pull request's head, and no revision (`/harness review`,
// #317 part 10): with `floor` (`strong` or `medium`) it waits for a reviewer of that tier or
// stronger. Returns the reply line.
func QueueReview(ctx *hctx.Context, number int, by string, floor string /* = "" */, notes string /* = "" */, ask *asks.Ask /* = None */) string {
	pull := ctx.GH.GetPull(number)
	names := LabelNames(pull)
	if pull.Get("state") != "open" {
		return fmt.Sprintf("#%d is not open, so there is nothing to review.", number)
	}
	if IsHuman(names) {
		return HumanReply(number, names)
	}
	if !names[config.LabelPR] {
		return fmt.Sprintf("#%d is not a pull request I opened; review runs are for mine only.", number)
	}
	waits := OwnReviewWaits(recordOf(ctx.Store.Load(), number))
	if config.IsLive(names) && !waits {
		pending(ctx, number, by, ask)
		return fmt.Sprintf("I am working on #%d right now. When this run ends I go round once more "+
			"with your comment.", number)
	}
	if waits {
		// Its own review run has not started (#2748): the review asked for takes its place.
		ctx.Store.Update(func(s *pyjson.Object) any {
			stateItem(s, number).Pop(OwnReviewKey)
			return nil
		}, fmt.Sprintf("own review #%d gives way", number))
	}
	SetStateLabel(ctx, number, names, config.LabelCross)

	change := func(s *pyjson.Object) any {
		record := stateItem(s, number)
		record.Set("queued_at", clock.ISO(ctx.Now())).Set("requested_by", by).
			Set("review_floor", floor).Set("review_notes", py.SliceTo(notes, 2000)).
			Set("stop_requested", false).Set("pending_request", false)
		asks.Note(record, ask)
		return nil
	}
	s := ctx.Store.Update(change, fmt.Sprintf("queue review #%d", number))
	tier := ""
	if floor != "" {
		tier = fmt.Sprintf(", by a %s model or stronger", floor)
	}
	return fmt.Sprintf("Queued a review run of #%d%s; I will do it %s.", number, tier, when(ctx, s))
}

// Start a forced run. If GitHub refuses, the item is still queued and still forced, and the
// next hourly run starts it, inside a subscription's hours or not.
func startNow(ctx *hctx.Context, number int, mode string, queued string) string {
	if exc, ok := herr.Catch[*herr.GitHubError](func() { ctx.Dispatch(&number, true, mode) }); ok {
		return fmt.Sprintf("%s, but starting a run now failed (%s). It stays forced, so "+
			"the next hourly run starts it.", queued, py.SliceTo(exc.Error(), 200))
	}
	return fmt.Sprintf("%s and started a run now (`--force`).", queued)
}

// A fresh request: it clears a stop, the failure and interruption counts and any pending
// note, and keeps `ci_fixes`, which only a person's `forget` clears.
func queued(s *pyjson.Object, number int, kind string, by string, force bool, ctx *hctx.Context, ask *asks.Ask /* = None */) {
	record := stateItem(s, number)
	record.Set("kind", kind).Set("queued_at", clock.ISO(ctx.Now())).Set("requested_by", by).
		Set("forced", force).Set("stop_requested", false).Set("failures", 0).
		Set("interruptions", 0).Set("pending_request", false)
	asks.Note(record, ask)
}

// Remember a request that arrived while a run held the thread; deliver requeues it.
func pending(ctx *hctx.Context, number int, by string, ask *asks.Ask /* = None */) {
	change := func(s *pyjson.Object) any {
		record := stateItem(s, number)
		record.Set("pending_request", true).Set("pending_by", by).Set("pending_at", clock.ISO(ctx.Now()))
		asks.Note(record, ask)
		return nil
	}
	ctx.Store.Update(change, fmt.Sprintf("pending #%d", number))
}

// Take a thread out of the queue. A run that holds it keeps its live label until it has
// stopped, so a request made in the meantime waits for that run instead of racing it.
func Stop(ctx *hctx.Context, number int, by string) string {
	thread := ctx.GH.GetIssue(number)
	names := LabelNames(thread)
	working := config.IsLive(names)
	// A pull request whose own review run has not started yet (#2748) carries `bot:reviewing` with
	// no run behind it: it leaves the queue, and its reservation goes.
	reserved := OwnReviewWaits(recordOf(ctx.Store.Load(), number))
	if reserved {
		working = false
		SetStateLabel(ctx, number, names, "")
	} else {
		SetStateLabel(ctx, number, names, LiveLabelOn(names))
	}
	if thread.Has("pull_request") && names[config.LabelPR] {
		herr.Catch[*herr.GitHubError](func() { HoldMerge(ctx, ctx.GH.GetPull(number)) })
	}
	var dropped []string
	change := func(s *pyjson.Object) any {
		record := stateItem(s, number)
		record.Set("stop_requested", true).Set("stopped_by", by).Set("stopped_at", clock.ISO(ctx.Now())).
			Set("forced", false).Set("pending_request", false)
		record.Pop("hold")         // out of the queue, it waits for no owner (its `bot:waiting` is off)
		record.Pop(BuildNextKey)   // nor keeps its planner's lane for a build (#2903)
		record.Pop(WaitingTurnKey) // nor waits its turn behind another pull request (#2948)
		record.Pop(TurnKey)
		if reserved {
			record.Pop(OwnReviewKey)
		}
		// Asks still waiting will never be answered; a run's own are settled when it stops.
		dropped = asks.PopWaiting(record)
		return nil
	}
	ctx.Store.Update(change, fmt.Sprintf("stop #%d", number))
	asks.React(ctx.GH, dropped, asks.NoAnswer)
	if working {
		return fmt.Sprintf("Stopping work on #%d; the run gives up at its next checkpoint.", number)
	}
	return fmt.Sprintf("#%d is out of the queue.", number)
}
