// Package deliver is bot/harness/deliver.py.
//
// The last job of a night run: check the model job's output and publish it.
//
// This job holds the bot's token and runs no model. It does not trust the model job's files: the
// item comes from the plan job's outputs and the branch is worked out again here, and the bundle
// must hold the head the result names, descend from the commit the work started at, change no
// forbidden path, and fast-forward the branch on GitHub.
//
// The review rule (`review_rule.py`): a commit ships when its lane's final audit approved it
// (one strong model for a medium or hard item, one medium model for an easy item), and no
// model's rejection of it stands. A weak approval never ships anything. Votes are
// kept per commit, one per review, with the tier of the seat that cast them (`_vote`), from the
// plan's seats, never from the model job. A pull request whose commit has that gets auto-merge,
// which merges it once the required CI checks pass; one still short of it waits in
// `bot:cross-review` for a review run.
//
// A commit that meets the rule is recorded as cleared (`_clear`). When `main` then leaves it with
// conflicts, the revision that resolves them carries the clearance (`_carry`) if it started from
// that commit, its own run's reviewer approved it, and git shows it changed nothing beyond the files
// the merge left conflicted; it then ships on that one approval, with no review run.
package deliver

import (
	"fmt"
	"math"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	planmod "github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// ReportChars is REPORT_CHARS.
var ReportChars = 30_000

// How much of a stopped run's notes and session trail is kept for the next agent.
var (
	HandoffNotes = 8000
	HandoffTrail = 6000
)

// DiedLimit is DIED_LIMIT: runs that die without a result on one item before it is blocked.
var DiedLimit = 2

// VaultAttempts is VAULT_ATTEMPTS: writes of a refreshed login tried before giving up (another
// writer raced it).
var VaultAttempts = 4

// NoResult is NO_RESULT.
var NoResult = "the model job left no result: it failed before the model started (the doctor " +
	"step, the install or the CLI setup), or it was cancelled"

// A label a split's sub-issue may not be given from the model's answer: a bot's own, or one
// the harness or a person sets (`human`, a difficulty, a priority, a method).
func reservedLabel(name string) bool {
	lowered := strings.ToLower(name)
	build := config.LabelBuild
	if i := strings.LastIndex(build, ":"); i >= 0 {
		build = build[:i]
	}
	prefixes := []string{build + ":"}
	for _, o := range config.Others {
		prefixes = append(prefixes, o.LabelPrefix)
	}
	prefixes = append(prefixes, "difficulty:", "priority:", "method:")
	if lowered == config.LabelHuman {
		return true
	}
	for _, p := range prefixes {
		if strings.HasPrefix(lowered, p) {
			return true
		}
	}
	return false
}

// `text` cut to `limit` characters: its beginning (a plan) or its end (a builder's log).
func cut(text string, limit int, head bool) string {
	if head {
		return py.SliceTo(text, limit)
	}
	return py.SliceFrom(text, -limit)
}

// LoadResult is deliver.load_result.
func LoadResult(outDir string) *pyjson.Object {
	path := filepath.Join(outDir, "result.json")
	// read_text raises UnicodeDecodeError, a ValueError, on a file that is not UTF-8.
	if data, err := os.ReadFile(path); err == nil && utf8.Valid(data) {
		if v, err := pyjson.Loads(data); err == nil {
			if o, ok := v.(*pyjson.Object); ok {
				return o
			}
		}
	}
	return pyjson.New("status", "infra", "reason", NoResult, "no_result", true)
}

// Deliverer is deliver.Deliverer.
type Deliverer struct {
	Ctx      *hctx.Context
	Cfg      *config.Config
	GH       gh.Client
	Plan     *pyjson.Object
	Provider *providers.Provider
	// The seats that built and reviewed in the run, from the plan's outputs: a vote's tier
	// comes from here, never from the model job's result.
	BuildSeat  *providers.Seat
	ReviewSeat *providers.Seat
	OutDir     string
	Repo       *git.Git
	Result     *pyjson.Object
	Log        []string
	Chain      bool
	// RatingNote is `self.rating_note`: the sentence `_item` had from `_apply_rating`, "" before.
	RatingNote string
}

// NewDeliverer is Deliverer(...).
func NewDeliverer(ctx *hctx.Context, plan *pyjson.Object, outDir string, repoDir string, action string /* = None */, number *int /* = None */, provider string /* = None */) *Deliverer {
	x := &Deliverer{Ctx: ctx, Cfg: ctx.Cfg, GH: ctx.GH, Plan: plan.Copy()}
	if action != "" {
		x.Plan.Set("action", action)
	}
	if number != nil && *number != 0 {
		x.Plan.Set("number", *number)
	}
	if provider != "" {
		x.Plan.Set("provider", provider)
	}
	pool := ctx.Cfg.Pool
	x.Provider = pool.Get(py.OrStr(x.Plan.Get("provider"), ""))
	if x.Provider == nil {
		x.Provider = pool.Get(providers.LegacyProvider)
	}
	if x.Provider == nil {
		x.Provider = pool.Ordered()[0]
	}
	seats := x.Plan.Obj("seats")
	main := pool.Seats(x.Provider)[0]

	seat := func(role string) *providers.Seat {
		entry := seats.Obj(role)
		if entry.Len() == 0 {
			return nil
		}
		return pool.Seat(x.Provider.ID, py.OrStr(entry.Get("model"), ""))
	}
	x.BuildSeat = seat("build")
	if x.BuildSeat == nil {
		x.BuildSeat = main
	}
	if seats.Len() > 0 {
		x.ReviewSeat = seat("review")
	} else {
		x.ReviewSeat = main
	}
	x.OutDir = outDir
	x.Repo = git.NewGit(repoDir, nil)
	x.Result = LoadResult(outDir)
	x.Log = []string{}
	x.Chain = true
	return x
}

func (x *Deliverer) log(line string) { x.Log = append(x.Log, line) }

// ------------------------------------------------------------------ entry

// Run is Deliverer.run.
func (x *Deliverer) Run() *pyjson.Object {
	action := x.Plan.Get("action")
	usage, resetAt := x.Result.Get("usage"), x.Result.Get("reset_at")
	// The refreshed login first: a rotated one that is lost cannot be recovered.
	x.storeVault()
	minutes, _ := py.Float(py.Or(x.Result.Get("minutes"), 0))
	tokens := pyjson.Obj(x.Result.Get("tokens"))
	cost, _ := py.Float(py.Or(x.Result.Get("cost_usd"), 0))
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		var spent *float64
		if cost > 0 {
			spent = &cost
		}
		providers.NoteUsage(s, x.Provider.ID, pyjson.Obj(usage), py.OrStr(resetAt, ""), x.Ctx.Now(),
			minutes, tokens, spent)
		return nil
	}, "usage "+x.Provider.ID)
	x.machineDisk()
	x.machineMemory()
	x.loginWorks()
	switch {
	case action == "suggest":
		x.Suggestions()
	case action == "plan" || action == "build" || action == "revise" || action == "review":
		x.rederive()
		x.Item()
	default:
		return pyjson.New("status", "nothing", "log", []any{"nothing was planned"})
	}
	x.Chain_()
	return pyjson.New("status", x.Result.Get("status"), "log", pyjson.ListOf(x.Log))
}

// Keep a login the CLI refreshed during the run (`vault.py`) for this provider's next
// run. The model job sealed it; this job cannot open it and only checks its shape.
func (x *Deliverer) storeVault() {
	path := filepath.Join(x.OutDir, "vault.enc")
	if !isFile(path) {
		return
	}
	text, err := py.ReadText(path)
	herr.Must(err)
	text = strings.TrimSpace(text)
	if !vault.LooksSealed(text) {
		x.log("ignored a vault that is not sealed")
		return
	}
	where := planmod.VaultPath(x.Provider.ID)
	problem := ""
	for range VaultAttempts {
		kept := false
		exc, raised := herr.Catch[*herr.GitHubError](func() {
			_, sha := x.GH.GetFile(where, config.StateBranch)
			x.GH.PutFile(where, text+"\n", config.StateBranch, sha, fmt.Sprintf("vault: %s [skip ci]", x.Provider.ID))
			x.log("kept the refreshed login of " + x.Provider.ID)
			kept = true
		})
		if kept {
			return
		}
		if raised {
			problem = exc.Error()
			if exc.Status != 409 && exc.Status != 422 {
				break // only a race with another writer is worth trying again
			}
		}
	}
	x.log(fmt.Sprintf("could not keep the refreshed login of %s: %s", x.Provider.ID, problem))
}

// A job on the bot's machine read its disk: keep the newest reading, and open, rewrite
// or close the issue about a disk filling up (`disk.py`).
func (x *Deliverer) machineDisk() {
	if providers.Hosted(x.Provider.RunsOn) {
		return
	}
	found := disk.Newest(x.Result)
	if found == nil {
		return
	}
	reported := py.OrStr(x.Result.Obj("disk").Get("runner"), "")
	runner := x.Provider.RunsOn
	if runnerName.MatchString(reported) {
		runner = reported
	}
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		disk.Note(s, found, runner, x.Cfg.RunURL())
		return nil
	}, "machine disk")
	var done string
	if exc, ok := herr.Catch[*herr.GitHubError](func() { done = disk.Alert(x.Ctx) }); ok {
		done = "could not settle the disk issue: " + exc.Error()
	}
	if done != "" {
		x.log(done)
	}
}

// A job on the bot's machine read its memory: keep its summary for the status issue's
// line on the machine's memory (`memory.py`, #312).
func (x *Deliverer) machineMemory() {
	if providers.Hosted(x.Provider.RunsOn) {
		return
	}
	found := memory.Summary(pyjson.Obj(x.Result.Get("memory")))
	if found == nil {
		return
	}
	reported := py.OrStr(x.Result.Obj("disk").Get("runner"), "")
	label := py.OrStr(x.Plan.Get("runs_on"), x.Provider.RunsOn) // a fill-in's too
	runner := label
	if runnerName.MatchString(reported) {
		runner = reported
	}
	box := memory.BoxOf(runner, label)
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		memory.Note(s, found, runner, box)
		return nil
	}, "machine memory")
}

// A model call went through on this run's subscription: any streak of runs that could
// not work there is over (`providers.clear_infra`), and the issue asking a person about it,
// if one was opened, is closed.
func (x *Deliverer) loginWorks() {
	calls, _ := py.Int(py.Or(x.Result.Get("model_calls"), 0))
	if x.Result.Get("status") == "infra" || calls == 0 {
		return
	}
	if _, isDict := providers.PeekRecord(x.Ctx.Store.Load(), x.Provider.ID).Get("infra").(*pyjson.Object); !isDict {
		return
	}
	var opened []int
	// As in the Python, a retried write runs this again, and each run adds what it cleared.
	x.Ctx.Store.Update(func(s *pyjson.Object) any {
		if n := providers.ClearInfra(s, x.Provider.ID); n != nil && *n != 0 {
			opened = append(opened, *n)
		}
		return nil
	}, x.Provider.ID+" works again")
	for _, number := range opened {
		x.try(func() {
			x.GH.CreateComment(number, fmt.Sprintf("`%s` works again: a run got a model call through "+
				"(%s). Closing this.", x.Provider.ID, x.link()))
		})
		x.try(func() { x.GH.UpdateIssue(number, pyjson.New("state", "closed")) })
		x.log(fmt.Sprintf("%s works again: closed #%d", x.Provider.ID, number))
	}
}

// After `providers.INFRA_ASK_AFTER` runs in a row that could not work on this
// subscription, open one issue asking a person to fix it (the bot cannot: a login is a
// secret, and `.harness/` is not the bot's to change).
func (x *Deliverer) askAboutInfra(streak int, reason string) {
	if streak < providers.InfraAskAfter {
		return
	}
	record := providers.PeekRecord(x.Ctx.Store.Load(), x.Provider.ID)
	if record.Obj("infra").Truthy("issue") {
		return
	}
	backoff := providers.InfraBackoffs[len(providers.InfraBackoffs)-1]
	hours := int(math.Floor(backoff.Seconds() / 3600))
	login := x.Provider.Secret
	if login == "" {
		login = "its login on the machine"
	}
	id := x.Provider.ID
	body := fmt.Sprintf("The last %d runs on `%s` (%s, `%s`) could not work. The last one (%s) said:\n\n",
		streak, id, x.Provider.CLI, x.Provider.Model, x.link()) +
		fmt.Sprintf("> %s\n\nUntil a run on it gets a model call through, unforced runs ", py.SliceTo(reason, 500)) +
		fmt.Sprintf("try it only once every %d hours, and the other subscriptions take its ", hours) +
		"work.\n\nTo fix it, a person (the bot cannot change secrets or `.harness/`):\n" +
		fmt.Sprintf("- renews its login (`%s`), if the reason is a refused or expired login;\n", login) +
		"- or switches it off (`\"enabled\": false` in `.harness/providers.json`).\n\n" +
		fmt.Sprintf("The bot closes this issue by itself once a run on `%s` works ", id) +
		"again."
	var issue *pyjson.Object
	if exc, ok := herr.Catch[*herr.GitHubError](func() {
		issue = x.GH.CreateIssue(fmt.Sprintf("Night bot: `%s` could not work %d times in a row", id, streak),
			body, []string{"night bot", config.LabelHuman})
	}); ok {
		x.log(fmt.Sprintf("could not open an issue about %s: %s", id, exc.Error()))
		return
	}
	number := py.MustInt(py.Or(issue.Get("number"), 0))
	if number != 0 {
		x.Ctx.Store.Update(func(s *pyjson.Object) any {
			infra := providers.Record(s, id).SetDefault("infra", pyjson.New())
			pyjson.Obj(infra).Set("issue", number)
			return nil
		}, id+" infra issue")
		x.log(fmt.Sprintf("%s could not work %d times: opened #%d", id, streak, number))
	}
}

// Work the branch out again from GitHub, whatever the plan file says.
func (x *Deliverer) rederive() {
	number := py.MustInt(x.Plan.Get("number"))
	if action := x.Plan.Get("action"); action == "plan" || action == "build" {
		x.Plan.Set("branch", queue.BranchForIssue(number))
		// Where a fullsend part lands (#505) comes from the state file, which deliver wrote
		// when it opened the part, never from the plan file alone.
		record := x.Record(number)
		parent := py.MustInt(py.Or(record.Get("part_of"), 0))
		x.Plan.Pop("onto")
		x.Plan.Pop("part_of")
		if parent != 0 && record.Get("onto") == queue.BranchForIssue(parent) {
			x.Plan.Set("onto", queue.BranchForIssue(parent))
			x.Plan.Set("part_of", parent)
		}
		return
	}
	pull := x.GH.GetPull(number)
	head := pull.Obj("head")
	if head.Obj("repo").Get("full_name") != x.Cfg.Repo {
		herr.Raise(&herr.GitHubError{Msg: fmt.Sprintf("#%d is not a branch of %s", number, x.Cfg.Repo), Status: 0})
	}
	x.Plan.Set("branch", head.GetOr("ref", ""))
	x.Plan.Set("bot_pr", queue.LabelNames(pull)[config.LabelPR])
}
