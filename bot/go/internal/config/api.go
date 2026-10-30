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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// REPO_ROOT = identity_mod.REPO_ROOT
var RepoRoot any // unported

// HOME = identity_mod.HOME
var Home any // unported

// IDENTITY = identity_mod.CURRENT
var Identity any // unported

// NAME = IDENTITY.name
var Name any // unported

// TITLE = IDENTITY.title
var Title any // unported

// SLASH = IDENTITY.slash
var Slash any // unported

// MODES = IDENTITY.modes
var Modes any // unported

// OTHERS = IDENTITY.others
var Others any // unported

// BRANCH_PREFIX = IDENTITY.branch_prefix
var BranchPrefix any // unported

// COMMIT_PREFIX = IDENTITY.commit_prefix
var CommitPrefix any // unported

// CONFIG_PATH = Path(HOME) / "config.json"
var ConfigPath any // unported

// TRUST_PATH = Path(IDENTITY.trust)
var TrustPath any // unported

// HALT_PATH = f"{HOME}/HALT"
var HaltPath any // unported

// PROMPTS_DIR = Path(__file__).resolve().parents[1] / "prompts"
var PromptsDir any // unported

// MARKER = IDENTITY.marker
var Marker any // unported

// STATE_BRANCH = IDENTITY.state_branch
var StateBranch any // unported

// STATE_FILE = "state.json"
var StateFile string = "state.json"

// NO_DEPLOY = {"vercel.json": '{ "git": { "deploymentEnabled": false } }\n'}
var NoDeploy = map[string]string{"vercel.json": `{ "git": { "deploymentEnabled": false } }
`}

// NIGHT_WORKFLOW = IDENTITY.workflow
var NightWorkflow any // unported

// STATUS_WORKFLOW = "bot-status.yml"
var StatusWorkflow string = "bot-status.yml"

// PROVIDER_SECRET_KEY = "HARNESS_PROVIDER_SECRET"
var ProviderSecretKey string = "HARNESS_PROVIDER_SECRET"

// SECRET_KEYS: tuple[str, ...] = ( "BOT_GITHUB_TOKEN", "SQUISHY_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN", PROVIDER_SECRET_KEY, *providers_mod.SECRETS, "ANTHROPIC
var SecretKeys []string // unported

// LABEL_DESCRIPTION_MAX = 100
var LabelDescriptionMax int = 100

// LABEL_PREFIX = IDENTITY.label_prefix
var LabelPrefix any // unported

// A sentence the night bot says of itself, said by the running bot: "the night bot" becomes
// "Squishy" in Squishy's process, and stays itself in the night bot's.

func Said(text string) string {
	panic("unported: config.said")
}

// TYPE_LABELS: tuple[str, ...] = tuple(_TYPES)
var TypeLabels []string // unported

// KIND_LABELS: tuple[str, ...] = ("kind:cards", "kind:ai", "kind:app", "kind:internal")
var KindLabels = []string{"kind:cards", "kind:ai", "kind:app", "kind:internal"}

// TIER_LABELS: tuple[str, ...] = ("tier:major", "tier:patch", "tier:micro")
var TierLabels = []string{"tier:major", "tier:patch", "tier:micro"}

// LABELS: dict[str, tuple[str, str]] = { # name: (color, description) **{f"{LABEL_PREFIX}{name}": (color, said(text)) for name, (color, text) in _OWN.items() if I
var Labels *pyjson.Object // unported

// LABEL_BUILD = f"{LABEL_PREFIX}build"
var LabelBuild any // unported

// LABEL_WORKING = f"{LABEL_PREFIX}working"
var LabelWorking any // unported

// LABEL_PR_OPEN = f"{LABEL_PREFIX}pr-open"
var LabelPROpen any // unported

// LABEL_REVISE = f"{LABEL_PREFIX}revise"
var LabelRevise any // unported

// LABEL_BLOCKED = f"{LABEL_PREFIX}blocked"
var LabelBlocked any // unported

// LABEL_PR = f"{LABEL_PREFIX}pr"
var LabelPR any // unported

// LABEL_SUGGESTION = f"{LABEL_PREFIX}suggestion"
var LabelSuggestion any // unported

// LABEL_READY = "ready for merge"
var LabelReady string = "ready for merge"

// LABEL_CROSS = f"{LABEL_PREFIX}cross-review"
var LabelCross any // unported

// LABEL_NEEDS_PLAN = f"{LABEL_PREFIX}needs-plan"
var LabelNeedsPlan any // unported

// LABEL_PLANNED = f"{LABEL_PREFIX}planned"
var LabelPlanned any // unported

// LABEL_STUCK = f"{LABEL_PREFIX}stuck"
var LabelStuck any // unported

// MODE_LABELS = {f"{LABEL_PREFIX}{mode}": mode for mode in MODES}
var ModeLabels any // unported

// LABEL_TREE = f"{LABEL_PREFIX}tree"
var LabelTree any // unported

// LABEL_HUMAN = "human"
var LabelHuman string = "human"

// LABEL_HUMAN_REQUIRED = "human required"
var LabelHumanRequired string = "human required"

// LABEL_PERSON_MERGES = "person merges"
var LabelPersonMerges string = "person merges"

// RETIRED_LABELS: tuple[str, ...] = ("method:manual", "method:use-bot", f"{LABEL_PREFIX}approved", f"{LABEL_PREFIX}needs-review", "large patch", "Info")
var RetiredLabels []string // unported

// DIFFICULTIES = ("easy", "medium", "hard")
var Difficulties = []string{"easy", "medium", "hard"}

// DIFFICULTY_LABELS = {f"difficulty:{name}": name for name in DIFFICULTIES}
var DifficultyLabels any // unported

// DEFAULT_DIFFICULTY = "medium"
var DefaultDifficulty string = "medium"

// MIN_TIER = {"easy": "weak", "medium": "medium", "hard": "strong"}
var MinTier = map[string]string{"easy": "weak", "medium": "medium", "hard": "strong"}

// PART_FLOOR = "medium"
var PartFloor string = "medium"

// PLAN_FLOOR = {"easy": "medium", "medium": "medium", "hard": "strong"}
var PlanFloor = map[string]string{"easy": "medium", "medium": "medium", "hard": "strong"}

// UNRATED_PLAN_FLOOR = "medium"
var UnratedPlanFloor string = "medium"

// STEP_UP_AFTER = 3
var StepUpAfter int = 3

// LABEL_PRIORITY_HIGH = "priority:high"
var LabelPriorityHigh string = "priority:high"

// LABEL_PRIORITY_MEDIUM = "priority:medium"
var LabelPriorityMedium string = "priority:medium"

// LABEL_PRIORITY_LOW = "priority:low"
var LabelPriorityLow string = "priority:low"

// SUGGESTION_CLASSES = ("fuzz", "checks", "telemetry", "refactor", "ai", "fairness", "security", "data", "polish", "feature")
var SuggestionClasses = []string{"fuzz", "checks", "telemetry", "refactor", "ai", "fairness", "security", "data", "polish", "feature"}

// PRIORITY_LEVELS = ("high", "medium", "low")
var PriorityLevels = []string{"high", "medium", "low"}

// Gate is config.Gate.

type Gate struct {
	Name           string
	Run            string
	TimeoutMinutes int
	Machine        bool // default: True
}

// RESOLVER_TAKES = ("main",)
var ResolverTakes = []string{"main"}

// A file whose merge conflict the harness resolves itself, with no model (`resolve.py`):
// `take` the version `main` brought in, or `run` a command that rewrites the file from the
// merged tree. One of the two, never both. From `resolvers` in `.harness/config.json`.

type Resolver struct {
	Path           string
	Take           string // default: ""
	Run            string // default: ""
	TimeoutMinutes int    // default: 10
}

// The limits of `difficulty:easy` (`harness/easy.py`), from `easy` in `.harness/config.json`:
// at most `max_files` files and `max_lines` lines added plus removed (`generated` files aside),
// and nothing under `off_limits`.

type EasyRule struct {
	MaxFiles  int      // default: 10
	MaxLines  int      // default: 400
	OffLimits []string // default: ()
	Generated []string // default: ()
}

// Another bot on the same subscription, and the steps of its workflows that spend it.

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

// Everything the harness reads. Built once by `load()`.

type Config struct {
	Root                   string
	Repo                   string
	DefaultBranch          string
	BotLogin               string
	BotUserID              int
	Operator               string
	Timezone               string
	MaxTurns               *pyjson.Object
	CallTimeoutMinutes     int
	JobBudgetMinutes       int
	MinMinutesForACall     int
	MaxReviewCycles        *pyjson.Object
	MaxFailures            int
	AutoMerge              bool
	MergeMethod            string
	CIWorkflow             string
	RequiredChecks         []string
	CIReruns               int
	SuggestionsEnabled     bool
	SuggestionsMaxOpen     int
	SuggestionsIdleMinutes int
	SuggestionPriorities   map[string]string
	Quiet                  *Quiet
	ForbiddenPaths         []string
	ReviewPaths            []string
	UploadTranscripts      bool
	Install                *Gate
	Gates                  []*Gate
	Pool                   *providers.Pool
	Resolvers              []*Resolver        // default: ()
	MaxSelfCheckRounds     int                // default: 3
	Easy                   *EasyRule          // default: field(default_factory=EasyRule)
	Controller             bool               // default: False
	ControllerLoop         bool               // default: False
	BotToken               string             // default: field(default="", repr=False)
	ActionsToken           string             // default: field(default="", repr=False)
	Backend                string             // default: "cli"
	DryRun                 bool               // default: False
	CLIBins                map[string]string  // default: field(default_factory=dict)
	RunID                  string             // default: ""
	ServerURL              string             // default: "https://github.com"
	NowOverride            string             // default: ""
	PCGist                 string             // default: ""
	Secrets                *providers.Secrets // default: field(default_factory=lambda: Secrets(frozenset(), False))
	ProviderSecrets        map[string]string  // default: field(default_factory=dict, repr=False)
}

func (x *Config) SecretFor(name string) string {
	panic("unported: config.Config.secret_for")
}

func (x *Config) Bin(cli string) string {
	panic("unported: config.Config.bin")
}

func (x *Config) ClaudeBin() string {
	panic("unported: config.Config.claude_bin")
}

// The token GitHub writes use: the bot account's when present, else the job's own.

func (x *Config) WriteToken() string {
	panic("unported: config.Config.write_token")
}

func (x *Config) RunURL() string {
	panic("unported: config.Config.run_url")
}

// LANE_REVIEW_CYCLES = {"hard": 10, "medium": 3, "easy": 3}
var LaneReviewCycles any // unported

// A `Config` from the committed JSON and an environment mapping. Raises `ConfigError`.
// The subscriptions come from `pool`, or from `.harness/providers.json` under `root`.

func Parse(raw *pyjson.Object, root string, env map[string]string, pool *providers.Pool /* = None */) *Config {
	panic("unported: config.parse")
}

// Read `.harness/config.json` under `root` (the repository root by default).

func Load(root string /* = None */, env map[string]string /* = None */) *Config {
	panic("unported: config.load")
}

// The live values of every secret key, for redaction. Short values are ignored.

func SecretValues(env map[string]string /* = None */) []string {
	panic("unported: config.secret_values")
}

// The environment for a child process: every secret key removed except those in `keep`.

func ChildEnv(env map[string]string /* = None */, keep []string /* = () */) map[string]string {
	panic("unported: config.child_env")
}

// The environment for a process run as another bot (`__main__._companion`, #60): this one's,
// with that bot's home and that bot's GitHub token in `BOT_GITHUB_TOKEN`. Without its token the
// process reads with the job's own and writes nothing as that bot.

func CompanionEnv(other *identity.Other, env map[string]string /* = None */) map[string]string {
	panic("unported: config.companion_env")
}

// The Actions facts a command reads: event name and path, output and summary files, the
// runner, CLI, secret name and shared flag a night run's model job was given (`PLAN_*`, which
// `plan-fetch --check` holds to its plan, #1455), and the runner's scratch directory, where the
// status loop delivers a run (`RUNNER_TEMP`, #1537).

func GithubEnv() map[string]string {
	panic("unported: config.github_env")
}
