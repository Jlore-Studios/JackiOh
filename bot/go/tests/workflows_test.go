// test_workflows.py: the workflows keep their safety shape. Parsed with regular expressions, as the
// Python does: standard library only.
//
// The bot's workflows moved to the Go harness (#467, `git log -- .github`): each job that set up
// Python (`actions/setup-python`) uses `./.github/actions/harness`, which builds bot/go once per
// version and puts `harness` on PATH, and each `python -m harness X` (or `python3 -m`) run line is
// `harness X`. A test whose assertion changed with them says so where it does.
package tests

import (
	"path/filepath"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"testing"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// workflowsDir is WORKFLOWS = config.REPO_ROOT / ".github" / "workflows".
var workflowsDir = filepath.Join(config.RepoRoot, ".github", "workflows")

// workflowsRead is read(): a workflow's text.
func workflowsRead(t *testing.T, name string) string {
	t.Helper()
	return readText(t, filepath.Join(workflowsDir, name))
}

var workflowsNextJob = regexp.MustCompile(`(?m)^  [a-z][\w-]*:\n`)

// workflowsJob is job(): the block of one job, from `  name:` to the next job at the same indent.
// The Python's lazy group ends at a lookahead (`(?=^  [a-z][\w-]*:\n|\Z)`), which Go's regexp
// lacks: the block ends where the next such line starts, or at the end.
func workflowsJob(t *testing.T, text, name string) string {
	t.Helper()
	head := regexp.MustCompile(`(?m)^  ` + regexp.QuoteMeta(name) + `:\n`)
	loc := head.FindStringIndex(text)
	if loc == nil {
		t.Fatalf("no job %s", name)
	}
	return workflowsUntil(text[loc[1]:], workflowsNextJob)
}

// workflowsUntil is `rest` up to where `next` first matches, or all of it.
func workflowsUntil(rest string, next *regexp.Regexp) string {
	if m := next.FindStringIndex(rest); m != nil {
		return rest[:m[0]]
	}
	return rest
}

// workflowsSelftest is selftest(): the `bot selftest` job's block, a job of CI's since #1455,
// which had a workflow of its own.
func workflowsSelftest(t *testing.T) string {
	t.Helper()
	return workflowsJob(t, workflowsRead(t, "ci.yml"), "bot-selftest")
}

// workflowsActionlintEveryWorkflow is ACTIONLINT_EVERY_WORKFLOW.
const workflowsActionlintEveryWorkflow = "rhysd/actionlint:1.7.7 -color $(ls .github/workflows/*.yml)"

// workflowsLinted is linted(): whether bot selftest's actionlint reads the workflow `name`: it
// reads every one there is.
func workflowsLinted(t *testing.T, name string) bool {
	t.Helper()
	if !strings.Contains(workflowsSelftest(t), workflowsActionlintEveryWorkflow) {
		return false
	}
	return isFile(filepath.Join(workflowsDir, name))
}

var workflowsNextStep = regexp.MustCompile(`(?m)^      - `)

// workflowsStep is step(): the block of the one step that starts `- <head>`, to the next step at
// the same indent.
func workflowsStep(t *testing.T, text, head string) string {
	t.Helper()
	start := regexp.MustCompile(`(?m)^      - ` + regexp.QuoteMeta(head) + `\n`)
	loc := start.FindStringIndex(text)
	if loc == nil {
		t.Fatalf("no step %s", head)
	}
	return workflowsUntil(text[loc[1]:], workflowsNextStep)
}

// workflowsScript is script(): the `run: |` script of the step named `name`, its comments after it
// left out.
func workflowsScript(t *testing.T, text, name string) string {
	t.Helper()
	lines := strings.Split(workflowsStep(t, text, "name: "+name), "\n")
	start := -1
	for i, line := range lines {
		if strings.TrimSpace(line) == "run: |" {
			start = i + 1
			break
		}
	}
	if start < 0 {
		t.Fatalf("StopIteration: the step %s has no `run: |`", name)
	}
	body := []string{}
	for _, line := range lines[start:] {
		if strings.TrimSpace(line) != "" && !strings.HasPrefix(line, strings.Repeat(" ", 10)) {
			break
		}
		body = append(body, line)
	}
	return strings.TrimRightFunc(strings.Join(body, "\n"), unicode.IsSpace)
}

var (
	workflowsWith  = regexp.MustCompile(`(?m)^        with:\n((?:^          .*\n)+)`)
	workflowsInput = regexp.MustCompile(`(?m)^          ([\w-]+): (.*)$`)
)

// workflowsInputs is inputs(): the `with:` inputs of the one step that uses `action`.
func workflowsInputs(t *testing.T, text, action string) map[string]string {
	t.Helper()
	block := workflowsWith.FindStringSubmatch(workflowsStep(t, text, "uses: "+action))
	if block == nil {
		t.Fatalf("AttributeError: the step that uses %s has no `with:`", action)
	}
	found := map[string]string{}
	for _, m := range workflowsInput.FindAllStringSubmatch(block[1], -1) {
		found[m[1]] = m[2]
	}
	return found
}

var (
	workflowsPermissions = regexp.MustCompile(`permissions:\n((?:\s{6}.*\n)+)`)
	workflowsGrant       = regexp.MustCompile(`(?m)^\s+([\w-]+):\s*(\w+)`)
	workflowsSecretUse   = regexp.MustCompile(`\$\{\{\s*(secrets[^}]*?)\s*\}\}`)
)

// workflowsGrants is `dict(re.findall(r"^\s+([\w-]+):\s*(\w+)", re.search(r"permissions:\n…",
// block).group(1), re.M))`, which the Python spells out in each test: the first `permissions:`
// block's grants.
func workflowsGrants(t *testing.T, block string) map[string]string {
	t.Helper()
	perms := workflowsPermissions.FindStringSubmatch(block)
	if perms == nil {
		t.Fatal("AttributeError: no permissions: block")
	}
	grants := map[string]string{}
	for _, m := range workflowsGrant.FindAllStringSubmatch(perms[1], -1) {
		grants[m[1]] = m[2]
	}
	return grants
}

// workflowsGroups is re.findall with groups: each match's groups.
func workflowsGroups(re *regexp.Regexp, text string) [][]string {
	found := [][]string{}
	for _, m := range re.FindAllStringSubmatch(text, -1) {
		found = append(found, m[1:])
	}
	return found
}

// workflowsFindall is re.findall with one group (or none): each match's group (or the match).
func workflowsFindall(re *regexp.Regexp, text string) []string {
	found := []string{}
	for _, m := range re.FindAllStringSubmatch(text, -1) {
		found = append(found, m[len(m)-1])
	}
	return found
}

// workflowsInt is int(s), which allows the whitespace around the number.
func workflowsInt(t *testing.T, s string) int {
	t.Helper()
	n, err := strconv.Atoi(strings.TrimSpace(s))
	if err != nil {
		t.Fatal(err)
	}
	return n
}

// workflowsSearch is `re.search(pattern, text).group(n)`: an AttributeError when nothing matches.
func workflowsSearch(t *testing.T, pattern, text string, n int) string {
	t.Helper()
	m := regexp.MustCompile(pattern).FindStringSubmatch(text)
	if m == nil {
		t.Fatalf("AttributeError: %q matches nothing", pattern)
	}
	return m[n]
}

// workflowsNextLine is `text[text.index(sub):].split("\n", 2)[1]`: the line after the one `sub`
// is on.
func workflowsNextLine(t *testing.T, text, sub string) string {
	t.Helper()
	parts := strings.SplitN(text[strIndex(t, text, sub):], "\n", 3)
	if len(parts) < 2 {
		t.Fatalf("IndexError: nothing after %q", sub)
	}
	return parts[1]
}

// ---------------------------------------------------------------------------- NightWorkflowTests

func workflowsNight(t *testing.T) string {
	t.Helper()
	return workflowsRead(t, "bot-night.yml")
}

func TestWorkflows_FourJobsInOrder(t *testing.T) {
	text := workflowsNight(t)
	assertIn(t, "needs: gate", workflowsJob(t, text, "plan"))
	assertIn(t, "if: needs.gate.outputs.go == 'true'", workflowsJob(t, text, "plan"))
	assertIn(t, "needs: plan", workflowsJob(t, text, "work"))
	assertIn(t, "needs: [plan, work]", workflowsJob(t, text, "deliver"))
	assertIn(t, "always()", workflowsJob(t, text, "deliver"))
}

func TestWorkflows_TheModelJobHoldsNoGithubWriteToken(t *testing.T) {
	work := workflowsJob(t, workflowsNight(t), "work")
	assertNotIn(t, "BOT_GITHUB_TOKEN", work)
	assertEqual(t, workflowsGrants(t, work), map[string]string{"contents": "read"})
	// Exactly one model secret: the one plan named, picked by name. A run the status loop
	// started names it in its inputs, which `plan-fetch --check` holds to the plan and to
	// providers.json before any step reads it (#1455).
	assertEqual(t, setOf(workflowsFindall(workflowsSecretUse, work)...),
		setOf("secrets[env.PLAN_SECRET]"))
	assertIn(t, "PLAN_SECRET: ${{ inputs.plan_ref && inputs.secret || "+
		"needs.plan.outputs.secret }}", work)
}

// A model secret's value goes only to the gate's quiet step and to the model job, which hold no
// write token. Every other job learns only whether each secret is set.
func TestWorkflows_ModelSecretsReachOnlyJobsThatCannotWrite(t *testing.T) {
	text := workflowsNight(t)
	for _, name := range []string{"plan", "deliver"} {
		block := workflowsJob(t, text, name)
		for _, secret := range providers.SecretNames {
			assertNotRegex(t, block, `\b`+secret+`\s*:`, name, secret)
			for _, mention := range workflowsFindall(regexp.MustCompile(`secrets\.`+secret+`\b[^}]*`), block) {
				assertTrue(t, strings.HasPrefix(mention, "secrets."+secret+" != '' && '"+secret+"'"),
					name, mention)
			}
		}
		assertNotIn(t, "secrets[", block)
	}
	gate := workflowsJob(t, text, "gate")
	assertRegex(t, gate, `CLAUDE_CODE_OAUTH_TOKEN:\s*\$\{\{ secrets\[steps\.peek\.outputs\.`+
		`quiet_secret\] \}\}`)
	for _, name := range []string{"gate", "work"} {
		block := workflowsJob(t, text, name)
		assertNotIn(t, "BOT_GITHUB_TOKEN", block)
		values := map[string]bool{}
		for _, grant := range workflowsGrants(t, block) {
			values[grant] = true
		}
		assertEqual(t, values, setOf("read"), name)
	}
}

var (
	workflowsSecretsSet = regexp.MustCompile(`HARNESS_SECRETS_SET: >-\n((?:\s+\$\{\{.*\}\}\n)+)`)
	workflowsSecretSet  = regexp.MustCompile(`secrets\.(\w+) != '' && '(\w+)'`)
)

// providers.SECRETS is the list the workflows hand over; each `HARNESS_SECRETS_SET` names every
// one, or a subscription would look unconfigured.
func TestWorkflows_EveryProviderSecretIsKnownToEveryList(t *testing.T) {
	for _, name := range []string{"bot-night.yml", "bot-commands.yml", "claude-wake.yml"} {
		text := workflowsRead(t, name)
		lists := workflowsFindall(workflowsSecretsSet, text)
		assertTrue(t, len(lists) > 0, name)
		for _, found := range lists {
			named := workflowsGroups(workflowsSecretSet, found)
			firsts := []string{}
			same := true
			for _, pair := range named {
				firsts = append(firsts, pair[0])
				same = same && pair[0] == pair[1]
			}
			assertEqual(t, firsts, providers.SecretNames, name)
			assertTrue(t, same, name)
		}
	}
}

func TestWorkflows_OnlyTheChosenCLIIsInstalledAndOnlyOnGithubsRunners(t *testing.T) {
	work := workflowsJob(t, workflowsNight(t), "work")
	for _, cli := range providers.SecretClis {
		assertRegex(t, work, `if: env\.PLAN_CLI == '`+cli+`' && `+
			`startsWith\(env\.PLAN_RUNS_ON, 'ubuntu-'\)`)
	}
	assertNotIn(t, "cli == 'agy'", work) // agy lives on the machine only
	const hosted = "if: startsWith(env.PLAN_RUNS_ON, 'ubuntu-')"
	for _, action := range []string{"pnpm/action-setup", "actions/setup-node", "dtolnay/rust-toolchain"} {
		assertTrue(t, strings.HasPrefix(strings.TrimSpace(workflowsNextLine(t, work, action)), hosted),
			action)
	}
	// The workflow changed (#467): `actions/setup-python` was the fourth of these, and the harness
	// action took its place. A Python set up here again would still be for GitHub's runners only
	// (the machine has its own, bot/machine/setup.sh); the harness action runs on the machine too,
	// which has no harness binary of its own, so it is never held to GitHub's runners.
	if strings.Contains(work, "actions/setup-python") {
		assertTrue(t, strings.HasPrefix(strings.TrimSpace(
			workflowsNextLine(t, work, "actions/setup-python")), hosted), "actions/setup-python")
	}
	assertTrue(t, !strings.HasPrefix(strings.TrimSpace(
		workflowsNextLine(t, work, "uses: ./.github/actions/harness")), "if:"), "./.github/actions/harness")
}

// The harness's checks run cargo, so a model job on GitHub's runners sets up the toolchain
// rust-toolchain.toml pins, with the components and target ci.yml's setup installs
// (.github/actions/setup); the machine's users have their own (setup.sh).
func TestWorkflows_TheModelJobsInstallTheRustCIInstalls(t *testing.T) {
	toml := readText(t, filepath.Join(config.RepoRoot, "rust-toolchain.toml"))
	channel := workflowsSearch(t, `(?m)^channel = "([^"]+)"`, toml, 1)
	ci := readText(t, filepath.Join(config.RepoRoot, ".github", "actions", "setup", "action.yml"))
	step := `- uses: dtolnay/rust-toolchain@` + regexp.QuoteMeta(channel) + `\n(?:\s+if: .*\n)?` +
		`\s+with:\n\s+components: rustfmt, clippy\n` +
		`\s+targets: wasm32-unknown-unknown\n`
	assertRegex(t, ci, step)
	for _, name := range []string{"bot-night.yml", "squishy-run.yml"} {
		assertRegex(t, workflowsJob(t, workflowsRead(t, name), "work"), step, name)
	}
}

func TestWorkflows_TheModelJobRunsOnItsSubscriptionsRunner(t *testing.T) {
	text := workflowsNight(t)
	assertIn(t, "runs_on: ${{ steps.plan.outputs.runs_on }}", workflowsJob(t, text, "plan"))
	assertIn(t, "runs-on: ${{ inputs.plan_ref && inputs.runs_on || needs.plan.outputs.runs_on "+
		"|| 'ubuntu-latest' }}", workflowsJob(t, text, "work"))
	for _, name := range []string{"gate", "plan", "deliver"} { // the jobs that hold a GitHub write token
		assertIn(t, "runs-on: ubuntu-latest", workflowsJob(t, text, name), name)
	}
}

// The partner bot excuses a rise on the shared Claude account while a step named `Build, check
// and review` runs, so a run on any other subscription must not use it.
func TestWorkflows_OnlyARunOnTheSharedSubscriptionLooksLikeSpendingIt(t *testing.T) {
	work := workflowsJob(t, workflowsNight(t), "work")
	shared := regexp.MustCompile(`- name: Build, check and review\n\s+if: (.*)\n`).FindStringSubmatch(work)
	other := regexp.MustCompile(`- name: (Work on another subscription.*)\n\s+if: (.*)\n`).FindStringSubmatch(work)
	if shared == nil || other == nil {
		t.Fatal("AttributeError: 'NoneType' object has no attribute 'group'")
	}
	assertEqual(t, shared[1], "env.PLAN_SHARED == 'true'")
	assertEqual(t, other[2], "env.PLAN_SHARED != 'true'")
	assertTrue(t, !strings.HasPrefix(other[1], "Build, check and review"))
	// The workflow changed (#467): `python3 -m harness work --plan` is `harness work --plan`.
	assertEqual(t, strings.Count(work, "harness work --plan"), 2)
}

// #1455: bot-status's loop plans, and starts a run with `plan_ref`. The run skips its gate (and
// so its plan job), names the plan and its runner in its title, and its model job holds the
// dispatch's runner, CLI, secret name and shared flag to the plan and providers.json before any
// step is handed a secret. #1537: it skips deliver too, which the loop does.
func TestWorkflows_ARunTheLoopStartedSkipsGatePlanAndDeliverAndChecksBeforeAnySecret(t *testing.T) {
	text := workflowsNight(t)
	assertIn(t, "run-name: ${{ inputs.plan_ref && format('bot-night {0} on {1}', "+
		"inputs.plan_ref, inputs.runs_on) || 'bot-night' }}", text)
	assertIn(t, "if: ${{ !inputs.plan_ref }}", workflowsJob(t, text, "gate"))
	work := workflowsJob(t, text, "work")
	assertIn(t, "if: ${{ !cancelled() && (inputs.plan_ref || ", work)
	afterSteps := strings.SplitN(work, "    steps:\n", 2)
	if len(afterSteps) < 2 {
		t.Fatal("IndexError: the work job has no steps")
	}
	steps := workflowsNextStep.Split(afterSteps[1], -1)[1:]
	heads := []string{}
	for _, body := range steps {
		heads = append(heads, strings.SplitN(body, "\n", 2)[0])
	}
	index := func(head string) int {
		t.Helper()
		i := slices.Index(heads, head)
		if i < 0 {
			t.Fatalf("ValueError: %q is not in list", head)
		}
		return i
	}
	check := index("name: The loop's plan, held to providers.json")
	assertLess(t, check, index("name: Doctor (stops here, before any model call, if "+
		"something is missing)"))
	for i, body := range steps {
		if strings.Contains(body, "secrets[") {
			if !(i > check) {
				t.Fatalf("%d not greater than %d %s", i, check, heads[i])
			}
		}
	}
	// The workflow changed (#467): `python3 -m harness plan-fetch` is `harness plan-fetch`.
	assertIn(t, `harness plan-fetch --ref "$PLAN_REF" --out `+
		`"$RUNNER_TEMP/plan/plan.json" --check`, work)
	deliver := workflowsJob(t, text, "deliver")
	assertIn(t, "if: ${{ always() && !inputs.plan_ref && contains(", deliver)
	// Only the plan job's artifact is left for it to read.
	assertNotIn(t, "plan-fetch", deliver)
	assertIn(t, "name: plan\n", deliver)
}

func TestWorkflows_PlansRunOneAtATimeAndRunsInParallel(t *testing.T) {
	text := workflowsNight(t)
	// As the Python: no re.M, so `^` is the start of the file only.
	assertNotRegex(t, text, `^concurrency:`, "a workflow-wide group would serialize runs")
	assertIn(t, "group: bot-night-plan", workflowsJob(t, text, "plan"))
}

var workflowsStepName = regexp.MustCompile(`- name: (.*)`)

// bright-bots-harness treats `Build, check and review` as this bot spending, and its own gate
// must never look like spending (bot/harness/quiet.py).
func TestWorkflows_TheStepNamesThePartnerReads(t *testing.T) {
	text := workflowsNight(t)
	work := workflowsJob(t, text, "work")
	assertIn(t, "- name: Build, check and review", work)
	gate := workflowsJob(t, text, "gate")
	assertIn(t, "- name: Wait until the subscription is quiet", gate)
	cfg := config.Load("", map[string]string{})
	prefixes := []string{}
	for _, partner := range cfg.Quiet.Partners {
		partner.Workflows.Items(func(_ string, steps any) bool {
			prefixes = append(prefixes, pyjson.Strings(steps)...)
			return true
		})
	}
	for _, name := range workflowsFindall(workflowsStepName, text) {
		if strings.HasPrefix(name, "Build, check and review") {
			continue
		}
		assertTrue(t, !slices.ContainsFunc(prefixes, func(p string) bool {
			return strings.HasPrefix(name, p)
		}), name)
	}
}

var workflowsStepHead = regexp.MustCompile(`(?m)^      - (?:name|uses): (.*)$`)

func TestWorkflows_HaltIsTheFirstStepOfTheGateAndThePlan(t *testing.T) {
	text := workflowsNight(t)
	for _, name := range []string{"gate", "plan"} {
		steps := workflowsFindall(workflowsStepHead, workflowsJob(t, text, name))
		if len(steps) == 0 {
			t.Fatalf("IndexError: the %s job has no steps", name)
		}
		assertTrue(t, strings.HasPrefix(steps[0], "HALT check"), name, steps)
	}
}

func TestWorkflows_NoCheckoutKeepsCredentials(t *testing.T) {
	for _, c := range []struct{ name, text string }{
		{"bot-night.yml", workflowsRead(t, "bot-night.yml")},
		{"bot-commands.yml", workflowsRead(t, "bot-commands.yml")},
		{"ci.yml's bot selftest", workflowsSelftest(t)},
	} {
		checkouts := strings.Count(c.text, "uses: actions/checkout@")
		assertEqual(t, strings.Count(c.text, "persist-credentials: false"), checkouts, c.name)
	}
}

func TestWorkflows_EveryJobHasATimeoutAndTheGroupNeverCancels(t *testing.T) {
	text := workflowsNight(t)
	for _, name := range []string{"gate", "plan", "work", "deliver"} {
		assertRegex(t, workflowsJob(t, text, name), `timeout-minutes: \d+`)
	}
	gate := workflowsInt(t, workflowsSearch(t, `timeout-minutes: (\d+)`, workflowsJob(t, text, "gate"), 1))
	// No gate step waits any more (the quiet wait is retired), so a stuck gate fails fast instead
	// of holding its run for hours.
	assertLessEqual(t, gate, 30)
	assertIn(t, "cancel-in-progress: false", text)
}

var workflowsCron = regexp.MustCompile(`cron: "([^"]+)"`)

// The gate reads the window itself, and a forced item never waits for the night.
func TestWorkflows_ARunFiresEveryHourAllDay(t *testing.T) {
	assertEqual(t, workflowsFindall(workflowsCron, workflowsNight(t)), []string{"17 * * * *"})
}

func TestWorkflows_DeliverTakesTheItemFromThePlanJobsOutputs(t *testing.T) {
	deliver := workflowsJob(t, workflowsNight(t), "deliver")
	assertIn(t, "needs.plan.outputs.action", deliver)
	assertIn(t, "needs.plan.outputs.number", deliver)
}

func TestWorkflows_TheBudgetFitsInsideTheJobTimeout(t *testing.T) {
	work := workflowsJob(t, workflowsNight(t), "work")
	timeout := workflowsInt(t, workflowsSearch(t, `timeout-minutes: (\d+)`, work, 1))
	cfg := config.Load("", map[string]string{})
	assertLess(t, cfg.JobBudgetMinutes+10, timeout)
	assertLessEqual(t, timeout, 360)
}

// ---------------------------------------------------------------------------- BuildCacheWorkflowTests
//
// bot-cache.yml saves the Rust cache the model job restores on GitHub's runners (#1263): only from
// main, with no model and no secret, keyed as the model job keys it, and holding what the gates
// compile.

// workflowsRustCache is BuildCacheWorkflowTests.RUST_CACHE.
const workflowsRustCache = "Swatinem/rust-cache@v2"

// workflowsInput1 is `inputs(text, action)[key]`: a KeyError when absent.
func workflowsInput1(t *testing.T, inputs map[string]string, key string) string {
	t.Helper()
	v, ok := inputs[key]
	if !ok {
		t.Fatalf("KeyError: %q", key)
	}
	return v
}

func TestWorkflows_MainAloneSavesItWithNoSecretAndNoPullRequestCode(t *testing.T) {
	cache := workflowsRead(t, "bot-cache.yml")
	assertNotIn(t, "secrets", cache)
	assertNotRegex(t, cache, `(?m)^  pull_request`, "a pull request's code must not write it")
	assertIn(t, "branches: [main]", cache)
	assertRegex(t, cache, `(?m)^permissions:\n  contents: read\n`)
	assertEqual(t, workflowsInput1(t, workflowsInputs(t, cache, workflowsRustCache), "save-if"),
		"${{ github.ref == 'refs/heads/main' }}")
	assertEqual(t, strings.Count(cache, "persist-credentials: false"),
		strings.Count(cache, "uses: actions/checkout@"))
}

// A model ran in that job: what it left must not reach a later job, on another subscription,
// through the cache.
func TestWorkflows_TheModelJobRestoresAndNeverSaves(t *testing.T) {
	work := workflowsJob(t, workflowsNight(t), "work")
	assertEqual(t, workflowsInput1(t, workflowsInputs(t, work, workflowsRustCache), "save-if"), "false")
	assertIn(t, "uses: actions/cache/restore@", work)
	assertNotRegex(t, work, `uses: actions/cache(/save)?@`)
	for _, head := range []string{"uses: " + workflowsRustCache, "uses: actions/cache/restore@v4",
		"name: The night bot's build directories", "name: Only the pinned toolchain"} {
		assertIn(t, "if: startsWith(env.PLAN_RUNS_ON, 'ubuntu-')",
			strings.SplitN(workflowsStep(t, work, head), "\n", 2)[0], head)
	}
	// The workflow changed (#467): the harness action this job uses saves the harness binary's
	// cache (.github/actions/harness), a save the job's own text does not show. It runs before any
	// step is handed a secret, so before any model, and nothing a model left reaches a later job
	// that way either.
	assertLess(t, strIndex(t, work, "uses: ./.github/actions/harness"),
		strIndex(t, work, "secrets["))
}

// rust-cache keys its entry on the environment, every toolchain rustup has and its own inputs, so
// a difference in any of them is a cache the model job never hits.
func TestWorkflows_BothKeyTheEntryAlike(t *testing.T) {
	cache := workflowsRead(t, "bot-cache.yml")
	work := workflowsJob(t, workflowsNight(t), "work")
	for _, name := range []string{"The night bot's build directories", "Only the pinned toolchain"} {
		assertEqual(t, workflowsScript(t, work, name), workflowsScript(t, cache, name), name)
		for _, text := range []string{work, cache} {
			assertLess(t, strIndex(t, text, "- name: "+name),
				strIndex(t, text, workflowsRustCache), name)
		}
	}
	ours, theirs := workflowsInputs(t, work, workflowsRustCache), workflowsInputs(t, cache, workflowsRustCache)
	workflowsInput1(t, ours, "save-if")
	delete(ours, "save-if")
	workflowsInput1(t, theirs, "save-if")
	delete(theirs, "save-if")
	assertEqual(t, ours, theirs)
	setup := readText(t, filepath.Join(config.RepoRoot, ".github", "actions", "setup", "action.yml"))
	assertIn(t, "for toolchain in $(rustup toolchain list --quiet); do", setup)
}

var workflowsCargo = regexp.MustCompile(`\bcargo (\w+)`)

func TestWorkflows_ItBuildsWhatTheGatesCompile(t *testing.T) {
	cache := workflowsRead(t, "bot-cache.yml")
	gates := map[string]string{}
	for _, gate := range config.Load("", map[string]string{}).Gates {
		gates[gate.Name] = gate.Run
	}
	// A gate that compiles something new needs a step here too.
	used := map[string]bool{}
	for _, run := range gates {
		for _, sub := range workflowsFindall(workflowsCargo, run) {
			used[sub] = true
		}
	}
	assertEqual(t, used, setOf("fmt", "jackioh", "clippy", "test"))
	aliases := readText(t, filepath.Join(config.RepoRoot, ".cargo", "config.toml"))
	assertIn(t, `jackioh = "run --release -p jackioh-tools --"`, aliases)
	assertIn(t, "run: cargo build --release -p jackioh-tools\n", cache)
	assertEqual(t, workflowsInput1(t, gates, "clippy"), "cargo clippy --workspace --all-targets -- -D warnings")
	assertIn(t, "run: cargo clippy --workspace --all-targets\n", cache)
	tests := strings.Split(workflowsInput1(t, gates, "cargo tests"), " && ")[0]
	assertIn(t, "run: "+tests+" --no-run\n", cache)
	assertTrue(t, strings.HasPrefix(workflowsInput1(t, gates, "web tests"), "sh scripts/build-wasm.sh && "))
	assertIn(t, "run: sh scripts/build-wasm.sh\n", cache)
}

func TestWorkflows_ActionlintReadsIt(t *testing.T) {
	assertTrue(t, workflowsLinted(t, "bot-cache.yml"))
}

// ---------------------------------------------------------------------------- CommandsWorkflowTests

func workflowsCommands(t *testing.T) string {
	t.Helper()
	return workflowsRead(t, "bot-commands.yml")
}

func TestWorkflows_NoModelAndNoPullRequestCode(t *testing.T) {
	text := workflowsCommands(t)
	// The model secrets are named only to say whether each is set, never handed over.
	for _, secret := range providers.SecretNames {
		assertNotRegex(t, text, `\b`+secret+`\s*:`)
		for _, mention := range workflowsFindall(regexp.MustCompile(`secrets\.`+secret+`\b[^}]*`), text) {
			assertTrue(t, strings.HasPrefix(mention, "secrets."+secret+" != '' && '"+secret+"'"), mention)
		}
	}
	assertNotIn(t, "secrets[", text)
	assertNotIn(t, "claude-code", text)
	assertNotIn(t, "github.event.pull_request.head", text)
	assertIn(t, "ref: ${{ github.event.repository.default_branch || 'main' }}", text)
	assertNotIn(t, "concurrency:", text)
}

func TestWorkflows_TheBotDoesNotWakeItself(t *testing.T) {
	text := workflowsCommands(t)
	cfg := config.Load("", map[string]string{})
	assertIn(t, "github.actor != '"+cfg.BotLogin+"'", text)
	assertIn(t, "'@"+cfg.BotLogin+"'", text)
	assertIn(t, "workflows: ["+cfg.CIWorkflow+"]", text)
}

func TestWorkflows_EveryWayARequestArrivesIsHeard(t *testing.T) {
	text := workflowsCommands(t)
	for _, trigger := range []string{"issue_comment:\n    types: [created, edited]",
		"pull_request_review_comment:\n    types: [created, edited]",
		"pull_request_review:", "issues:", "pull_request_target:", "workflow_run:",
		"workflow_dispatch:"} {
		assertIn(t, trigger, text)
	}
	// The workflow changed (#467): `python -m harness sweep` is `harness sweep`.
	assertIn(t, "harness sweep", text)
}

// It is also the fallback that starts a dropped night run (harness/sweep.py).
func TestWorkflows_TheSweepLeavesNoGapLongerThanTenMinutes(t *testing.T) {
	crons := workflowsFindall(workflowsCron, workflowsCommands(t))
	assertEqual(t, len(crons), 1)
	parts := strings.SplitN(crons[0], " ", 2)
	if len(parts) != 2 {
		t.Fatalf("ValueError: not enough values to unpack: %q", crons[0])
	}
	minutes, rest := parts[0], parts[1]
	assertEqual(t, rest, "* * * *")
	marks := []int{}
	for _, m := range strings.Split(minutes, ",") {
		marks = append(marks, workflowsInt(t, m))
	}
	sort.Ints(marks)
	ends := append(slices.Clone(marks[1:]), marks[0]+60)
	most := 0
	for i, a := range marks {
		if gap := ends[i] - a; i == 0 || gap > most {
			most = gap
		}
	}
	assertLessEqual(t, most, 10, marks)
	assertTrue(t, !slices.Contains(marks, 0), marks) // the top of the hour is when GitHub drops the most
}

func TestWorkflows_PullRequestEventsUseTarget(t *testing.T) {
	text := workflowsCommands(t)
	assertIn(t, "pull_request_target:", text)
	// As the Python: no re.M, so `^` is the start of the file only.
	assertNotRegex(t, text, `^  pull_request:`)
}

// ---------------------------------------------------------------------------- StatusIssueStepTests
//
// The pinned status issue is rewritten after every sweep, and never fails the sweep.

func TestWorkflows_AfterTheSweepOnTheSweepsScheduleOnly(t *testing.T) {
	text := workflowsCommands(t)
	step := text[strIndex(t, text, "- name: Rewrite the pinned status issue"):]
	// The workflow changed (#467): `python -m harness sweep` and `python -m harness dashboard` are
	// `harness sweep` and `harness dashboard`.
	assertLess(t, strIndex(t, text, "harness sweep"),
		strIndex(t, text, "- name: Rewrite the pinned status issue"))
	assertIn(t, "if: github.event_name == 'schedule' || github.event_name == "+
		"'workflow_dispatch'", step)
	assertIn(t, "continue-on-error: true", step)
	assertIn(t, "run: harness dashboard", step)
	assertIn(t, "BOT_GITHUB_TOKEN: ${{ secrets.BOT_GITHUB_TOKEN }}", step)
}

// ---------------------------------------------------------------------------- StatusLoopWorkflowTests
//
// bot-status keeps the pinned status issue current every five minutes, whatever GitHub does with
// schedules: one job loops and starts the next loop itself.

func workflowsStatus(t *testing.T) string {
	t.Helper()
	return workflowsRead(t, "bot-status.yml")
}

func TestWorkflows_OneJobRewritesItEveryFiveMinutes(t *testing.T) {
	text := workflowsStatus(t)
	refresh := workflowsJob(t, text, "refresh")
	assertIn(t, "runs-on: ubuntu-latest", refresh)
	assertIn(t, `EVERY_SECONDS: "300"`, text)
	assertEqual(t, dashboard.EveryMinutes*60, 300) // what the issue says it does
	// The workflow changed (#467): `python -m harness dashboard` is `harness dashboard`.
	assertIn(t, "harness dashboard --sweep --stats --companions --controller \\\n"+
		`            --every "${EVERY_SECONDS}" --runners-every `+
		`"${RUNNERS_EVERY_SECONDS}" \`+"\n"+`            --for "${LOOP_SECONDS}"`, refresh)
	// Its runners section is redrawn alone between the rewrites, as often as the issue says.
	assertIn(t, `RUNNERS_EVERY_SECONDS: "120"`, text)
	assertEqual(t, dashboard.RunnersMinutes*60, 120)
	// The loop ends inside GitHub's six-hour cap, ahead of its own timeout.
	loop := workflowsInt(t, workflowsSearch(t, `LOOP_SECONDS: "(\d+)"`, text, 1))
	timeout := workflowsInt(t, workflowsSearch(t, `timeout-minutes: (\d+)`, refresh, 1))
	assertLess(t, loop, timeout*60)
	assertLessEqual(t, timeout, 360)
}

func TestWorkflows_ItStartsTheNextLoopAndAScheduleRestartsAStoppedOne(t *testing.T) {
	text := workflowsStatus(t)
	assertRegex(t, text, `schedule:\n\s+- cron: `)
	assertIn(t, "workflow_dispatch: {}", text)
	assertIn(t, "group: bot-status\n  cancel-in-progress: false", text)
	nxt := text[strIndex(t, text, "- name: Start the next loop"):]
	assertIn(t, "if: success() || failure()", nxt) // not after a cancellation
	assertIn(t, `gh workflow run bot-status.yml --repo "${REPO}" --ref "${REF}"`, nxt)
	assertIn(t, "actions: write", workflowsJob(t, text, "refresh"))
}

func TestWorkflows_NoModelSecret(t *testing.T) {
	text := workflowsStatus(t)
	assertNotIn(t, "HARNESS_PROVIDER_SECRET", text)
	assertNotIn(t, "secrets[", text)
	assertNotIn(t, "harness work", text)
}

// ---------------------------------------------------------------------------- TriageWorkflowTests
//
// #664: one job on GitHub's runner, from fixed rules: no model, no machine, no wait.

func workflowsTriage(t *testing.T) string {
	t.Helper()
	return workflowsRead(t, "triage.yml")
}

var (
	workflowsRef     = regexp.MustCompile(`ref: (.*)`)
	workflowsJobName = regexp.MustCompile(`(?m)^  (\w[\w-]*):$`)
)

func TestWorkflows_IssuesAndPullRequestsWhenOpenedNeverTheirCode(t *testing.T) {
	text := workflowsTriage(t)
	assertRegex(t, text, `issues:\n\s+types: \[opened, labeled\]`)
	assertRegex(t, text, `pull_request_target:\n\s+types: \[opened\]`)
	// A person can call it on any thread by number.
	assertRegex(t, text, `workflow_dispatch:\n\s+inputs:\n\s+number:`)
	// As the Python: no re.M, so `^` is the start of the file only.
	assertNotRegex(t, text, `^  pull_request:`, "would run the pull request's code")
	refs := workflowsFindall(workflowsRef, text)
	assertEqual(t, len(refs), 1)
	assertIn(t, "default_branch", refs[0])
	assertIn(t, "persist-credentials: false", text)
	assertNotIn(t, "secrets.", text)
}

func TestWorkflows_OneJobOnGithubsRunnerThatRunsNoModel(t *testing.T) {
	text := workflowsTriage(t)
	jobs := strings.SplitN(text, "\njobs:\n", 2)
	if len(jobs) < 2 {
		t.Fatal("IndexError: no jobs")
	}
	assertEqual(t, workflowsFindall(workflowsJobName, jobs[1]), []string{"triage"})
	triageJob := workflowsJob(t, text, "triage")
	assertIn(t, "runs-on: ubuntu-latest", triageJob)
	assertNotIn(t, "night-vm", text)
	assertNotIn(t, "muse", strings.ToLower(text))
	assertEqual(t, workflowsGrants(t, workflowsJob(t, text, "triage")),
		map[string]string{"contents": "read", "issues": "write", "pull-requests": "write"})
	// The workflow changed (#467): `python3 -m harness triage` is `harness triage`.
	assertIn(t, `harness triage --payload "$GITHUB_EVENT_PATH"`, triageJob)
	assertIn(t, "timeout-minutes:", triageJob)
	assertTrue(t, workflowsLinted(t, "triage.yml"))
}

func TestWorkflows_OnlyHumanOfTheLabelsStartsIt(t *testing.T) {
	assertIn(t, "if: github.event_name != 'issues' || github.event.action == 'opened' || "+
		"github.event.label.name == 'human'", workflowsJob(t, workflowsTriage(t), "triage"))
}

// ---------------------------------------------------------------------------- WakeWorkflowTests

func workflowsWake(t *testing.T) string {
	t.Helper()
	return workflowsRead(t, "claude-wake.yml")
}

func TestWorkflows_EachWakeHoldsOneSecretAndNoWriteToken(t *testing.T) {
	ping := workflowsJob(t, workflowsWake(t), "ping")
	assertIn(t, "permissions: {}", ping)
	assertEqual(t, setOf(workflowsFindall(workflowsSecretUse, ping)...),
		setOf("secrets[matrix.account.secret]"))
}

var workflowsSecretMention = regexp.MustCompile(`secrets\.(\w+)[^}]*`)

func TestWorkflows_TheClockReadsOnlyAndLearnsOnlyWhichSecretsAreSet(t *testing.T) {
	clock := workflowsJob(t, workflowsWake(t), "clock")
	assertEqual(t, workflowsGrants(t, clock), map[string]string{"contents": "read", "actions": "read"})
	for _, mention := range workflowsFindall(workflowsSecretMention, clock) {
		assertIn(t, "secrets."+mention+" != '' && '"+mention+"'", clock)
	}
	assertNotIn(t, "secrets[", clock)
}

func TestWorkflows_ItIsLinted(t *testing.T) {
	assertTrue(t, workflowsLinted(t, "claude-wake.yml"))
}

// ---------------------------------------------------------------------------- LearnSetWorkflowTests
//
// A new set stops a learn export still early on the set before (learn-set.yml); every learn run
// plays the newest patch, reading main as it is when it starts (learn.yml, plan).

func TestWorkflows_AShippedPatchOnMainStartsItOnGithubsRunner(t *testing.T) {
	text := workflowsRead(t, "learn-set.yml")
	assertIn(t, "  push:\n    branches: [main]\n    paths: [\"crates/cards/patches/patches.json\"]\n", text)
	assertNotIn(t, "schedule:", text)
	stop := workflowsJob(t, text, "stop")
	assertIn(t, "runs-on: ubuntu-latest", stop)
	assertIn(t, "timeout-minutes:", stop)
	assertIn(t, "cancel-in-progress: false", text)
	assertTrue(t, workflowsLinted(t, "learn-set.yml"))
}

func TestWorkflows_OnlyASetStopsARunAndOnlyBeforeHalfItsShards(t *testing.T) {
	text := workflowsRead(t, "learn-set.yml")
	learn := workflowsRead(t, "learn.yml")
	assertIn(t, `select(.tier == "major")`, text)
	keep := regexp.MustCompile(`LEARN_SET_KEEP_SHARDS: "(\d+)"`).FindStringSubmatch(text)
	shards := regexp.MustCompile(`shards: \$\{\{ inputs\.shards \|\| '(\d+)' \}\}`).FindStringSubmatch(learn)
	assertTrue(t, keep != nil, "unexpectedly None")
	assertTrue(t, shards != nil, "unexpectedly None")
	assertEqual(t, workflowsInt(t, keep[1])*2, workflowsInt(t, shards[1]))
	assertIn(t, `[ "$finished" -ge "$LEARN_SET_KEEP_SHARDS" ]`, text)
}

func TestWorkflows_ALearnRunPlansAndBuildsMainAsItIsWhenItStarts(t *testing.T) {
	learn := workflowsRead(t, "learn.yml")
	plan := workflowsJob(t, learn, "plan")
	assertIn(t, "- uses: actions/checkout@v4\n        with:\n          ref: main\n", plan)
	assertIn(t, `echo "sha=$(git rev-parse HEAD)"`, plan)
	assertIn(t, "ref: ${{ needs.plan.outputs.sha }}", workflowsJob(t, learn, "export"))
	build := workflowsJob(t, workflowsRead(t, "selfplay.yml"), "build")
	assertIn(t, "- uses: actions/checkout@v4\n        with:\n          ref: ${{ inputs.ref }}\n", build)
}

// ---------------------------------------------------------------------------- RequiredChecksTests

var (
	workflowsOnPullRequest = regexp.MustCompile(`(?m)^  pull_request:|^  pull_request_target:\n\s+types: \[[^\]]*` +
		`\bsynchronize\b`)
	workflowsJobTitle  = regexp.MustCompile(`(?m)^    name: (.+)$`)
	workflowsInputName = regexp.MustCompile(`\$\{\{ inputs\.[\w-]+ == '[^']*' && '[^']*' \|\| '([^']*)' \}\}`)
)

func TestWorkflows_EveryRequiredCheckIsAJobThatRunsOnEveryPullRequest(t *testing.T) {
	cfg := config.Load("", map[string]string{})
	names := map[string]bool{}
	paths, err := filepath.Glob(filepath.Join(workflowsDir, "*.yml"))
	if err != nil {
		t.Fatal(err)
	}
	for _, path := range paths {
		text := readText(t, path)
		// A `pull_request_target` workflow that runs again on every push counts too: the title
		// check (`pr-title.yml`, #187), should it be made required.
		if !workflowsOnPullRequest.MatchString(text) {
			continue
		}
		for _, raw := range workflowsFindall(workflowsJobTitle, text) {
			raw = strings.Trim(strings.TrimSpace(raw), `"`)
			// A name chosen by a dispatch input (`inputs.x == 'v' && 'A' || 'B'`) is its `B` on a
			// pull request, which has no inputs.
			raw = workflowsInputName.ReplaceAllString(raw, "${1}")
			if strings.Contains(raw, "${{ matrix.browser }}") {
				for _, b := range []string{"chrome", "electron"} {
					names[strings.ReplaceAll(raw, "${{ matrix.browser }}", b)] = true
				}
			} else {
				names[raw] = true
			}
		}
	}
	missing := []string{}
	for _, check := range cfg.RequiredChecks {
		if !names[check] {
			missing = append(missing, check)
		}
	}
	assertEqual(t, missing, []string{}, "required checks with no job", missing, "jobs", sortedKeys(names))
}

// `main`'s merge queue (infra/github.tf) waits for the same checks on its own builds: a workflow
// that reports one and not on `merge_group` takes every pull request out at the queue's timeout.
// The `changes` job scopes a group's diff as a pull request's.
func TestWorkflows_TheRequiredChecksRunInTheMergeQueueToo(t *testing.T) {
	text := workflowsRead(t, "ci.yml")
	assertRegex(t, text, `(?m)^  merge_group:`)
	scope := workflowsJob(t, text, "changes")
	assertIn(t, "HEAD: ${{ github.event.pull_request.head.sha"+
		" || github.event.merge_group.head_sha }}", scope)
	assertIn(t, `if [ "$EVENT" = pull_request ] || [ "$EVENT" = merge_group ]; then`, scope)
}

// Each required check's own job cancels the rest of a merge-queue build that it failed. A cancel
// that ran before a summary job would leave that check skipped, which counts as passing, so each
// waits on the summary itself and runs only on its failure.
func TestWorkflows_AFailedQueueBuildIsCancelledOnlyAfterItsCheckReported(t *testing.T) {
	text := workflowsRead(t, "ci.yml")
	jobs := [][2]string{{"rust", "rust"}, {"web", "web"}, {"e2e-smoke", "e2e smoke"}, {"db", "db"},
		{"bot-selftest", "bot selftest"}}
	values := map[string]bool{}
	for _, j := range jobs {
		values[j[1]] = true
	}
	assertEqual(t, values, setOf(config.Load("", map[string]string{}).RequiredChecks...))
	for _, j := range jobs {
		summary := j[0]
		cancel := workflowsJob(t, text, "queue-cancel-"+summary)
		assertIn(t, "needs: "+summary+"\n", cancel)
		assertIn(t, "if: ${{ failure() && github.event_name == 'merge_group' }}", cancel)
		assertIn(t, `gh run cancel "$RUN"`, cancel)
	}
}
