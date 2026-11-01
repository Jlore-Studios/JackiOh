package deliver

import (
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	statemod "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// ------------------------------------------------------------------ the review rule

// Difficulty is Deliverer._difficulty: the item's difficulty: its labels, and what its record
// carries (a bot PR keeps its issue's).
func (x *Deliverer) Difficulty(number int) string {
	return queue.DifficultyOf(x.Labels(number), py.OrStr(x.Record(number).Get("difficulty"), ""))
}

// The head's approvals as (family, tier), one per review.
func (x *Deliverer) approvals(votes *pyjson.Object) [][2]string {
	return reviewrule.Approvals(votes, x.Cfg.Pool.FamilyTier)
}

func tiersOf(approvals [][2]string) []string {
	tiers := make([]string, len(approvals))
	for i, a := range approvals {
		tiers[i] = a[1]
	}
	return tiers
}

// RuleMet is Deliverer._rule_met.
//
// A commit ships when its approvals meet the review rule for its difficulty
// (`review_rule.met`) and no model's rejection of it stands (a model that rejected it and
// later approved it has withdrawn it).
func (x *Deliverer) RuleMet(votes *pyjson.Object, difficulty string /* = "medium" */) bool {
	approvals := py.Each(votes.Get("approvals"))
	for _, family := range py.Each(votes.Get("rejections")) {
		if !containsEqual(approvals, family) {
			return false
		}
	}
	if reviewrule.Carried(votes) {
		return true
	}
	return reviewrule.Met(tiersOf(x.approvals(votes)), difficulty)
}

// Vote is Deliverer._vote.
//
// Record this run's reviewer's verdict on `head`, with its tier: an approval adds one
// review (once per run and family), a rejection withdraws that family's approvals of the
// head. A new
// head starts afresh: its builder is this run's model when this run built it, and unknown
// otherwise. `carried` is the clearance a conflict resolution carries to it (`_carry`).
func (x *Deliverer) Vote(pr int, head string, approve bool, builder bool, carried *pyjson.Object /* = None */) *pyjson.Object {
	family, tier := x.Provider.Family, "weak"
	if x.ReviewSeat != nil {
		family, tier = x.ReviewSeat.Family(), x.ReviewSeat.Tier
	}
	run := x.Cfg.RunID
	votes := pyjson.New()

	change := func(state *pyjson.Object) any {
		entry := statemod.Item(state, pr)
		current := pyjson.Obj(entry.Get("votes")).Copy()
		if head == "" || current.Get("sha") != head {
			by := ""
			if builder {
				by = x.Provider.Family
			}
			current = pyjson.New("sha", head, "builder", by, "approvals", []any{}, "rejections", []any{},
				"tiers", pyjson.New(), "reviews", []any{})
		}
		reviews, isList := current.Get("reviews").([]any)
		if !isList {
			// A record from before reviews were counted one by one: one per family.
			reviews = []any{}
			for _, a := range reviewrule.Approvals(current, x.Cfg.Pool.FamilyTier) {
				reviews = append(reviews, pyjson.New("family", a[0], "tier", a[1]))
			}
		}
		if !approve {
			kept := []any{}
			for _, r := range reviews {
				if pyjson.Obj(r).Get("family") != family {
					kept = append(kept, r)
				}
			}
			reviews = kept
		} else if !(run != "" && slices.ContainsFunc(reviews, func(r any) bool {
			return pyjson.Obj(r).Get("run") == run && pyjson.Obj(r).Get("family") == family
		})) {
			// Once per run: a deliver job run again must not count its review twice.
			review := pyjson.New("family", family, "tier", tier)
			if run != "" {
				review.Set("run", run)
			}
			reviews = append(append([]any{}, reviews...), review)
		}
		key, other := "approvals", "rejections"
		if !approve {
			key, other = "rejections", "approvals"
		}
		current.Set(key, uniq(append(append([]any{}, py.Each(current.GetOr(key, []any{}))...), family)))
		rest := []any{}
		for _, f := range py.Each(current.GetOr(other, []any{})) {
			if f != family {
				rest = append(rest, f)
			}
		}
		current.Set(other, rest)
		current.Set("tiers", pyjson.Obj(current.Get("tiers")).Copy().Set(family, tier))
		current.Set("reviews", reviews)
		if carried.Len() > 0 {
			current.Set("carried", carried)
		}
		entry.Set("votes", current)
		votes.Update(current)
		return nil
	}

	x.Ctx.Store.Update(change, fmt.Sprintf("votes #%d", pr))
	return votes
}

// Record this run's reviewer's approval of `head` on the PR, then turn on auto-merge
// when the rule is met (recording `head` as cleared); otherwise turn it off and queue a
// review run. Returns the line for the comment.
func (x *Deliverer) approved(pr int, head string, merge bool /* = True */, vote bool /* = True */, builder bool /* = True */, carried *pyjson.Object /* = None */) string {
	var votes *pyjson.Object
	if vote {
		if x.ReviewSeat != nil {
			votes = x.Vote(pr, head, true, builder, carried)
		} else {
			by := ""
			if builder {
				by = x.Provider.Family
			}
			votes = x.voteReset(pr, head, by)
		}
	} else {
		votes = x.voteReset(pr, head, "")
	}
	difficulty := x.Difficulty(pr)
	met := x.RuleMet(votes, difficulty)
	if met {
		x.clear_(pr, head, votes)
	}
	if !merge {
		return ""
	}
	labels := x.Labels(pr)
	if !labels[config.LabelPR] {
		return "" // never auto-merge a pull request the bot did not open
	}
	if met {
		return x.autoMerge(x.GH.GetPull(pr), head)
	}
	return x.waitForReview(pr, votes, labels, difficulty)
}

// Record `head` as cleared: its approvals met the review rule. A conflict with `main`
// later needs only the resolving run's own review (`_carry`). A clearance carried across
// a resolution keeps who cleared the change, and adds who reviewed the resolution.
func (x *Deliverer) clear_(pr int, head string, votes *pyjson.Object) {
	who := reviewrule.Who(x.approvals(votes))
	entry := pyjson.New("sha", head, "at", clock.ISO(x.Ctx.Now()), "by", who)
	// The item's strikes end once this head is green in CI too (#316): the sweep sees that
	// (`sweep._green_heads`), and a merge does (`events.on_pull_closed`).
	if carried, ok := votes.Get("carried").(*pyjson.Object); ok {
		entry.Set("by", py.OrStr(carried.Get("by"), who))
		entry.Set("resolved_by", who)
		entry.Set("carried_from", py.OrStr(carried.Get("from"), ""))
	}
	x.remember(pr, pyjson.New("cleared", entry))
}

// Carry is Deliverer._carry.
//
// Whether a revision carries its change's clearance to `head` (`review_rule`): it was
// queued because `main` left the change with conflicts, it started from the commit the
// rule cleared, a medium or strong model reviewed it in the run, and it changed nothing but
// the files the merge left conflicted. That last is worked out here from git, never from
// the model job: `head` against the merge git makes by itself of the cleared commit and the
// `main` that `head` took in. Returns the carry, or None and why not (empty when the
// revision was not a conflict on a cleared change).
func (x *Deliverer) Carry(record *pyjson.Object, head string) (*pyjson.Object, string) {
	cleared := queue.Cleared(record)
	if record.Get("source") != "conflict" || cleared.Len() == 0 {
		return nil, ""
	}
	start := py.OrStr(x.Result.Get("start"), "")
	if cleared.Get("sha") != start {
		return nil, "the branch had moved on from the commit they cleared"
	}
	seat := x.ReviewSeat
	if seat == nil || !providers.TierAtLeast(seat.Tier, "medium") {
		return nil, "no medium or strong model reviewed the resolution in this run"
	}
	const noRedo = "git could not redo the merge to compare against"
	var main string
	var conflicts, beyond []string
	unreadable := false
	if _, ok := herr.Catch[*herr.GitError](func() {
		main = x.Repo.MergeBase("origin/"+x.Cfg.DefaultBranch, head)
		proc := x.Repo.Run(false, 600, nil, "merge-tree", "--write-tree", "--name-only",
			"--no-messages", start, main)
		var lines []string
		for _, line := range py.Splitlines(proc.Stdout) {
			if line = strings.TrimSpace(line); line != "" {
				lines = append(lines, line)
			}
		}
		if (proc.Returncode != 0 && proc.Returncode != 1) || len(lines) == 0 {
			unreadable = true
			return
		}
		tree := lines[0]
		conflicts = lines[1:]
		for _, p := range x.Repo.NamesBetween(tree, head) {
			if !slices.Contains(conflicts, p) {
				beyond = append(beyond, p)
			}
		}
	}); ok || unreadable {
		return nil, noRedo
	}
	if len(beyond) > 0 {
		return nil, fmt.Sprintf("it changed more than the conflicts (%s)", shownPaths(beyond))
	}
	return pyjson.New("from", start, "main", main, "conflicts", pyjson.ListOf(conflicts),
		"by", py.OrStr(cleared.Get("by"), "")), ""
}

// What a conflict revision of a cleared change did with the clearance, for the comment
// ("" for any other revision).
func (x *Deliverer) carryLine(record *pyjson.Object, carry *pyjson.Object, whyNot string) string {
	cleared := queue.Cleared(record)
	if cleared.Len() == 0 || !(carry.Len() > 0 || whyNot != "") {
		return ""
	}
	by := ""
	if cleared.Truthy("by") {
		by = fmt.Sprintf(" (%s)", py.Str(cleared.Get("by")))
	}
	said := fmt.Sprintf("The reviews had cleared this change at `%s`%s", py.SliceTo(py.Str(cleared.Get("sha")), 12), by)
	if carry == nil {
		return fmt.Sprintf("%s, but that does not carry over: %s. ", said, whyNot)
	}
	var files []string
	for _, p := range py.Each(carry.Get("conflicts")) {
		files = append(files, py.Str(p))
	}
	resolved := "merged it cleanly"
	if len(files) > 0 {
		resolved = "resolved the conflicts in " + shownPaths(files)
	}
	return fmt.Sprintf("%s. This revision only merged `main` and %s, and its own reviewer "+
		"approved that, so the clearance carries over and no review run is needed. ", said, resolved)
}

// A change no model in its run could review: its commit has no vote yet, and it waits
// for a review run.
func (x *Deliverer) awaitReview(pr int, head string, queueIt bool /* = True */) string {
	votes := x.voteReset(pr, head, x.Provider.Family)
	if !queueIt {
		x.holdAutoMerge(pr)
		return ""
	}
	labels := x.Labels(pr)
	if !labels[config.LabelPR] {
		return "A person reviews it."
	}
	return x.waitForReview(pr, votes, labels, x.Difficulty(pr))
}

func (x *Deliverer) waitForReview(pr int, votes *pyjson.Object, labels map[string]bool, difficulty string) string {
	x.holdAutoMerge(pr)
	queue.SetStateLabel(x.Ctx, pr, labels, config.LabelCross)
	x.remember(pr, pyjson.New("queued_at", clock.ISO(x.Ctx.Now())))
	approvals := x.approvals(votes)
	inRun := x.inRunVerdicts()
	var have string
	if len(approvals) > 0 {
		have = reviewrule.Who(approvals) + " approved it"
	} else if inRun != 0 {
		have = fmt.Sprintf("Its own reviewer read it %d time(s) in the run without approving "+
			"it", inRun)
	} else {
		have = "No model has reviewed it yet"
	}
	need := reviewrule.Missing(tiersOf(approvals), difficulty)
	line := fmt.Sprintf("%s, so it waits for %s (`%s`) before auto-merge turns on.", have, need, config.LabelCross)
	if !x.reviewerSetUp(votes, difficulty) {
		line += " No subscription that could give that review is set up, so it waits for " +
			"you to merge it, or for one to be set up."
	}
	return line
}

// A head no model reviewed (someone pushed during the run, or no model here may
// review): its votes start empty.
func (x *Deliverer) voteReset(pr int, head string, builder string /* = "" */) *pyjson.Object {
	votes := pyjson.New("sha", head, "builder", builder, "approvals", []any{}, "rejections", []any{},
		"tiers", pyjson.New(), "reviews", []any{})
	x.remember(pr, pyjson.New("votes", votes))
	return votes
}

// Whether the set-up subscriptions could still give the reviews the head needs
// (`review_rule.reachable`).
func (x *Deliverer) reviewerSetUp(votes *pyjson.Object, difficulty string) bool {
	secrets := x.Cfg.Secrets
	var available []string
	for _, provider := range x.Cfg.Pool.Ordered() {
		if !provider.Enabled || !slices.Contains(provider.Roles, "review") {
			continue
		}
		if provider.Login != "machine" {
			if has := secrets.Has(provider.Secret); has != nil && !*has {
				continue
			}
		}
		for _, seat := range x.Cfg.Pool.OwnSeats(provider) {
			if !slices.Contains(available, seat.Tier) {
				available = append(available, seat.Tier)
			}
		}
	}
	return reviewrule.Reachable(tiersOf(x.approvals(votes)), available, difficulty)
}

// Carried is Deliverer._carried.
//
// The issue's labels its pull request starts with, approved or a draft: its difficulty,
// so the same models revise and review it, its priority tier (#90), and `person merges`
// (#839).
func (x *Deliverer) Carried(issue int) []string {
	out := []string{}
	for _, name := range sortedKeys(x.Labels(issue)) {
		lowered := strings.ToLower(name)
		if config.DifficultyLabels.Has(lowered) || isPriorityTier(lowered) || lowered == config.LabelPersonMerges {
			out = append(out, name)
		}
	}
	return out
}

// SecondReview is Deliverer._second_review: act on a review run's verdict on a bot pull
// request.
func (x *Deliverer) SecondReview(number int, status string) {
	record := x.Record(number)
	if status != "infra" && status != "interrupted" {
		// A review a person asked for is answered: the next one takes any tier again.
		x.remember(number, pyjson.New("review_floor", "", "review_notes", ""))
	}
	if status == "infra" {
		x.infra(number, "review", config.LabelCross)
		return
	}
	if status == "interrupted" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelCross)
		x.GH.CreateComment(number, fmt.Sprintf("The second review was cut short (%s): "+
			"%s. It stays queued.", x.link(), py.Str(x.Result.Get("reason"))))
		return
	}
	if status == "stopped" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.GH.CreateComment(number, "Stopped, as asked. It will not merge by itself.")
		return
	}
	if status != "reviewed" {
		x.failReview(number, record)
		return
	}
	head := x.branchHead(number)
	reviewed := py.OrStr(x.Result.Get("reviewed_sha"), "")
	who := x.Provider.Describe()
	if reviewed != head {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelCross)
		x.GH.CreateComment(number, fmt.Sprintf("The branch moved while %s reviewed it, so that "+
			"review does not count. It stays queued for a second review.", who))
		return
	}
	if x.Result.Get("verdict") == "approve" {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), "")
		x.remember(number, pyjson.New("cross_rounds", 0, "failures", 0))
		waiting := record.Truthy("pending_request")
		note := x.approved(number, head, !waiting, true, false, nil)
		if waiting {
			note = "A comment arrived during the review, so a revision answers it first."
		}
		x.GH.CreateComment(number, fmt.Sprintf("Second review (%s): %s approved it. %s", x.link(), who, note))
		return
	}
	// A rejection stands against this head until that model approves it or the head moves.
	x.Vote(number, head, false, false, nil)
	x.holdAutoMerge(number)
	first := x.firstFinding()
	if first == "" {
		first = "see its findings"
	}
	if x.strike(number, fmt.Sprintf("%s rejected it: %s", who, first)) {
		return
	}
	rounds := py.MustInt(record.GetOr("cross_rounds", 0)) + 1
	findings := py.Or(x.Result.Get("findings"), []any{})
	if rounds >= x.Cfg.MaxFailures {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.remember(number, pyjson.New("cross_rounds", rounds, "last_findings", findings))
		x.GH.CreateComment(number, fmt.Sprintf("Second review (%s): %s still found "+
			"blocking problems, after %d rounds between the models, "+
			"so I stopped. It needs a person.\n\n%s", x.link(), who, rounds, x.findingsMd()))
		return
	}
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelRevise)
	x.remember(number, pyjson.New("cross_rounds", rounds, "last_findings", findings, "source", "cross-review",
		"kind", "revise", "queued_at", clock.ISO(x.Ctx.Now()), "failures", 0,
		"audit_tier", x.auditTier()))
	x.GH.CreateComment(number, fmt.Sprintf("Second review (%s): %s found blocking "+
		"problems, so a revision is queued to answer them.\n\n"+
		"%s", x.link(), who, x.findingsMd()))
}

// The pull request's head as git sees it on origin now (the API can lag a push).
func (x *Deliverer) branchHead(number int) string {
	branch := py.OrStr(x.Plan.Get("branch"), "")
	if branch != "" {
		x.fetch(branch, false)
		if found := x.Repo.Rev("origin/" + branch); found != "" {
			return found
		}
	}
	return py.OrStr(x.GH.GetPull(number).Obj("head").Get("sha"), "")
}

func (x *Deliverer) failReview(number int, record *pyjson.Object) {
	fails := py.MustInt(record.GetOr("failures", 0)) + 1
	x.remember(number, pyjson.New("failures", fails))
	if fails >= x.Cfg.MaxFailures {
		queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelBlocked)
		x.GH.CreateComment(number, fmt.Sprintf("The second review failed %d times "+
			"(%s): %s. It needs a "+
			"person.", fails, x.link(), py.Str(x.Result.Get("reason"))))
		return
	}
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelCross)
	x.GH.CreateComment(number, fmt.Sprintf("The second review failed (%s): "+
		"%s. It stays queued.", x.link(), py.Str(x.Result.Get("reason"))))
}
