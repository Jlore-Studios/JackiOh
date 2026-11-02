// test_repo_meta.py: the repository's meta (#187): the pull request title rule and the check that
// runs it, the issue forms, and the one list of labels. Standard library only, so the YAML is
// read with regular expressions.
package tests

import (
	"bytes"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/disk"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// repoMetaRoot is ROOT = config.REPO_ROOT.
var repoMetaRoot = config.RepoRoot

// repoMetaForms is FORMS = ROOT / ".github" / "ISSUE_TEMPLATE".
var repoMetaForms = filepath.Join(repoMetaRoot, ".github", "ISSUE_TEMPLATE")

// repoMetaRead is `(ROOT / path).read_text(encoding="utf-8")`.
func repoMetaRead(t *testing.T, parts ...string) string {
	t.Helper()
	return readText(t, filepath.Join(append([]string{repoMetaRoot}, parts...)...))
}

// repoMetaLabel is `config.LABELS[name]`: its (color, description).
func repoMetaLabel(t *testing.T, name string) (string, string) {
	t.Helper()
	v, ok := config.Labels.Lookup(name)
	if !ok {
		t.Fatalf("KeyError: %q", name)
	}
	pair := pyjson.List(v)
	return pyjson.Str(pair[0]), pyjson.Str(pair[1])
}

// ---------------------------------------------------------------------------- PullTitleTests
//
// `triage.pull_title_ok`: the convention, the titles tools give their own pull requests, and a
// revert of either. Everything else fails the `pr title` check.

// repoMetaAutomated is PullTitleTests.AUTOMATED.
var repoMetaAutomated = []string{
	"patches ship: v0.3.1", // patches-ship.yml
	"patches ship: v0.2.16b, v0.2.17",
	"patches log: 6d44be2..c9ca419",                          // promote-production.yml's technical log
	"Promote main to production: 28 commit(s) up to 6d44be2", // scripts/promote-production.sh
	"Promote main to production: 1 commit(s) up to c9ca419",
	"AI gen 8 (improve): the opening plays its two-drops first",             // training/improve.md
	"AI gen 12 (unban): Mirror Image is played again",                       // training/unban.md
	"AI gen 14 (learn): the learned evaluation, retrained on 100,000 games", // learn.yml
}

// repoMetaConventional is PullTitleTests.CONVENTIONAL.
var repoMetaConventional = []string{
	// #1360: every thread's form, a shipped card change's, and a part's.
	"Patch: Activate abilities work again",
	"Patch v0.3.18: Activate abilities work again",
	"Patch v0.4.0c: one more fix",
	"Patch (part 2 of 8): fragments without a version",
	"Patch: patch numbers (part 2 of 8): fragments without a version",
	`Revert "Patch: Activate abilities work again"`,
	"Patch v0.3.1: card text pass",
	"Patch v0.2.7b: one more fix",
	`Revert "Patch v0.4.0c: one more fix"`,
	`Revert "patches ship: v0.3.1"`,
}

// repoMetaBad is PullTitleTests.BAD.
var repoMetaBad = []string{
	// The older prefixes, retired by the title retcon (#1372).
	"Patch v0.3.X: Activate abilities work again",
	"Patch v0.3.Y: fix the UI of the rematch button",
	"Patch v0.3.X (part 2 of 3): the lobby",
	"v0.3.0: the Rust rewrite",
	"v0.4.0 (part 1 of 5): the engine",
	"Night bot: a seventh subscription, any hour with no caps",
	"CI: daily super run failed",
	"Architecture: issue and PR templates, labels as code and repository meta",
	`Revert "Patch v0.3.X: Activate abilities work again"`,
	"Share a room as an invite link that opens the join form filled in (R767)", // #490
	"Practice: a worker whose script never loaded says to reload",              // #471
	"v0.3.0 part 40: the sweep of record",                                      // #472
	"bot: build pass 1 for #85",                                                // a bot commit's subject, #169
	"Patch v0.2.X Classic & Classic+ balance",
	"patch v0.3.X: lower case",
	"Patch:",
	"Patch : a space",
	"Patch v0.3.Z: no such guess",
	"Patch (part two of 8): words",
	"Architecture:",
	"patches ship: ",
	"patches ship: v0.3.Y", // ship names a micro patch's version
	"patches log: soon",
	"Promote main to production: soon",
	"AI gen 3 (sideways): a lane that does not exist",
	`Revert "Share a room"`,
	"",
}

func TestRepoMeta_ConventionalAndAutomatedTitlesPass(t *testing.T) {
	for _, title := range append(append([]string{}, repoMetaConventional...), repoMetaAutomated...) {
		assertTrue(t, triage.PullTitleOK(title), title)
	}
}

func TestRepoMeta_EveryOtherTitleFails(t *testing.T) {
	for _, title := range repoMetaBad {
		assertTrue(t, !triage.PullTitleOK(title), title)
	}
}

// Each tool's title, as its source writes it, so a change to one fails here first.
func TestRepoMeta_TheAutomatedTitlesAreTheOnesTheirToolsWrite(t *testing.T) {
	assertIn(t, `--title "patches ship: $versions"`, workflowsRead(t, "patches-ship.yml"))
	assertIn(t, `versions=$(sed -n 's/^patches ship: shipped //p'`, workflowsRead(t, "patches-ship.yml"))
	assertIn(t, `--title "patches log: $range"`, workflowsRead(t, "promote-production.yml"))
	assertIn(t, `range="${OLD:0:7}..${NEW:0:7}"`, workflowsRead(t, "promote-production.yml"))
	promote := repoMetaRead(t, "scripts", "promote-production.sh")
	assertIn(t, `--title "Promote main to production: $COUNT commit(s) up to ${cand:0:7}"`, promote)
	loop := repoMetaRead(t, "training", "loop.sh")
	assertIn(t, "title=$(git log -1 --format=%s)", loop)
	assertIn(t, `gh pr create --base main --head "$branch" --title "$title"`, loop)
	// The lane's draft, while a session works (training/README.md, How a lane runs).
	assertIn(t, `title="AI gen $((generation + 1)) ($lane): in training, nothing promoted yet"`, loop)
	assertTrue(t, triage.PullTitleOK("AI gen 2 (unban): in training, nothing promoted yet"))
	for _, lane := range []string{"improve", "unban"} {
		text := repoMetaRead(t, "training", lane+".md")
		assertIn(t, `git commit -m "AI gen <N> (`+lane+`): <what changed>"`, text)
	}
}

func TestRepoMeta_ABotTakesItsBuildersTitleWhenItPassesElseItsIssues(t *testing.T) {
	assertEqual(t, triage.PullTitle("Patch v0.3.2: the rules, v2", "Patch: the rules"),
		"Patch v0.3.2: the rules, v2")
	assertEqual(t, triage.PullTitle("Make the rules v2", "Patch: the rules"), "Patch: the rules")
	assertEqual(t, triage.PullTitle("", ""), "")
}

// A workflow's alert keeps its old prefix (`titles.LEFT_ALONE`), so the fix of one, as #1548 and
// #1549 were, failed the title check.
func TestRepoMeta_ABotThatHasNoPassingTitleWritesOne(t *testing.T) {
	for _, c := range [][3]string{
		{"Make the rules v2", "the rules", "Patch: Make the rules v2"},
		{"", "the rules", "Patch: the rules"},
		{"CI: a job ran over 7 minutes", "CI: a job ran over 7 minutes",
			"Patch: a job ran over 7 minutes"},
		{"", "CI: daily super run failed", "Patch: daily super run failed"},
		{"Architecture: weekly hygiene, dead files, 2026-10-21",
			"Architecture: weekly hygiene, dead files, 2026-10-21",
			"Patch: weekly hygiene, dead files, 2026-10-21"},
	} {
		title := triage.PullTitle(c[0], c[1])
		assertEqual(t, title, c[2])
		assertTrue(t, triage.PullTitleOK(title), title)
	}
}

// ---------------------------------------------------------------------------- TitleWorkflowTests

func TestRepoMeta_EveryTitleAndHeadIsCheckedNeverWithThePullRequestsCode(t *testing.T) {
	text := workflowsRead(t, "pr-title.yml")
	assertRegex(t, text, `pull_request_target:\n\s+types: \[opened, edited, reopened, `+
		`synchronize\]`)
	assertNotRegex(t, text, `(?m)^  pull_request:`, "would run the pull request's code")
	assertEqual(t, workflowsFindall(workflowsRef, text),
		[]string{"${{ github.event.repository.default_branch }}"})
	assertIn(t, "persist-credentials: false", text)
	assertNotIn(t, "secrets.", text)
	assertRegex(t, text, `(?m)^permissions: \{\}$`)
	block := workflowsJob(t, text, "title")
	assertEqual(t, workflowsGrants(t, block), map[string]string{"contents": "read"})
	assertIn(t, "timeout-minutes:", block)
}

func TestRepoMeta_TheTitleReachesTheBotsRuleThroughTheEnvironment(t *testing.T) {
	text := workflowsRead(t, "pr-title.yml")
	assertIn(t, "TITLE: ${{ github.event.pull_request.title }}", text)
	assertEqual(t, strings.Count(text, "github.event.pull_request.title"), 1)
	// The workflow changed (#467): the inline Python that imported `harness.triage.pull_title_ok`
	// is the binary's `harness pr-title`, which takes the title from the step's environment.
	assertIn(t, `harness pr-title --title="$TITLE"`, text)
	assertTrue(t, workflowsLinted(t, "pr-title.yml"))
}

// ---------------------------------------------------------------------------- IssueFormTests

// repoMetaForm is one entry of IssueFormTests.forms, a dict in sorted order.
type repoMetaForm struct{ name, text string }

// repoMetaFormsRead is IssueFormTests.forms: each form but config.yml, by name, in sorted order.
func repoMetaFormsRead(t *testing.T) []repoMetaForm {
	t.Helper()
	paths, err := filepath.Glob(filepath.Join(repoMetaForms, "*.yml"))
	if err != nil {
		t.Fatal(err)
	}
	sort.Strings(paths)
	forms := []repoMetaForm{}
	for _, path := range paths {
		if filepath.Base(path) != "config.yml" {
			forms = append(forms, repoMetaForm{filepath.Base(path), readText(t, path)})
		}
	}
	return forms
}

var (
	repoMetaFormTitle  = `(?m)^title: "(.*)"$`
	repoMetaFormLabels = `(?m)^labels: (\[.*\])$`
)

func TestRepoMeta_OneFormPerKind(t *testing.T) {
	// #1370: the Patch and Micro patch forms are one, since no form guesses a version.
	names := []string{}
	for _, form := range repoMetaFormsRead(t) {
		names = append(names, form.name)
	}
	sort.Strings(names)
	assertEqual(t, names, []string{"architecture.yml", "bug.yml", "night-bot.yml", "patch.yml"})
}

func TestRepoMeta_EveryFormTitlesItsIssuePatchWithNoVersion(t *testing.T) {
	for _, form := range repoMetaFormsRead(t) {
		assertEqual(t, workflowsSearch(t, repoMetaFormTitle, form.text, 1), "Patch: ", form.name)
		assertNotRegex(t, form.text, `v0\.\d+\.[XY]\b`, form.name)
	}
}

func TestRepoMeta_EachFormTitlesAndLabelsItsIssueAndLeavesItsTypeToTriage(t *testing.T) {
	for _, form := range repoMetaFormsRead(t) {
		title := workflowsSearch(t, repoMetaFormTitle, form.text, 1)
		assertTrue(t, triage.Convention.MatchString(title+"what it does"), form.name)
		raw, err := pyjson.LoadsString(workflowsSearch(t, repoMetaFormLabels, form.text, 1))
		if err != nil {
			t.Fatal(err)
		}
		labels := pyjson.Strings(raw)
		// The sync creates every one, and every issue carries a type label.
		typed := false
		for _, label := range labels {
			assertTrue(t, config.Labels.Has(label), form.name, label)
			typed = typed || setOf(config.TypeLabels...)[label]
		}
		assertTrue(t, typed, form.name)
		// Triage types it (Task, Bug, Feature) and only types an untyped issue; who does it is the
		// dropdown's answer, so no form puts `human` on by itself.
		assertNotRegex(t, form.text, `(?m)^type:`, form.name)
		assertTrue(t, !setOf(labels...)[config.LabelHuman], form.name)
		assertIn(t, "docs/issues-and-patches.md", form.text, form.name)
	}
}

// repoMetaDropdown is one of dropdowns()'s values, with its id.
type repoMetaDropdown struct {
	id, label string
	options   []string
}

var (
	repoMetaFieldType = regexp.MustCompile(`(?m)^  - type: `)
	repoMetaOption    = regexp.MustCompile(`(?m)^        - (.+)$`)
)

// repoMetaDropdowns is IssueFormTests.dropdowns(): each dropdown's id, label and options, from the
// form's YAML, in the form's order.
func repoMetaDropdowns(t *testing.T, text string) []repoMetaDropdown {
	t.Helper()
	found := []repoMetaDropdown{}
	for _, block := range repoMetaFieldType.Split(text, -1)[1:] {
		if !strings.HasPrefix(block, "dropdown") {
			continue
		}
		ident := workflowsSearch(t, `(?m)^    id: (\S+)$`, block, 1)
		label := strings.TrimSpace(workflowsSearch(t, `(?m)^      label: (.+)$`, block, 1))
		options := []string{}
		for _, o := range workflowsFindall(repoMetaOption, block) {
			options = append(options, strings.Trim(strings.TrimSpace(o), `"`))
		}
		// A dict: a later dropdown of the same id replaces the earlier in its place.
		replaced := false
		for i := range found {
			if found[i].id == ident {
				found[i] = repoMetaDropdown{ident, label, options}
				replaced = true
			}
		}
		if !replaced {
			found = append(found, repoMetaDropdown{ident, label, options})
		}
	}
	return found
}

// Triage turns the dropdowns into the labels a person would set by hand: who does it is `human`
// or `bot:build`, and "Decide later", "Not sure" or "No priority" is nothing.
func TestRepoMeta_EveryFormAsksWhoDoesItAndEachAnswerIsItsLabel(t *testing.T) {
	for _, form := range repoMetaFormsRead(t) {
		dropdowns := repoMetaDropdowns(t, form.text)
		ids := map[string]bool{}
		for _, d := range dropdowns {
			ids[d.id] = true
		}
		assertTrue(t, ids["method"], form.name, "method")
		assertTrue(t, ids["difficulty"], form.name, "difficulty")
		for _, d := range dropdowns {
			answers := [][]string{}
			for _, option := range d.options {
				answers = append(answers, triage.FormLabels("### "+d.label+"\n\n"+option))
			}
			switch d.id {
			case "method":
				assertEqual(t, answers, [][]string{{}, {config.LabelHuman}, {config.LabelBuild}}, form.name)
			case "difficulty":
				want := [][]string{{}}
				for _, label := range config.DifficultyLabels.Keys() {
					want = append(want, []string{label})
				}
				assertEqual(t, answers, want, form.name)
			case "priority":
				assertEqual(t, answers, [][]string{{}, {config.LabelPriorityHigh},
					{config.LabelPriorityMedium}, {config.LabelPriorityLow}}, form.name)
			}
		}
	}
}

func TestRepoMeta_NoBlankIssuesAndALinkToTheConventions(t *testing.T) {
	text := readText(t, filepath.Join(repoMetaForms, "config.yml"))
	assertIn(t, "blank_issues_enabled: false", text)
	assertIn(t, "/blob/main/docs/issues-and-patches.md", text)
	assertTrue(t, isFile(filepath.Join(repoMetaRoot, "docs", "issues-and-patches.md")))
}

// ---------------------------------------------------------------------------- FormLabelsTests
//
// `triage.form_labels`: an issue form's body as GitHub writes it, to labels.

// repoMetaBody is FormLabelsTests.BODY.
const repoMetaBody = "### What it does\n\nA thing.\r\n\r\n### Difficulty (a guess)\n\nhard: engine rules, the AI\n\n" +
	"### Priority\n\nlow\n\n### Who does it\n\nThe night bot (bot:build)\n"

func TestRepoMeta_TheAnswersInOrder(t *testing.T) {
	assertEqual(t, triage.FormLabels(repoMetaBody),
		[]string{"difficulty:hard", config.LabelPriorityLow, config.LabelBuild})
}

func TestRepoMeta_NothingFromABodyNoFormWroteOrUnansweredFields(t *testing.T) {
	assertEqual(t, triage.FormLabels(""), []string{})
	assertEqual(t, triage.FormLabels("Please fix the (bot:build) thing"), []string{})
	assertEqual(t, triage.FormLabels("### Priority\n\n_No response_\n\n### Who does it\n\n"+
		"Decide later"), []string{})
}

func TestRepoMeta_TextAPersonWroteIsNotAnAnswer(t *testing.T) {
	body := "### What it does\n\nUse (human) here, and priority high\n"
	assertEqual(t, triage.FormLabels(body), []string{})
}

// ---------------------------------------------------------------------------- LabelListTests
//
// `config.LABELS` is the one list of the repository's labels: the sweep creates a missing one and
// `harness setup` brings a hand-changed one back.

var (
	repoMetaLabelRow = regexp.MustCompile("(?m)^\\| `([^`]+)` \\|")
	repoMetaPutsOn   = regexp.MustCompile(`--(?:add-)?label "([^"$]+)"`)
)

func TestRepoMeta_EveryLabelInUseIsOnTheList(t *testing.T) {
	docs := repoMetaRead(t, "docs", "issues-and-patches.md")
	table := workflowsSearch(t, `(?ms)^## Labels\n(.*?)^## `, docs, 1)
	named := workflowsFindall(repoMetaLabelRow, table)
	if len(named) < 4 {
		t.Fatalf("%q != the four type labels", named)
	}
	assertEqual(t, named[:4], []string{"patch", "major version", "architecture", "night bot"})
	used := setOf(named...)
	for _, group := range [][]string{dashboard.Labels, disk.Labels, triage.TypeLabels,
		{config.LabelHuman, config.LabelReady}, config.DifficultyLabels.Keys(),
		{config.LabelPriorityHigh, config.LabelPriorityMedium, config.LabelPriorityLow}} {
		for _, name := range group {
			used[name] = true
		}
	}
	// And every label a workflow or a script puts on by name.
	workflowsPaths, err := filepath.Glob(filepath.Join(workflowsDir, "*.yml"))
	if err != nil {
		t.Fatal(err)
	}
	scripts, err := filepath.Glob(filepath.Join(repoMetaRoot, "scripts", "*.sh"))
	if err != nil {
		t.Fatal(err)
	}
	for _, path := range append(workflowsPaths, scripts...) {
		for _, name := range workflowsFindall(repoMetaPutsOn, readText(t, path)) {
			used[name] = true
		}
	}
	missing := []string{}
	for name := range used {
		if !config.Labels.Has(name) {
			missing = append(missing, name)
		}
	}
	sort.Strings(missing)
	assertEqual(t, missing, []string{})
}

// #664: the sweep deletes the retired labels, so none may come back through the list, the
// conventions or a form.
func TestRepoMeta_NoRetiredLabelIsInUse(t *testing.T) {
	retired := map[string]bool{}
	for _, name := range config.RetiredLabels {
		retired[strings.ToLower(name)] = true
	}
	both := map[string]bool{}
	for _, name := range config.Labels.Keys() {
		if retired[strings.ToLower(name)] {
			both[strings.ToLower(name)] = true
		}
	}
	assertEqual(t, both, map[string]bool{})
	forms, err := filepath.Glob(filepath.Join(repoMetaForms, "*.yml"))
	if err != nil {
		t.Fatal(err)
	}
	for _, path := range append([]string{filepath.Join(repoMetaRoot, "docs", "issues-and-patches.md")}, forms...) {
		text := strings.ToLower(readText(t, path))
		found := []string{}
		for _, name := range sortedKeys(retired) {
			if strings.Contains(text, "`"+name+"`") {
				found = append(found, name)
			}
		}
		assertEqual(t, found, []string{}, filepath.Base(path))
	}
}

// promote-production.sh creates its label with `--force`, which would undo any other.
func TestRepoMeta_TheProductionMergeLabelIsAsItsScriptWritesIt(t *testing.T) {
	script := repoMetaRead(t, "scripts", "promote-production.sh")
	color, description := repoMetaLabel(t, "production merge")
	assertIn(t, `: "${RELEASE_LABEL:=production merge}"`, script)
	assertIn(t, `gh label create "$RELEASE_LABEL" --color `+color+` --force`, script)
	assertIn(t, `--description "`+description+`"`, script)
}

// ci.yml's `rust (classify)` creates its labels with `--force`, which would undo any other colour
// or description (#1370).
func TestRepoMeta_TheKindAndTierLabelsAreAsCIWritesThem(t *testing.T) {
	block := workflowsJob(t, workflowsRead(t, "ci.yml"), "rust-classify")
	for _, name := range append(append([]string{}, config.KindLabels...), config.TierLabels...) {
		color, description := repoMetaLabel(t, name)
		assertIn(t, "["+name+"]="+color, block, name)
		assertIn(t, "["+name+`]="`+description+`"`, block, name)
	}
}

func TestRepoMeta_SetupCreatesTheMissingAndBringsBackTheHandChanged(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	patchColor, patchText := repoMetaLabel(t, "patch")
	gh.Labels = pyjson.New(
		"human", pyjson.New("name", "human", "color", "f5126e", "description", "old words"),
		"patch", pyjson.New("name", "patch", "color", strings.ToUpper(patchColor), "description", patchText))
	cfg := testsupport.MakeConfig()
	var out bytes.Buffer
	ctx, stdout := cli.Ctx, cli.Stdout
	t.Cleanup(func() { cli.Ctx, cli.Stdout = ctx, stdout })
	cli.Ctx = func(cfg *config.Config, _ bool) *hctx.Context {
		return testsupport.MakeCtx(gh, testsupport.Ctx{Cfg: cfg})
	}
	cli.Stdout = &out
	assertEqual(t, cli.CmdSetup(cfg, &cli.Args{RepoSettings: false}), 0)
	cli.Ctx, cli.Stdout = ctx, stdout
	said := out.String()
	assertIn(t, "updated label human", said)
	assertIn(t, "created label production merge", said)
	assertNotIn(t, "label patch", said) // it matched, whatever the colour's case
	for _, name := range config.Labels.Keys() {
		color, description := repoMetaLabel(t, name)
		label := gh.Labels.Obj(name)
		assertEqual(t, [2]string{strings.ToLower(label.Str("color")), label.Str("description")},
			[2]string{color, description}, name)
	}
}

// ---------------------------------------------------------------------------- CodeownersTests

var repoMetaOwner = regexp.MustCompile(`(?m)^(\S+)\s+(.+)$`)

func TestRepoMeta_ThePathsTheBotsMayNotChangeAskAPerson(t *testing.T) {
	text := repoMetaRead(t, ".github", "CODEOWNERS")
	owners := map[string]string{}
	for _, pair := range workflowsGroups(repoMetaOwner, text) {
		owners[pair[0]] = pair[1]
	}
	for _, path := range []string{"/.github/", "/bot/", "/.harness/", "/.squishy/"} {
		assertEqual(t, strings.Fields(owners[path]), []string{"@jgoetzmann", "@MaxGoetzmann"}, path)
	}
}
