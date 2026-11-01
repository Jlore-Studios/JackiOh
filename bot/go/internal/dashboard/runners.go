package dashboard

// ------------------------------------------------------------------ every runner

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/memory"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
)

// RunnersMinutes is RUNNERS_MINUTES: how often the `bot-status` loop redraws the runners section
// alone, between its rewrites (`RUNNERS_EVERY_SECONDS` in bot-status.yml, `refresh_runners`).
var RunnersMinutes = 2

// RunnersStart and RunnersEnd are RUNNERS_START and RUNNERS_END.
var (
	RunnersStart = "<!-- jackioh-bot:runners -->"
	RunnersEnd   = "<!-- /jackioh-bot:runners -->"
)

// Explained is EXPLAINED: marks the comment on the status issue that says what each runner is for
// (`explain`): it changes only with the pool or the workflows, so it is kept out of the body, which
// is full.
var Explained = "<!-- jackioh-bot:runners-explained -->"

// HostedRunners is HOSTED_RUNNERS: GitHub's runners the organization may hold at once, its every
// repository's jobs together (the Free plan's 20: selfplay.yml's header, #1455).
var HostedRunners = 20

// RunnerRuns is RUNNER_RUNS: the runs going or queued whose jobs the section reads, the oldest
// first: a call each, which costs no rate limit while the run's jobs are as they were at the last
// read (`gh._CACHED`).
var RunnerRuns = 40

// RunnerRows is RUNNER_ROWS: how many busy runners the table lists at most; fewer when the issue
// has no room for them.
var RunnerRows = 25

// LocalLabel and GatherLabel are LOCAL_LABEL and GATHER_LABEL: the owner's PC's own label, and its
// gather slot's, outside the PC's one job at a time (bot/machine/README.md, The local runner;
// #2134).
var (
	LocalLabel  = "jackioh-selfplay-local"
	GatherLabel = "jackioh-selfplay-local-gather"
)

// Workflows is WORKFLOWS: what each workflow is for, by file. A workflow added without a line here
// fails `bot selftest` (test_dashboard.RunnersTests).
var Workflows = map[string]string{
	"bot-cache.yml":    "the night bot's Rust build cache, built from main for its jobs on GitHub's",
	"bot-commands.yml": "the night bot's ears: commands, labels, reviews, CI results; its sweep",
	"bot-night.yml": "the night bot: gate, plan, `work` (one item planned, built, checked, " +
		"reviewed), deliver",
	"bot-status.yml": "this issue's loop: rewrites it, sweeps, plans and starts night runs, " +
		"delivers them",
	"ci-duration.yml": "after CI, an issue for any job over seven minutes",
	"ci.yml":          "CI: the five required checks on a pull request; everything on a push to main",
	"claude-wake.yml": "starts each Claude account's 5-hour session every 4 hours from 06:00 " +
		"Central",
	"code-audit.yml":         "every three days: the code audit's issue for the night bot",
	"db-size.yml":            "twice a day: an issue once production's database nears its Supabase plan's cap",
	"deploy-watch.yml":       "asks Render to deploy each push to main; an issue if it never arrives",
	"gather.yml":             "each new patch's dev games on the owner's PC, filed as totals (GATHER_ON_PATCH)",
	"hygiene-delete.yml":     "deletes the branches a person ticked in the hygiene issue",
	"hygiene.yml":            "weekly: the hygiene issues for Squishy",
	"learn-set.yml":          "a new set stops a learn export still early on the set before",
	"learn.yml":              "the learn lane: self-play, retrain and gate of the learned evaluation, back to back",
	"lock-conversations.yml": "locks every conversation to collaborators",
	"patches-ship.yml":       "ships the pending card-patch fragments after a merge",
	"pr-title.yml":           "holds a pull request's title to the convention",
	"promote-production.yml": "merges main into production once a day",
	"releases.yml":           "a GitHub release per shipped patch, a pre-release while only staging has it",
	"selfplay-shards.yml":    "one round of a self-play export's shards",
	"selfplay.yml":           "a self-play export, on the owner's PC unless asked otherwise",
	"spec-fold.yml":          "folds the pending §11 paragraphs and rewrites the rulings index once a day",
	"squishy-commands.yml":   "Squishy's ears and its sweep",
	"squishy-run.yml":        "Squishy: gate, plan, `work` on its own Claude account, deliver",
	"super.yml":              "the daily super run: 10,000 fuzz seeds, the full gates, every e2e spec, the db",
	"sweep.yml":              "sweeps the shadow bans again when the AI's generation moves",
	"triage.yml":             "labels each new issue and pull request from fixed rules",
	"voice.yml":              "renders a branch's voice lines (macOS `say`, Windows SAPI), checked on Linux",
	"winrates-refresh.yml":   "daily: the Tech decks' win-rate table, once a patch",
}

// HostedRoles is HOSTED_ROLES: GitHub's runners and the owner's PC, and what each is for, as
// (label, where, what it runs); the bot's machine's are drawn from the pool (`explanation`).
var HostedRoles = [][3]string{
	{"ubuntu-latest", "GitHub's Linux: a fresh machine each job, four vCPUs",
		"nearly every job: CI, every bot job that holds a write token (gate, plan, deliver, the " +
			"sweeps, this issue's loop), the Claude accounts' and Squishy's model jobs, every other " +
			"workflow"},
	{"macos-15", "GitHub's macOS", "`voice`'s macOS `say` lines"},
	{"windows-2022", "GitHub's Windows", "`voice`'s Windows SAPI lines"},
}

// LocalRoles is LOCAL_ROLES.
var LocalRoles = [][3]string{
	{LocalLabel, "the owner's PC: a container a job, `pc-selfplay-<time>`, only while the PC is " +
		"otherwise idle", "`learn`, every job of it, back to back, and `selfplay` exports"},
	{GatherLabel, "the owner's PC, a slot of its own beside the learn lane's, ahead of it for the " +
		"CPU: a container a job, `pc-gather-<time>`, only while the PC is otherwise idle", "`gather`"},
}

// BoxesNote is BOXES_NOTE. A variable the tests replace.
var BoxesNote = "The night box and the Muse box (a runner name ending `-b2`) are the bot's two AWS " +
	"machines, 2 vCPUs and 8 GB each; each starts when one of its jobs queues and stops " +
	"after 30 idle minutes. A lane past a subscription's first is `-2`, `-3` …. A " +
	"subscription logged in on the owner's PC may take its job there too " +
	"(`pc-<id>-<time>`). The PC's supervisor sends a heartbeat every two minutes (its " +
	"CPU and memory in use, with and without its runners, and each runner's CPUs and " +
	"whether it is slowed), which the PC's group in the runners section and the learn " +
	"lane show with whether it is working, the group with the gather workflow's state. " +
	"bot/machine/README.md has the rest."

// Explanation is explanation: the comment that says what each runner is for: GitHub's, each
// subscription's on the bot's machine (from the pool, so a subscription added there shows), and
// the owner's PC; then what each workflow is for.
func Explanation(ctx *hctx.Context) string {
	pool := ctx.Cfg.Pool
	var rows []string
	for _, role := range HostedRoles {
		rows = append(rows, "| `"+role[0]+"` | "+role[1]+" | "+role[2]+" |")
	}
	for _, provider := range pool.Ordered() {
		if !pool.OnMachine(provider.ID) {
			continue
		}
		lanes := "1 lane"
		if provider.Lanes > 1 {
			lanes = fmt.Sprintf("%d lanes", provider.Lanes)
		}
		rows = append(rows, "| `"+provider.RunsOn+"` | the bot's AWS boxes, "+lanes+" | `bot-night`'s "+
			"`work` for `"+provider.ID+"`: the `"+provider.CLI+"` CLI with "+
			"`"+provider.Model+"`, as the Linux user `agent-"+provider.ID+"`, which holds "+
			"that login alone |")
		if provider.FillIn != nil {
			var of []string
			for _, i := range provider.FillIn.Of {
				of = append(of, "`"+i+"`")
			}
			rows = append(rows, "| `"+provider.FillIn.RunsOn+"` | the bot's AWS boxes, in the lanes "+
				"of "+strings.Join(of, ", ")+" | `"+provider.ID+"`'s `work` in those lanes while they "+
				"cannot start work (capped, off, backing off), as the same user |")
		}
	}
	for _, role := range LocalRoles {
		rows = append(rows, "| `"+role[0]+"` | "+role[1]+" | "+role[2]+" |")
	}
	names := make([]string, 0, len(Workflows))
	for name := range Workflows {
		names = append(names, name)
	}
	sort.Strings(names)
	var flows []string
	for _, name := range names {
		flows = append(flows, "| `"+strings.TrimSuffix(name, ".yml")+"` | "+Workflows[name]+" |")
	}
	lines := []string{"## 🖥️ What each runner is for", "",
		"<sub>The status issue's runners section says what each runner is doing now; this says " +
			"what each is for. The bot rewrites it when a runner or a workflow changes.</sub>", "",
		"| Runner | Where | What it runs |", "|---|---|---|"}
	lines = append(lines, rows...)
	lines = append(lines, "", "<sub>"+BoxesNote+"</sub>",
		"", "### Each workflow", "", "| Workflow | What it is for |", "|---|---|")
	lines = append(lines, flows...)
	lines = append(lines, "", Explained)
	return strings.Join(lines, "\n")
}

// explainedURL is _explained_url.
func explainedURL(ctx *hctx.Context, state *pyjson.Object) string {
	kept := state.Obj("runners_explained")
	if !kept.Truthy("issue") || !kept.Truthy("id") {
		return ""
	}
	return ctx.Cfg.ServerURL + "/" + ctx.Cfg.Repo + "/issues/" + py.Str(kept.Get("issue")) +
		"#issuecomment-" + py.Str(kept.Get("id"))
}

// Explain is explain: keep the status issue's comment on what each runner is for: written once,
// and again only when its text changes (the state file keeps its id and a digest), so a rewrite
// costs no call. `state` takes the record too, so the rewrite in hand links the comment.
func Explain(ctx *hctx.Context, number int, state *pyjson.Object) {
	text := Explanation(ctx)
	sum := sha256.Sum256([]byte(text))
	digest := hex.EncodeToString(sum[:])[:16]
	kept := state.Obj("runners_explained")
	if pyjson.Equal(kept.Get("issue"), number) && pyjson.Equal(kept.Get("digest"), digest) &&
		kept.Truthy("id") {
		return
	}
	var commentID any
	if pyjson.Equal(kept.Get("issue"), number) {
		commentID = kept.Get("id")
	}
	if exc, failed := herr.Catch[*herr.GitHubError](func() {
		if pyjson.Truthy(commentID) {
			ctx.GH.UpdateComment(py.MustInt(commentID), text)
		}
	}); failed {
		if exc.Status != 404 {
			herr.Raise(exc)
		}
		commentID = nil // deleted by hand: written again
	}
	if !pyjson.Truthy(commentID) {
		commentID = ctx.GH.CreateComment(number, text).Get("id")
	}
	record := pyjson.New("issue", number, "id", commentID, "digest", digest)
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("runners_explained", record)
		return nil
	}, "runners explained")
	state.Set("runners_explained", record)
}

// PCFile and PCQuiet are PC_FILE and PC_QUIET: the owner's PC's heartbeat: the gist file its
// supervisor rewrites every `heartbeatSeconds` (bot/machine/local/supervisor.ps1, `Send-Heartbeat`),
// and how old it may be before the issue says the PC has gone quiet (several missed beats: off,
// asleep, or the supervisor stopped).
var (
	PCFile  = "pc.json"
	PCQuiet = 10 * time.Minute
)

// GatherWorkflow is GATHER_WORKFLOW: the gather workflow, whose one job runs on the owner's PC
// (training/gather.md).
var GatherWorkflow = "gather.yml"

// BoxRow and BoxSteps are BOX_ROW and BOX_STEPS: how many boxes a row of the runners diagram
// holds, and how many of GitHub's runners it draws at most when the issue is short of room
// (`runners_section`).
var (
	BoxRow   = 6
	BoxSteps = []int{12, 6, 0}
)

// RunnerLegend is RUNNER_LEGEND.
var RunnerLegend = "🟢 in a job · ⏸️ in a job, slowed while the PC is busy · ⏳ queued for a runner · " +
	"⚪ idle, waiting for a job · 💤 no job: idle, or its box is off · ⚫ off · ❌ its " +
	"last run failed. GitHub's runners exist only while they hold a job."

// Found is what the runners section draws, read once: each busy job and each queued one with its
// run (each pair is [run, job]), how many runs were not read and whether the runs could be read;
// the gather workflow's last runs; and the owner's PC's heartbeat (nil, with `PCNote` saying why,
// when there is none).
type Found struct {
	Busy   [][2]*pyjson.Object
	Queued [][2]*pyjson.Object
	Unread int              // default: 0
	Failed bool             // default: False
	Gather []*pyjson.Object // default: field(default_factory=list)
	PC     *pyjson.Object   // default: None
	PCNote string           // default: ""
}

// ReadRunners is read_runners: the jobs of the runs going and queued, from GitHub, since it lists
// a repository's own runners only to an admin, which the bot is not; the gather workflow's last
// runs; and the PC's heartbeat. Every read is a conditional request (`gh._CACHED`).
func ReadRunners(ctx *hctx.Context) *Found {
	var runs []*pyjson.Object
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		going := ctx.GH.ListRepoRuns("in_progress", 100, true)
		queued := ctx.GH.ListRepoRuns("queued", 100, true)
		runs = slices.Concat(going, queued)
	}); failed {
		return &Found{Failed: true}
	}
	sort.SliceStable(runs, func(i, j int) bool {
		return py.OrStr(runs[i].Get("created_at"), "") < py.OrStr(runs[j].Get("created_at"), "")
	})
	found := &Found{Unread: max(0, len(runs)-RunnerRuns)}
	for _, run := range upTo(runs, RunnerRuns) {
		var jobs []*pyjson.Object
		id, has := run.Lookup("id")
		failed := !has // run["id"]'s KeyError
		if !failed {
			_, failed = herr.Catch[*herr.GitHubError](func() { jobs = ctx.GH.ListJobs(id, true) })
		}
		if failed {
			found.Unread++
			continue
		}
		for _, job := range jobs {
			switch job.Str("status") {
			case "in_progress":
				found.Busy = append(found.Busy, [2]*pyjson.Object{run, job})
			case "queued":
				found.Queued = append(found.Queued, [2]*pyjson.Object{run, job})
			}
		}
	}
	sort.SliceStable(found.Busy, func(i, j int) bool {
		a, _ := Where(found.Busy[i][1])
		b, _ := Where(found.Busy[j][1])
		if a != b {
			return a < b
		}
		return py.OrStr(found.Busy[i][1].Get("started_at"), "") < py.OrStr(found.Busy[j][1].Get("started_at"), "")
	})
	herr.Catch[*herr.GitHubError](func() {
		var gather []*pyjson.Object
		for _, run := range ctx.GH.ListRuns(GatherWorkflow, "", 5, true) {
			if isGather(run) {
				gather = append(gather, run)
			}
		}
		found.Gather = gather
	})
	found.PC, found.PCNote = heartbeat(ctx)
	return found
}

// heartbeat is _heartbeat: the owner's PC's last heartbeat, from the gist the repository variable
// JACKIOH_PC_GIST names (`cfg.pc_gist`), or nil and why not.
func heartbeat(ctx *hctx.Context) (*pyjson.Object, string) {
	if ctx.Cfg.PCGist == "" {
		return nil, "no heartbeat set up (JACKIOH_PC_GIST)"
	}
	var beat any
	unreadable := false
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		files := ctx.GH.GetGist(ctx.Cfg.PCGist, true).Obj("files")
		text := "{}"
		if content := files.Obj(PCFile).Get("content"); pyjson.Truthy(content) {
			text = py.Str(content)
		}
		var err error
		beat, err = pyjson.LoadsString(text)
		unreadable = err != nil // json.loads's ValueError
	}); failed || unreadable {
		return nil, "its heartbeat could not be read"
	}
	b, ok := beat.(*pyjson.Object)
	if !ok || b == nil || clock.ParseISOValue(b.Get("at")).IsZero() {
		return nil, "no heartbeat yet"
	}
	return b, ""
}

// isGather is _is_gather.
func isGather(run *pyjson.Object) bool {
	return strings.HasSuffix(py.OrStr(run.Get("path"), ""), "/"+GatherWorkflow)
}

// Where is _where: where a job runs, from its runner's name and its label, as (order, words).
func Where(job *pyjson.Object) (int, string) {
	name := py.OrStr(job.Get("runner_name"), "")
	label := ""
	if labels := pyjson.List(job.Get("labels")); len(labels) > 0 {
		label = py.Str(labels[0])
	}
	if strings.HasPrefix(name, "pc-") || label == LocalLabel || label == GatherLabel {
		return 3, "owner's PC"
	}
	if strings.HasPrefix(name, "night-vm-") || strings.HasPrefix(label, "night-vm-") {
		if memory.BoxOf(name, label) != "" {
			return 2, "Muse box"
		}
		return 1, "night box"
	}
	for _, s := range [][2]string{{"ubuntu", "Linux"}, {"macos", "macOS"}, {"windows", "Windows"}} {
		if strings.HasPrefix(label, s[0]) {
			return 0, s[1]
		}
	}
	if providers.Hosted(label) {
		return 0, "GitHub's"
	}
	return 4, "unknown box"
}

// StepWords is STEP_WORDS: steps whose names say less than they seem to: bot-night's model step is
// named one way on the shared Claude account and the other on every other subscription, for the
// partner bot's sake (bot-night.yml), and both build, check and review.
var StepWords = map[string]string{"Build, check and review": "build, check and review",
	"Work on another subscription (build, check and review)": "build, check and review"}

// Step_ is _step: the step a running job is on, or the last it finished.
func Step_(job *pyjson.Object, limit int /* = 45 */) string {
	steps := pyjson.Objs(job.Get("steps"))
	for _, s := range steps {
		if s.Str("status") == "in_progress" {
			name := py.OrStr(s.Get("name"), "")
			if words, ok := StepWords[name]; ok {
				name = words
			}
			return Cell(name, limit)
		}
	}
	var done []*pyjson.Object
	for _, s := range steps {
		if s.Str("status") == "completed" {
			done = append(done, s)
		}
	}
	if len(done) > 0 {
		return "after " + Cell(done[len(done)-1].Get("name"), limit-6)
	}
	return "setting up"
}

// nightItems is _night_items: the night bot's runs its state file names, by run id (or
// `pending:<ref>`), to the item each works on (0: the suggestion survey), in the state's order.
func nightItems(state *pyjson.Object) *pyjson.Object {
	found := pyjson.New()
	state.Obj("items").Items(func(number string, v any) bool {
		if record, ok := v.(*pyjson.Object); ok && record.Truthy("run_id") && py.IsDecimal(number) {
			n, _ := strconv.Atoi(number)
			found.Set(py.Str(record.Get("run_id")), n)
		}
		return true
	})
	if survey := state.Obj("suggest"); survey.Truthy("run_id") {
		found.Set(py.Str(survey.Get("run_id")), 0)
	}
	return found
}

// runFor is _run_for: what a run is for: the night bot's item and what it does to it, the pull
// request it checks, or its title.
func runFor(state *pyjson.Object, items *pyjson.Object, run *pyjson.Object) string {
	number, known := items.Lookup(py.Str(run.Get("id")))
	title := py.OrStr(run.Get("display_title"), "")
	if !known {
		items.Items(func(ref string, n any) bool {
			if strings.HasPrefix(ref, plan.Pending) &&
				strings.HasPrefix(title, "bot-night "+strings.TrimPrefix(ref, plan.Pending)+" ") {
				number, known = n, true
				return false
			}
			return true
		})
	}
	if known && number.(int) == 0 {
		return "the suggestion survey"
	}
	if known {
		verb, item, _ := doing(state, number.(int))
		provider := status.Record(state, number.(int)).Get("provider")
		text := verb + " " + item
		if pyjson.Truthy(provider) {
			text += " on `" + py.Str(provider) + "`"
		}
		return text
	}
	if pulls := pyjson.List(run.Get("pull_requests")); len(pulls) > 0 {
		return "#" + py.Str(pyjson.Obj(pulls[0]).Get("number")) + " " + Cell(title, 36)
	}
	return py.Str(run.Get("event")) + ": " + Cell(title, 36)
}

// flow is _flow: `ci`: a run's workflow by its file, or by its name when the run does not say its
// file.
func flow(run *pyjson.Object) string {
	path := py.OrStr(run.Get("path"), "")
	name := strings.TrimSuffix(path[strings.LastIndex(path, "/")+1:], ".yml")
	if name != "" {
		return name
	}
	return Cell(run.Get("name"), 24)
}

// jobName is _job_name: `ci · rust (test engine)`: the workflow by its file, then the job.
func jobName(run, job *pyjson.Object) string {
	return flow(run) + " · " + Cell(job.Get("name"), 32)
}

var boxMarkup = strings.NewReplacer("`", "", `"`, "'", "<", "‹", ">", "›")

// Text is _text: text for a box: one line, clipped, with nothing Mermaid or HTML would read as
// markup.
func Text(value any, limit int /* = 34 */) string {
	flat := boxMarkup.Replace(strings.Join(strings.Fields(py.OrStr(value, "")), " "))
	if py.Len(flat) <= limit {
		return flat
	}
	return py.SliceTo(flat, limit-1) + "…"
}

// RunnerWrap is RUNNER_WRAP: the runner boxes' lines wrap at 280 pixels rather than Mermaid's 200,
// so a box's few lines (`_text` clips each at 34 characters) stay one row each.
var RunnerWrap = `%%{init: {"flowchart": {"wrappingWidth": 280}}}%%`

// GhostStyle is GHOST_STYLE: an invisible box, which pads a short last row of boxes so that it
// starts at the left: Mermaid lines up rows of unequal length at their right ends.
var GhostStyle = "    classDef ghost fill:transparent,stroke:transparent,color:transparent"

// Box is one box of a subgraph (`_boxes`): its lines, and its style.
type Box struct {
	Lines []string
	Style string
}

// Boxes is _boxes: a subgraph of boxes, each (its lines, its style), `BOX_ROW` to a row, each row
// a chain of invisible links; past one row, the last is padded with invisible boxes
// (`GHOST_STYLE`).
func Boxes(sid string, title string, boxes []Box) []string {
	if len(boxes) > BoxRow && len(boxes)%BoxRow != 0 {
		boxes = slices.Clone(boxes)
		for pad := BoxRow - len(boxes)%BoxRow; pad > 0; pad-- {
			boxes = append(boxes, Box{[]string{" "}, "ghost"})
		}
	}
	nodes := make([]string, len(boxes))
	for i := range boxes {
		nodes[i] = sid + strconv.Itoa(i)
	}
	lines := []string{"    subgraph " + sid + `["` + Text(title, 120) + `"]`, "        direction LR"}
	for i, b := range boxes {
		var shown []string
		for _, t := range b.Lines {
			if t != "" {
				shown = append(shown, t)
			}
		}
		lines = append(lines, "        "+nodes[i]+`["`+strings.Join(shown, "<br/>")+`"]:::`+b.Style)
	}
	for i := 0; i < len(nodes); i += BoxRow {
		if row := nodes[i:min(i+BoxRow, len(nodes))]; len(row) > 1 {
			lines = append(lines, "        "+strings.Join(row, " ~~~ "))
		}
	}
	return append(lines, "    end")
}

// fresh is _fresh: the PC's heartbeat while it is recent enough to trust.
func fresh(ctx *hctx.Context, found *Found) *pyjson.Object {
	at := clock.ParseISOValue(found.PC.Get("at"))
	if !at.IsZero() && ctx.Now().Sub(at) <= PCQuiet {
		return found.PC
	}
	return nil
}

// pcLoad is _pc_load: how loaded the PC is, as its CPU and memory in use, runners included; with
// `full`, also its memory's size and its load without its runners (what decides whether it lends
// itself). A heartbeat from a supervisor older than its memory's size says its load without its
// runners, in GB.
func pcLoad(beat *pyjson.Object, full bool) string {
	// The except (KeyError, TypeError, ValueError, ZeroDivisionError).
	const unreported = "its load not reported"
	// float(beat[key])
	num := func(key string) (float64, bool) {
		v, ok := beat.Lookup(key)
		if !ok {
			return 0, false
		}
		f, err := py.Float(v)
		return f, err == nil
	}
	if !beat.Truthy("memTotalGB") {
		cpu, ok := num("cpu")
		if !ok {
			return unreported
		}
		mem, ok := num("memGB")
		if !ok {
			return unreported
		}
		return fmt.Sprintf("CPU %.0f%%, %.1f GB in use", cpu, mem)
	}
	whole, ok := num("memTotalGB")
	if !ok {
		return unreported
	}
	cpuOwn, ok := beat.Lookup("cpu") // beat.get("cpuAll", beat["cpu"]) reads beat["cpu"] first
	if !ok {
		return unreported
	}
	cpu, err := py.Float(beat.GetOr("cpuAll", cpuOwn))
	if err != nil {
		return unreported
	}
	memOwn, ok := beat.Lookup("memGB")
	if !ok {
		return unreported
	}
	mem, err := py.Float(beat.GetOr("memAllGB", memOwn))
	if err != nil || whole == 0 {
		return unreported
	}
	load := fmt.Sprintf("CPU %.0f%%, memory %.0f%%", cpu, 100*mem/whole)
	if full {
		ownCPU, ok := num("cpu")
		if !ok {
			return unreported
		}
		ownMem, ok := num("memGB")
		if !ok {
			return unreported
		}
		load += fmt.Sprintf(" of %.0f GB (%.0f%% and %.0f%% without its runners)", whole, ownCPU,
			100*ownMem/whole)
	}
	return load
}

// pcJobsOf is _pc_jobs: the jobs the PC's runners hold now, as GitHub lists them.
func pcJobsOf(found *Found) [][2]*pyjson.Object {
	var out [][2]*pyjson.Object
	for _, pair := range found.Busy {
		if order, _ := Where(pair[1]); order == 3 {
			out = append(out, pair)
		}
	}
	return out
}

// pcWork is _pc_work: whether the PC is working: the workflows of the jobs its runners hold
// (GitHub's jobs, or the heartbeat's busy runners when GitHub has not listed them yet), with `full`
// each job's name and whether it is slowed; or that it is not, and whether a runner of its waits for
// a job.
func pcWork(beat *pyjson.Object, jobs [][2]*pyjson.Object, full bool) string {
	runners := pcRunners(beat)
	anyBusy, anyIdle := false, false
	for _, r := range runners {
		if r.Truthy("busy") {
			anyBusy = true
		} else {
			anyIdle = true
		}
	}
	if len(jobs) == 0 && !anyBusy {
		if anyIdle {
			return "not working: a runner waits for a job"
		}
		return "not working: no job"
	}
	if len(jobs) == 0 {
		return "working on a job"
	}
	if !full {
		var flows []string
		for _, pair := range jobs {
			if f := flow(pair[0]); !slices.Contains(flows, f) {
				flows = append(flows, f)
			}
		}
		return "working on " + strings.Join(flows, " and ")
	}
	slowed := false
	for _, pair := range jobs {
		if pcRunner(beat, py.OrStr(pair[1].Get("runner_name"), "")).Truthy("throttled") {
			slowed = true
			break
		}
	}
	var parts []string
	for _, pair := range jobs {
		parts = append(parts, flow(pair[0])+" ("+Cell(pair[1].Get("name"), 48)+")")
	}
	text := "working on " + strings.Join(parts, "; ")
	if slowed {
		text += ", slowed while the PC is busy"
	}
	return text
}

// pcWords is _pc_words: the PC from its heartbeat: lending itself or busy, whether it is working
// (on `jobs`, the ones GitHub lists on its runners), how loaded, and when it was heard; or that it
// has gone quiet, or why there is no heartbeat (`note`).
func pcWords(ctx *hctx.Context, beat *pyjson.Object, note string, jobs [][2]*pyjson.Object,
	full bool) string {
	if beat == nil {
		return note
	}
	at := clock.ParseISOValue(beat.Get("at"))
	if at.IsZero() || ctx.Now().Sub(at) > PCQuiet {
		return "not heard since " + clockText(ctx, at) + ": off, asleep, or its supervisor stopped"
	}
	mood := "busy with its owner's work"
	if beat.Truthy("calm") {
		mood = "calm, lending itself"
	}
	return mood + ", " + pcWork(beat, jobs, full) + ", " + pcLoad(beat, full) + " · heard " +
		clockText(ctx, at)
}

// pcTitle is _pc_title: the PC's group: whether it is working, how loaded it is, from its
// heartbeat, and when it was heard.
func pcTitle(ctx *hctx.Context, found *Found) string {
	return "The owner's PC: " + pcWords(ctx, found.PC, found.PCNote, pcJobsOf(found), false)
}

// pcRunners is _pc_runners: the heartbeat's runner containers (PowerShell 5.1 may write a list as
// `{"value": [...]}`).
func pcRunners(beat *pyjson.Object) []*pyjson.Object {
	runners := beat.Get("runners")
	if o, ok := runners.(*pyjson.Object); ok && o.Len() > 0 {
		runners = o.Get("value")
	}
	return pyjson.Objs(runners)
}

// pcRunner is _pc_runner: the heartbeat's runner named `name`, or nil (which reads as {}).
func pcRunner(beat *pyjson.Object, name string) *pyjson.Object {
	for _, r := range pcRunners(beat) {
		if n, ok := r.Get("name").(string); ok && n == name {
			return r
		}
	}
	return nil
}

// gatherBox is _gather_box: the gather workflow on the owner's PC: gathering now (and how hard the
// PC lets it), waiting for the PC's runner, or how its last run ended.
func gatherBox(ctx *hctx.Context, found *Found, beat *pyjson.Object) Box {
	title := "<b>gather</b>"
	for _, pair := range found.Busy {
		if !isGather(pair[0]) {
			continue
		}
		job := pair[1]
		since := clock.ParseISOValue(job.Get("started_at"))
		took := "?"
		if !since.IsZero() {
			took = clock.HumanDelta(ctx.Now().Sub(since))
		}
		lines := []string{title, "🟢 gathering", Text(Step_(job, 45), 34), "for " + took}
		runner := pcRunner(beat, py.OrStr(job.Get("runner_name"), ""))
		if runner.Truthy("throttled") {
			return Box{append(lines, "⏸️ slowed: the PC is busy"), "paused"}
		}
		if cpus := runner.Get("cpus"); cpus != nil {
			n := py.MustFloat(cpus)
			lines = append(lines, fmt.Sprintf("on %.1f CPUs", n))
		}
		return Box{lines, "busy"}
	}
	for _, pair := range found.Queued {
		if !isGather(pair[0]) {
			continue
		}
		since := clock.ParseISOValue(pair[1].Get("created_at"))
		waited := ""
		if !since.IsZero() {
			waited = "for " + clock.HumanDelta(ctx.Now().Sub(since))
		}
		return Box{[]string{title, "⏳ waiting for the PC's runner", waited}, "next"}
	}
	var done []*pyjson.Object
	for _, r := range found.Gather {
		if r.Str("status") == "completed" {
			done = append(done, r)
		}
	}
	sort.SliceStable(done, func(i, j int) bool { // sorted(…, reverse=True): equal ones keep their order
		return py.OrStr(done[i].Get("created_at"), "") > py.OrStr(done[j].Get("created_at"), "")
	})
	if len(done) == 0 {
		return Box{[]string{title, "no run yet", "it plays each patch", "as it ships"}, "off"}
	}
	last, at := done[0], clock.ParseISOValue(done[0].Get("created_at"))
	when := "?"
	if !at.IsZero() {
		when = clockText(ctx, at)
	}
	if pyjson.Equal(last.Get("conclusion"), "skipped") {
		return Box{[]string{title, "⚫ off: GATHER_ON_PATCH", "is not true (skipped " + when + ")"}, "off"}
	}
	if pyjson.Equal(last.Get("conclusion"), "success") {
		return Box{[]string{title, "⚪ not gathering", "last played " + when}, "idle"}
	}
	return Box{[]string{title, "❌ its last run: " + Text(last.Get("conclusion"), 12), when}, "broken"}
}

// LearnGames is LEARN_GAMES: learn.yml's `games` default: the export a scheduled learn run plays,
// in shards whose ranges the heartbeat names, so a shard's games read as the export's (`progress`).
var LearnGames = 100_000

// ProgressStart and ProgressEnd are PROGRESS_START and PROGRESS_END: the top of the issue after its
// date: how far the PC's two local jobs have played, between these markers, so the runners' redraw
// keeps them as fresh as the heartbeat (`refresh_runners`).
var (
	ProgressStart = "<!-- jackioh-bot:progress -->"
	ProgressEnd   = "<!-- /jackioh-bot:progress -->"
)

// playedGames is what `_played` returns: the games a runner plays, those done, its --from and its
// --patch.
type playedGames struct {
	games, done, from int
	patch             string
}

// played is _played: a heartbeat runner's `progress` (bot/machine/local/supervisor.ps1,
// `Get-Progress`) when it is playing games with `jackioh <kind>`: the games it plays, those done,
// and its --from and --patch.
func played(runner *pyjson.Object, kind string) *playedGames {
	progress, ok := runner.Get("progress").(*pyjson.Object)
	if !ok || progress == nil {
		return nil
	}
	if k, ok := progress.Get("kind").(string); !ok || k != kind {
		return nil
	}
	intAt := func(key string) (int, bool) { // int(progress[key])
		v, ok := progress.Lookup(key)
		if !ok {
			return 0, false
		}
		n, err := py.Int(v)
		return n, err == nil
	}
	games, ok := intAt("games")
	if !ok {
		return nil
	}
	done, ok := intAt("done")
	if !ok {
		return nil
	}
	start := 1
	if v := progress.Get("from"); pyjson.Truthy(v) {
		var err error
		if start, err = py.Int(v); err != nil {
			return nil
		}
	}
	if games <= 0 || done < 0 {
		return nil
	}
	return &playedGames{games, min(done, games), start, py.OrStr(progress.Get("patch"), "")}
}

// ProgressChart is PROGRESS_CHART: the progress chart's colours, the usage charts' (`USAGE_CHART`):
// a grey bar to 100% behind, the games played in green over it; and its height, low enough to sit
// at the top of the issue.
var ProgressChart = `%%{init: {"xyChart": {"height": 220}, "themeVariables": {"xyChart": ` +
	`{"plotColorPalette": "#d0d7de, #1f883d"}}}}%%`

// Progress is progress: the top of the issue, between `PROGRESS_START` and `PROGRESS_END`: how far
// the owner's PC's two local jobs have played, as a Mermaid bar chart in percent, with a line under
// it. The learn lane's export is the game it is at of `LEARN_GAMES`, then its shard's games; gather
// is its games of its patch. Each comes from the games its `jackioh` has written so far (the
// heartbeat's `progress`); a job playing none has a bar at 0 that says why. Without `found` it
// reads the heartbeat alone.
func Progress(ctx *hctx.Context, found *Found /* = None */) string {
	if found == nil {
		found = &Found{}
		found.PC, found.PCNote = heartbeat(ctx)
	}
	beat := fresh(ctx, found)
	runners := pcRunners(beat)
	type bar struct {
		label string
		value float64
	}
	var bars []bar
	var notes []string
	if beat == nil {
		at := clock.ParseISOValue(found.PC.Get("at"))
		why := found.PCNote
		if !at.IsZero() {
			why = "not heard since " + clockText(ctx, at)
		} else if why == "" {
			why = "no heartbeat"
		}
		bars = []bar{{"learn: no heartbeat", 0.0}, {"gather: no heartbeat", 0.0}}
		notes = []string{"The owner's PC: " + why}
	} else {
		queued := map[string]bool{}
		for _, pair := range found.Queued {
			if containsStr(pyjson.List(pair[1].Get("labels")), LocalLabel) {
				queued["selfplay"] = true
				break
			}
		}
		for _, pair := range found.Queued {
			if isGather(pair[0]) {
				queued["stats"] = true
				break
			}
		}
		for _, j := range []struct{ name, label, kind, between string }{
			{"learn", LocalLabel, "selfplay", "a build, the retrain, the screen or the gate"},
			{"gather", GatherLabel, "stats", "its build"}} {
			var busy []*pyjson.Object
			for _, r := range runners {
				if l, ok := r.Get("label").(string); ok && l == j.label && r.Truthy("busy") {
					busy = append(busy, r)
				}
			}
			var p *playedGames
			for _, r := range busy {
				if p = played(r, j.kind); p != nil {
					break
				}
			}
			if p == nil {
				state := "not playing"
				if len(busy) > 0 {
					state = "working, no games"
				} else if queued[j.kind] {
					state = "waiting for its runner"
				}
				bars = append(bars, bar{j.name + ": " + state, 0.0})
				if len(busy) > 0 {
					notes = append(notes, j.name+" is playing no games now ("+j.between+")")
				}
				continue
			}
			games, done, first := p.games, p.done, p.from
			if j.kind == "stats" {
				bars = append(bars, bar{"gather: game " + py.Thousands(done) + " of " + py.Thousands(games),
					float64(done) / float64(games)})
				if p.patch != "" {
					notes = append(notes, "gather plays "+p.patch)
				}
				continue
			}
			at, last := first-1+done, first+games-1
			if last <= LearnGames {
				bars = append(bars, bar{"learn: game " + py.Thousands(at) + " of " + py.Thousands(LearnGames),
					float64(at) / float64(LearnGames)})
			} else {
				notes = append(notes, "learn is at game "+py.Thousands(at)+", past learn.yml's "+
					py.Thousands(LearnGames))
			}
			bars = append(bars, bar{"its shard: " + py.Thousands(done) + " of " + py.Thousands(games),
				float64(done) / float64(games)})
			notes = append(notes, "the shard plays games "+py.Thousands(first)+"–"+py.Thousands(last))
		}
		notes = append(notes, "heard "+clockText(ctx, clock.ParseISOValue(beat.Get("at"))))
	}
	var labels, full, values []string
	for _, b := range bars {
		labels = append(labels, `"`+Text(b.label, 40)+`"`)
		full = append(full, "100")
		v := b.value // min(max(v, 0.0), 1.0)
		if 0.0 > v {
			v = 0.0
		}
		if 1.0 < v {
			v = 1.0
		}
		values = append(values, fmt.Sprintf("%.1f", v*100))
	}
	note := strings.Join(notes, " · ")
	lines := []string{ProgressStart, "```mermaid", ProgressChart, "xychart-beta horizontal",
		`    title "The owner's PC: how far its two local jobs have played, %"`,
		"    x-axis [" + strings.Join(labels, ", ") + "]", `    y-axis "% played" 0 --> 100`,
		"    bar [" + strings.Join(full, ", ") + "]",
		"    bar [" + strings.Join(values, ", ") + "]", "```",
		"<sub>" + strings.ToUpper(py.SliceTo(note, 1)) + py.SliceFrom(note, 1) + ".</sub>",
		ProgressEnd}
	return strings.Join(lines, "\n")
}

// BoxSlot is one AWS box of BOX_SLOTS: the place `_where` names it, the subscription whose lane
// each slot is, and the subscriptions whose runners there take no slot of their own.
type BoxSlot struct {
	Place  string
	Owners []string
	Extras []string
}

// BoxSlots is BOX_SLOTS: each AWS box as its slots, as register-runners.sh lays the runners out
// (its header, and bot/machine/README.md, The Muse box; #2069): the place `_where` names it, the
// subscription whose lane each slot is, and the subscriptions whose runners there take no slot of
// their own (none now), drawn after the slots while they hold a job. A fill-in (`FillIn`) takes
// the slot of a subscription it fills while that one has no job in it. Change it with the boxes.
// A variable the tests replace.
var BoxSlots = []BoxSlot{
	{"night box", []string{"gpt", "agy", "muse"}, nil},
	{"Muse box", []string{"muse", "muse", "muse"}, nil},
}

var placeOrder = map[string]int{"night box": 1, "Muse box": 2}

// boxSlots is _box_slots: one AWS box as its slots: each the lane of a subscription, holding its
// job, a fill-in's job, or nothing (free, or a job queued for it); then its other runners' jobs.
// The box's title says how many slots are busy, or that it holds no job (idle, or switched off).
func boxSlots(ctx *hctx.Context, state *pyjson.Object, items *pyjson.Object, place string,
	owners []string, extras []string, jobs [][2]*pyjson.Object,
	asked map[string]int) (string, []Box) {
	pool := ctx.Cfg.Pool
	label := map[string]string{}
	for _, p := range pool.Ordered() {
		label[p.ID] = p.RunsOn
	}
	type fillIn struct{ id, runsOn string }
	filler := map[string]fillIn{}
	for _, p := range pool.Ordered() {
		if p.FillIn != nil {
			for _, owner := range p.FillIn.Of {
				filler[owner] = fillIn{p.ID, p.FillIn.RunsOn}
			}
		}
	}
	left := slices.Clone(jobs)

	take := func(wanted string) ([2]*pyjson.Object, bool) {
		for i, pair := range left {
			if containsStr(pyjson.List(pair[1].Get("labels")), wanted) {
				left = slices.Delete(left, i, i+1)
				return pair, true
			}
		}
		return [2]*pyjson.Object{}, false
	}

	working := func(pair [2]*pyjson.Object) []string {
		since := clock.ParseISOValue(pair[1].Get("started_at"))
		took := "?"
		if !since.IsZero() {
			took = clock.HumanDelta(ctx.Now().Sub(since))
		}
		return []string{Text(runFor(state, items, pair[0]), 34), Text(Step_(pair[1], 45), 34), took}
	}

	var slots []Box
	for n, owner := range owners {
		head := fmt.Sprintf("<b>Slot %d · %s's lane</b>", n+1, Text(owner, 34))
		own, hasOwn := take(label[owner])
		fill, hasFill := filler[owner]
		var borrowed [2]*pyjson.Object
		hasBorrowed := false
		if !hasOwn && hasFill {
			borrowed, hasBorrowed = take(fill.runsOn)
		}
		switch {
		case hasOwn:
			slots = append(slots, Box{append([]string{head, "🟢 " + Text(owner, 34)}, working(own)...), "busy"})
		case hasBorrowed:
			slots = append(slots, Box{append([]string{head, "🟢 " + Text(fill.id, 34) + " fills it"},
				working(borrowed)...), "busy"})
		case asked[label[owner]] != 0:
			slots = append(slots, Box{[]string{head, "⏳ a " + Text(owner, 34) + " job is queued",
				"its box is starting, or full"}, "next"})
		default:
			may := ""
			if hasFill {
				may = Text(fill.id, 34) + " may fill it"
			}
			slots = append(slots, Box{[]string{head, "⚪ free", may}, "idle"})
		}
	}
	busy := 0
	for _, s := range slots {
		if s.Style == "busy" {
			busy++
		}
	}
	for _, pair := range left { // its other runners' jobs, and any no slot names
		slots = append(slots, Box{append([]string{"<b>" + Text(pair[1].Get("runner_name"), 34) + "</b>",
			"🟢 " + Text(jobName(pair[0], pair[1]), 34)}, working(pair)...), "busy"})
	}
	name := "The " + place
	if place == "night box" {
		name = "The night box"
	}
	if len(jobs) == 0 {
		return name + " (AWS): no job, so idle or switched off", slots
	}
	others := ""
	if len(left) > 0 {
		others = fmt.Sprintf(" · %d more on its other runners", len(left))
	}
	return fmt.Sprintf("%s (AWS): %d of %d slots busy%s", name, busy, len(owners), others), slots
}

// RunnerDiagram is runner_diagram: the runners as boxes, as the subscriptions are drawn: GitHub's
// runners (at most `boxes` of them), the night box's and the Muse box's, the owner's PC with its
// heartbeat and the gather workflow, then the jobs queued for a runner, by label.
func RunnerDiagram(ctx *hctx.Context, state *pyjson.Object, found *Found, boxes int) []string {
	items := nightItems(state)
	beat := fresh(ctx, found)

	busyBox := func(run, job *pyjson.Object, named bool) Box {
		since := clock.ParseISOValue(job.Get("started_at"))
		took := "?"
		if !since.IsZero() {
			took = clock.HumanDelta(ctx.Now().Sub(since))
		}
		what := Text(jobName(run, job), 34)
		lines := []string{"<b>" + what + "</b>"}
		if named {
			lines = []string{"<b>" + Text(job.Get("runner_name"), 34) + "</b>", "🟢 " + what}
		}
		lines = append(lines, Text(runFor(state, items, run), 34), Text(Step_(job, 45), 34), took)
		if pcRunner(beat, py.OrStr(job.Get("runner_name"), "")).Truthy("throttled") {
			return Box{append(lines, "⏸️ slowed: the PC is busy"), "paused"}
		}
		return Box{lines, "busy"}
	}

	byPlace := map[int][][2]*pyjson.Object{}
	for _, pair := range found.Busy {
		order, _ := Where(pair[1])
		byPlace[order] = append(byPlace[order], pair)
	}
	lines := []string{"```mermaid", RunnerWrap, "flowchart TB"}
	var groups []string
	if hosted := byPlace[0]; len(hosted) > 0 {
		var shown []Box
		for _, pair := range upTo(hosted, boxes) {
			shown = append(shown, busyBox(pair[0], pair[1], false))
		}
		if len(hosted) > boxes {
			shown = append(shown, Box{[]string{fmt.Sprintf("<b>… and %d more</b>", len(hosted)-boxes),
				"not drawn: no room"}, "closed"})
		}
		lines = append(lines, Boxes("gh", fmt.Sprintf("GitHub's runners: %d busy, of the %d "+
			"the organization may hold", len(hosted), HostedRunners), shown)...)
		groups = append(groups, "gh")
	}
	pool := ctx.Cfg.Pool
	held := map[string]bool{}
	for _, pair := range found.Busy {
		for _, l := range pyjson.List(pair[1].Get("labels")) {
			held[py.Str(l)] = true
		}
	}
	asked := map[string]int{}
	for _, pair := range found.Queued {
		for _, l := range pyjson.List(pair[1].Get("labels")) {
			asked[py.Str(l)]++
		}
	}
	placed := map[string]bool{}
	for i, sid := range []string{"nb", "mb"} {
		if i >= len(BoxSlots) {
			break
		}
		slot := BoxSlots[i]
		title, here := boxSlots(ctx, state, items, slot.Place, slot.Owners, slot.Extras,
			byPlace[lookup(placeOrder, slot.Place)], asked)
		lines = append(lines, Boxes(sid, title, here)...)
		groups = append(groups, sid)
		for _, id := range slices.Concat(slot.Owners, slot.Extras) {
			placed[id] = true
		}
	}
	// A machine subscription no box's slots name (one added since `BOX_SLOTS` was written).
	var labels []string
	for _, p := range pool.Ordered() {
		if pool.OnMachine(p.ID) && !placed[p.ID] {
			labels = append(labels, p.RunsOn)
			if p.FillIn != nil {
				labels = append(labels, p.FillIn.RunsOn)
			}
		}
	}
	var idle []Box
	seen := map[string]bool{}
	for _, l := range labels {
		if seen[l] {
			continue
		}
		seen[l] = true
		if held[l] {
			continue
		}
		if asked[l] != 0 {
			idle = append(idle, Box{[]string{"<b>" + Text(l, 34) + "</b>", fmt.Sprintf("⏳ %d queued for it",
				asked[l]), "its box is starting, or full"}, "next"})
		} else {
			idle = append(idle, Box{[]string{"<b>" + Text(l, 34) + "</b>", "💤 no job",
				"idle, or its box is off"}, "closed"})
		}
	}
	if len(idle) > 0 {
		lines = append(lines, Boxes("nj", "Other AWS labels with no job", idle)...)
		groups = append(groups, "nj")
	}
	pcJobs := byPlace[3]
	var pc []Box
	withJobs := map[string]bool{}
	for _, pair := range pcJobs {
		pc = append(pc, busyBox(pair[0], pair[1], true))
	}
	for _, pair := range pcJobs {
		withJobs[py.OrStr(pair[1].Get("runner_name"), "")] = true
	}
	for _, runner := range pcRunners(beat) {
		if withJobs[py.OrStr(runner.Get("name"), "")] {
			continue
		}
		name := "<b>" + Text(runner.Get("name"), 34) + "</b>"
		if runner.Truthy("busy") {
			pc = append(pc, Box{[]string{name, "🟢 in a job"}, "busy"})
		} else {
			pc = append(pc, Box{[]string{name, "⚪ idle, waiting", "for a " + Text(runner.Get("label"), 30) +
				" job"}, "idle"})
		}
	}
	if len(pc) == 0 {
		why := "idle, or the PC is off"
		if beat != nil && !beat.Truthy("calm") {
			why = "the PC is busy"
		}
		pc = append(pc, Box{[]string{"<b>" + LocalLabel + "</b>", "💤 no job", why}, "closed"})
	}
	pc = append(pc, gatherBox(ctx, found, beat))
	lines = append(lines, Boxes("pc", pcTitle(ctx, found), pc)...)
	groups = append(groups, "pc")
	if others := byPlace[4]; len(others) > 0 {
		title := fmt.Sprintf("Other runners: %d busy", len(others))
		var bs []Box
		for _, pair := range others {
			bs = append(bs, busyBox(pair[0], pair[1], true))
		}
		lines = append(lines, Boxes("ob", title, bs)...)
		groups = append(groups, "ob")
	}
	type queuedFor struct {
		label string
		jobs  [][2]*pyjson.Object
	}
	var waiting []*queuedFor
	byLabel := map[string]*queuedFor{}
	for _, pair := range found.Queued {
		var ls []string
		for _, l := range pyjson.List(pair[1].Get("labels")) {
			ls = append(ls, py.Str(l))
		}
		key := strings.Join(ls, ", ")
		if key == "" {
			key = "any"
		}
		if byLabel[key] == nil {
			byLabel[key] = &queuedFor{label: key}
			waiting = append(waiting, byLabel[key])
		}
		byLabel[key].jobs = append(byLabel[key].jobs, pair)
	}
	if len(waiting) > 0 {
		sort.SliceStable(waiting, func(i, j int) bool { return len(waiting[i].jobs) > len(waiting[j].jobs) })
		var queue []Box
		for _, w := range waiting {
			oldest := w.jobs[0] // min(…): the first of the equal ones
			for _, pair := range w.jobs[1:] {
				if py.OrStr(pair[1].Get("created_at"), "") < py.OrStr(oldest[1].Get("created_at"), "") {
					oldest = pair
				}
			}
			since := clock.ParseISOValue(oldest[1].Get("created_at"))
			count := fmt.Sprintf("⏳ %d jobs", len(w.jobs))
			if len(w.jobs) == 1 {
				count = "⏳ 1 job"
			}
			longest := ""
			if !since.IsZero() {
				longest = "the longest " + clock.HumanDelta(ctx.Now().Sub(since))
			}
			queue = append(queue, Box{[]string{"<b>" + Text(w.label, 34) + "</b>", count, longest,
				Text(jobName(oldest[0], oldest[1]), 34)}, "next"})
		}
		word := "jobs"
		if len(found.Queued) == 1 {
			word = "job"
		}
		lines = append(lines, Boxes("qu", fmt.Sprintf("Queued for a runner: %d ", len(found.Queued))+word,
			queue)...)
		groups = append(groups, "qu")
	}
	if len(groups) > 1 {
		lines = append(lines, "    "+strings.Join(groups, " ~~~ "))
	}
	lines = append(lines, Styles(lines, []string{GhostStyle})...)
	return append(lines, "```")
}

// RunnersSection is runners_section: every runner and what it is doing, as boxes
// (`runner_diagram`) and as a table: each job a runner holds now, the step it is on, what it is for
// and how long it has run, linked to the job; and a link to the comment on what each runner is for
// (`explain`). Each part folds away. Short of `room`, the table lists fewer runners, then the boxes
// show fewer of GitHub's.
func RunnersSection(ctx *hctx.Context, state *pyjson.Object, found *Found /* = None */, room int /* = MAX_BODY */) string {
	if found == nil {
		found = ReadRunners(ctx)
	}
	read := clockText(ctx, ctx.Now())
	explained := "What each runner and workflow is for is in a comment below."
	if url := explainedURL(ctx, state); url != "" {
		explained = "What each runner and workflow is for: [its comment](" + url + ")."
	}
	head := []string{RunnersStart, "## 🖥️ Every runner", ""}
	if found.Failed {
		return strings.Join(append(head, "The runs going could not be read at "+read+".", "",
			"<sub>"+explained+"</sub>", RunnersEnd), "\n")
	}
	busy, queued := found.Busy, found.Queued
	hosted, places := 0, pyjson.New()
	for _, pair := range busy {
		order, place := Where(pair[1])
		if order == 0 {
			hosted++
		} else {
			places.Set(place, places.Int(place)+1)
		}
	}
	counts := []string{fmt.Sprintf("%d on GitHub's (of the %d the organization may hold)", hosted,
		HostedRunners)}
	places.Items(func(place string, n any) bool {
		counts = append(counts, fmt.Sprintf("%d on the %s", n.(int), place))
		return true
	})
	runnersWord, jobsWord := "runners", "jobs"
	if len(busy) == 1 {
		runnersWord = "runner"
	}
	if len(queued) == 1 {
		jobsWord = "job"
	}
	head = append(head, fmt.Sprintf("🟢 **%d %s busy**: ", len(busy), runnersWord)+
		strings.Join(counts, ", ")+fmt.Sprintf(" · ⏳ **%d ", len(queued))+jobsWord+
		" queued** for a runner · read at "+read,
		"", fmt.Sprintf("<sub>Redrawn every %d minutes from the runs going and queued, "+
			"their jobs and the owner's PC's heartbeat. %s</sub>", RunnersMinutes, explained), "")
	items := nightItems(state)
	var rows []string
	for _, pair := range upTo(busy, RunnerRows) {
		run, job := pair[0], pair[1]
		since := clock.ParseISOValue(job.Get("started_at"))
		took := "?"
		if !since.IsZero() {
			took = clock.HumanDelta(ctx.Now().Sub(since))
		}
		link := py.OrStr(job.Get("html_url"), "")
		name := "—"
		if job.Truthy("runner_name") {
			name = "`" + py.Str(job.Get("runner_name")) + "`"
		}
		_, where := Where(job)
		running := took
		if link != "" {
			running = "[" + took + "](" + link + ")"
		}
		rows = append(rows, "| "+name+" ("+where+") | "+jobName(run, job)+" | "+Step_(job, 45)+
			" | "+runFor(state, items, run)+" | "+running+" |")
	}
	var tail []string
	if found.Unread != 0 {
		word := "runs"
		if found.Unread == 1 {
			word = "run"
		}
		tail = append(tail, fmt.Sprintf("<sub>%d more %s not read.</sub>", found.Unread, word), "")
	}
	tail = append(tail, RunnersEnd)

	drawn := func(shown, boxes int) string {
		diagram := []string{"<details open><summary><b>The runners, as boxes</b></summary>", ""}
		diagram = append(diagram, RunnerDiagram(ctx, state, found, boxes)...)
		diagram = append(diagram, "", "<sub>"+RunnerLegend+"</sub>", "", "</details>", "")
		var table []string
		if len(busy) == 0 {
			table = []string{"No runner holds a job right now.", ""}
		} else {
			more := len(busy) - shown
			table = []string{"<details open><summary><b>Every busy runner's job</b>, as a table " +
				fmt.Sprintf("(%d)</summary>", len(busy)), ""}
			if shown != 0 {
				table = append(table, "| Runner | Workflow · job | Step now | For | Running |",
					"|---|---|---|---|---|")
				table = append(table, upTo(rows, shown)...)
			}
			if more > 0 {
				if shown != 0 {
					table = append(table, fmt.Sprintf("\n…and %d more.", more))
				} else {
					table = append(table, fmt.Sprintf("%d not listed: no room.", more))
				}
			}
			table = append(table, "", "</details>", "")
		}
		return strings.Join(slices.Concat(head, diagram, table, tail), "\n")
	}
	hostedBoxes := 0
	for _, pair := range busy {
		if order, _ := Where(pair[1]); order == 0 {
			hostedBoxes++
		}
	}
	var shownSteps []int
	for _, n := range []int{len(rows), len(rows) / 2, min(len(rows), 5), 0} {
		if !slices.Contains(shownSteps, n) {
			shownSteps = append(shownSteps, n)
		}
	}
	sort.Sort(sort.Reverse(sort.IntSlice(shownSteps)))
	var steps [][2]int
	for _, n := range shownSteps {
		steps = append(steps, [2]int{n, hostedBoxes})
	}
	for _, n := range BoxSteps {
		if n < hostedBoxes {
			steps = append(steps, [2]int{0, n})
		}
	}
	var section string
	for _, step := range steps {
		section = drawn(step[0], step[1])
		if py.Len(section) <= room {
			return section
		}
	}
	return section
}

// RefreshRunners is refresh_runners; a variable so the tests can replace it.
var RefreshRunners = refreshRunners

// refreshRunners is refresh_runners: redraw the status issue's runners section alone, between the
// loop's rewrites, and the PC's progress chart at its top (`progress`): the rest of the issue stays
// as the last rewrite left it, and the table lists what room it leaves.
func refreshRunners(ctx *hctx.Context) string {
	state := ctx.Store.Load()
	number := state.Obj("dashboard").Get("issue")
	if !pyjson.Truthy(number) {
		return "no status issue yet"
	}
	issue := ctx.GH.GetIssue(py.MustInt(number))
	body := py.OrStr(issue.Get("body"), "")
	start, end := strings.Index(body, RunnersStart), strings.Index(body, RunnersEnd)
	if !pyjson.Equal(issue.Get("state"), "open") || start < 0 || end < start {
		return "#" + py.Str(number) + " has no runners section to redraw: the next rewrite draws it"
	}
	found := ReadRunners(ctx)
	top, bottom := strings.Index(body, ProgressStart), strings.Index(body, ProgressEnd)
	if 0 <= top && top < bottom && bottom < start {
		body = body[:top] + Progress(ctx, found) + body[bottom+len(ProgressEnd):]
		start, end = strings.Index(body, RunnersStart), strings.Index(body, RunnersEnd)
	}
	// Lengths in code points, as Python's len: the indices above are bytes.
	at := func(i int) int { return utf8.RuneCountInString(body[:i]) }
	rest := py.Len(body) - (at(end) + py.Len(RunnersEnd) - at(start))
	fresh := body[:start] + RunnersSection(ctx, state, found, MaxBody-rest) + body[end+len(RunnersEnd):]
	if py.Len(fresh) > MaxBody {
		return "#" + py.Str(number) + " would pass GitHub's limit: the next rewrite makes room"
	}
	ctx.GH.UpdateIssue(py.MustInt(number), pyjson.New("body", fresh))
	return "redrew #" + py.Str(number) + "'s runners"
}
