// Package config is bot/harness/config.py.
//
// Configuration: `<home>/config.json` for the knobs, the environment for secrets and run facts.
//
// The home is `.harness/` for the night bot and `.squishy/` for Squishy (`identity.py`, #60): the
// labels, the slash command, the branches, the state branch, the workflow and the marker below are
// the running bot's. This module and `identity.py` are the only ones that read `os.environ`.
// Everything else receives a `Config`.
package config

import (
	"path/filepath"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// RepoRoot is the repository root: `bot/harness/config.py` sits two directories below it.
var RepoRoot = identity.RepoRoot

// Home is which bot this process is (`identity.py`): the night bot's, or Squishy's.
var Home = identity.Home_

// Identity is IDENTITY = identity_mod.CURRENT.
var Identity = identity.Current

// Name is how the bot names itself: "the night bot" in a sentence, "Night bot" at the head of a
// title.
var Name = Identity.Name

// Title is IDENTITY.title.
var Title = Identity.Title

// Slash is its slash command (`/harness`, `/squishy`): IDENTITY.slash.
var Slash = "/" + Identity.Command

// Modes are the modes it has beyond a plain build (`identity.MODES`): Squishy's `oneshot`,
// `split`, `split-bot` and `fullsend`; the night bot's `fullsend` (#505).
var Modes = Identity.Modes

// Others are the other bots in the repository, whose issues this one leaves alone
// (`queue.owner`).
var Others = Identity.Others

// BranchPrefix is what the bot's branches start with: `bot/issue-<n>`, `bot/wip/<pr>`,
// `bot/old/…`.
var BranchPrefix = Identity.BranchPrefix

// CommitPrefix is what its commits' messages start with (`bot: build pass 1 for #85`):
// IDENTITY.commit_prefix, the label prefix without its trailing colons.
var CommitPrefix = strings.TrimRight(Identity.LabelPrefix, ":")

// ConfigPath is CONFIG_PATH = Path(HOME) / "config.json".
var ConfigPath = filepath.Join(Home, "config.json")

// TrustPath is TRUST_PATH = Path(IDENTITY.trust).
var TrustPath = filepath.Clean(Identity.Trust)

// HaltPath is HALT_PATH = f"{HOME}/HALT".
var HaltPath = Home + "/HALT"

// PromptsDir is PROMPTS_DIR, `bot/prompts` under the repository root.
var PromptsDir = filepath.Join(RepoRoot, "bot", "prompts")

// Marker: every comment, issue and PR body the harness writes carries this, so the event workflow
// can tell the bot's own words from a person's.
var Marker = Identity.Marker

// StateBranch is STATE_BRANCH = IDENTITY.state_branch.
var StateBranch = Identity.StateBranch

// StateFile is STATE_FILE = "state.json".
var StateFile string = "state.json"

// NoDeploy is the files an orphan branch the bot writes to starts with beside its own: Vercel
// counts a deployment for every push to every branch and reads its settings from the commit
// pushed, so a branch with no `vercel.json` of its own deploys on every write
// (docs/architecture.md, §4.2).
var NoDeploy = map[string]string{"vercel.json": "{ \"git\": { \"deploymentEnabled\": false } }\n"}

// NightWorkflow is NIGHT_WORKFLOW = IDENTITY.workflow.
var NightWorkflow = Identity.Workflow

// StatusWorkflow is the status loop's workflow (`bot-status.yml`): with `controller` on, the loop
// it runs is the one place that plans and starts the night runs (#1455).
var StatusWorkflow string = "bot-status.yml"

// ProviderSecretKey is the environment key a run's model job receives its provider's secret in
// (`providers.py`).
var ProviderSecretKey string = "HARNESS_PROVIDER_SECRET"

// SecretKeys are environment keys whose values are secrets. They are redacted from everything the
// harness writes and stripped from the model's environment; a backend puts back only its own
// login.
var SecretKeys = append(append([]string{
	"BOT_GITHUB_TOKEN",
	"SQUISHY_GITHUB_TOKEN",
	"GITHUB_TOKEN",
	"GH_TOKEN",
	ProviderSecretKey,
}, providers.SecretNames...),
	"ANTHROPIC_API_KEY",
	"ANTHROPIC_AUTH_TOKEN",
	"OPENAI_API_KEY",
	"CODEX_API_KEY",
	"GEMINI_API_KEY",
	"GOOGLE_API_KEY",
	"META_API_KEY",
	"ACTIONS_RUNTIME_TOKEN",
	"ACTIONS_ID_TOKEN_REQUEST_TOKEN",
)

// LabelDescriptionMax: GitHub refuses a label whose description is longer than this (422), so
// `harness setup` and the sweep could never create the method labels (#307, gone since #664)
// while theirs were 107 and 111.
var LabelDescriptionMax int = 100

// LabelPrefix is the prefix of the labels that are the bot's own (`bot:`, `squishy:`).
var LabelPrefix = Identity.LabelPrefix

// Said is a sentence the night bot says of itself, said by the running bot: "the night bot"
// becomes "Squishy" in Squishy's process, and stays itself in the night bot's.
func Said(text string) string { return said(Name, text) }

func said(name, text string) string {
	plain := name
	if strings.HasPrefix(name, "the ") {
		plain = py.SliceFrom(name, 4)
	}
	text = strings.ReplaceAll(text, "The night bot", strings.ToUpper(py.SliceTo(name, 1))+py.SliceFrom(name, 1))
	text = strings.ReplaceAll(text, "the night bot", name)
	return strings.ReplaceAll(text, "night bot", plain)
}

// labelDef is one entry of a label table: name: (color, description).
type labelDef struct{ name, color, text string }

// own is _OWN: the bot's own labels, by what follows the prefix.
var own = []labelDef{
	{"build", "1d76db", "Queued for the night bot to build"},
	{"working", "fbca04", "The night bot is working on this now"},
	{"pr-open", "0e8a16", "The night bot opened a PR for this"},
	{"revise", "5319e7", "Queued for the night bot to revise this PR"},
	{"blocked", "b60205", "The night bot needs a person before it can go on"},
	{"pr", "c5def5", "A pull request the night bot opened"},
	{"suggestion", "d4c5f9", "An improvement the night bot suggests; add bot:build to have it built"},
	{"cross-review", "0052cc", "A night bot pull request waiting for its review: one strong model, or a second medium one"},
	{"needs-plan", "1d76db", "Queued for the night bot: a strong model plans it first, into the description"},
	{"planned", "0075ca", "It has a plan: in its description, which the builder starts from"},
	{"stuck", "d93f0b", "It failed every review round it had: a comment says why, for a person to review"},
}

// modeLabelDefs is _MODE_LABELS: the labels of the modes (`identity.MODES`), for a bot that has
// them, and of a parent whose sub-issues the bot made (`bot:tree`, `squishy:tree`).
var modeLabelDefs = []labelDef{
	{"oneshot", "5319e7", "Queued for Squishy to build in one run, many agents at once (fullsend)"},
	{"split", "1d76db", "Queued for Squishy to break into sub-issues that Squishy builds"},
	{"split-bot", "0e8a16", "Queued for Squishy to break into sub-issues that the night bot builds"},
	{"fullsend", "b60205", "Queued for the night bot to split into parts that land on one branch, then reconcile into one PR"},
	{"tree", "c5def5", "A parent whose sub-issues the night bot made and watches; it closes once their work is on main"},
}

// saidModeLabels is _SAID_MODE_LABELS: the mode labels both bots have (#505), written as the
// night bot says them and said by the running bot (`said`). The others are Squishy's alone, and
// `split-bot` names both bots.
var saidModeLabels = []string{"fullsend", "tree"}

// shared is _SHARED: labels both bots use, kept as the night bot wrote them whichever bot
// creates them.
var shared = []labelDef{
	{"ready for merge", "0e8a16", "The night bot's reviews approved it, but auto-merge could not turn on: a person merges it"},
	{"human", "ededed", "People do this: the night bot never queues, plans, builds or labels it"},
	{"human required", "e99695", "The night bot found it cannot do this (bot/, .github/, a secret, a dashboard): people do it"},
	{"person merges", "1d76db", "The bot builds it, but a person reviews and merges its pull request"},
	{"difficulty:easy", "c2e0c6", "Any model may build it, the weakest first"},
	{"difficulty:medium", "fef2c0", "A medium model or stronger builds it (the default with no difficulty label)"},
	{"difficulty:hard", "b60205", "Only a strong model (Opus) plans, builds and reviews it"},
	{"priority:high", "d73a4a", "The night bot picks this up first"},
	{"priority:medium", "fbca04", "The night bot picks this up after priority:high"},
	{"priority:low", "0e8a16", "The night bot picks this up last, after unlabelled work"},
}

// types is _TYPES: the type labels (`docs/issues-and-patches.md`, Labels): every issue carries at
// least one, and they are the only labels triage gives a pull request (`triage.TYPE_LABELS`).
// Made by hand until #187; the colours are the ones they were made with.
var types = []labelDef{
	{"patch", "fbca04", "A numbered release of the game: cards, rules, the client, the server's features"},
	{"major version", "0052cc", "A vX.Y.0 release that changes the game or the codebase broadly, and each of its parts"},
	{"architecture", "0e8a16", "The repository, tooling, CI, deploys and agent setup"},
	{"night bot", "5319e7", "The night bot and Squishy themselves: bot/, .harness/, .squishy/ and their workflows"},
}

// TypeLabels is TYPE_LABELS = tuple(_TYPES).
var TypeLabels = defNames(types)

// workflowLabels is _WORKFLOW_LABELS: labels a workflow puts on, written here exactly as it
// writes them (it creates its own with `--force`, which would undo any other colour or
// description): `production merge` is scripts/promote-production.sh's `RELEASE_LABEL`.
var workflowLabels = []labelDef{
	{"production merge", "5319e7", "The merge of main into production (promote-production.yml)"},
	// #1370: ci.yml's `rust (classify)` puts a pull request's kind and, for a card change, its tier
	// on (the classifier, R1365000), and takes any other one off.
	{"kind:cards", "d93f0b", "It changes how a card plays, or its words: a patch version (#1360)"},
	{"kind:ai", "5319e7", "It changes the practice and ladder AI's decisions; no version (#1360)"},
	{"kind:app", "1d76db", "It changes the client or the server, what players see and use; no version (#1360)"},
	{"kind:internal", "c5def5", "CI, bots, tooling, docs, tests or a refactor; no version (#1360)"},
	{"tier:major", "b60205", "A set ships: the next v0.Z.0 (#1360)"},
	{"tier:patch", "fbca04", "15 reprinted, 50 that play differently, or a rule every game meets: the next v0.Z.X (#1360, #2499)"},
	{"tier:micro", "fef2c0", "Fewer cards, or their words alone: the next letter (#1360)"},
}

// KindLabels are the labels the classifier puts on a pull request (#1370), its kind first.
var KindLabels = []string{"kind:cards", "kind:ai", "kind:app", "kind:internal"}

// TierLabels is TIER_LABELS.
var TierLabels = []string{"tier:major", "tier:patch", "tier:micro"}

// noSuggestions is _NO_SUGGESTIONS: a bot without suggestions (Squishy) has no label for them.
var noSuggestions = []string{"suggestion"}

func defNames(defs []labelDef) []string {
	out := make([]string, len(defs))
	for i, d := range defs {
		out[i] = d.name
	}
	return out
}

// Labels is every label the repository uses, with its colour and description: the one list
// (#187). The sweep creates any the repository lacks; `harness setup` creates them and
// brings each one's colour and description back to this list. The bot's own labels are the
// running bot's: `bot:*` in the night bot's process, `squishy:*` in Squishy's
// (`HARNESS_HOME=.squishy`).
//
// Each value is the tuple (color, description) as a two-item list, so json.dumps of the items
// reads as the Python's.
var Labels = labels(Identity, LabelPrefix, Modes)

// LabelSpec is one of LABELS' items: a label, its colour and its description.
type LabelSpec struct{ Name, Color, Description string }

// LabelSpecs is `LABELS.items()`, in order.
func LabelSpecs() []LabelSpec {
	var out []LabelSpec
	Labels.Items(func(name string, v any) bool {
		spec := pyjson.Strings(v)
		out = append(out, LabelSpec{name, spec[0], spec[1]})
		return true
	})
	return out
}

func labels(id *identity.Identity, prefix string, modes []string) *pyjson.Object {
	out := pyjson.New()
	for _, d := range own {
		if id.Suggestions || !slices.Contains(noSuggestions, d.name) {
			out.Set(prefix+d.name, []any{d.color, said(id.Name, d.text)})
		}
	}
	for _, d := range modeLabelDefs {
		if d.name == "tree" && len(modes) > 0 || slices.Contains(modes, d.name) {
			text := d.text
			if slices.Contains(saidModeLabels, d.name) {
				text = said(id.Name, text)
			}
			out.Set(prefix+d.name, []any{d.color, text})
		}
	}
	for _, table := range [][]labelDef{shared, types, workflowLabels} {
		for _, d := range table {
			out.Set(d.name, []any{d.color, d.text})
		}
	}
	return out
}

// LabelBuild is LABEL_BUILD.
var LabelBuild = LabelPrefix + "build"

// LabelWorking is LABEL_WORKING.
var LabelWorking = LabelPrefix + "working"

// LabelPROpen is LABEL_PR_OPEN.
var LabelPROpen = LabelPrefix + "pr-open"

// LabelRevise is LABEL_REVISE.
var LabelRevise = LabelPrefix + "revise"

// LabelBlocked is LABEL_BLOCKED.
var LabelBlocked = LabelPrefix + "blocked"

// LabelPR is LABEL_PR.
var LabelPR = LabelPrefix + "pr"

// LabelSuggestion is one of the bot's own ideas (`prompts/suggest.md`): a person who queues it
// (`bot:build`, an assignment or `/harness build`) approves it, and the label comes off
// (`queue.queue_build`).
var LabelSuggestion = LabelPrefix + "suggestion"

// LabelReady is a bot pull request whose head the review rule approved but which auto-merge could
// not take (a review-only path, `main`'s protection, GitHub refusing it, or `auto_merge` off), so
// it waits for a person to merge it. It comes off when the pull request goes back into the queue
// (`queue.set_state_label`).
var LabelReady string = "ready for merge"

// LabelCross is LABEL_CROSS.
var LabelCross = LabelPrefix + "cross-review"

// LabelNeedsPlan is the Needs plan stage: a queued item (still `bot:build`) waiting for a strong
// model's plan, which goes into its description (`issueplan.py`). Not a state label: it sits
// beside `bot:build`.
var LabelNeedsPlan = LabelPrefix + "needs-plan"

// LabelPlanned: a plan is in the description (`issueplan.py`): beside `bot:build` while queued,
// and kept after.
var LabelPlanned = LabelPrefix + "planned"

// LabelStuck: a build or revision used every review round (`max_review_cycles`) without an
// approval; its comment says why, round by round (`failures.py`), for a person to review.
var LabelStuck = LabelPrefix + "stuck"

// ModeLabels is a mode's label beside the queue label (`queue.mode_of`): `squishy:oneshot`,
// `squishy:split`, `squishy:split-bot` and `bot:fullsend` say how the queued item is built (#60,
// #505). `squishy:tree` marks a parent whose sub-issues the bot made, until their work is on
// `main` and it closes too. Label -> mode, in MODES's order.
var ModeLabels = modeLabels()

func modeLabels() *pyjson.Object {
	out := pyjson.New()
	for _, mode := range Modes {
		out.Set(LabelPrefix+mode, mode)
	}
	return out
}

// LabelTree is LABEL_TREE.
var LabelTree = LabelPrefix + "tree"

// LabelHuman: people do a thread labelled `human` (#96): a decision, an account or a secret, or
// repository work the bot may not do (a change to bot/, .harness/ or .github/) or that a person is
// already building. The bot never queues, plans, builds, labels or assigns it, and a request to
// build it gets a reply saying so.
var LabelHuman string = "human"

// LabelHumanRequired is the bot's hand-over (#793): it found the item needs what only a person can
// do, a change to bot/, .harness/, .squishy/ or .github/, a secret, a dashboard or an account
// (`human.py`), so it took its own labels off, unassigned itself, assigned both people and said
// what in a comment naming them. From then on it is `human`: people do it, and the bot never
// touches it. A person puts it on too (`triage`). Taking it off hands the item back.
var LabelHumanRequired string = "human required"

// LabelPersonMerges: the bot builds and reviews it, but never auto-merges its pull request (#839):
// the pull request carries the label from its issue (`deliver._carried`), and the reviews'
// approval hands it to a person as a review path does. The weekly hygiene issue carries it.
var LabelPersonMerges string = "person merges"

// RetiredLabels are labels the bot used to have: the sweep deletes them from the repository
// (`sweep._labels`), so nobody puts on one that no longer does anything. #664 took the method
// labels and `bot:approved` (`bot:build` and `human` hand an issue on), `bot:needs-review` (always
// beside `ready for merge`), and `large patch` and `Info`, which nothing read.
var RetiredLabels = []string{"method:manual", "method:use-bot", LabelPrefix + "approved",
	LabelPrefix + "needs-review", "large patch", "Info"}

// Difficulties: an item's difficulty decides which models may plan, build and review it; no label
// counts as medium, and with several the hardest counts. Like `human`, these match whatever their
// case.
var Difficulties = []string{"easy", "medium", "hard"}

// DifficultyLabels is DIFFICULTY_LABELS: label -> difficulty, in DIFFICULTIES's order.
var DifficultyLabels = difficultyLabels()

func difficultyLabels() *pyjson.Object {
	out := pyjson.New()
	for _, name := range Difficulties {
		out.Set("difficulty:"+name, name)
	}
	return out
}

// DefaultDifficulty is DEFAULT_DIFFICULTY.
var DefaultDifficulty string = "medium"

// MinTier is the weakest tier that may build an item of each difficulty (`providers.TIERS`).
var MinTier = map[string]string{"easy": "weak", "medium": "medium", "hard": "strong"}

// PartFloor is the weakest tier that may build a fullsend part (#505) of any difficulty once its
// plan meets its plan floor, on a subscription with no seat of the part's own tier (Muse): a part
// lands with no checks or review, and its tree's reconcile, a strong model's, checks and reviews
// them all.
var PartFloor string = "medium"

// PlanFloor is the weakest tier whose plan an item of each difficulty builds from (#317 part 6,
// #695): a medium model plans an easy or medium item, only a strong one a hard one. An item no one
// has rated yet (`UNRATED_PLAN_FLOOR`) may be rated and planned by a medium model, whose plan then
// stands only if it rates the item easy or medium (#317 part 8).
var PlanFloor = map[string]string{"easy": "medium", "medium": "medium", "hard": "strong"}

// UnratedPlanFloor is UNRATED_PLAN_FLOOR.
var UnratedPlanFloor string = "medium"

// StepUpAfter is failures of an item's own (not infra, a usage pause, a halt or a stop) at one
// difficulty before the bot raises it a step, easy to medium to hard; three more at hard block it
// (#317 part 8).
var StepUpAfter int = 3

// LabelPriorityHigh is the first of the pickup tiers, first to last; a thread with no priority
// label sits between medium and low (#90). Like `human`, these match whatever their case.
var LabelPriorityHigh string = "priority:high"

// LabelPriorityMedium is LABEL_PRIORITY_MEDIUM.
var LabelPriorityMedium string = "priority:medium"

// LabelPriorityLow is LABEL_PRIORITY_LOW.
var LabelPriorityLow string = "priority:low"

// SuggestionClasses are the classes a suggestion survey files each suggestion under
// (`prompts/suggest.md`, #1409). `suggestions.priorities` maps a class to the `priority:*` level
// its suggestions open with; a class it leaves out (`feature`, the owner's call) opens with none.
var SuggestionClasses = []string{"fuzz", "checks", "telemetry", "refactor", "ai", "fairness", "security",
	"data", "polish", "feature"}

// PriorityLevels is PRIORITY_LEVELS.
var PriorityLevels = []string{"high", "medium", "low"}

// Gate is config.Gate.
type Gate struct {
	Name           string
	Run            string
	TimeoutMinutes int
	// Machine: runs in a model job on the bot's machine too. `false` leaves it to CI on the pull
	// request there: the machine's two vCPUs are shared by every job on it, GitHub's runners are
	// not.
	Machine bool // default: True
}

// NewGate is Gate(name, run, timeout_minutes), with `machine` at its default (True).
func NewGate(name, run string, timeoutMinutes int) *Gate {
	return &Gate{Name: name, Run: run, TimeoutMinutes: timeoutMinutes, Machine: true}
}

// ResolverTakes is the one side a `take` resolver takes: the version the merge brought in.
var ResolverTakes = []string{"main"}

// Resolver is a file whose merge conflict the harness resolves itself, with no model
// (`resolve.py`): `take` the version `main` brought in, or `run` a command that rewrites the file
// from the merged tree. One of the two, never both. From `resolvers` in `.harness/config.json`.
type Resolver struct {
	Path           string
	Take           string // default: ""
	Run            string // default: ""
	TimeoutMinutes int    // default: 10
}

// NewResolver is Resolver(path), with its defaults: no take, no run, ten minutes.
func NewResolver(path string) *Resolver {
	return &Resolver{Path: path, TimeoutMinutes: 10}
}

// EasyRule is the limits of `difficulty:easy` (`harness/easy.py`), from `easy` in
// `.harness/config.json`: at most `max_files` files and `max_lines` lines added plus removed
// (`generated` files aside), and nothing under `off_limits`.
type EasyRule struct {
	MaxFiles  int      // default: 10
	MaxLines  int      // default: 400
	OffLimits []string // default: ()
	Generated []string // default: ()
}

// NewEasyRule is EasyRule(): its defaults.
func NewEasyRule() *EasyRule {
	return &EasyRule{MaxFiles: 10, MaxLines: 400, OffLimits: []string{}, Generated: []string{}}
}

// Partner is another bot on the same subscription, and the steps of its workflows that spend it.
// Workflows maps a workflow to its step prefixes, each value a list of strings.
type Partner struct {
	Repo      string
	Workflows *pyjson.Object
}

// Quiet is config.Quiet.
type Quiet struct {
	Enabled         bool
	IntervalMinutes int
	MaxWaitMinutes  int
	PingModel       string
	Partners        []*Partner
}

// Config is everything the harness reads. Built once by `load()`.
type Config struct {
	Root               string
	Repo               string
	DefaultBranch      string
	BotLogin           string
	BotUserID          int
	Operator           string
	Timezone           string
	MaxTurns           *pyjson.Object
	CallTimeoutMinutes int
	JobBudgetMinutes   int
	MinMinutesForACall int
	// MaxReviewCycles: review verdict rounds per in-run review loop, by difficulty (#725): a
	// medium or easy item gets 3 verdicts from its lane owner, then goes to its final audit; a
	// hard item stays with its Opus owner for up to 10. Only rounds that end in a readable review
	// verdict count: usage pauses, infra failures, halts and stops keep the work and pause the
	// count.
	MaxReviewCycles    *pyjson.Object
	MaxFailures        int
	AutoMerge          bool
	MergeMethod        string
	CIWorkflow         string
	RequiredChecks     []string
	CIReruns           int
	SuggestionsEnabled bool
	SuggestionsMaxOpen int
	// SuggestionsIdleMinutes: a survey is due once a free lane has had nothing to start for this
	// long (#797).
	SuggestionsIdleMinutes int
	// SuggestionPriorities: suggestion class -> the priority level it opens with (#1409).
	SuggestionPriorities map[string]string
	Quiet                *Quiet
	ForbiddenPaths       []string
	ReviewPaths          []string
	UploadTranscripts    bool
	Install              *Gate
	Gates                []*Gate
	// Pool is the subscriptions, from `.harness/providers.json`.
	Pool *providers.Pool
	// Resolvers are the conflicted files the harness resolves itself when it merges `main`
	// (`resolve.py`).
	Resolvers []*Resolver // default: ()
	// MaxSelfCheckRounds: self checks a self-checking builder gets before its change goes to
	// review anyway.
	MaxSelfCheckRounds int // default: 3
	// Easy is what `difficulty:easy` allows (`harness/easy.py`).
	Easy *EasyRule // default: field(default_factory=EasyRule)
	// Controller: the status loop plans and starts the runs (`controller.py`, #1455): bot-night's
	// gate and plan jobs run only while no loop is going. The config's `controller`.
	Controller bool // default: False
	// ControllerLoop: this process is that loop (`dashboard --controller`), the one place that
	// plans then.
	ControllerLoop bool // default: False
	// From the environment.
	BotToken     string // default: field(default="", repr=False)
	ActionsToken string // default: field(default="", repr=False)
	Backend      string // default: "cli"
	DryRun       bool   // default: False
	// CLIBins is the binary of each CLI (`HARNESS_<CLI>_BIN` overrides it, for tests).
	CLIBins     map[string]string // default: field(default_factory=dict)
	RunID       string            // default: ""
	ServerURL   string            // default: "https://github.com"
	NowOverride string            // default: ""
	// PCGist is the gist the owner's PC writes its heartbeat to (the repository variable
	// JACKIOH_PC_GIST, which its supervisor sets; bot/machine/README.md, The local runner).
	PCGist string // default: ""
	// Secrets says which provider secrets the workflow has; the plan job learns that, never their
	// values.
	Secrets *providers.Secrets // default: field(default_factory=lambda: Secrets(frozenset(), False))
	// ProviderSecrets is the model job's one provider secret (`HARNESS_PROVIDER_SECRET`), or, run
	// by hand, the provider secrets this shell happens to hold, by name.
	ProviderSecrets map[string]string // default: field(default_factory=dict, repr=False)
}

// NewConfig is a Config with the dataclass's defaults set on the fields that have them; the
// caller sets the rest.
func NewConfig() *Config {
	return &Config{
		Resolvers:          []*Resolver{},
		MaxSelfCheckRounds: 3,
		Easy:               NewEasyRule(),
		Backend:            "cli",
		CLIBins:            map[string]string{},
		ServerURL:          "https://github.com",
		Secrets:            &providers.Secrets{Present: map[string]bool{}, Known: false},
		ProviderSecrets:    map[string]string{},
	}
}

// SecretFor is config.Config.secret_for.
func (x *Config) SecretFor(name string) string {
	if s := x.ProviderSecrets[ProviderSecretKey]; s != "" {
		return s
	}
	return x.ProviderSecrets[name]
}

// Bin is config.Config.bin.
func (x *Config) Bin(cli string) string {
	if b := x.CLIBins[cli]; b != "" {
		return b
	}
	return cli
}

// ClaudeBin is config.Config.claude_bin.
func (x *Config) ClaudeBin() string {
	return x.Bin("claude")
}

// WriteToken is the token GitHub writes use: the bot account's when present, else the job's own.
func (x *Config) WriteToken() string {
	if x.BotToken != "" {
		return x.BotToken
	}
	return x.ActionsToken
}

// RunURL is config.Config.run_url.
func (x *Config) RunURL() string {
	// A claim the status loop made before its run was listed has no page yet (#1455).
	if x.RunID == "" || strings.HasPrefix(x.RunID, "pending:") {
		return ""
	}
	return x.ServerURL + "/" + x.Repo + "/actions/runs/" + x.RunID
}
