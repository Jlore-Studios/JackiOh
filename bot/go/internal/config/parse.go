package config

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

var required = []string{
	"repo",
	"default_branch",
	"bot_login",
	"bot_user_id",
	"operator",
	"timezone",
	"max_turns",
	"call_timeout_minutes",
	"job_budget_minutes",
	"min_minutes_for_a_call",
	"max_review_cycles",
	"max_failures",
	"auto_merge",
	"merge_method",
	"ci_workflow",
	"required_checks",
	"ci_reruns",
	"suggestions",
	"quiet",
	"forbidden_paths",
	"review_paths",
	"upload_transcripts",
	"install",
	"gates",
}

var roles = []string{"plan", "build", "fix", "revise", "review", "suggest"}

// optional is _OPTIONAL: keys `.harness/config.json` may leave out, with their defaults.
// `identity` is read once, when the process starts (`identity.py`); `parse` only checks it.
var optional = pyjson.New("max_self_check_rounds", 3, "easy", pyjson.New(), "identity", nil,
	"controller", false, "resolvers", []any{})

func raiseConfig(format string, args ...any) {
	herr.Raise(&herr.ConfigError{Msg: fmt.Sprintf(format, args...)})
}

// unknownKeys is sorted(set(raw) - known).
func unknownKeys(raw *pyjson.Object, known ...[]string) []string {
	out := []string{}
	for _, k := range raw.Keys() {
		found := false
		for _, list := range known {
			found = found || slices.Contains(list, k)
		}
		if !found {
			out = append(out, k)
		}
	}
	sort.Strings(out)
	return out
}

func easyRule(raw any) *EasyRule {
	o, ok := raw.(*pyjson.Object)
	if !ok {
		raiseConfig("easy: expected an object")
	}
	if unknown := unknownKeys(o, []string{"max_files", "max_lines", "off_limits", "generated"}); len(unknown) > 0 {
		raiseConfig("easy: unknown keys %s", strings.Join(unknown, ", "))
	}
	rule, err := func() (*EasyRule, error) {
		maxFiles, err := py.Int(o.GetOr("max_files", 10))
		if err != nil {
			return nil, err
		}
		maxLines, err := py.Int(o.GetOr("max_lines", 400))
		if err != nil {
			return nil, err
		}
		offLimits, err := strs(py.Or(o.Get("off_limits"), []any{}))
		if err != nil {
			return nil, err
		}
		generated, err := strs(py.Or(o.Get("generated"), []any{}))
		if err != nil {
			return nil, err
		}
		return &EasyRule{MaxFiles: maxFiles, MaxLines: maxLines, OffLimits: offLimits, Generated: generated}, nil
	}()
	if err != nil {
		raiseConfig("easy: %s", err)
	}
	if rule.MaxFiles < 1 || rule.MaxLines < 1 {
		raiseConfig("easy: max_files and max_lines must be at least 1")
	}
	return rule
}

func gate(raw any, where string) *Gate {
	o, ok := raw.(*pyjson.Object)
	if !ok {
		raiseConfig("%s: expected an object", where)
	}
	g, err := func() (*Gate, error) {
		name, err := subscript(o, "name")
		if err != nil {
			return nil, err
		}
		run, err := subscript(o, "run")
		if err != nil {
			return nil, err
		}
		timeout, err := subscript(o, "timeout_minutes")
		if err != nil {
			return nil, err
		}
		minutes, err := py.Int(timeout)
		if err != nil {
			return nil, err
		}
		return &Gate{Name: py.Str(name), Run: py.Str(run), TimeoutMinutes: minutes,
			Machine: pyjson.Truthy(o.GetOr("machine", true))}, nil
	}()
	if err != nil {
		raiseConfig("%s: needs name, run and timeout_minutes (%s)", where, err)
	}
	return g
}

// resolvers is _resolvers: `resolvers`: each names one file by its plain repository path, and
// either `take` (`main`) or `run` (a command, with an optional whole `timeout_minutes`); a path at
// most once, and one `run` resolver at most, since each command builds the merged tree, which
// another's conflict markers would break.
func resolvers(raw any) []*Resolver {
	list, ok := raw.([]any)
	if !ok {
		raiseConfig("resolvers: expected a list")
	}
	out := []*Resolver{}
	for i, item := range list {
		where := fmt.Sprintf("resolvers[%d]", i)
		entry, ok := item.(*pyjson.Object)
		if !ok {
			raiseConfig("%s: expected an object", where)
		}
		if unknown := unknownKeys(entry, []string{"path", "take", "run", "timeout_minutes"}); len(unknown) > 0 {
			raiseConfig("%s: unknown keys %s", where, strings.Join(unknown, ", "))
		}
		values := map[string]string{}
		for _, k := range []string{"path", "take", "run"} {
			s, ok := entry.GetOr(k, "").(string)
			if !ok {
				raiseConfig("%s: `path`, `take` and `run` are strings", where)
			}
			values[k] = s
		}
		path := strings.TrimSpace(values["path"])
		take := strings.TrimSpace(values["take"])
		run := strings.TrimSpace(values["run"])
		badPart := false
		for _, part := range strings.Split(path, "/") {
			badPart = badPart || part == "" || part == "." || part == ".."
		}
		if path == "" || strings.ContainsAny(path, "*?[\\") || strings.HasPrefix(path, "/") || badPart {
			raiseConfig("%s: `path` must name one file by its plain repository path, with no glob", where)
		}
		if (take != "") == (run != "") {
			raiseConfig("%s: give exactly one of `take` and `run`", where)
		}
		if take != "" && !slices.Contains(ResolverTakes, take) {
			raiseConfig("%s: `take` is one of %s", where, strings.Join(ResolverTakes, ", "))
		}
		if take != "" && entry.Has("timeout_minutes") {
			raiseConfig("%s: `timeout_minutes` is for a `run` resolver", where)
		}
		minutes, isInt := entry.GetOr("timeout_minutes", 10).(int)
		if !isInt || minutes < 1 {
			raiseConfig("%s: timeout_minutes must be a whole number of at least 1", where)
		}
		for _, r := range out {
			if r.Path == path {
				raiseConfig("%s: %s has a resolver already", where, path)
			}
		}
		if run != "" {
			for _, r := range out {
				if r.Run != "" {
					raiseConfig("%s: one `run` resolver at most", where)
				}
			}
		}
		out = append(out, &Resolver{Path: path, Take: take, Run: run, TimeoutMinutes: minutes})
	}
	return out
}

// priorities is _priorities: `suggestions.priorities`: each key a known class, each value a
// priority level.
func priorities(raw any) map[string]string {
	o, ok := raw.(*pyjson.Object)
	if !ok {
		raiseConfig("suggestions.priorities: expected an object")
	}
	unknown := []string{}
	bad := []string{}
	out := map[string]string{}
	o.Items(func(k string, v any) bool {
		if !slices.Contains(SuggestionClasses, k) {
			unknown = append(unknown, k)
		}
		if s, isStr := v.(string); !isStr || !slices.Contains(PriorityLevels, s) {
			bad = append(bad, k+": "+py.Repr(v))
		}
		out[k] = py.Str(v)
		return true
	})
	sort.Strings(unknown)
	if len(unknown) > 0 {
		raiseConfig("suggestions.priorities: unknown classes %s (known: %s)",
			strings.Join(unknown, ", "), strings.Join(SuggestionClasses, ", "))
	}
	sort.Strings(bad)
	if len(bad) > 0 {
		raiseConfig("suggestions.priorities: %s is not one of %s", strings.Join(bad, "; "),
			strings.Join(PriorityLevels, ", "))
	}
	return out
}

func quiet(raw any) *Quiet {
	o, ok := raw.(*pyjson.Object)
	if !ok {
		raiseConfig("quiet: expected an object")
	}
	partners := []*Partner{}
	entries, err := py.Iter(orNoList(o.Get("partners")))
	herr.Must(err)
	for i, entry := range entries {
		p, err := func() (*Partner, error) {
			found, err := subscript(entry, "workflows")
			if err != nil {
				return nil, err
			}
			raw, err := pyDict(found)
			if err != nil {
				return nil, err
			}
			workflows := pyjson.New()
			for _, k := range raw.Keys() {
				steps, err := strs(raw.Get(k))
				if err != nil {
					return nil, err
				}
				workflows.Set(k, pyjson.ListOf(steps))
			}
			repo, err := subscript(entry, "repo")
			if err != nil {
				return nil, err
			}
			return &Partner{Repo: py.Str(repo), Workflows: workflows}, nil
		}()
		if err != nil {
			raiseConfig("quiet.partners[%d]: needs repo and workflows (%s)", i, err)
		}
		partners = append(partners, p)
	}
	interval := py.MustInt(o.GetOr("interval_minutes", 10))
	if interval < 1 {
		raiseConfig("quiet.interval_minutes: must be at least 1")
	}
	return &Quiet{
		Enabled:         pyjson.Truthy(o.GetOr("enabled", true)),
		IntervalMinutes: interval,
		MaxWaitMinutes:  py.MustInt(o.GetOr("max_wait_minutes", 120)),
		PingModel:       py.Str(o.GetOr("ping_model", "haiku")),
		Partners:        partners,
	}
}

// orNoList is `v or []`.
func orNoList(v any) any {
	if !pyjson.Truthy(v) {
		return []any{}
	}
	return v
}

// LaneReviewCycles is review verdict rounds per difficulty when `max_review_cycles` names no
// mapping (#725).
var LaneReviewCycles = pyjson.New("hard", 10, "medium", 3, "easy", 3)

// reviewCycles is _review_cycles: `max_review_cycles` as a per-difficulty mapping: a mapping
// names each difficulty, and a bare number keeps the old shape (every difficulty gets it).
func reviewCycles(raw any) *pyjson.Object {
	cycles := pyjson.New()
	if o, ok := raw.(*pyjson.Object); ok {
		o.Items(func(k string, v any) bool {
			cycles.Set(k, py.MustInt(v))
			return true
		})
	} else {
		for _, difficulty := range LaneReviewCycles.Keys() {
			cycles.Set(difficulty, py.MustInt(raw))
		}
	}
	absent := []string{}
	for _, difficulty := range LaneReviewCycles.Keys() {
		if !cycles.Has(difficulty) {
			absent = append(absent, difficulty)
		}
	}
	if len(absent) > 0 {
		raiseConfig("max_review_cycles: missing %s", strings.Join(absent, ", "))
	}
	return cycles
}

// parseIdentity is identity_mod.parse(raw), whose ValueError is the config's error. An error
// value it raises stands for the ValueError; a runtime fault or a panic that is not an error goes
// on up, as any other exception did.
func parseIdentity(raw any) {
	err := herr.Try(func() {
		_, perr := identity.Parse(raw, true)
		herr.Must(perr)
	})
	if err == nil {
		return
	}
	var fault runtime.Error
	var notError *herr.Panic
	if herr.IsHarness(err) || errors.As(err, &fault) || errors.As(err, &notError) {
		panic(err)
	}
	raiseConfig("%s", err)
}

// Parse is a `Config` from the committed JSON and an environment mapping. Raises `ConfigError`.
// The subscriptions come from `pool`, or from `.harness/providers.json` under `root`.
func Parse(raw *pyjson.Object, root string, env map[string]string, pool *providers.Pool /* = None */) *Config {
	missing := []string{}
	for _, key := range required {
		if !raw.Has(key) {
			missing = append(missing, key)
		}
	}
	if len(missing) > 0 {
		raiseConfig("%s: missing keys %s", ConfigPath, strings.Join(missing, ", "))
	}
	if unknown := unknownKeys(raw, required, optional.Keys()); len(unknown) > 0 {
		raiseConfig("%s: unknown keys %s", ConfigPath, strings.Join(unknown, ", "))
	}
	turns := pyjson.New()
	rawTurns := mustDict(raw.Get("max_turns"))
	for _, k := range rawTurns.Keys() {
		turns.Set(k, py.MustInt(rawTurns.Get(k)))
	}
	absent := []string{}
	for _, role := range roles {
		if !turns.Has(role) {
			absent = append(absent, role)
		}
	}
	if len(absent) > 0 {
		raiseConfig("max_turns: missing %s", strings.Join(absent, ", "))
	}
	if m, ok := raw.Get("merge_method").(string); !ok || !slices.Contains([]string{"squash", "merge", "rebase"}, m) {
		raiseConfig("merge_method: %s is not squash, merge or rebase", py.Repr(raw.Get("merge_method")))
	}
	suggestions := mustDict(raw.Get("suggestions"))
	rawGates, err := py.Iter(raw.Get("gates"))
	herr.Must(err)
	gates := []*Gate{}
	for i, g := range rawGates {
		gates = append(gates, gate(g, fmt.Sprintf("gates[%d]", i)))
	}
	if len(gates) == 0 {
		raiseConfig("gates: at least one gate is required")
	}
	selfChecks := py.MustInt(raw.GetOr("max_self_check_rounds", optional.Get("max_self_check_rounds")))
	if selfChecks < 1 {
		raiseConfig("max_self_check_rounds: must be at least 1")
	}
	parseIdentity(raw.Get("identity"))
	repo := env["GITHUB_REPOSITORY"]
	if repo == "" {
		repo = py.Str(raw.Get("repo"))
	}
	backend, ok := env["HARNESS_BACKEND"]
	if !ok || backend == "" {
		backend = "cli"
	}
	if backend != "cli" && backend != "fake" {
		raiseConfig("HARNESS_BACKEND: %s is not cli or fake", py.Repr(backend))
	}
	c := &Config{Root: root, Repo: repo}
	c.DefaultBranch = py.Str(raw.Get("default_branch"))
	c.BotLogin = py.Str(raw.Get("bot_login"))
	c.BotUserID = py.MustInt(raw.Get("bot_user_id"))
	c.Operator = py.Str(raw.Get("operator"))
	c.Timezone = py.Str(raw.Get("timezone"))
	c.MaxTurns = turns
	c.CallTimeoutMinutes = py.MustInt(raw.Get("call_timeout_minutes"))
	c.JobBudgetMinutes = py.MustInt(raw.Get("job_budget_minutes"))
	c.MinMinutesForACall = py.MustInt(raw.Get("min_minutes_for_a_call"))
	c.MaxReviewCycles = reviewCycles(raw.Get("max_review_cycles"))
	c.MaxFailures = py.MustInt(raw.Get("max_failures"))
	c.AutoMerge = pyjson.Truthy(raw.Get("auto_merge"))
	c.MergeMethod = py.Str(raw.Get("merge_method"))
	c.CIWorkflow = py.Str(raw.Get("ci_workflow"))
	c.RequiredChecks = mustStrs(raw.Get("required_checks"))
	c.CIReruns = py.MustInt(raw.Get("ci_reruns"))
	c.SuggestionsEnabled = pyjson.Truthy(suggestions.GetOr("enabled", true))
	c.SuggestionsMaxOpen = py.MustInt(suggestions.GetOr("max_open", 4))
	c.SuggestionsIdleMinutes = py.MustInt(suggestions.GetOr("idle_minutes", 30))
	c.SuggestionPriorities = priorities(suggestions.GetOr("priorities", pyjson.New()))
	c.Quiet = quiet(raw.Get("quiet"))
	c.ForbiddenPaths = mustStrs(raw.Get("forbidden_paths"))
	c.ReviewPaths = mustStrs(raw.Get("review_paths"))
	c.UploadTranscripts = pyjson.Truthy(raw.Get("upload_transcripts"))
	install, ok := raw.Get("install").(*pyjson.Object)
	if !ok {
		herr.Raise(typeError("unsupported operand type(s) for |: '%s' and 'dict'", py.TypeName(raw.Get("install"))))
	}
	c.Install = gate(install.Copy().Set("name", "install"), "install")
	c.Gates = gates
	if pool != nil {
		c.Pool = pool
	} else {
		c.Pool = providers.Load(root)
	}
	c.Resolvers = resolvers(raw.GetOr("resolvers", optional.Get("resolvers")))
	c.MaxSelfCheckRounds = selfChecks
	c.Easy = easyRule(raw.GetOr("easy", optional.Get("easy")))
	c.Controller = pyjson.Truthy(raw.GetOr("controller", optional.Get("controller")))
	c.BotToken = env["BOT_GITHUB_TOKEN"]
	c.ActionsToken = env["GITHUB_TOKEN"]
	if c.ActionsToken == "" {
		c.ActionsToken = env["GH_TOKEN"]
	}
	c.Backend = backend
	dry := strings.ToLower(env["HARNESS_DRY_RUN"])
	c.DryRun = dry == "1" || dry == "true" || dry == "yes"
	c.CLIBins = map[string]string{}
	for _, cli := range providers.Clis {
		if bin := env["HARNESS_"+strings.ToUpper(cli)+"_BIN"]; bin != "" {
			c.CLIBins[cli] = bin
		}
	}
	c.RunID = env["GITHUB_RUN_ID"]
	c.ServerURL = env["GITHUB_SERVER_URL"]
	if c.ServerURL == "" {
		c.ServerURL = "https://github.com"
	}
	c.NowOverride = env["HARNESS_NOW"]
	c.PCGist = env["JACKIOH_PC_GIST"]
	c.Secrets = providers.SecretsOf(env)
	c.ProviderSecrets = map[string]string{}
	for _, k := range append([]string{ProviderSecretKey}, providers.SecretNames...) {
		if v := env[k]; v != "" {
			c.ProviderSecrets[k] = v
		}
	}
	return c
}

// Load reads `.harness/config.json` under `root` (the repository root by default: root "").
// A nil env is os.environ.
func Load(root string /* = None */, env map[string]string /* = None */) *Config {
	base := root
	if base == "" {
		base = RepoRoot
	}
	path := filepath.Join(base, ConfigPath)
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		raiseConfig("%s does not exist", path)
	}
	herr.Must(err)
	raw, err := pyjson.Loads(data)
	if err != nil {
		// Python names the place in its own words ("Expecting value: line 1 column 1 (char 0)").
		raiseConfig("%s is not valid JSON: %s", path, err)
	}
	o, ok := raw.(*pyjson.Object)
	if !ok {
		raiseConfig("%s must hold a JSON object", path)
	}
	if env == nil {
		env = environ()
	}
	return Parse(o, base, env, nil)
}

// environ is os.environ as a mapping.
func environ() map[string]string {
	out := map[string]string{}
	for _, kv := range os.Environ() {
		if k, v, ok := strings.Cut(kv, "="); ok && k != "" {
			out[k] = v
		}
	}
	return out
}

// SecretValues is the live values of every secret key, for redaction. Short values are ignored.
// A nil env is os.environ.
func SecretValues(env map[string]string /* = None */) []string {
	source := env
	if source == nil {
		source = environ()
	}
	out := []string{}
	for _, k := range SecretKeys {
		if py.Len(source[k]) >= 8 {
			out = append(out, source[k])
		}
	}
	return out
}

// ChildEnv is the environment for a child process: every secret key removed except those in
// `keep`. A nil env is os.environ.
func ChildEnv(env map[string]string /* = None */, keep []string /* = () */) map[string]string {
	source := env
	if source == nil {
		source = environ()
	}
	out := map[string]string{}
	for k, v := range source {
		if !slices.Contains(SecretKeys, k) || slices.Contains(keep, k) {
			out[k] = v
		}
	}
	return out
}

// CompanionEnv is the environment for a process run as another bot (`__main__._companion`,
// #60): this one's, with that bot's home and that bot's GitHub token in `BOT_GITHUB_TOKEN`.
// Without its token the process reads with the job's own and writes nothing as that bot. A nil
// env is os.environ.
func CompanionEnv(other *identity.Other, env map[string]string /* = None */) map[string]string {
	source := map[string]string{}
	if env == nil {
		env = environ()
	}
	for k, v := range env {
		source[k] = v
	}
	source["HARNESS_HOME"] = other.Home
	token := ""
	if other.TokenEnv != "" {
		token = source[other.TokenEnv]
	}
	if token != "" {
		source["BOT_GITHUB_TOKEN"] = token
	} else {
		delete(source, "BOT_GITHUB_TOKEN")
	}
	return source
}

// GithubEnv is the Actions facts a command reads: event name and path, output and summary files,
// the runner, CLI, secret name and shared flag a night run's model job was given (`PLAN_*`, which
// `plan-fetch --check` holds to its plan, #1455), and the runner's scratch directory, where the
// status loop delivers a run (`RUNNER_TEMP`, #1537). A variable so the tests can replace it.
var GithubEnv = githubEnv

func githubEnv() map[string]string {
	keys := []string{"GITHUB_EVENT_NAME", "GITHUB_EVENT_PATH", "GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY",
		"PLAN_RUNS_ON", "PLAN_CLI", "PLAN_SECRET", "PLAN_SHARED", "RUNNER_TEMP"}
	out := map[string]string{}
	for _, k := range keys {
		out[k] = os.Getenv(k)
	}
	return out
}
