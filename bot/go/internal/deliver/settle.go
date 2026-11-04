package deliver

// Run 2's final check, delivered (#2948, section 2; `work/settle.go` runs it).
//
// The work job test-merged the approved draft with `main` (merging and resolving `main` when it
// conflicted, which its reviewer then approved) and after each open pull request that overlaps it,
// and read its CI again. Here, with the state file:
//
//   - a red CI it fixed in the run counts toward `max_failures` as a CI fix does (`countCIFix`);
//   - the pull requests ahead of it (`ahead`) are those of its test-merges that conflicted, approved
//     before it at the head they have (`cleared`), conflict-free with `main`, open, and not waiting
//     for a person with their CI green. With one, it waits its turn (`waitTurn`): still a draft,
//     approved (its votes and clearance recorded, auto-merge off), `bot:waiting-turn`, and
//     `waiting_turn` on its record for `plan.SyncTurns`, at most `hold_minutes`. With none, it goes
//     ready and auto-merge turns on, `main` unmerged when it merged cleanly.
//
// When the work job could not read the open pull requests, deliver test-merges them itself.

import (
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/settle"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stepup"
)

// settled is the run's final check (its result's `settle`), or nil when it made none.
func (x *Deliverer) settled() *pyjson.Object { return x.Result.Obj("settle") }

// countCIFix counts a red CI the final check fixed in the run toward `max_failures` (the record's
// `ci_fixes`), and makes the head it pushed the CI fix's (`ci_fixed_head`: CI red on it again is a
// strike, `events.fixCI`).
func (x *Deliverer) countCIFix(number int, head string, pushed bool) {
	if !x.settled().Obj("ci").Truthy("fixed") {
		return
	}
	fields := pyjson.New("ci_fixes", py.MustInt(py.Or(x.Record(number).Get("ci_fixes"), 0))+1)
	if pushed {
		fields.Set("ci_fixed_head", head)
	}
	x.remember(number, fields)
}

// ahead is the pull requests ahead of `number` (at `head`) that it conflicts with once they land
// (the package's rules above), in its test-merges' order.
func (x *Deliverer) ahead(number int, head string) []int {
	found := x.settled()
	if found == nil {
		return nil
	}
	var pulls []*settle.Pull
	if found.Has("pulls") && py.OrStr(found.Get("pulls_read"), "") == "" {
		for _, entry := range pyjson.Objs(found.Get("pulls")) {
			pulls = append(pulls, settle.FromDict(entry))
		}
	} else {
		// The work job could not read them: test-merge them here, with the write token's client.
		x.fetch(x.Cfg.DefaultBranch, false)
		if exc := herr.Try(func() {
			pulls, _ = settle.Pulls(x.Repo, x.GH, number, head, x.Cfg.DefaultBranch,
				git.IdentityBot(x.Cfg.BotLogin, x.Cfg.BotUserID))
		}); exc != nil {
			x.log("could not test-merge the open pull requests: " + py.SliceTo(herr.Str(exc), 200))
		}
	}
	state := x.Ctx.Store.Load()
	now := x.Ctx.Now()
	var out []int
	for _, p := range pulls {
		if !p.Conflicted() {
			continue
		}
		record := state.Obj("items").Obj(strconv.Itoa(p.Number))
		cleared := queue.Cleared(record)
		at := clock.ParseISOValue(cleared.Get("at"))
		if !pyjson.Equal(cleared.Get("sha"), p.Head) || at.IsZero() || !at.Before(now) {
			continue // not approved at the head it has, or not before this one
		}
		var pull *pyjson.Object
		if _, failed := herr.Catch[*herr.GitHubError](func() { pull = x.GH.GetPull(p.Number) }); failed ||
			!pyjson.Equal(pull.Get("state"), "open") {
			continue
		}
		names := queue.LabelNames(pull)
		if names[config.LabelBlocked] || queue.IsHuman(names) {
			continue // it waits for a person to unblock it, not for a lane
		}
		if queue.PersonMerges(names) && (queue.GaveUp(record, p.Head) || stepup.Green(x.Ctx, p.Head)) {
			continue // green, and waiting for a person to merge it: it gives up its precedence
		}
		out = append(out, p.Number)
	}
	return out
}

// waitTurn keeps approved draft `number` (at `head`) waiting its turn behind `ahead`: its record's
// `waiting_turn` (the subscription and the model that approved it, since when, until when at most,
// the pull requests it waits behind, the head) and `bot:waiting-turn`. Returns the comment's line.
func (x *Deliverer) waitTurn(number int, head string, ahead []int) string {
	now := x.Ctx.Now()
	ends := now.Add(time.Duration(x.Cfg.HoldMinutes) * time.Minute)
	model := x.headModel()
	behind := make([]any, len(ahead))
	refs := make([]string, len(ahead))
	for i, n := range ahead {
		behind[i] = n
		refs[i] = fmt.Sprintf("#%d", n)
	}
	x.remember(number, pyjson.New(queue.WaitingTurnKey, pyjson.New("provider", x.Provider.ID, "model", model,
		"since", clock.ISO(now), "ends", clock.ISO(ends), "behind", behind, "head", head,
		"from_run", x.Cfg.RunID)))
	queue.SetStateLabel(x.Ctx, number, x.Labels(number), config.LabelWaitingTurn)
	x.log(fmt.Sprintf("#%d waits its turn behind %s until %s at most", number, strings.Join(refs, ", "),
		clock.ISO(ends)))
	return fmt.Sprintf("Its final check test-merged it after the open pull requests that change the same "+
		"files: it conflicts once %s land(s), approved before it and conflict-free, so it waits its turn "+
		"(`%s`), a draft, with auto-merge off. When that merges, `%s` takes it up again on `%s` in a "+
		"fresh session, merges `main` and resolves the conflict, and it goes ready; if it has not merged "+
		"by %s (`hold_minutes`: %d), this goes ahead anyway.", strings.Join(refs, ", "),
		config.LabelWaitingTurn, x.Provider.ID, model, planmod.Clock(x.Ctx, ends), x.Cfg.HoldMinutes)
}

// settleSaid is what the approval's comment says of the final check's merge of `main` and its CI
// fix, "" when it made neither.
func (x *Deliverer) settleSaid() string {
	found := x.settled()
	var said []string
	if main := found.Obj("main"); pyjson.Equal(main.Get("state"), "merged") {
		var conflicts []string
		for _, p := range pyjson.List(main.Get("conflicts")) {
			conflicts = append(conflicts, py.Str(p))
		}
		said = append(said, fmt.Sprintf("Its final check found it conflicts with `main` (in %s), so it "+
			"merged `main` once, resolved it, ran the checks again, and its reviewer approved the merge.",
			shownPaths(conflicts)))
	} else if pyjson.Equal(main.Get("state"), "clean") {
		said = append(said, "Its final check test-merged it with `main`: clean, so `main` is not merged "+
			"in (the merge queue tests the two together).")
	}
	if ci := found.Obj("ci"); ci.Truthy("fixed") {
		said = append(said, fmt.Sprintf("Its CI, read again before it goes ready, was red (%s): it fixed "+
			"that in this run, which counts as a CI fix.", ticked(pyjson.Strings(ci.Get("red")))))
	}
	return strings.Join(said, " ")
}
