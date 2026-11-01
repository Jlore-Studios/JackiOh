package deliver

import (
	"fmt"
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/failures"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	splitmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/split"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// ------------------------------------------------------------------ a split (#60)

// SplitOut is Deliverer._split_out.
//
// A split's end: open its sub-issues, or close a tree whose close-out found it done.
//
// The tree is checked again here (`split.from_dict`), not trusted from the model job. The
// sub-issues are opened in build order, each with its plan as the **Plan** section of its
// description (which both bots build from without planning again), its type labels as far
// as the repository has them, the parent's priority, its difficulty, and the queue label of
// whoever builds it: this bot's for `split` and `fullsend`, the other bot's for `split-bot`.
// Then they become GitHub sub-issues of the parent, linked by "blocked by", and the parent
// becomes a tree (`squishy:tree`) with a checklist.
//
// A fullsend split (#505) first makes the parent's own branch from `main`'s head, the branch
// its parts land on (`_land_part`), and each part's record says so (`onto`, `part_of`).
func (x *Deliverer) SplitOut(number int) {
	tree := splitmod.FromDict(x.Result.Get("tree"), config.Difficulties)
	mode := py.OrStr(x.Plan.Get("mode"), "split")
	names := x.Labels(number)
	if !tree.OK() {
		x.Result.Set("reason", "the split could not be used: "+strings.Join(tree.Problems, "; "))
		x.Fail(number, "build")
		return
	}
	onto := ""
	if mode == "fullsend" && !tree.Done {
		var problem string
		onto, problem = x.integrationBranch(number)
		if problem != "" {
			x.Result.Set("reason", problem)
			x.Fail(number, "build")
			return
		}
	}
	queue.SetStateLabel(x.Ctx, number, names, "")
	queue.SetMode(x.Ctx, number, names, "")
	if tree.Done {
		if names[config.LabelTree] {
			x.try(func() { x.GH.RemoveLabel(number, config.LabelTree) })
		}
		x.GH.CreateComment(number, fmt.Sprintf("Every sub-issue of this has closed, and its end state "+
			"holds on `main` (%s), so I am closing it.\n\n"+
			"%s", x.link(), tree.Summary))
		x.try(func() {
			x.GH.UpdateIssue(number, pyjson.New("state", "closed", "state_reason", "completed"))
		})
		x.remember(number, pyjson.New("tree_done", clock.ISO(x.Ctx.Now())))
		x.log(fmt.Sprintf("#%d: its tree is done; closed it", number))
		return
	}
	queueLabel, builder := config.LabelBuild, "me"
	if all := config.Others; mode == "split-bot" && len(all) > 0 {
		other := all[0]
		queueLabel = other.LabelPrefix + "build"
		builder = fmt.Sprintf("%s (@%s)", other.Name, other.Login)
	}
	who := fmt.Sprintf("%s (`%s`, %s)", config.Title, x.BuildSeat.Model, x.link())
	repoLabels := map[string]bool{}
	for _, label := range x.GH.ListLabels() {
		repoLabels[py.Str(label.Get("name"))] = true
	}
	var priority []string
	for _, n := range sortedKeys(names) {
		if isPriorityTier(strings.ToLower(n)) {
			priority = append(priority, n)
		}
	}
	type madeIssue struct {
		issue *splitmod.SubIssue
		child int
	}
	var made []madeIssue
	ids := map[string]int{}
	numbers := map[string]int{}
	for _, issue := range splitmod.Ordered(tree.Issues) {
		waits := []int{}
		for _, k := range issue.BlockedBy {
			if n, ok := numbers[k]; ok {
				waits = append(waits, n)
			}
		}
		waits = append(waits, issue.BlockedByIssues...)
		text := splitmod.Body(issue, number, who, waits, onto)
		if issue.Plan != "" {
			text = issueplan.WithPlan(text, redact.Redact(issue.Plan, nil), who, "")
		}
		labels := []string{}
		for _, label := range issue.Labels {
			if repoLabels[label] && !reservedLabel(label) {
				labels = append(labels, label)
			}
		}
		if issue.Difficulty != "" {
			labels = append(labels, "difficulty:"+issue.Difficulty)
		}
		labels = append(labels, priority...)
		labels = append(labels, queueLabel)
		var created *pyjson.Object
		if exc, ok := herr.Catch[*herr.GitHubError](func() {
			created = x.GH.CreateIssue(issue.Title, text, labels)
		}); ok {
			x.log(fmt.Sprintf("could not open %s: %s", py.Repr(issue.Key), exc.Error()))
			continue
		}
		numbers[issue.Key] = py.MustInt(py.Or(created.Get("number"), 0))
		ids[issue.Key] = py.MustInt(py.Or(created.Get("id"), 0))
		made = append(made, madeIssue{issue, numbers[issue.Key]})
		if onto != "" {
			x.remember(numbers[issue.Key], pyjson.New("onto", onto, "part_of", number))
		}
	}
	for _, m := range made {
		issue, child := m.issue, m.child
		c := ids[issue.Key]
		x.try(func() { x.GH.AddSubIssue(number, c) })
		for _, key := range issue.BlockedBy {
			if b, ok := ids[key]; ok {
				x.try(func() { x.GH.AddBlockedBy(child, b) })
			}
		}
		for _, blocker := range issue.BlockedByIssues {
			x.try(func() { x.GH.AddBlockedBy(child, py.MustInt(py.Or(x.GH.GetIssue(blocker).Get("id"), 0))) })
		}
	}
	if len(made) == 0 {
		x.Result.Set("reason", "none of the sub-issues could be opened")
		x.Fail(number, "build")
		return
	}
	if !x.Labels(number)[config.LabelTree] {
		x.GH.AddLabels(number, []string{config.LabelTree})
	}
	pairs := make([]splitmod.Made, len(made))
	children := make([]int, len(made))
	for i, m := range made {
		pairs[i] = splitmod.Made{Issue: m.issue, Number: m.child}
		children[i] = m.child
	}
	x.GH.CreateComment(number, splitmod.Checklist(number, pairs, builder, tree.Summary, x.link(), onto))
	treeRecord := pyjson.New("mode", mode, "children", pyjson.ListOf(children), "at", clock.ISO(x.Ctx.Now()))
	if onto != "" {
		treeRecord.Set("onto", onto)
	}
	x.remember(number, pyjson.New("tree", treeRecord, "closeout", false))
	shown := make([]string, len(children))
	for i, c := range children {
		shown[i] = fmt.Sprintf("#%d", c)
	}
	x.log(fmt.Sprintf("#%d: split into %s", number, strings.Join(shown, ", ")))
}

// The branch a fullsend tree's parts land on (#505): the parent's own, made from `main`'s
// head when GitHub does not have it yet. `(branch, problem)`.
func (x *Deliverer) integrationBranch(number int) (string, string) {
	onto := queue.BranchForIssue(number)
	defaultBranch := x.Cfg.DefaultBranch
	x.fetch(defaultBranch, true)
	x.fetch(onto, false)
	if x.Repo.Rev("origin/"+onto) != "" {
		return onto, "" // a later stage of the same tree lands on what is there
	}
	sha := x.Repo.Rev("origin/" + defaultBranch)
	if x.Cfg.DryRun {
		x.log(fmt.Sprintf("dry run: would make %s at %s", onto, sha))
		return onto, ""
	}
	url := x.remoteURL()
	if exc, ok := herr.Catch[*herr.GitError](func() {
		x.Repo.Push(url, sha+":refs/heads/"+onto, x.Cfg.WriteToken(), "")
	}); ok {
		return "", fmt.Sprintf("the branch its parts land on, `%s`, could not be made: %s", onto, exc.Error())
	}
	x.log(fmt.Sprintf("made %s at %s for #%d's parts", onto, py.SliceTo(sha, 12), number))
	return onto, ""
}

// LandPart is Deliverer._land_part.
//
// A fullsend part's end (#505): its change lands on its tree's branch (`onto`), never on
// `main`, with no pull request. It fast-forwards the branch, or joins the parts that landed
// while it was built in a merge commit; the branch is never pushed with force. A part that
// conflicts with them, or loses the race to push twice, is kept on its own branch instead
// (`parked` on the parent's record), for the reconcile to merge. The bundle is refused as
// `_publish` refuses one: the head the result names, descending from where the work started,
// no forbidden path, no conflict marker. Either way the part closes as done: the reconcile
// checks and reviews every part at once.
func (x *Deliverer) LandPart(number int) {
	onto, parent := py.Str(x.Plan.Get("onto")), py.MustInt(x.Plan.Get("part_of"))
	if x.easyBreach(number, "build") {
		return
	}
	sha, problem := x.land(onto, number)
	own := queue.BranchForIssue(number)
	parked := ""
	if problem == "" && sha == "" {
		parked = own
		problem = x.park(own)
	}
	if problem != "" {
		x.Result.Set("reason", problem)
		x.Fail(number, "build")
		return
	}
	if parked != "" {
		change := func(state *pyjson.Object) any {
			entry := statemod.Item(state, parent)
			set := map[string]bool{parked: true}
			for _, b := range py.Each(entry.Get("parked")) {
				set[py.Str(b)] = true
			}
			kept := make([]string, 0, len(set))
			for b := range set {
				kept = append(kept, b)
			}
			sort.Strings(kept)
			entry.Set("parked", pyjson.ListOf(kept))
			return nil
		}
		x.Ctx.Store.Update(change, fmt.Sprintf("#%d parked for #%d", number, parent))
	} else if x.Repo.Rev("origin/"+own) != "" {
		// A cut-off run's work, kept there, is on `onto` now.
		x.try(func() { x.GH.DeleteBranch(own) })
	}
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
	var said string
	if parked != "" {
		said = fmt.Sprintf("It conflicts with the parts already on `%s`, so it is kept on "+
			"`%s` instead, for #%d's reconcile to merge.", onto, parked, parent)
		x.log(fmt.Sprintf("#%d: parked on %s for #%d", number, parked, parent))
	} else {
		said = fmt.Sprintf("It landed on `%s` at `%s`.", onto, py.SliceTo(sha, 12))
		x.log(fmt.Sprintf("#%d: landed on %s at %s", number, onto, py.SliceTo(sha, 12)))
	}
	x.GH.CreateComment(number, fmt.Sprintf("Built on %s (%s), as a "+
		"fullsend part of #%d, with no checks or review of its own. "+
		"%s Once every part of #%d has closed, one run "+
		"reconciles them, makes every check green and opens one pull "+
		"request into `main`. Closing this.", x.builtOn(), x.link(), parent, said, parent))
	x.try(func() {
		x.GH.UpdateIssue(number, pyjson.New("state", "closed", "state_reason", "completed"))
	})
	x.remember(number, pyjson.New("landed", sha, "parked_on", parked, "last_findings", []any{}, "question", "",
		"failures", 0, "forced", false, "interruptions", 0, "died", 0, "pending_request", false))
}

// Push the part's bundle onto `onto`: `(sha landed, problem)`, and `("", "")` when it
// conflicts with what is there, or the push lost the race twice.
func (x *Deliverer) land(onto string, number int) (string, string) {
	name := x.Result.Get("bundle")
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if !pyjson.Truthy(name) || !isFile(outPath(x.OutDir, py.Str(name))) {
		return "", "the run left no bundle to land"
	}
	base := "origin/" + onto
	own := queue.BranchForIssue(number)
	for _, ref := range []string{onto, own} {
		x.fetch(ref, false)
	}
	if x.Repo.Rev(base) == "" {
		return "", fmt.Sprintf("the branch its parts land on, `%s`, is gone", onto)
	}
	local := "deliver/" + branch
	var head string
	if exc, ok := herr.Catch[*herr.GitError](func() {
		head = x.Repo.FetchBundle(outPath(x.OutDir, py.Str(name)), branch, local)
	}); ok {
		return "", "the bundle could not be read: " + exc.Error()
	}
	if head != x.Result.Get("head") {
		return "", "the bundle's head is not the head the result names"
	}
	start := py.OrStr(x.Result.Get("start"), "")
	if start == "" || x.isAncestor(start, head) != 0 {
		return "", "the bundle does not descend from the commit the work started at"
	}
	forbidden := x.Repo.Unsanctioned(local, []string{start, x.Repo.MergeBase(base, local)}, x.Cfg.ForbiddenPaths)
	if len(forbidden) > 0 {
		return "", "the change touches paths the bot may not change: " + strings.Join(forbidden, ", ")
	}
	markers := x.Repo.Markers(local, x.Repo.ChangedPaths(base, local))
	if len(markers) > 0 {
		return "", "conflict markers are still in " + shownPaths(markers) + ", so I landed nothing"
	}
	if x.Cfg.DryRun {
		x.log(fmt.Sprintf("dry run: would land %s on %s", head, onto))
		return head, ""
	}
	url := x.remoteURL()
	for range 2 {
		tip := x.Repo.Rev(base)
		var target string
		if x.isAncestor(tip, local) == 0 {
			target = head // nothing landed since it started: a fast-forward
		} else {
			target = x.mergeCommit(tip, local, fmt.Sprintf("%s: land #%d on %s", config.CommitPrefix, number, onto))
			if target == "" {
				return "", "" // it conflicts with the parts that landed meanwhile
			}
		}
		if _, ok := herr.Catch[*herr.GitError](func() {
			x.Repo.Push(url, target+":refs/heads/"+onto, x.Cfg.WriteToken(), "")
		}); !ok {
			return target, ""
		}
		// Another part landed first: read the branch again, and try once more.
		x.fetch(onto, false)
	}
	return "", ""
}

// A merge commit of `local` onto `tip`, made without a worktree, or "" when the two
// conflict.
func (x *Deliverer) mergeCommit(tip string, local string, message string) string {
	probe := x.Repo.Run(false, 600, nil, "merge-tree", "--write-tree", "--no-messages", tip, local)
	tree := ""
	if probe.Returncode == 0 && strings.TrimSpace(probe.Stdout) != "" {
		tree = strings.Fields(probe.Stdout)[0]
	}
	if tree == "" {
		return ""
	}
	who := git.IdentityBot(x.Cfg.BotLogin, x.Cfg.BotUserID)
	env := map[string]string{"GIT_AUTHOR_NAME": who.Name, "GIT_AUTHOR_EMAIL": who.Email,
		"GIT_COMMITTER_NAME": who.Name, "GIT_COMMITTER_EMAIL": who.Email}
	return strings.TrimSpace(x.Repo.Run(true, 600, env, "commit-tree", tree, "-p", tip, "-p", local,
		"-m", message).Stdout)
}

// Keep a part that could not land on its own branch, for the reconcile: "" when it is
// there, or the problem.
func (x *Deliverer) park(own string) string {
	if x.Cfg.DryRun {
		x.log("dry run: would keep the part on " + own)
		return ""
	}
	url := x.remoteURL()
	if exc, ok := herr.Catch[*herr.GitError](func() {
		x.Repo.Push(url, fmt.Sprintf("deliver/%s:refs/heads/%s", py.Str(x.Plan.Get("branch")), own),
			x.Cfg.WriteToken(), "")
	}); ok {
		return fmt.Sprintf("it could not land, and keeping it on `%s` was refused: %s", own, exc.Error())
	}
	return ""
}

// CyclesCap is Deliverer._cycles_cap: in-run review verdict rounds for this item's difficulty
// (#725).
func (x *Deliverer) CyclesCap() int {
	return py.MustInt(x.Cfg.MaxReviewCycles.GetOr(py.OrStr(x.Plan.Get("difficulty"), "medium"), 3))
}

// Verdict rounds this item used with its current owner, and that owner's tier (#725),
// for the record: usage pauses and infra never reach a verdict, so they never spend one.
func (x *Deliverer) rounds() (int, string) {
	return py.MustInt(py.Or(x.Result.Get("verdict_rounds"), 0)), py.OrStr(x.Result.Get("round_tier"), "")
}

// The tier of the seat that just reviewed this pull request (#725): a revision that
// answers its findings stays at that tier, so the auditor fixes and pushes it.
func (x *Deliverer) auditTier() string {
	seats := x.Plan.Obj("seats")
	tier := ""
	if review, ok := seats.Get("review").(*pyjson.Object); ok {
		tier = py.OrStr(review.Get("tier"), "")
	}
	if tier != "" {
		return tier
	}
	return x.Provider.Tier
}

// The first words of a comment on a build or revision that was not approved: the real
// round count, and the real reason when it stopped before its last round.
func (x *Deliverer) notApprovedHead() string {
	rounds := pyLen(py.Or(x.Result.Get("cycles"), []any{}))
	limit := x.CyclesCap()
	if failures.Stuck(objList(x.Result.Get("cycles")), limit) {
		return fmt.Sprintf("After all %d rounds the reviewer still had blocking findings", limit)
	}
	reason := py.OrStr(x.Result.Get("reason"), "no reason recorded")
	return fmt.Sprintf("It stopped without an approval after %d of %d round(s): %s", rounds, limit, reason)
}

func (x *Deliverer) whyMd() string {
	return failures.Why(objList(x.Result.Get("cycles")), x.CyclesCap(), py.OrStr(x.Result.Get("reason"), ""))
}

// Label a run that used every round `bot:stuck`, for a person to review; the sentence
// that says so, or "" for one that stopped sooner.
func (x *Deliverer) markStuck(numbers ...int) string {
	if !failures.Stuck(objList(x.Result.Get("cycles")), x.CyclesCap()) {
		return ""
	}
	for _, number := range numbers {
		if number != 0 {
			x.try(func() { x.GH.AddLabels(number, []string{config.LabelStuck}) })
		}
	}
	return fmt.Sprintf("Labelled `%s`: it failed every round, so a person should read the "+
		"above before anyone tries again. ", config.LabelStuck)
}

// An approval clears `bot:stuck`.
func (x *Deliverer) unstick(numbers ...int) {
	for _, number := range numbers {
		if number != 0 && x.Labels(number)[config.LabelStuck] {
			x.try(func() { x.GH.RemoveLabel(number, config.LabelStuck) })
		}
	}
}

// inRunVerdicts is `sum(1 for c in self.result.get("cycles") or [] if isinstance(c.get("review"),
// dict) and c["review"].get("verdict") in ("approve", "changes"))`.
func (x *Deliverer) inRunVerdicts() int {
	n := 0
	for _, c := range objList(x.Result.Get("cycles")) {
		if review, ok := c.Get("review").(*pyjson.Object); ok {
			if v := review.Get("verdict"); v == "approve" || v == "changes" {
				n++
			}
		}
	}
	return n
}

// What a change built without a shipping review in its run went through.
func (x *Deliverer) selfCheckSaid() string {
	rounds := 0
	for _, c := range objList(x.Result.Get("cycles")) {
		rounds += pyLen(py.Or(c.Get("self_check"), []any{}))
	}
	verdicts := x.inRunVerdicts()
	openFindings := pyLen(py.Or(x.Result.Get("self_check_findings"), []any{}))
	if verdicts != 0 {
		return fmt.Sprintf("Its own reviewer read it %d time(s) in the run and asked for "+
			"changes; the kept work goes to its final audit, which judges it from "+
			"scratch.", verdicts)
	}
	if rounds == 0 {
		return "No model on its subscription may review it."
	}
	if openFindings != 0 {
		return fmt.Sprintf("It checked itself %d time(s) and ran out with %d "+
			"finding(s) still open, which its reviewer gets.", rounds, openFindings)
	}
	return fmt.Sprintf("It checked itself %d time(s), the last one clean; that is not a review.", rounds)
}

func (x *Deliverer) createPull(title string, branch string, body string, draft bool) *pyjson.Object {
	var pull *pyjson.Object
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		pull = x.GH.CreatePull(title, branch, x.Cfg.DefaultBranch, body, draft)
	}); ok {
		existing := x.openPull()
		if exc.Status == 422 && existing != nil {
			x.GH.UpdatePull(py.MustInt(existing.Get("number")), pyjson.New("title", title, "body", body))
			return existing
		}
		herr.Raise(exc)
	}
	return pull
}

// Revise is Deliverer._revise.
func (x *Deliverer) Revise(number int, status string) {
	botPR := x.Plan.Truthy("bot_pr")
	if status == "failed" {
		x.Fail(number, "revise")
		return
	}
	if status == "stopped" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.GH.CreateComment(number, "Stopped, as asked. I pushed nothing.")
		return
	}
	if status == "blocked" {
		question := py.Or(x.Result.Get("question"), x.Result.Get("reason"))
		if x.Result.Truthy("human_required") {
			x.holdAutoMerge(0)
			x.NeedsPerson(number, py.OrStr(question, ""), "revise this", "", fmt.Sprintf(
				"`%s revise <notes>` (or `@%s <notes>`) queues the "+
					"rest", config.Slash, x.Cfg.BotLogin))
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.holdAutoMerge(0)
		x.GH.CreateComment(number, "I need a decision before I can revise this.\n\n"+py.Str(question))
		x.remember(number, pyjson.New("question", py.OrStr(question, "")))
		return
	}
	if (status == "approved" || status == "built") && x.easyBreach(number, "revise") {
		return
	}
	// As the run found it: the source and the clearance `_carry` reads are reset below.
	before := x.Record(number)
	started := before.Get("started_at")
	if status == "built" && !botPR {
		// No review run follows on a person's pull request: unreviewed work is not pushed.
		status = "not_approved"
	}
	if status != "approved" && status != "built" {
		x.dropWip(number) // nothing of it ships: the next revision starts afresh
		first := x.firstFinding()
		if first == "" {
			first = py.Str(x.Result.Get("reason"))
		}
		if x.strike(number, "a revision was not approved: "+first) {
			return
		}
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.holdAutoMerge(0)
		stuck := x.markStuck(number)
		tail := ""
		if stuck != "" {
			tail = "\n\n" + stuck
		}
		x.GH.CreateComment(number, fmt.Sprintf("I pushed nothing from this revision. "+
			"%s (%s).\n\n"+
			"%s\n\n%s", x.notApprovedHead(), x.link(), x.findingsMd(), x.whyMd())+tail)
		rounds, roundTier := x.rounds()
		x.remember(number, pyjson.New("last_findings", py.Or(x.Result.Get("findings"), []any{}),
			"feedback_since", started, "verdict_rounds", rounds, "round_tier", roundTier))
		return
	}
	approved := status == "approved"
	pushed, problem := x.publish(approved)
	if problem != "" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.holdAutoMerge(0)
		x.GH.CreateComment(number, fmt.Sprintf("I could not push the revision (%s): %s.", x.link(), problem))
		return
	}
	x.dropWip(number)
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
	if approved {
		x.unstick(number)
	}
	report := py.SliceTo(py.OrStr(x.Result.Get("report"), ""), ReportChars)
	rerun := !pushed && x.Record(number).Truthy("ci_run_id")
	// The head a CI fix pushed: CI red on it is a strike (`events.on_ci`, #316).
	fixed := pyjson.New()
	if pushed && x.Record(number).Get("source") == "ci" {
		fixed.Set("ci_fixed_head", py.OrStr(x.Result.Get("head"), ""))
	}
	rounds, roundTier := x.rounds()
	x.remember(number, pyjson.New("feedback_since", started, "failures", 0, "source", "", "last_findings", []any{},
		"question", "", "verdict_rounds", rounds, "round_tier", roundTier,
		"self_check_findings", py.Or(x.Result.Get("self_check_findings"), []any{})).Update(fixed))
	note := ""
	if !pushed && approved {
		note = "\n\nI changed nothing: the reviewer agreed no change was needed."
		if rerun {
			note += " I asked CI to run the failed jobs again."
			x.rerunCI(number)
		}
	}
	mergeNote := ""
	if botPR {
		pull := x.GH.GetPull(number)
		// The commit the reviewer approved: what was pushed, or with nothing pushed, where
		// the work started. Someone may have pushed meanwhile; that head nobody reviewed.
		reviewed := py.OrStr(x.Result.Get("start"), "")
		if pushed {
			reviewed = py.OrStr(x.Result.Get("head"), "")
		}
		nowHead := x.branchHead(number)
		waiting := x.Record(number).Truthy("pending_request")
		var rule string
		if reviewed != "" && nowHead != "" && reviewed != nowHead {
			x.holdAutoMerge(0)
			rule = x.approved(number, nowHead, !waiting, false, true, nil)
		} else {
			var carry *pyjson.Object
			whyNot := ""
			if pushed {
				carry, whyNot = x.Carry(before, reviewed)
			}
			target := reviewed
			if target == "" {
				target = nowHead
			}
			if !approved {
				rule = x.awaitReview(number, target, !waiting)
			} else {
				if pull.Truthy("draft") && !waiting {
					x.try(func() { x.GH.MarkReady(py.Str(pull.Get("node_id"))) })
				}
				rule = x.approved(number, target, !waiting, true, true, carry)
			}
			rule = x.carryLine(before, carry, whyNot) + rule
		}
		if !waiting {
			mergeNote = "\n\n" + rule
		}
	}
	var said string
	if approved {
		said = x.ReviewSeat.Describe() + " reviewed it adversarially and approved it"
	} else {
		said = x.selfCheckSaid()
	}
	done := "done"
	if pushed {
		done = "pushed"
	}
	x.GH.CreateComment(number, fmt.Sprintf("Revision %s (%s) on %s; %s%s\n\n%s%s",
		done, x.link(), x.builtOn(), said, note, report, mergeNote))
}
