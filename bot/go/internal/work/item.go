package work

// ------------------------------------------------------------------ an item

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/easy"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gates"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/human"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/issueplan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/reviewrule"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/split"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/threads"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
)

// isObj is isinstance(v, dict).
func isObj(v any) bool {
	o, ok := v.(*pyjson.Object)
	return ok && o != nil
}

// fetchRef is `repo.run("fetch", "--quiet", "--no-tags", "origin", f"+refs/heads/{ref}:refs/remotes/origin/{ref}", check=check)`.
func fetchRef(repo *git.Git, ref string, check bool) int {
	code, _ := run(repo, check, "fetch", "--quiet", "--no-tags", "origin",
		"+refs/heads/"+ref+":refs/remotes/origin/"+ref)
	return code
}

// backticked is `", ".join(f"`{p}`" for p in paths)`.
func backticked(paths []string) string {
	quoted := make([]string, len(paths))
	for i, p := range paths {
		quoted[i] = "`" + p + "`"
	}
	return strings.Join(quoted, ", ")
}

// upTo is `list[:n]`.
func upTo(list []string, n int) []string {
	if len(list) > n {
		return list[:n]
	}
	return list
}

// gateDicts is `[{**r.to_dict(), "tail": r.tail[-1500:]} for r in results]`.
func gateDicts(results []*gates.GateResult) []any {
	out := []any{}
	for _, r := range results {
		d := r.ToDict().Copy()
		d.Set("tail", py.SliceFrom(r.Tail, -1500))
		out = append(out, d)
	}
	return out
}

// findingDicts is `[f.to_dict() for f in findings]`.
func findingDicts(findings []*verdicts.Finding) []any {
	out := []any{}
	for _, f := range findings {
		out = append(out, f.ToDict())
	}
	return out
}

// newFinding is Finding(severity, where, claim, evidence).
func newFinding(severity, where, claim, evidence string) *verdicts.Finding {
	return &verdicts.Finding{Severity: severity, Where: where, Claim: claim, Evidence: evidence}
}

// resetTo puts the worktree back at `head` when a reader moved it, and drops what a reader left
// in it.
func (x *Worker) resetTo(head string) {
	if x.Wt.Head() != head {
		run(x.Wt, true, "reset", "--quiet", "--hard", head)
	}
	if len(x.Wt.Dirty()) > 0 {
		x.Wt.DiscardWorktreeChanges()
	}
}

func (x *Worker) install() *gates.GateResult {
	timeout := int(x.SecondsLeft())
	result := gates.RunGate(x.Cfg.Install, x.Wt.Cwd, x.Env, &timeout)
	if result.OK {
		x.InstalledAt = x.Wt.Head()
	}
	return result
}

func (x *Worker) prepare(mergeMain bool /* = True */, install bool /* = True */) []string {
	number := py.MustInt(x.planItem("number"))
	branch := py.Str(x.planItem("branch"))
	fetchRef(x.Repo, x.Cfg.DefaultBranch, true)
	fetchRef(x.Repo, branch, false)
	onto := py.OrStr(x.Plan.Get("onto"), "")
	if onto != "" {
		// A fullsend part (#505) starts from its tree's branch, and that is its base: every
		// diff it measures leaves out the parts that landed before it.
		if fetchRef(x.Repo, onto, false) != 0 || x.Repo.Rev("origin/"+onto) == "" {
			herr.Raise(&py.Error{Type: "RuntimeError", Msg: "the integration branch " + onto + " is gone"})
		}
		x.BaseRef = "origin/" + onto
	}
	for _, parked := range pyjson.List(x.Plan.Get("parked")) {
		// The parts a fullsend reconcile merges first (#505).
		fetchRef(x.Repo, py.Str(parked), false)
	}
	remote := "origin/" + branch
	hasRemote := x.Repo.Rev(remote) != ""
	if action := x.planItem("action"); (action == "revise" || action == "review") && !hasRemote {
		herr.Raise(&py.Error{Type: "RuntimeError", Msg: "the pull request's branch " + branch + " is not on origin"})
	}
	start := x.BaseRef
	if hasRemote {
		start = remote
	}
	x.BaseSHA = x.Repo.Rev(x.BaseRef)
	x.StartSHA = x.Repo.Rev(start)
	begin := start
	if x.planItem("action") == "revise" {
		begin = x.wipStart(number, start)
	}
	x.Wt = git.WorktreeAdd(x.Repo, filepath.Join(x.WorkDir, fmt.Sprintf("item-%d", number)), branch, begin)
	x.excludeNotes()
	x.seedNotes()
	x.Result.Set("branch", branch)
	x.Result.Set("base", x.BaseSHA)
	x.Result.Set("start", x.StartSHA)
	x.Result.Set("remote_branch_existed", hasRemote)
	conflicts := []string{}
	why := "from `" + start + "`"
	if begin != start {
		why = "from `" + begin + "`"
	}
	x.Step("start", why, pyjson.New("outcome", "at `"+first12(x.Wt.Head())+"`"))
	if hasRemote && mergeMain {
		conflicts = x.Wt.Merge(x.BaseRef, x.Who)
		outcome := "clean"
		if len(conflicts) > 0 {
			outcome = fmt.Sprintf("conflicts in %d file(s): ", len(conflicts)) +
				strings.Join(upTo(conflicts, 5), ", ")
		}
		x.Step("merge", "bring the branch up to date with `main`", pyjson.New("outcome", outcome))
	}
	for _, p := range conflicts {
		if IsManifest(p) {
			return conflicts // the builder resolves the manifests first; install runs after
		}
	}
	if !install {
		return conflicts
	}
	installed := x.install()
	if !installed.OK && !hasRemote {
		x.Result.Set("infra_scope", "repository") // not the subscription's fault
		herr.Raise(NewInterrupt(fmt.Sprintf("dependency install failed on untouched main (exit %d): %s",
			installed.ExitCode, py.SliceFrom(installed.Tail, -1500)), "infra", ""))
	}
	if len(conflicts) > 0 {
		conflicts = x.Resolve(conflicts, installed.OK)
	}
	return conflicts
}

// Resolve is the conflicts the merge of `main` left that the harness settles itself
// (`resolve.py`, `resolvers` in the config); the rest are the builder's. Returns those.
func (x *Worker) Resolve(conflicts []string, commands bool) []string {
	if len(x.Cfg.Resolvers) == 0 {
		return conflicts
	}
	started := time.Now()
	// A command must leave the run time for a review of what it made (`resolve.MIN_SECONDS`).
	left, done := resolve.Resolve(x.Wt, conflicts, x.Cfg.Resolvers, x.Env, x.SecondsLeft, commands,
		float64(x.Cfg.MinMinutesForACall*60), strings.TrimPrefix(x.BaseRef, "origin/"))
	if len(done) == 0 {
		return conflicts
	}
	x.Resolved = []*resolve.Resolution{}
	dicts := []any{}
	for _, r := range done {
		if r.OK {
			x.Resolved = append(x.Resolved, r)
		}
		dicts = append(dicts, r.ToDict())
	}
	x.Result.Set("resolved", dicts)
	x.Step("resolve", "the conflicts the harness settles itself", pyjson.New(
		"minutes", py.RoundN(time.Since(started).Seconds()/60, 1),
		"outcome", resolve.Summary(done, left)))
	return left
}

// What the builder is told of the files the harness resolved ("" when none): one it took
// stays as it is; one a command regenerated is that command's output from the merged tree,
// so it goes stale when the builder changes what the command reads.
func (x *Worker) resolvedNote() string {
	if len(x.Resolved) == 0 {
		return ""
	}
	commands := map[string]string{}
	for _, r := range x.Cfg.Resolvers {
		if r.Run != "" {
			commands[r.Path] = r.Run
		}
	}
	var lines []string
	for _, r := range x.Resolved {
		tail := "leave it as it is"
		if command, ok := commands[r.Path]; ok {
			tail = "it is that command's output from the merged tree, so run `" + command +
				"` again if you change anything it reads"
		}
		lines = append(lines, "- `"+r.Path+"`: "+r.How+"; "+tail)
	}
	return "\n\nThe harness resolved these conflicts itself:\n" + strings.Join(lines, "\n")
}

// What every reviewer of the run is told of the files the harness resolved ("" when
// none): by a rule, so the branch's side of each is gone on purpose.
func (x *Worker) resolvedReviewNote() string {
	if len(x.Resolved) == 0 {
		return ""
	}
	var lines []string
	for _, r := range x.Resolved {
		lines = append(lines, "- `"+r.Path+"`: "+r.How)
	}
	return "The harness resolved these conflicts of the merge of `main` itself, by a rule in " +
		"its config: a file it took from `main` keeps `main`'s version on purpose (a job on " +
		"`main` rewrites it), and a file it regenerated is the tool's output from the " +
		"merged tree. Judge them by that rule, not as a builder's resolution:\n" +
		strings.Join(lines, "\n")
}

// Where a revision's worktree starts: the last cut-off revision's work on
// `bot/wip/<pr>` (#317 part 3), when it started from the pull request's head as it is now
// and descends from it; otherwise (someone pushed since) the pull request's head. The
// work's `start` stays the pull request's head either way, which deliver checks.
func (x *Worker) wipStart(number int, start string) string {
	wip := pyjson.Obj(x.Plan.Get("wip"))
	if wip == nil || !wip.Truthy("sha") {
		return start
	}
	ref := config.BranchPrefix + "wip/" + strconv.Itoa(number)
	fetchRef(x.Repo, ref, false)
	head := x.Repo.Rev("origin/" + ref)
	if began, ok := wip.Get("start").(string); head != "" && ok && began == x.StartSHA {
		if code, _ := run(x.Repo, false, "merge-base", "--is-ancestor", x.StartSHA, head); code == 0 {
			x.WipUsed = ref
			x.Result.Set("wip", pyjson.New("used", ref, "sha", head))
			return "origin/" + ref
		}
	}
	x.Result.Set("wip", pyjson.New("ignored", ref, "why", "the pull request's branch moved since"))
	return start
}

// isStr is isinstance(v, str).
func isStr(v any) bool {
	_, ok := v.(string)
	return ok
}

// Tell git to ignore the notes file and the journal in every worktree of this clone
// (the machine keeps its clone between jobs, so this lasts there).
func (x *Worker) excludeNotes() {
	_, out := run(x.Wt, false, "rev-parse", "--path-format=absolute", "--git-common-dir")
	common := strings.TrimSpace(out)
	if common == "" {
		return
	}
	exclude := filepath.Join(common, "info", "exclude")
	mkdirAll(filepath.Dir(exclude))
	var lines []string
	if exists(exclude) {
		lines = py.Splitlines(readText(exclude, false))
	}
	// A one-shot's scratch and its agents' worktrees (`.fullsend/`, #60) never ship either.
	var missing []string
	for _, name := range []string{NotesFile, journal.JournalFile, FullsendDir, PlanDraftFile} {
		if !contains(lines, "/"+name) {
			missing = append(missing, "/"+name)
		}
	}
	if len(missing) > 0 {
		writeText(exclude, strings.Join(append(append([]string{}, lines...), missing...), "\n")+"\n")
	}
}

// Start the notes file from the handoff's notes (a planner's plan, or an earlier
// builder's notes), so this run's builder keeps them going, and the draft file from a
// cut-off planner's draft, so this run's planner does; and put the item's journal beside
// them (#342).
func (x *Worker) seedNotes() {
	handoff := pyjson.Obj(x.Plan.Get("handoff"))
	cwd := x.Wt.Cwd
	if notes := py.OrStr(handoff.Get("notes"), ""); strings.TrimSpace(notes) != "" {
		writeText(filepath.Join(cwd, NotesFile), py.Rstrip(notes)+"\n")
	}
	if draft := py.OrStr(handoff.Get("draft"), ""); strings.TrimSpace(draft) != "" {
		writeText(filepath.Join(cwd, PlanDraftFile), py.Rstrip(draft)+"\n")
	}
	if text := py.OrStr(x.Plan.Get("journal"), ""); strings.TrimSpace(text) != "" {
		writeText(filepath.Join(cwd, journal.JournalFile), py.Rstrip(text)+"\n")
	}
}

// Handoff is what the next agent needs if this run did not finish the item: the builder's notes
// and the end of its last session. None (nil) when there is neither.
func (x *Worker) Handoff() *pyjson.Object {
	if x.Wt == nil {
		return nil
	}
	notesPath := filepath.Join(x.Wt.Cwd, NotesFile)
	notes := ""
	if isFile(notesPath) {
		notes = readText(notesPath, true)
		// A planning run's notes are its plan, which starts at the top; a builder's log is
		// newest at the end.
		if x.Plan.Get("action") == "plan" {
			notes = py.SliceTo(notes, NotesChars)
		} else {
			notes = journal.Compact(notes, NotesChars)
		}
	}
	trail := ""
	if trailOf, ok := x.Runner.(trailer); ok && x.BuildTranscript != "" && exists(x.BuildTranscript) {
		trail = trailOf.Trail(x.BuildTranscript)
	}
	draft := x.draft()
	if strings.TrimSpace(notes) == "" && strings.TrimSpace(trail) == "" && draft == "" {
		return nil
	}
	handoff := pyjson.New("provider", x.Provider.ID, "family", x.Provider.Family,
		"at", clock.ISO(x.Now()), "reason", py.OrStr(x.Result.Get("reason"), ""),
		"notes", redact.Redact(notes, nil), "trail", redact.Redact(trail, nil))
	if draft != "" {
		handoff.Set("draft", redact.Redact(draft, nil))
	}
	if draft != "" || (x.Planning && !isObj(x.Result.Get("plan"))) {
		// An unfinished plan: deliver keeps it as a `draft`, for the next planner only.
		handoff.Set("planning", true)
	}
	return handoff
}

// The planner's draft, while no plan is finished: its beginning, where the goal and
// the first steps are. A finished plan supersedes it, so a run that has one hands on none.
func (x *Worker) draft() string {
	if x.Wt == nil {
		return ""
	}
	path := filepath.Join(x.Wt.Cwd, PlanDraftFile)
	if !isFile(path) || isObj(x.Result.Get("plan")) {
		return ""
	}
	text := strings.TrimSpace(readText(path, true))
	if py.Len(text) > issueplan.PlanChars {
		text = py.Rstrip(py.SliceTo(text, issueplan.PlanChars-py.Len(PlanCut))) + PlanCut
	}
	return text
}

// HandoffText is what a first prompt gets when other agents worked on this before: the last
// one's handoff, and the journal of every earlier run (#342).
func (x *Worker) HandoffText(builder bool /* = True */) string {
	return x.handoffSection(builder) + x.journalText(builder)
}

// What a planner is told when an earlier planning run was cut off before it finished:
// where its draft is, and the end of its session.
func (x *Worker) draftSection(handoff *pyjson.Object) string {
	draft := strings.TrimSpace(py.OrStr(handoff.Get("draft"), ""))
	trail := strings.TrimSpace(py.OrStr(handoff.Get("trail"), ""))
	if draft == "" && trail == "" {
		return ""
	}
	parts := []string{"## An unfinished plan from an earlier run\n\nA planning run on `" +
		py.Str(handoff.GetOr("provider", "?")) + "` (" + py.Str(handoff.GetOr("family", "?")) +
		") was cut off before it finished: " + py.OrStr(handoff.Get("reason"), "no reason recorded") + "."}
	if draft != "" {
		parts[0] += " It kept a draft as it went, which is now `" + PlanDraftFile + "` at the " +
			"top of your worktree, and below. Go on from it rather than starting " +
			"over: check what it says against the code before you keep it, finish " +
			"what it had not reached, and keep the file up to date as you go."
		parts = append(parts, prompts.Data(draft, "Its draft"))
	} else {
		parts[0] += " It left no draft, only the end of its session below: use what it had " +
			"found rather than reading it all again."
	}
	if trail != "" {
		parts = append(parts, prompts.Data(trail, "The end of its session"))
	}
	return "\n\n" + strings.Join(parts, "\n\n")
}

// The section a first prompt gets when earlier runs left a journal (`journal.py`).
func (x *Worker) journalText(builder bool /* = True */) string {
	runs := journal.RunsIn(py.OrStr(x.Plan.Get("journal"), ""))
	if len(runs) == 0 {
		return ""
	}
	shown := runs
	if len(runs) > JournalRunsShown {
		shown = runs[len(runs)-JournalRunsShown:]
	}
	lines := make([]string, len(shown))
	for i, r := range shown {
		lines[i] = "- " + r
	}
	listed := strings.Join(lines, "\n")
	if len(runs) > len(shown) {
		listed = "- … " + strconv.Itoa(len(runs)-len(shown)) + " earlier run(s)\n" + listed
	}
	text := "\n\n## The journal of earlier runs\n\n" + strconv.Itoa(len(runs)) + " earlier run(s) " +
		"worked on this. Read the `## State` section of `" + NotesFile + "` first: in a minute it " +
		"says what is done, what is left and what did not work. The full journal is `" +
		journal.JournalFile + "` at the top of your worktree (git ignores it), with what each run " +
		"did step by step; read the runs you need for detail, not the whole file up front (#695)."
	if builder {
		text += " Then bring the `## State` part of `" + NotesFile + "` up to date with it, " +
			"in a few lines: what is done, what is left, and what was tried and did not " +
			"work, so that you and whoever comes after you need not read the whole " +
			"journal again. Then go on from where the last run stopped; do not redo " +
			"what an earlier run finished."
	}
	return text + "\n\n" + prompts.Data(listed, "Earlier runs, oldest first")
}

// The section a first prompt gets when another agent worked on this before.
func (x *Worker) handoffSection(builder bool /* = True */) string {
	handoff, ok := x.Plan.Get("handoff").(*pyjson.Object)
	if !ok || handoff == nil {
		return ""
	}
	provider := py.Str(handoff.GetOr("provider", "?"))
	family := py.Str(handoff.GetOr("family", "?"))
	if handoff.Get("kind") == "draft" {
		// A builder starts from a finished plan (this run's planner's, or the issue's),
		// never from a draft; a planner goes on from it.
		if builder {
			return ""
		}
		return x.draftSection(handoff)
	}
	if handoff.Get("kind") == "plan" {
		if strings.TrimSpace(py.OrStr(handoff.Get("notes"), "")) == "" {
			return ""
		}
		if x.Plan.Truthy("plan_in_issue") {
			return "\n\n## The plan\n\nThis was planned before anyone built it. " +
				"The plan is the **Plan** section of the issue's description above (a " +
				"person may have edited it since, and that version is the plan), and it is " +
				"at the top of `" + NotesFile + "`, which you keep going. Follow it step by " +
				"step unless the code shows it is wrong, and say where you departed from " +
				"it and why."
		}
		return "\n\n## The plan\n\nA planning run on `" + provider + "` (" + family + ") wrote the " +
			"plan below before anyone built this; it is also at the top of `" + NotesFile + "`, " +
			"which you keep going. Follow it unless the code shows it is wrong, and say " +
			"where you departed from it and why.\n\n" +
			prompts.Data(py.Str(handoff.Get("notes")), "The plan")
	}
	where := "on the branch"
	if x.WipUsed != "" {
		where = "in your worktree (see above)"
	}
	parts := []string{"## Picking up from another agent\n\nAn earlier run on `" + provider + "` (" +
		family + ") worked on this and stopped: " + py.OrStr(handoff.Get("reason"), "no reason recorded") +
		". Its work so far is " + where + ". Below are the notes it kept and the end of its " +
		"session. Check them against the diff and re-run the checks before you trust any of it. " +
		"Go on from where it stopped rather than starting over."}
	if strings.TrimSpace(py.OrStr(handoff.Get("notes"), "")) != "" {
		parts = append(parts, prompts.Data(py.Str(handoff.Get("notes")), "Its notes"))
	}
	if strings.TrimSpace(py.OrStr(handoff.Get("trail"), "")) != "" {
		parts = append(parts, prompts.Data(py.Str(handoff.Get("trail")), "The end of its session"))
	}
	return "\n\n" + strings.Join(parts, "\n\n")
}

// What a revision is told about the last cut-off run's work (#317 part 3).
func (x *Worker) wipText() string {
	if x.WipUsed != "" {
		return "\n\n## Unfinished work from the last run\n\nThe last revision run was cut " +
			"off before it finished. Your worktree starts from `" + x.WipUsed + "`, " +
			"which holds its commits (\"" + config.CommitPrefix + ": work in progress …\") on " +
			"top of the pull request: go on from where it stopped rather than starting over, and " +
			"check what it did against the task before you build on it."
	}
	if found := pyjson.Obj(x.Result.Get("wip")); found != nil && found.Truthy("ignored") {
		return "\n\n## Unfinished work from the last run\n\nThe last revision run was cut " +
			"off before it finished, but the pull request's branch moved after it " +
			"stopped, so you start from the branch as it is now; that run's commits are " +
			"not in your worktree, and its notes, if any, say what it had done."
	}
	return ""
}

func (x *Worker) branchState() string {
	log := x.Wt.Log(x.BaseRef, "HEAD")
	if log == "" {
		return "The branch starts at `main` with no commits of its own yet."
	}
	stat := x.Wt.Diffstat(x.BaseRef, "HEAD")
	return prompts.Data(log+"\n\n"+stat, "Commits on this branch beyond main")
}

// GateList is work.Worker._gate_list.
func (x *Worker) GateList() string {
	lines := []string{"The harness's checks, run in this order after you stop:"}
	lines = append(lines, "- install: `"+x.Cfg.Install.Run+"` (again whenever a manifest changed)")
	for _, g := range x.Gates {
		lines = append(lines, "- "+g.Name+": `"+g.Run+"`")
	}
	lines = append(lines, "CI on the pull request also runs 200 fuzz seeds, the web's typecheck, "+
		"eslint and unit tests, the e2e smoke specs and, when the change touches the "+
		"database, the Postgres suites before anything merges; the AI gates, "+
		"coverage and the full e2e suite run daily.")
	var left []string
	for _, g := range x.Cfg.Gates {
		if !x.runsGate(g) {
			left = append(left, g.Name+" (`"+g.Run+"`)")
		}
	}
	if len(left) > 0 {
		lines = append(lines, "This run is on the bot's shared machine, so the harness leaves "+
			strings.Join(left, ", ")+" to CI on the pull request, which runs before anything merges. "+
			"Don't run whole suites yourself either (`cargo test --workspace`, `cargo clippy "+
			"--workspace`, `cargo jackioh fuzz`, `cargo jackioh gate`, the web's whole vitest "+
			"run, e2e): check only what you changed, such as `cargo test -p <crate> <test "+
			"name>`, `cargo clippy -p <crate>` or `pnpm --dir apps/web test <test file>` "+
			"(which builds the WASM module the web's tests load first).")
	}
	return strings.Join(lines, "\n")
}

// runsGate is `g in self.gates` (a dataclass: equal fields).
func (x *Worker) runsGate(g *config.Gate) bool {
	for _, mine := range x.Gates {
		if *mine == *g {
			return true
		}
	}
	return false
}

func (x *Worker) findingsText(findings []*verdicts.Finding, label string /* = "Blocking findings" */) string {
	if len(findings) == 0 {
		return "No blocking findings were recorded."
	}
	lines := make([]string, len(findings))
	for i, f := range findings {
		lines[i] = f.Markdown()
	}
	return prompts.Data(strings.Join(lines, "\n"), label)
}

// FirstWhy is why the run's first builder pass runs, for the journal.
func (x *Worker) FirstWhy(role string, conflicts []string) string {
	handoff := pyjson.Obj(x.Plan.Get("handoff"))
	var why string
	if role == "revise" {
		why = "revise the pull request (" + py.OrStr(x.Plan.Get("source"), "request") + ")"
		if len(conflicts) > 0 {
			why += fmt.Sprintf("; `main` merged with conflicts in %d file(s)", len(conflicts))
		}
	} else {
		why = "build the issue"
		if x.Plan.Truthy("plan_in_issue") || handoff.Get("kind") == "plan" {
			why += " from its plan"
		}
	}
	if kind := handoff.Get("kind"); handoff.Len() > 0 && kind != "plan" && kind != "draft" {
		why += "; picking up from `" + py.Str(handoff.GetOr("provider", "?")) + "`, which stopped: " +
			py.OrStr(handoff.Get("reason"), "no reason recorded")
	}
	if x.WipUsed != "" {
		why += "; starting from `" + x.WipUsed + "`"
	}
	return why
}

// FirstPrompt is work.Worker._first_prompt: the first builder pass's role and prompt.
func (x *Worker) FirstPrompt(conflicts []string) (string, string) {
	plan := x.Plan
	number := x.planItem("number")
	if x.planItem("action") == "revise" {
		var merged string
		if len(conflicts) > 0 {
			merged = prompts.Data(strings.Join(conflicts, "\n"), "Files merged with conflict markers")
		} else if len(x.Resolved) > 0 {
			merged = "Merging `main` left conflicts, and the harness resolved every one (below). " +
				"The merge is staged, not committed: the harness commits it after you."
		} else {
			merged = "The merge of `main` into the branch was clean."
		}
		conflictText := merged + x.resolvedNote() + x.CarryBuild
		prompt := x.Render("revise", pyjson.New(
			"number", number, "repo", x.Cfg.Repo, "branch", x.planItem("branch"), "base", x.BaseSHA,
			"source", plan.GetOr("source", "request"), "pull", plan.GetOr("pull", ""),
			"issue", plan.GetOr("issue", ""), "feedback", plan.GetOr("feedback", ""),
			"conflicts", conflictText, "branch_state", x.branchState(),
			"gate_list", x.GateList(),
		))
		return "revise", prompt + x.wipText() + x.HandoffText(true)
	}
	previous := ""
	if plan.Truthy("previous_pr") {
		kept := ""
		if plan.Truthy("previous_branch") {
			kept = "; its branch is kept as `" + py.Str(plan.Get("previous_branch")) + "` if you want to look"
		}
		previous = "An earlier pull request for this issue, #" + py.Str(plan.Get("previous_pr")) +
			", was closed and this issue built again from `main`: " +
			py.OrStr(plan.Get("previous_why"), "it kept failing") + kept + ". Start from " +
			"`main`, learn from what went wrong, and do not repeat it.\n\n"
	}
	if prior := plan.Get("previous_findings"); pyjson.Truthy(prior) {
		var findings []*verdicts.Finding
		for _, f := range pyjson.List(prior) {
			findings = append(findings, finding(f))
		}
		previous += "An earlier run left this unfinished. The last review's blocking " +
			"findings were:\n\n" + x.findingsText(findings, "Blocking findings")
	}
	if question := strings.TrimSpace(py.OrStr(plan.Get("previous_question"), "")); question != "" {
		previous += "\n\nAn earlier run stopped to ask the question below. The answer, if " +
			"someone gave one, is in the comments of the task above.\n\n" +
			prompts.Data(question, "The question asked")
	}
	if len(conflicts) > 0 {
		previous += "\n\n`main` moved since then; merging it left conflict markers in: " +
			backticked(conflicts) + ". Resolve them, keeping both sides' meaning. Do not commit."
	}
	previous += x.resolvedNote()
	if x.PlanText != "" {
		previous += "\n\nA planner wrote the plan below for this change before you started; " +
			"it is also at the top of `" + NotesFile + "`. Follow it unless the code " +
			"shows it is wrong, and say where you departed from it and why.\n\n" +
			prompts.Data(x.PlanText, "The plan")
	}
	branchState := x.branchState()
	if plan.Truthy("onto") {
		branchState = pyFormat(split.PartNote, map[string]any{"parent": plan.Get("part_of"),
			"onto": x.planItem("onto")}) + branchState
	}
	values := pyjson.New("number", number, "repo", x.Cfg.Repo, "branch", x.planItem("branch"),
		"base", x.BaseSHA, "thread", plan.GetOr("thread", ""), "branch_state", branchState,
		"previous", previous, "gate_list", x.GateList())
	var prompt string
	if mode := plan.Get("mode"); mode == "oneshot" {
		values.Set("slice_prefix", config.BranchPrefix+"oneshot-"+py.Str(number)+"/")
		prompt = x.Render("oneshot", values)
	} else if mode == "fullsend" && plan.Truthy("children") {
		var parked []string
		for _, b := range pyjson.List(plan.Get("parked")) {
			parked = append(parked, py.Str(b))
		}
		parkedText := "   None: every part landed on the branch."
		if len(parked) > 0 {
			lines := make([]string, len(parked))
			for i, b := range parked {
				lines[i] = "   - `origin/" + b + "`"
			}
			parkedText = strings.Join(lines, "\n")
		}
		values.Set("children", x.planItem("children"))
		values.Set("parked", parkedText)
		prompt = x.Render("reconcile", values)
	} else {
		prompt = x.Render("build", values)
	}
	return "build", prompt + x.HandoffText(true)
}

// What a conflict revision of a cleared change tells its builder and its reviewer, when
// it starts from the commit the review rule cleared: only its own review stands between
// the resolution and `main`. ("", "") for any other run. Deliver decides the carry from
// git alone (`deliver._carry`).
func (x *Worker) clearedNotes(conflicts []string) (string, string) {
	clear := pyjson.Obj(x.Plan.Get("cleared"))
	if sha, _ := clear.Get("sha").(string); x.Plan.Get("action") != "revise" ||
		x.Plan.Get("source") != "conflict" || clear == nil || x.StartSHA == "" ||
		!isStr(clear.Get("sha")) || sha != x.StartSHA {
		return "", ""
	}
	by := ""
	if clear.Truthy("by") {
		by = " (" + py.Str(clear.Get("by")) + ")"
	}
	clearedAt := "at `" + first12(x.StartSHA) + "`" + by
	build := "\n\nThe reviews had cleared this change " + clearedAt + " before `main` moved. " +
		"Resolve the conflicts and change nothing else: if this revision changes only " +
		"the conflicted files and its reviewer approves, it merges without another " +
		"review run. If making it correct takes more (say `main` renamed something the " +
		"change uses), make that change anyway and say so in your report: it then goes " +
		"back to a review run."
	var parts []string
	for _, r := range x.Resolved {
		parts = append(parts, "the harness resolved `"+r.Path+"` itself ("+r.How+")")
	}
	if len(conflicts) > 0 {
		parts = append(parts, "the builder resolved the conflicts in "+backticked(conflicts))
	}
	what := "the merge was clean"
	if len(parts) > 0 {
		what = strings.Join(parts, ", and ")
	}
	review := "This revision resolves a conflict on a change the review rule had already " +
		"cleared " + clearedAt + ": `main` moved and the branch no longer merged, so the " +
		"harness merged `main` into it and " + what + ". If you approve and the revision " +
		"changed nothing beyond those files, it merges with no further review: yours " +
		"is the only review of the resolution, so judge it hardest. In each conflicted " +
		"file the builder resolved, both `main`'s change and the branch's change must " +
		"survive with their meaning (the files the harness resolved follow its rule, " +
		"below), nothing `main` brought may be dropped or reverted, and the merged " +
		"whole must still be correct. `git log --merges -1` finds the merge commit, and " +
		"`git show --remerge-diff <it>` shows how each conflict was resolved."
	return build, review
}

func (x *Worker) fixPrompt(cycle int, findings []*verdicts.Finding, failures string, label string /* = "Blocking findings" */) string {
	values := pyjson.New(
		"number", x.planItem("number"), "repo", x.Cfg.Repo, "branch", x.planItem("branch"),
		"base", x.BaseSHA, "cycle", cycle, "max_cycles", x.CyclesCap(),
		"thread", x.Plan.GetOr("thread", ""), "branch_state", x.branchState(),
		"findings", x.findingsText(findings, label))
	gateFailures := "Every check passed or was already red on main."
	if failures != "" {
		gateFailures = prompts.Data(failures, "Checks this change turned red")
	}
	values.Set("gate_failures", gateFailures)
	values.Set("gate_list", x.GateList())
	return x.Render("fix", values) + x.resolvedNote()
}

func (x *Worker) commit(message string) bool {
	return x.Wt.CommitAll(message, x.Who)
}

// Changed files that still hold a conflict marker line.
func (x *Worker) unresolvedMarkers() []string {
	var changed []string
	for _, p := range x.Wt.ChangedPaths(x.BaseRef, "HEAD") {
		if isFile(filepath.Join(x.Wt.Cwd, p)) {
			changed = append(changed, p)
		}
	}
	if len(changed) == 0 {
		return []string{}
	}
	_, out := run(x.Wt, false, append([]string{"grep", "-l", "-E", git.MarkerLine, "--"}, changed...)...)
	found := []string{}
	for _, p := range py.Splitlines(out) {
		if strings.TrimSpace(p) != "" {
			found = append(found, p)
		}
	}
	return found
}

// Commits the branch may carry content from: where it started, and the main it merged.
func (x *Worker) anchors() []string {
	var out []string
	for _, s := range []string{x.StartSHA, x.BaseSHA} {
		if s != "" {
			out = append(out, s)
		}
	}
	return out
}

// Guard is Worker._guard as the run calls it; a variable so the tests can replace it
// (`mock.patch.object(Worker, "_guard", ...)`).
var Guard = (*Worker).guard

// Put back forbidden paths the work changed, and report them, conflict markers left
// behind, and an empty change.
func (x *Worker) guard() []*verdicts.Finding {
	found := []*verdicts.Finding{}
	touched := x.Wt.Unsanctioned("HEAD", x.anchors(), x.Cfg.ForbiddenPaths)
	x.PutBack = touched
	if len(touched) > 0 {
		x.Wt.RestoreFrom(x.anchors(), touched)
		x.commit(config.CommitPrefix + ": put back paths the bot may not change")
		found = append(found, newFinding(
			"blocking", touched[0],
			"The change edited "+strings.Join(touched, ", ")+", which the bot may not change ("+
				strings.Join(x.Cfg.ForbiddenPaths, ", ")+"); the harness put them back.",
			"the harness's path guard",
		))
	}
	if markers := x.unresolvedMarkers(); len(markers) > 0 {
		found = append(found, newFinding("blocking", markers[0],
			"Conflict markers are still in "+strings.Join(markers, ", ")+".", "git grep"))
	}
	if x.planItem("action") == "build" && len(x.Wt.ChangedPaths(x.BaseRef, "HEAD")) == 0 {
		found = append(found, newFinding("blocking", "task", "The branch has no change from main.",
			"git diff main...HEAD is empty"))
	}
	return found
}

// Install again when a manifest changed (or never succeeded), then every gate.
func (x *Worker) checks() []*gates.GateResult {
	results := []*gates.GateResult{}
	stale := x.InstalledAt == ""
	if !stale {
		for _, p := range x.Wt.NamesBetween(x.InstalledAt, "HEAD") {
			if IsManifest(p) {
				stale = true
				break
			}
		}
	}
	if stale {
		install := x.install()
		results = append(results, install)
		if !install.OK {
			for _, g := range x.Gates {
				results = append(results, &gates.GateResult{Name: g.Name, Run: g.Run, OK: false,
					ExitCode: -1, Seconds: 0.0, Tail: "", Skipped: "the install failed"})
			}
			x.checksStep(results)
			return results
		}
	}
	results = append(results, gates.RunAll(x.Gates, x.Wt.Cwd, x.Env, x.SecondsLeft)...)
	x.readMemory() // the install and the typecheck are what a machine job's memory goes on
	x.MarkPreExisting(results)
	x.checksStep(results)
	return results
}

// A check run, for the journal, then a checkpoint.
func (x *Worker) checksStep(results []*gates.GateResult) {
	var red []string
	took := 0.0
	for _, r := range results {
		if !r.OK && r.Skipped == "" {
			name := r.Name
			if r.PreExisting {
				name += " (red on main too)"
			}
			red = append(red, name)
		}
		took += r.Seconds
	}
	why := ""
	if x.Wt != nil {
		why = "on `" + first12(x.Wt.Head()) + "`"
	}
	outcome := "green"
	if len(red) > 0 {
		outcome = "red: " + strings.Join(red, ", ")
	}
	x.Step("checks", why, pyjson.New("minutes", py.RoundN(took/60, 1), "outcome", outcome))
	x.checkpoint()
}

// MarkPreExisting runs each gate this change left red again on the untouched base. A gate that
// timed out is not: it is inconclusive (`gates.mark_inconclusive`), and on the base it would only
// spend its whole timeout again.
func (x *Worker) MarkPreExisting(results []*gates.GateResult) {
	names := map[string]bool{}
	for _, g := range x.Gates {
		names[g.Name] = true
	}
	gates.MarkInconclusive(results, names)
	for _, result := range results {
		if result.OK || result.Skipped != "" || result.Inconclusive || result.Name == x.Cfg.Install.Name {
			continue
		}
		var gate *config.Gate
		for _, g := range x.Gates {
			if g.Name == result.Name {
				gate = g
				break
			}
		}
		if gate == nil {
			continue
		}
		if !x.BaseGateCache.Has(result.Name) {
			base := x.baseWorktree()
			if base == nil {
				return
			}
			timeout := int(x.SecondsLeft())
			again := gates.RunGate(gate, base.Cwd, x.Env, &timeout)
			x.BaseGateCache.Set(result.Name, !again.OK)
		}
		result.PreExisting = x.BaseGateCache.Bool(result.Name)
	}
}

// baseWorktree is work.Worker._base_worktree. As in Python, a base whose install failed is kept
// in `BaseWt` all the same, so the next red gate is run on it without asking again.
func (x *Worker) baseWorktree() *git.Git {
	if x.BaseWt == nil {
		if x.SecondsLeft() < 20*60 {
			return nil
		}
		path := filepath.Join(x.WorkDir, "base")
		x.BaseWt = git.WorktreeAdd(x.Repo, path, "bot-base-check", x.BaseSHA)
		timeout := int(x.SecondsLeft())
		if install := gates.RunGate(x.Cfg.Install, path, x.Env, &timeout); !install.OK {
			return nil
		}
	}
	return x.BaseWt
}

// review is `self._review(...)` as the run calls it: through ReviewFunc when a test replaced it.
func (x *Worker) review(cycle int, report *verdicts.BuildReport, results []*gates.GateResult, previous []*verdicts.Finding, context string, maxCycles *int, role string) *verdicts.Review {
	if x.ReviewFunc != nil {
		return x.ReviewFunc(cycle, report, results, previous, context, maxCycles, role)
	}
	return x.Review(cycle, report, results, previous, context, maxCycles, role)
}

// Review is work.Worker._review.
func (x *Worker) Review(cycle int, report *verdicts.BuildReport, results []*gates.GateResult, previous []*verdicts.Finding, context string /* = "" */, maxCycles *int /* = None */, role string /* = "review" */) *verdicts.Review {
	diff, cut := x.Wt.Diff(x.BaseRef, "HEAD", DiffInPrompt)
	note := "The whole diff against the base is below."
	if cut {
		note = "The diff below is cut short; run `git diff " + x.BaseSHA + "...HEAD` for the rest."
	}
	cycles := 0
	if maxCycles != nil {
		cycles = *maxCycles
	}
	if cycles == 0 {
		cycles = x.CyclesCap()
	}
	gatesText := "The harness ran no checks in this run; CI runs every check on the pull " +
		"request before anything merges, and you may run any of them yourself."
	if len(results) > 0 {
		gatesText = gates.Table(results)
	}
	previousFindings := context
	if previousFindings == "" {
		previousFindings = "This is the first review of this change."
		if len(previous) > 0 {
			previousFindings = x.findingsText(previous, "Blocking findings")
		}
	}
	prompt := x.Render("review", pyjson.New(
		"number", x.planItem("number"), "repo", x.Cfg.Repo, "cycle", cycle,
		"max_cycles", cycles, "branch", x.planItem("branch"),
		"base", x.BaseSHA, "thread", x.Plan.GetOr("thread", ""),
		"report", prompts.Data(or(report.Body, "(the builder wrote no report)"), "Builder's report"),
		"gates", gatesText,
		"previous_findings", previousFindings,
		"diff_note", note, "diff", prompts.Data(diff, "git diff main...HEAD"),
	))
	head := x.Wt.Head()
	review := &verdicts.Review{Readable: false, Verdict: "unreadable", Findings: []*verdicts.Finding{}}
	what := "review"
	if role == "self_check" {
		what = "self check"
	}
	for attempt := 0; attempt < 2; attempt++ {
		why := what + " of round " + strconv.Itoa(cycle) + ", on `" + first12(head) + "`"
		if attempt > 0 {
			why += ", again: the last answer could not be read"
		}
		result := x.Call(role, prompt, x.Wt.Cwd, true, why, false)
		x.resetTo(head)
		review = verdicts.Review_(result.Text)
		x.cameOfIt(verdictOutcome(review, result))
		if review.Readable {
			break
		}
		if !result.OK {
			review.Error = py.SliceTo(redact.Redact(or(result.Error, "it failed without an error"), nil), 2000)
		}
		x.Check()
	}
	review.ReviewedSHA = head
	x.checkpoint()
	return review
}

// A planning session before any building: the planner reads the task and the code and
// writes the plan, which the harness puts at the top of the notes file (and so into the
// handoff) for the builder, this run's or a later one's. It keeps a draft as it goes
// (`PLAN_DRAFT_FILE`), which a cut-off run hands on (`_handoff`) for the next planner.
func (x *Worker) planning() {
	x.Check()
	seat := x.SeatFor("plan")
	prompt := x.Render("plan", pyjson.New(
		"number", x.planItem("number"), "repo", x.Cfg.Repo, "branch", x.planItem("branch"),
		"base", x.BaseSHA, "thread", x.Plan.GetOr("thread", ""),
		"branch_state", x.branchState(), "gate_list", x.GateList(),
		"difficulty", py.OrStr(x.Plan.Get("difficulty"), "medium"),
		"rating", x.ratingAsk(), "easy_rule", x.easyRule(),
		"plan_words", py.Thousands(issueplan.PlanWords), "plan_chars", py.Thousands(issueplan.PlanChars),
		"draft_file", PlanDraftFile,
	)) + x.HandoffText(false)
	head := x.Wt.Head()
	x.Planning = true
	why := "plan the item before anyone builds it"
	if isFile(filepath.Join(x.Wt.Cwd, PlanDraftFile)) {
		why += ", going on from the last planner's draft"
	}
	result := x.Call("plan", prompt, x.Wt.Cwd, true, why, true)
	x.resetTo(head)
	rated, text := verdicts.Rating(redact.Redact(strings.TrimSpace(result.Text), nil))
	if py.Len(text) > issueplan.PlanChars {
		text = py.Rstrip(py.SliceTo(text, issueplan.PlanChars-py.Len(PlanCut))) + PlanCut
	}
	if !result.OK || text == "" {
		msg := "the planner on " + seat.Model + " wrote no plan"
		if result.Error != "" {
			msg += ": " + result.Error
		}
		herr.Raise(&py.Error{Type: "RuntimeError", Msg: msg})
	}
	if len(rated) > 0 && x.ratingSource() == "person" {
		rated = nil // a person's label stands; the planner was told not to rate it
	}
	x.PlanText = text
	notes := filepath.Join(x.Wt.Cwd, NotesFile)
	earlier := ""
	if isFile(notes) {
		earlier = readText(notes, true)
	}
	writeText(notes, "# Plan ("+seat.Model+", "+seat.Tier+")\n\n"+text+"\n\n# Notes\n\n"+earlier)
	plan := pyjson.New("seat", seat.ToDict(), "text", text)
	x.Result.Set("plan", plan)
	if len(rated) > 0 {
		plan.Set("rating", pyjson.New("difficulty", rated["difficulty"], "why", rated["why"]))
	}
	// The plan is finished: its draft is spent, and the builder never reads it.
	unlinkMissingOK(filepath.Join(x.Wt.Cwd, PlanDraftFile))
}

func (x *Worker) ratingSource() string {
	rating := pyjson.Obj(x.Plan.Get("rating"))
	return py.OrStr(rating.Get("source"), "")
}

// What the planner is told about rating the item (#317 part 8).
func (x *Worker) ratingAsk() string {
	rating := pyjson.Obj(x.Plan.Get("rating"))
	difficulty := py.OrStr(rating.Get("difficulty"), py.OrStr(x.Plan.Get("difficulty"), "medium"))
	if x.ratingSource() == "person" {
		return "A person set this item's difficulty: `difficulty:" + difficulty + "`. Plan for " +
			"that, and give no rating line. If the rule below says it is harder, say so " +
			"under **Open questions**."
	}
	line := "    <!-- bot: {\"difficulty\": \"easy\", \"why\": \"one line: the rule's line you were " +
		"least sure of, and why it holds\"} -->"
	ask := "Rate how hard it is, by the rule below. The very first line of your answer is a " +
		"single HTML comment carrying JSON, with nothing before it, then the plan:\n\n" +
		line + "\n\n`difficulty` is `easy`, `medium` or `hard`. The rating decides who " +
		"builds it: an easy item goes to the weakest model free, which may be unable to " +
		"fill gaps, resolve a hard conflict, or judge what it cannot see. When in doubt " +
		"between easy and medium, rate medium: a borderline easy costs a weak builder's " +
		"failed run, while a medium plan still builds fast."
	if x.ratingSource() == "bot" {
		by := ""
		if rating.Truthy("by") {
			by = " (`" + py.Str(rating.Get("by")) + "`)"
		}
		ask += " The bot rated it `difficulty:" + difficulty + "` already" + by +
			": you may rate it higher, never lower."
	}
	return ask
}

func (x *Worker) easyRule() string {
	rule := x.Cfg.Easy
	return pyTemplate(easy.Rule).safeSubstitute(map[string]string{
		"max_files": strconv.Itoa(rule.MaxFiles), "max_lines": strconv.Itoa(rule.MaxLines)})
}

// A round the reviewer sent back, built on `built_on`. On a lane with two models,
// `SWITCH_UP_AFTER` of them in a row on the weaker one move the builder to the stronger for
// the next pass; one the stronger built counts towards nothing.
func (x *Worker) sentBack(cycle int, builtOn *providers.Seat) {
	if len(x.Switchable) == 0 {
		return
	}
	strongest := strongestSeat(x.Switchable)
	if builtOn.Tier == strongest.Tier {
		x.SentBack = 0
		return
	}
	x.SentBack++
	if x.SentBack >= SwitchUpAfter {
		x.Switch(strongest.Model, fmt.Sprintf("the reviewer sent back %d rounds in a row on `%s`",
			x.SentBack, builtOn.Model), cycle)
	}
}

// A build run whose planner rated the item (an unrated one, or one the bot rated before
// and the planner rated higher) stops after planning when the rating needs a stronger model
// than this run's builder, or a stronger planner than this one (`PLAN_FLOOR`): the item goes
// back to the queue, rated, for those. A lane with two models switches instead.
func (x *Worker) ratedOut() bool {
	if !isObj(x.Result.Obj("plan").Get("rating")) || x.ratingSource() == "person" {
		return false
	}
	difficulty := x.Difficulty()
	planner := x.SeatFor("plan")
	if providers.TierAtLeast(planner.Tier, config.PlanFloor[difficulty]) && len(x.Switchable) > 0 {
		// A lane with two models builds what its planner rated on the weakest of them that
		// meets the rating, up or down: no need to send the item back to the queue.
		var fits []*providers.Seat
		for _, seat := range x.Switchable {
			if providers.TierAtLeast(seat.Tier, config.MinTier[difficulty]) {
				fits = append(fits, seat)
			}
		}
		if len(fits) > 0 {
			seat := weakestSeat(fits)
			x.Switch(seat.Model, "its planner rated it difficulty:"+difficulty, 0)
		}
	}
	if providers.TierAtLeast(x.BuildSeat.Tier, config.MinTier[difficulty]) &&
		providers.TierAtLeast(planner.Tier, config.PlanFloor[difficulty]) {
		return false
	}
	role := "build"
	if !providers.TierAtLeast(planner.Tier, config.PlanFloor[difficulty]) {
		role = "plan"
	}
	x.Result.Set("status", "planned")
	x.Result.Set("reason", "rated difficulty:"+difficulty+", which needs a stronger model than "+
		"this run's to "+role+" it")
	return true
}

// A planning run: the plan, and nothing built. The deliver job keeps it as the item's
// handoff and queues the item to build from it.
func (x *Worker) planRun() {
	x.prepare(false, false)
	x.planning()
	if !x.needsAPerson() {
		x.Result.Set("status", "planned")
		x.Result.Set("reason", "planned on "+x.SeatFor("plan").Model)
	}
	x.finish()
}

// A plan that leaves the bot nothing it may build (#793, `human.plan_needs`): the run
// ends `blocked`, flagged `human_required`, before any builder is spent on it, and the
// plan goes into the description for the person who does it.
func (x *Worker) needsAPerson() bool {
	why := human.PlanNeeds(x.PlanText, x.Cfg.ForbiddenPaths)
	if why == "" {
		return false
	}
	x.cameOfIt("its plan leaves nothing the bot may do: a person must")
	x.Result.Set("status", "blocked")
	x.Result.Set("human_required", true)
	x.Result.Set("question", why)
	x.Result.Set("reason", "its plan leaves nothing the bot may do")
	return true
}

// One builder session and its commit. Returns its report, or None (nil) when it stopped to
// ask a person (the run then ends as `blocked`), and the round's record.
func (x *Worker) buildPass(cycle int, role string, prompt string, why string /* = "" */) (*verdicts.BuildReport, *pyjson.Object) {
	before := x.Wt.Head()
	built := x.Call(role, prompt, x.Wt.Cwd, false, or(why, fmt.Sprintf("round %d: %s", cycle, role)), false)
	report := verdicts.BuildReport_(built.Text)
	builder := pyjson.New("role", role, "ok", built.OK, "turns", intOrNone(built.Turns),
		"minutes", py.RoundN(built.DurationS/60, 1), "status", report.Status,
		"error", noneIfEmpty(built.Error), "timed_out", built.TimedOut,
		"model", x.BuildSeat.Model, "tier", x.BuildSeat.Tier)
	entry := pyjson.New("n", cycle, "builder", builder)
	if !built.OK && !built.TimedOut && x.Wt.Head() == before && len(x.Wt.Dirty()) == 0 {
		// A session that failed and changed nothing is no pass (#317 part 1): the checks and
		// the self check would only judge the branch as it was. One that failed after doing
		// work (Claude's turn limit, say) keeps it, and the checks and the review judge it.
		builder.Set("changed", false)
		x.Result.Set("status", "failed")
		x.Result.Set("reason", py.SliceTo(redact.Redact("the builder's session failed and changed "+
			"nothing: "+or(built.Error, "no error given"), nil), 2000))
		x.finish()
		return nil, entry
	}
	if report.Status == "blocked" {
		x.saveWip("the builder needs a decision")
		reason := "the builder needs a person to decide something"
		if report.HumanRequired {
			reason = "only a person can do it, its builder says"
		}
		x.Result.Set("status", "blocked")
		x.Result.Set("question", report.Question)
		x.Result.Set("report", report.Body)
		x.Result.Set("reason", reason)
		x.Result.Set("human_required", report.HumanRequired)
		x.finish()
		return nil, entry
	}
	x.commit(config.CommitPrefix + ": " + role + " pass " + strconv.Itoa(cycle) + " for #" +
		py.Str(x.planItem("number")))
	changed := x.Wt.Head() != before
	builder.Set("changed", changed)
	if built.OK {
		what := "changed nothing"
		if changed {
			what = "committed `" + first12(x.Wt.Head()) + "`"
		}
		x.cameOfIt(report.Status + "; " + what)
	}
	if report.NextModel != "" && len(x.Switchable) > 0 {
		// Taken once this round is judged, so its review counts against the model that
		// built it (`_sent_back`).
		builder.Set("next_model", report.NextModel)
		x.Asked = &AskedModel{Model: report.NextModel, Cycle: cycle}
	}
	x.checkpoint()
	return report, entry
}

// MergeOnly is a conflict revision with nothing left for a builder: the harness resolved every
// conflict the merge of `main` left (`_resolve`), and nothing else was asked of it (no new
// trusted feedback, no earlier findings, no cut-off work to pick up). Its checks and its
// reviewer judge the merge as it stands, so no builder session is spent on it; findings the
// review sends back go to the builder as on any round.
func (x *Worker) MergeOnly(conflicts []string) bool {
	plan := x.Plan
	return plan.Get("action") == "revise" && plan.Get("source") == "conflict" &&
		len(x.Resolved) > 0 && len(conflicts) == 0 && x.WipUsed == "" &&
		!plan.Truthy("previous_findings") &&
		threads.NoFeedback_(py.OrStr(plan.Get("feedback"), ""), py.MustInt(x.planItem("number")))
}

// Round one of a `_merge_only` revision: commit the merge as the harness resolved it,
// with no builder session. Returns the report the pull request shows in the builder's place
// and the round's record, shaped as `_build_pass`'s.
func (x *Worker) mergePass(cycle int) (*verdicts.BuildReport, *pyjson.Object) {
	before := x.Wt.Head()
	var paths, lines []string
	for _, r := range x.Resolved {
		paths = append(paths, r.Path)
		lines = append(lines, "- `"+r.Path+"`: "+r.How)
	}
	message := config.CommitPrefix + ": merge " + x.Cfg.DefaultBranch + " for #" +
		py.Str(x.planItem("number")) + "; the harness resolved " + strings.Join(paths, ", ")
	if !x.commit(message) && x.Wt.MergeInProgress() {
		// The merged tree is the branch's own (`main` brought nothing else): the merge commit
		// still records `main`, or the pull request would stay in conflict.
		x.Wt.Run(true, 600, map[string]string{
			"GIT_AUTHOR_NAME":     x.Who.Name,
			"GIT_AUTHOR_EMAIL":    x.Who.Email,
			"GIT_COMMITTER_NAME":  x.Who.Name,
			"GIT_COMMITTER_EMAIL": x.Who.Email,
		}, "commit", "--quiet", "--no-verify", "--allow-empty", "-m", message)
	}
	changed := x.Wt.Head() != before
	outcome := "changed nothing"
	if changed {
		outcome = "committed `" + first12(x.Wt.Head()) + "`"
	}
	x.Step("commit", "the merge as the harness resolved it; no builder session",
		pyjson.New("outcome", outcome))
	body := "## What changed\n\n`main` moved and the branch no longer merged. The harness " +
		"merged it and resolved every conflict itself, so no builder ran:\n\n" +
		strings.Join(lines, "\n")
	entry := pyjson.New(
		"n", cycle,
		"builder", pyjson.New("role", "merge", "ok", true, "turns", 0, "minutes", 0.0,
			"status", "done", "error", "", "timed_out", false, "model", "the harness", "tier", "",
			"changed", changed),
	)
	x.checkpoint()
	return &verdicts.BuildReport{Status: "done", Title: "", Question: "", Body: body}, entry
}

// Self checks until one is clean: a fresh session of the builder's own model reads the
// change as a reviewer would; on findings the builder fixes them, the checks run and the
// self check goes again, up to `max_self_check_rounds`. Returns the last report, checks and
// path guard, and the findings still open (none after a clean self check). The report is
// None (nil) when a fix stopped to ask a person.
func (x *Worker) selfCheckLoop(cycle int, entry *pyjson.Object, report *verdicts.BuildReport, results []*gates.GateResult, guard []*verdicts.Finding) (*verdicts.BuildReport, []*gates.GateResult, []*verdicts.Finding, []*verdicts.Finding) {
	limit := x.Cfg.MaxSelfCheckRounds
	rounds := []any{}
	entry.Set("self_check", rounds)
	flagged := []*verdicts.Finding{}
	for n := 1; n <= limit; n++ {
		x.Check()
		verdict := x.review(cycle, report, results, []*verdicts.Finding{},
			pyFormat(SelfCheckContext, map[string]any{"n": n, "cap": limit}), nil, "self_check")
		flagged = append(append([]*verdicts.Finding{}, guard...), verdict.Blocking()...)
		if !verdict.Readable {
			flagged = append(flagged, newFinding("blocking", "self check",
				"The self check gave no verdict: "+verdict.WhyUnreadable()+".", "the harness"))
		}
		if !gates.Green(results) {
			flagged = append(flagged, newFinding("blocking", "checks", "Checks this change turned red "+
				"must pass.", "the harness's gate run"))
		}
		round := pyjson.New("n", n, "review", verdict.ToDict(), "flagged", len(flagged))
		rounds = append(rounds, round)
		entry.Set("self_check", rounds)
		if len(flagged) == 0 || n == limit {
			break
		}
		x.Check()
		fixed, fixEntry := x.buildPass(cycle, "fix",
			x.fixPrompt(cycle, flagged, gates.FailuresText(results), "Your self check's blocking findings"),
			fixWhy(fmt.Sprintf("self check %d of round %d", n, cycle), flagged))
		round.Set("fix", fixEntry.Get("builder"))
		if fixed == nil {
			return nil, results, guard, flagged
		}
		report = fixed
		guard = Guard(x)
		x.Check()
		results = x.checks()
		round.Set("gates", gateDicts(results))
	}
	return report, results, guard, flagged
}

// Item is work.Worker._item: a build or a revision.
func (x *Worker) Item() {
	mode := py.OrStr(x.Plan.Get("mode"), "")
	if strings.HasPrefix(mode, "split") || (mode == "fullsend" && !x.Plan.Truthy("children")) {
		x.Split() // a fullsend with sub-issues already is its reconcile: a build (#505)
		return
	}
	if x.Plan.Truthy("onto") {
		x.Part()
		return
	}
	conflicts := x.prepare(true, true)
	x.CarryBuild, x.CarryReview = x.clearedNotes(conflicts)
	if x.PlanSeat != nil {
		x.planning()
		if x.needsAPerson() || x.ratedOut() {
			x.finish()
			return
		}
	}
	if x.roundsSpent() >= x.CyclesCap() && x.planIn("action", "build", "revise") {
		// Earlier runs used every verdict round with this owner (#725): no new review here,
		// so the kept work goes to its final audit untouched, with no model call spent.
		status, reason := x.roundsOut(x.roundsSpent())
		x.Result.Set("status", status)
		x.Result.Set("reason", reason)
		x.lastCheckpoint()
		x.finish()
		return
	}
	findings := []*verdicts.Finding{}
	for _, f := range pyjson.List(x.Plan.Get("previous_findings")) {
		findings = append(findings, finding(f))
	}
	failures := ""
	report := &verdicts.BuildReport{Status: "unknown", Title: "", Question: "", Body: ""}
	var review *verdicts.Review
	results := []*gates.GateResult{}
	openSelfCheck := []*verdicts.Finding{}
	// Verdict rounds (#725): only rounds that end in a readable review verdict count toward
	// the lane's cap. Usage pauses, infra failures, halts and stops return early with the
	// work kept, burning nothing; a new owner tier starts its own count.
	ownerTier := x.BuildSeat.Tier
	used := x.roundsSpent()
	laneVerdicts := 0
	x.Result.Set("round_tier", ownerTier)
	x.Result.Set("verdict_rounds", used)
	mergeOnly := x.MergeOnly(conflicts)
	cycle := 0
	for {
		cycle++
		x.Check()
		if x.Asked != nil {
			model, askedIn := x.Asked.Model, x.Asked.Cycle
			x.Asked = nil
			x.Switch(model, "the builder asked for it", askedIn)
		}
		builtOn := x.BuildSeat
		harnessRound := mergeOnly && cycle == 1
		var built *verdicts.BuildReport
		var entry *pyjson.Object
		if harnessRound {
			built, entry = x.mergePass(cycle)
		} else {
			var role, prompt, why string
			if cycle == 1 {
				role, prompt = x.FirstPrompt(conflicts)
				why = x.FirstWhy(role, conflicts)
			} else {
				role, prompt = "fix", x.fixPrompt(cycle, findings, failures, "Blocking findings")
				why = fixWhy(fmt.Sprintf("round %d's review", cycle-1), findings)
			}
			built, entry = x.buildPass(cycle, role, prompt, why)
		}
		appendTo(x.Result, "cycles", entry)
		if built == nil {
			return
		}
		report = built
		guard := Guard(x)
		if x.OnlyForbidden() {
			return
		}
		x.Check()
		results = x.checks()
		entry.Set("gates", gateDicts(results))
		if x.SelfCheck && !harnessRound {
			var checked *verdicts.BuildReport
			checked, results, guard, openSelfCheck = x.selfCheckLoop(cycle, entry, report, results, guard)
			if checked == nil {
				return
			}
			report = checked
		}
		if x.ReviewSeat == nil {
			// No seat here reviews in the run (a weak one only in a review run): a review
			// run judges it. But never with conflict markers left in it (#317 part 2): its
			// self checks may run out with them still open, as a weak builder's did on #203,
			// #214 and #287, and nothing after this would catch them before CI.
			var left *verdicts.Finding
			left, report = x.clearMarkers(cycle, entry, report)
			if report == nil {
				return
			}
			if left != nil {
				findings = []*verdicts.Finding{left}
				x.Result.Set("status", "not_approved")
				x.Result.Set("reason", "conflict markers are still in "+left.Where+" after its last fix")
				break
			}
			reason := "built; no model on this subscription may approve it, so it waits for its " +
				"final audit"
			if len(openSelfCheck) > 0 {
				reason += fmt.Sprintf(", with %d self-check finding(s) still open", len(openSelfCheck))
			}
			x.Result.Set("status", "built")
			x.Result.Set("reason", reason)
			break
		}
		x.Check()
		context := ""
		if len(openSelfCheck) > 0 {
			if len(findings) > 0 {
				context = x.findingsText(findings, "Blocking findings") + "\n\n"
			}
			context += "The builder's own self checks ran out with these findings still " +
				"open; judge them too:\n\n" +
				x.findingsText(openSelfCheck, "Open self-check findings")
		}
		if x.CarryReview != "" {
			context = x.withEarlier(context, findings, x.CarryReview)
		}
		if len(x.Resolved) > 0 {
			context = x.withEarlier(context, findings, x.resolvedReviewNote())
		}
		if x.catchUp() {
			// `main` moved and merged cleanly: the checks and the review judge the merged head.
			results = x.checks()
			entry.Set("gates", gateDicts(results))
			entry.Set("caught_up", x.BaseSHA)
			guard = Guard(x)
		}
		review = x.review(cycle, report, results, findings, context, nil, "review")
		entry.Set("review", review.ToDict())
		if !review.Readable {
			x.Result.Set("status", "not_approved")
			x.Result.Set("reason", review.WhyUnreadable())
			break
		}
		laneVerdicts++
		x.Result.Set("verdict_rounds", used+laneVerdicts)
		if review.Approved() && gates.Green(results) && len(guard) == 0 {
			x.ApprovedSHA = review.ReviewedSHA
			x.Result.Set("status", "approved")
			x.Result.Set("reason", "the reviewer approved and every check passed")
			break
		}
		findings = append(append([]*verdicts.Finding{}, guard...), review.Blocking()...)
		failures = gates.FailuresText(results)
		if len(findings) == 0 && failures != "" {
			findings = []*verdicts.Finding{newFinding("blocking", "checks", "Checks this change "+
				"turned red must pass.", "the harness's gate run")}
		}
		if !harnessRound {
			x.sentBack(cycle, builtOn) // a rejected harness merge is no builder's round
		}
		if laneVerdicts+used >= x.CyclesCap() {
			status, reason := x.roundsOut(laneVerdicts + used)
			x.Result.Set("status", status)
			x.Result.Set("reason", reason)
			break
		}
	}
	x.lastCheckpoint()
	status := x.Result.Get("status")
	var title any = report.Title
	if report.Title == "" {
		title = x.Plan.GetOr("title", "")
	}
	var reviewDict any
	if review != nil {
		reviewDict = review.ToDict()
	}
	kept := []any{}
	if status != "approved" && status != "built" {
		kept = findingDicts(findings)
	}
	gateList := []any{}
	for _, r := range results {
		gateList = append(gateList, r.ToDict())
	}
	x.Result.Set("title", title)
	x.Result.Set("report", report.Body)
	x.Result.Set("review", reviewDict)
	x.Result.Set("gates", gateList)
	x.Result.Set("findings", kept)
	x.Result.Set("self_check_findings", findingDicts(openSelfCheck))
	x.finish()
}

// withEarlier is `earlier = context or (findings_text(findings) if findings else "")` and
// `(earlier + "\n\n" if earlier else "") + note`.
func (x *Worker) withEarlier(context string, findings []*verdicts.Finding, note string) string {
	earlier := context
	if earlier == "" && len(findings) > 0 {
		earlier = x.findingsText(findings, "Blocking findings")
	}
	if earlier != "" {
		return earlier + "\n\n" + note
	}
	return note
}

// Before a change leaves the run with no reviewer in it: one fix pass when conflict
// markers are left in it and there is time. Returns the marker finding still open (None when
// there are none), and the builder's latest report (None when that pass stopped to ask a
// person, which ends the run).
func (x *Worker) clearMarkers(cycle int, entry *pyjson.Object, report *verdicts.BuildReport) (*verdicts.Finding, *verdicts.BuildReport) {
	markers := x.unresolvedMarkers()
	if len(markers) == 0 {
		return nil, report
	}
	finding := newFinding("blocking", strings.Join(upTo(markers, 8), ", "),
		"Conflict markers are still in "+strings.Join(markers, ", ")+": resolve every "+
			"one, keeping both sides' meaning.", "git grep")
	if x.SecondsLeft() >= float64(x.Cfg.MinMinutesForACall*60) {
		fixed, fix := x.buildPass(cycle, "fix", x.fixPrompt(
			cycle, []*verdicts.Finding{finding}, "", "Conflict markers left in the change"),
			fixWhy("the conflict-marker check", []*verdicts.Finding{finding}))
		entry.Set("marker_fix", fix.Get("builder"))
		if fixed == nil {
			return finding, nil
		}
		report = fixed
		markers = x.unresolvedMarkers()
		if len(markers) == 0 {
			return nil, report
		}
		finding = newFinding("blocking", strings.Join(upTo(markers, 8), ", "),
			"Conflict markers are still in "+strings.Join(markers, ", ")+".", "git grep")
	}
	return finding, report
}

// Before a review: merge `main` again when it moved since the run merged it and merges
// cleanly (#160, #317 part 12), so what is approved still merges; `main` moves about every
// half hour and a long run's approval was often stale before it was delivered. A merge
// that would conflict is left for the conflict path, and a conflict revision of a cleared
// change is left alone (deliver redoes its merge to check the carry). True when merged.
func (x *Worker) catchUp() bool {
	if x.CarryBuild != "" || x.Wt == nil {
		return false
	}
	fetchRef(x.Repo, x.Cfg.DefaultBranch, false)
	now := x.Repo.Rev(x.BaseRef)
	if now == "" || now == x.BaseSHA {
		return false
	}
	if code, _ := run(x.Wt, false, "merge-base", "--is-ancestor", now, "HEAD"); code == 0 {
		return false
	}
	if code, _ := run(x.Wt, false, "merge-tree", "--write-tree", "--no-messages", "HEAD", now); code != 0 {
		return false
	}
	var conflicts []string
	if err := herr.Try(func() { conflicts = x.Wt.Merge(now, x.Who) }); err != nil {
		// A merge that fails leaves the branch as it was.
		run(x.Wt, false, "merge", "--abort")
		return false
	}
	if len(conflicts) > 0 {
		run(x.Wt, false, "merge", "--abort")
		return false
	}
	x.BaseSHA = now
	x.BaseWt = nil // the checks red on `main` are `main`'s new self's now
	x.BaseGateCache = pyjson.New()
	x.Step("merge", "`main` moved during the run; merged again before the review",
		pyjson.New("outcome", "clean, at `"+first12(now)+"`"))
	return true
}

// OnlyForbidden is a build whose whole change was in paths the bot may not change (`bot/`,
// `.harness/`, `.github/`, ...): the path guard put it all back, so no fix or review can deliver
// it. The run ends now, asking a person, with nothing bundled, instead of spending its rounds.
func (x *Worker) OnlyForbidden() bool {
	if x.planItem("action") != "build" || len(x.PutBack) == 0 {
		return false
	}
	if len(x.Wt.ChangedPaths(x.BaseRef, "HEAD")) > 0 {
		return false
	}
	run(x.Wt, true, "reset", "--quiet", "--hard", x.StartSHA)
	paths := backticked(upTo(x.PutBack, 8))
	x.Result.Set("status", "blocked")
	x.Result.Set("reason", "the change was all in paths the bot may not change")
	x.Result.Set("human_required", true)
	x.Result.Set("question", "Everything this run changed ("+paths+") is in paths the bot may not "+
		"change, such as `bot/`, `.harness/` and `.github/`, so the harness put it "+
		"back and there is nothing to deliver. A person has to make this change, or "+
		"narrow the issue to the part outside those paths.")
	x.finish()
	return true
}

// A halt or a stop said during the last review still counts: no finished change is
// handed on past one. The clock, the usage stop and a suspension of the run's subscription
// no longer matter here: its model calls are over, so the finished change is handed on.
func (x *Worker) lastCheckpoint() {
	if x.Probe == nil {
		return
	}
	if reason, kind, ok := x.Probe(nil); ok && (kind == "halt" || kind == "stop") {
		herr.Raise(NewInterrupt(reason, kind, ""))
	}
}

// SecondReview is a review run: the lane's final audit (#725). A model reads a bot pull request
// (a strong one for a medium or hard item, the weakest shipping seat for an easy one).
// Nothing is built or pushed: the verdict goes
// to `deliver`, which merges once the approvals for the same commit meet the review rule
// (`review_rule.py`), or asks for a revision at the auditor's tier.
// Nothing is installed or run either: the reviewer holds another subscription's login, and
// the builder's code (a postinstall script, a test) must not run beside it. CI runs every
// check on the pull request before it can merge.
func (x *Worker) SecondReview() {
	x.prepare(false, false)
	x.Check()
	seat := x.SeatFor("review")
	builder := py.OrStr(x.Plan.Get("builder"), "an unknown model")
	var approved []string
	for _, f := range pyjson.List(x.Plan.Get("approved")) {
		approved = append(approved, py.Str(f))
	}
	soFar := "Nobody has approved it yet."
	if len(approved) > 0 {
		soFar = "It already has an approval from " + backticked(approved) + ", which is not " +
			"evidence: judge it from scratch."
	}
	context := "This is a review run. `" + builder + "` built this change; you are `" +
		x.Provider.Family + "` on `" + seat.Model + "` (" + seat.Tier + " tier). It merges once " +
		reviewrule.Summary + " approved the same commit. " + soFar + " Dependencies are " +
		"not installed in this run and you " +
		"should not run the branch's code: read it. CI runs every check on the pull " +
		"request before it can merge."
	if notes := strings.TrimSpace(py.OrStr(x.Plan.Get("review_notes"), "")); notes != "" {
		context += "\n\nA person asked for this review (`" + config.Slash + " review`) with " +
			"these notes:\n\n" + prompts.Data(notes, "Their notes")
	}
	var openFindings []*verdicts.Finding
	for _, f := range pyjson.List(x.Plan.Get("self_check_findings")) {
		openFindings = append(openFindings, finding(f))
	}
	if len(openFindings) > 0 {
		context += "\n\nThe builder's own self checks ran out with these findings still " +
			"open; judge them too:\n\n" +
			x.findingsText(openFindings, "Open self-check findings")
	}
	report := &verdicts.BuildReport{Status: "done", Title: py.OrStr(x.Plan.Get("title"), ""),
		Question: "", Body: py.OrStr(x.Plan.Get("pull"), "")}
	one := 1
	review := x.review(1, report, []*gates.GateResult{}, []*verdicts.Finding{}, context, &one, "review")
	appendTo(x.Result, "cycles", pyjson.New("n", 1, "review", review.ToDict()))
	if !review.Readable {
		x.Result.Set("status", "failed")
		x.Result.Set("reason", review.WhyUnreadable())
		return
	}
	verdict := "changes"
	reason := fmt.Sprintf("`%s` (%s) found %d blocking problem(s)", x.Provider.ID, seat.Tier,
		len(review.Blocking()))
	if review.Approved() {
		verdict = "approve"
		reason = fmt.Sprintf("`%s` (%s) approved it", x.Provider.ID, seat.Tier)
	}
	x.Result.Set("status", "reviewed")
	x.Result.Set("verdict", verdict)
	x.Result.Set("reviewed_sha", review.ReviewedSHA)
	x.Result.Set("review", review.ToDict())
	x.Result.Set("findings", findingDicts(review.Blocking()))
	x.Result.Set("reason", reason)
}

// Split is a split (#60): one session reads the issue and the code and answers with sub-issues,
// each small enough for one run (`split.py`); nothing is built, installed or committed. The
// deliver job opens them. With `children` in the plan, every sub-issue the issue had has
// closed and this is its close-out: `done`, or what is still missing.
func (x *Worker) Split() {
	x.prepare(false, false)
	x.Check()
	children := py.OrStr(x.Plan.Get("children"), "")
	fullsend := ""
	values := pyjson.New("number", x.planItem("number"), "repo", x.Cfg.Repo,
		"thread", x.Plan.GetOr("thread", ""),
		"builders", py.OrStr(x.Plan.Get("builders"), "I"),
		"children", or(children, "The issue has no sub-issues yet."))
	if x.Plan.Get("mode") == "fullsend" {
		fullsend = pyFormat(split.FullsendNotes, map[string]any{"branch": x.planItem("branch")})
	}
	values.Set("fullsend", fullsend)
	values.Set("easy_rule", x.easyRule())
	values.Set("max_issues", split.MaxIssues)
	prompt := x.Render("split", values) + x.HandoffText(false)
	head := x.Wt.Head()
	why := "split the issue into sub-issues"
	if children != "" {
		why = "close the tree out: check the issue against `main`"
	}
	result := x.Call("plan", prompt, x.Wt.Cwd, true, why, false)
	x.resetTo(head)
	tree := split.Parse(redact.Redact(result.Text, nil), []string{"easy", "medium", "hard"})
	x.Result.Set("tree", tree.ToDict())
	if !tree.OK() {
		problems := or(strings.Join(tree.Problems, "; "), "no tree")
		x.cameOfIt("no usable tree: " + problems)
		reason := "the split could not be used: " + problems
		if result.Error != "" {
			reason += " (" + result.Error + ")"
		}
		x.Result.Set("status", "failed")
		x.Result.Set("reason", py.SliceTo(reason, 2000))
		return
	}
	said := fmt.Sprintf("%d sub-issue(s)", len(tree.Issues))
	if tree.Done {
		said = "done"
	}
	x.cameOfIt(said)
	x.Result.Set("status", "split")
	x.Result.Set("reason", "split: "+said)
}

// Part is a fullsend part (#505): one builder pass from its tree's branch (`onto`, the base
// `_prepare` set), and the path guard, and nothing else: no checks, no self check, no
// catch-up with `main` and no review. The parts land on that branch blind to each other, as
// fullsend's builders write their slices, and the tree's reconcile checks and reviews them
// all at once. The deliver job lands it (`deliver.Deliverer._land_part`).
func (x *Worker) Part() {
	conflicts := x.prepare(true, true)
	if x.PlanSeat != nil {
		x.planning()
		if x.needsAPerson() || x.ratedOut() {
			x.finish()
			return
		}
	}
	x.Check()
	role, prompt := x.FirstPrompt(conflicts)
	built, entry := x.buildPass(1, role, prompt, x.FirstWhy(role, conflicts))
	appendTo(x.Result, "cycles", entry)
	if built == nil {
		return
	}
	guard := Guard(x)
	if x.OnlyForbidden() {
		return
	}
	x.lastCheckpoint()
	if len(x.Wt.ChangedPaths(x.BaseRef, "HEAD")) == 0 {
		x.Result.Set("status", "failed")
		x.Result.Set("reason", "the part changed nothing")
	} else {
		x.Result.Set("status", "part")
		x.Result.Set("reason", "a fullsend part, for the reconcile: no checks or review")
	}
	var title any = built.Title
	if built.Title == "" {
		title = x.Plan.GetOr("title", "")
	}
	x.Result.Set("title", title)
	x.Result.Set("report", built.Body)
	x.Result.Set("findings", findingDicts(guard))
	x.finish()
}
