package usage

import (
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The usage issue (#2749, section 2): **Night bot usage**, which the status loop draws from the
// usage history at each flush (`Recorder.draw`, every 30 minutes with the statistics, never on the
// five-minute tick), so the limits in `.harness/providers.json` can be tuned from evidence. The
// loop opens it the first time, finds it again by its marker as it finds the status issue
// (`dashboard.Find`), keeps its number in the state file (`dashboard.UsageIssueKey`), labels it as
// the status issue is labelled (`night bot` and `tracking`), and pins it when a pin is free. It
// rewrites only an issue the state file names, or one with its title and marker opened by the bot
// or someone on the trust list (`dashboard.Trusted`). Sixteen sections, each folded under its
// heading (`dashboard.Fold`), with mermaid bars and lines, never pies, kept under GitHub's limit on
// an issue's body (`dashboard.MaxBody`) by `Fit`. #148 links to it (`dashboard.Render`) and is
// otherwise unchanged. Squishy draws none: it keeps no usage history.

// IssueTitle and IssueMarker are the usage issue's title and the marker it is found again by.
var (
	IssueTitle  = "Night bot usage"
	IssueMarker = "<!-- jackioh-bot:usage -->"
)

// IssueLabels is the usage issue's labels: the status issue's (`dashboard.Labels`: `night bot`
// and `tracking`, which keeps every queue path off it).
var IssueLabels = dashboard.Labels

// branchName is the usage branch: the identity's `usage_branch`.
func branchName() string { return config.Identity.UsageBranch }

// Draw draws the usage issue from the history under `root`, a clone of the usage branch, at each
// flush (`Recorder.draw`); it says what it did. A variable the tests replace.
var Draw = DrawIssue

// DrawIssue draws the usage issue from the history under `root` and writes it: opened (and its
// pin tried) the first time, rewritten after; its number and whether it is pinned kept in the
// state file (`dashboard.UsageIssueKey`), and a pin that failed tried again at the next drawing
// (a pin frees when a person unpins an issue). A bot that keeps no history draws nothing ("").
// Raises when the files cannot be read; the recorder logs it and pushes all the same.
func DrawIssue(ctx *hctx.Context, root string) string {
	if !ctx.Cfg.RecordsUsage() {
		return ""
	}
	facts, err := ReadFacts(root, ctx.Now())
	herr.Must(err)
	body := Fit(ctx, facts)
	state := ctx.Store.Load()
	record := state.Obj(dashboard.UsageIssueKey)
	pinned := record.Truthy("pinned")
	issue := FindIssue(ctx, state)
	var number int
	var done string
	if issue == nil {
		issue = ctx.GH.CreateIssue(IssueTitle, body, IssueLabels)
		number = py.MustInt(issue.Get("number"))
		done, pinned = fmt.Sprintf("opened #%d", number), false
	} else {
		number = py.MustInt(issue.Get("number"))
		if !pyjson.Equal(record.Get("issue"), number) {
			pinned = false // adopted: its pin is not known
		}
		labelled(ctx, issue)
		ctx.GH.UpdateIssue(number, pyjson.New("body", body))
		done = fmt.Sprintf("rewrote #%d", number)
	}
	if node := py.OrStr(issue.Get("node_id"), ""); !pinned && node != "" {
		if exc, failed := herr.Catch[*herr.GitHubError](func() { ctx.GH.PinIssue(node) }); failed {
			done += fmt.Sprintf("; could not pin it (%d): no pin is free, so the next drawing tries again", exc.Status)
		} else {
			pinned = true
			done += " and pinned it"
		}
	}
	if !pyjson.Equal(record.Get("issue"), number) || record.Truthy("pinned") != pinned {
		ctx.Store.Update(func(s *pyjson.Object) any {
			s.Set(dashboard.UsageIssueKey, pyjson.New("issue", number, "pinned", pinned))
			return nil
		}, "usage issue")
	}
	return done
}

// FindIssue is the open usage issue: the one the state file names, or one to adopt, with its
// title and marker, opened by the bot or someone on the trust list (as `dashboard.Find` finds the
// status issue).
func FindIssue(ctx *hctx.Context, state *pyjson.Object) *pyjson.Object {
	if number := state.Obj(dashboard.UsageIssueKey).Get("issue"); pyjson.Truthy(number) {
		var named *pyjson.Object
		herr.Catch[*herr.GitHubError](func() {
			issue := ctx.GH.GetIssue(py.MustInt(number))
			if pyjson.Equal(issue.Get("state"), "open") && !issue.Has("pull_request") {
				named = issue
			}
		})
		if named != nil {
			return named
		}
	}
	for _, issue := range ctx.GH.ListIssues("", "open", 300, "") {
		if !issue.Has("pull_request") && pyjson.Equal(issue.Get("title"), IssueTitle) &&
			strings.Contains(py.OrStr(issue.Get("body"), ""), IssueMarker) && dashboard.Trusted(ctx, issue) {
			return issue
		}
	}
	return nil
}

// labelled puts on the usage issue any of `IssueLabels` it lacks (one a person took off).
func labelled(ctx *hctx.Context, issue *pyjson.Object) {
	names := map[string]bool{}
	for _, label := range pyjson.Objs(issue.GetOr("labels", []any{})) {
		names[py.OrStr(label.Get("name"), "")] = true
	}
	var missing []string
	for _, label := range IssueLabels {
		if !names[label] {
			missing = append(missing, label)
		}
	}
	if len(missing) > 0 {
		ctx.GH.AddLabels(py.MustInt(issue.Get("number")), missing)
	}
}

// Fit is the issue's body at the most detail that keeps under GitHub's limit on an issue's body
// (`dashboard.MaxBody`, #148's): each of `Shrinks` in turn, then, past the last, whole sections cut
// from the bottom (`Cuts`: the suggestions and the raw data stay), saying so in a line.
func Fit(ctx *hctx.Context, f *Facts) string {
	var body string
	for _, detail := range Shrinks {
		if body = Render(ctx, f, detail, 0); py.Len(body) <= dashboard.MaxBody {
			return body
		}
	}
	last := Shrinks[len(Shrinks)-1]
	for cut := 1; cut <= len(Cuts()); cut++ {
		if body = Render(ctx, f, last, cut); py.Len(body) <= dashboard.MaxBody {
			return body
		}
	}
	return body
}

// Render is the issue's body at `detail`, with the first `cut` of `Cuts` left out: its date, its
// title, a line on what it is, which shrinks it took, then each section under its heading, folded,
// and its marker.
func Render(ctx *hctx.Context, f *Facts, detail Detail, cut int) string {
	d := newDrawing(ctx, f, detail)
	cfg := ctx.Cfg
	lines := []string{"**Last drawn at " + dashboard.Local(ctx, f.Now).Format("2006-01-02 15:04 MST") + "**", "",
		"# 📈 " + IssueTitle, "",
		note(fmt.Sprintf("`bot-status` draws this issue every %d minutes from the CSV files on "+
			"[`%s`](%s/%s/tree/%s/%s) ([the usage issue](%s-issue)): how each subscription's limits in "+
			"`.harness/providers.json` work out over time, and what they suggest. Hours are the bot's time zone's "+
			"(%s). Nothing here changes a limit: a person edits providers.json. #148, the status issue, has "+
			"the usage now.", int(FlushEvery.Minutes()), branchName(), cfg.ServerURL, cfg.Repo, branchName(), Dir,
			d.readme(), cfg.Timezone)), ""}
	if i := slices.IndexFunc(Shrinks, func(x Detail) bool { return x.Name == detail.Name }); i > 0 {
		var steps []string
		for _, x := range Shrinks[1 : i+1] {
			steps = append(steps, x.Name)
		}
		lines = append(lines, "> Shrunk to keep under GitHub's limit on an issue's body: "+strings.Join(steps, "; ")+".", "")
	}
	cuts := Cuts()[:min(max(cut, 0), len(Cuts()))]
	if len(cuts) > 0 {
		names := make([]string, len(cuts))
		for i, n := range cuts {
			names[i] = fmt.Sprint(n)
		}
		lines = append(lines, "> Left out to keep under GitHub's limit on an issue's body: section "+
			joinAnd(names)+". The raw data (section 16) has every row.", "")
	}
	for _, s := range sections {
		if slices.Contains(cuts, s.n) {
			continue
		}
		lines = append(lines, fmt.Sprintf("## %d. %s", s.n, s.heading), "")
		lines = append(lines, dashboard.Fold(s.summary, s.draw(d))...)
	}
	return strings.Join(append(lines, IssueMarker), "\n")
}
