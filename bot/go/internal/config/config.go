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
	"fmt"
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

// SessionHandoffMarker marks a handoff a Claude session outside the harness leaves on an issue or
// pull request (CLAUDE.md rule 14, `.claude/skills/handoff/SKILL.md`, #2748 section 10). Both bots
// read it, so it is not an identity's: a claim puts the newest trusted ones in the run's journal
// (`journal.SessionHandoffs`), and no command, vote or mention is ever read from one
// (`commands.CommandLines`), whatever words its notes quote.
const SessionHandoffMarker = "<!-- jackioh-session:handoff -->"

// IsSessionHandoff: `body` is a session's handoff (it carries SessionHandoffMarker).
func IsSessionHandoff(body string) bool { return strings.Contains(body, SessionHandoffMarker) }

// StateBranch is STATE_BRANCH = IDENTITY.state_branch.
var StateBranch = Identity.StateBranch

// StateFile is STATE_FILE = "state.json".
var StateFile string = "state.json"

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

// own is _OWN: the bot's own labels, by what follows the prefix. The live labels (`LiveRuns`)
// follow `build`, where `working` was until #2748.
var own = []labelDef{
	{"build", "1d76db", "Queued for the night bot to build"},
	{"planning", "fbca04", "The night bot is planning this now"},
	{"splitting", "fbca04", "The night bot is splitting this into parts now"},
	{"building", "fbca04", "The night bot is building this now"},
	{"revising", "fbca04", "The night bot is revising this pull request now"},
	{"fixing-ci", "fbca04", "The night bot is fixing this pull request's red CI or merge queue build now"},
	{"resolving", "fbca04", "The night bot is resolving this pull request's merge conflict now"},
	{"reviewing", "fbca04", "The night bot is reviewing this pull request now"},
	{"reconciling", "fbca04", "The night bot is reconciling this tree's parts into one pull request now"},
	{"pr-open", "0e8a16", "The night bot opened a PR for this"},
	{"revise", "5319e7", "Queued for the night bot to revise this PR"},
	{"blocked", "b60205", "The night bot needs a person before it can go on"},
	{"pr", "c5def5", "A pull request the night bot opened"},
	{"suggestion", "d4c5f9", "An improvement the night bot suggests; add bot:build to have it built"},
	{"cross-review", "0052cc", "A night bot pull request waiting for its final audit (a review run)"},
	{"needs-plan", "1d76db", "Queued for the night bot: a model of its tier plans it first, into the description"},
	{"planned", "0075ca", "It has a plan: in its description, which the builder starts from"},
	{"stuck", "d93f0b", "It failed every review round it had: a comment says why, for a person to review"},
	{"merging", "0e8a16", "Approved, with auto-merge on: the night bot's pull request is in CI or the merge queue"},
	{"waiting", "c5def5", "The night bot holds it for the subscription that owns it until that can start again: see its comment"},
	{"waiting-turn", "c2e0c6", "Approved: the night bot's draft waits for an overlapping pull request ahead of it to merge first"},
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
	// #2986, #3009: a question a person answers and the bot carries out, beside `bot:blocked`;
	// the `suggestion` label's colour.
	{"needs decision", "d4c5f9", "A bot asks a person to decide: its comment explains the problem and suggests; a reply revises it"},
	{"human", "ededed", "People do this: the night bot never queues, plans, builds or labels it"},
	{"human required", "e99695", "The night bot found it cannot do this (bot/, .github/, a secret, a dashboard): people do it"},
	{"person merges", "1d76db", "The bot builds it, but a person reviews and merges its pull request"},
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
	// #2748: trackers, the production countdown and the status issues; the bot comments on them
	// and rewrites its own, but never queues, plans, builds or splits them (`LabelTracking`).
	{"tracking", "bfd4f2", "A tracker or status issue: the bots comment on it, but never queue, plan or build it"},
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

// LiveRuns are the kinds of run that hold an item, each with its live label, the prefix and the
// kind (`bot:building`, #2748): while a run holds an item it carries exactly one of them, put on
// by the claim (`plan.Claim`, from `LiveRun`) and taken off as the run is delivered. They replace
// `bot:working` (`LabelWorking`), which said only that some run held it.
var LiveRuns = []string{"planning", "splitting", "building", "revising", "fixing-ci", "resolving", "reviewing",
	"reconciling"}

// LiveLabels are the live labels, in `LiveRuns`' order.
var LiveLabels = liveLabels()

func liveLabels() []string {
	out := make([]string, len(LiveRuns))
	for i, run := range LiveRuns {
		out[i] = LabelPrefix + run
	}
	return out
}

// LabelPlanning is a planning run's live label (the Needs plan stage's run).
var LabelPlanning = LabelPrefix + "planning"

// LabelSplitting is a split run's: a fullsend parent's split into parts (#505), or one of
// Squishy's splits.
var LabelSplitting = LabelPrefix + "splitting"

// LabelBuilding is a build's: a fresh build of an issue, a fullsend part's or a one-shot's.
var LabelBuilding = LabelPrefix + "building"

// LabelRevising is a revision's of a pull request, asked for by a person, a review or a conflict.
var LabelRevising = LabelPrefix + "revising"

// LabelFixingCI is a revision's whose cause is a red CI run or the merge queue (`source` "ci").
var LabelFixingCI = LabelPrefix + "fixing-ci"

// LabelResolving is a revision's whose cause is a merge conflict (`source` "conflict", #2948): it
// merges `main` and resolves it, the harness first (`resolve`), on the lane that owns the pull
// request.
var LabelResolving = LabelPrefix + "resolving"

// LabelReviewing is a review run's.
var LabelReviewing = LabelPrefix + "reviewing"

// LabelReconciling is a fullsend parent's build once its parts have landed: the reconcile.
var LabelReconciling = LabelPrefix + "reconciling"

// LabelWorking is LABEL_WORKING: what a run that held an item put on until #2748, retired
// (`RetiredLabels`). A thread that still carries it, from a claim made before, is held as ever
// (`HeldLabels`), and the sweep moves it to the live label of the run its record names before it
// deletes the label (`sweep.Labels`).
var LabelWorking = LabelPrefix + "working"

// HeldLabels are the labels that say a run holds a thread: the live labels, and `bot:working`
// while any thread still carries it.
var HeldLabels = append(slices.Clone(LiveLabels), LabelWorking)

// IsLive is: a run holds the thread: one of `HeldLabels` is on it. Every reader of "a run holds
// it" asks this, never one label.
func IsLive(names map[string]bool) bool {
	for _, label := range HeldLabels {
		if names[label] {
			return true
		}
	}
	return false
}

// LiveLabel is the live label of a kind of run (one of `LiveRuns`).
func LiveLabel(run string) string {
	if !slices.Contains(LiveRuns, run) {
		panic(fmt.Errorf("ValueError: %s is not a live run (%s)", py.Repr(run), strings.Join(LiveRuns, ", ")))
	}
	return LabelPrefix + run
}

// LiveRun is the kind of run (`LiveRuns`) that holds an item: `kind` is the item's (`build`,
// `revise`, `review`; "" reads as a build, or a revision when `pr`), `action` the run's (`plan` for
// a planning run), `mode` how a build is made (`queue.ModeOf`: `fullsend`, Squishy's `split`,
// …), `source` what queued a revision (`ci`, from a red CI run or the merge queue, is a CI fix;
// `conflict`, a merge conflict, is a resolution, #2948), and `reconcile` whether a fullsend build
// merges parts already split (its tree has them).
func LiveRun(kind, action, mode, source string, reconcile, pr bool) string {
	if kind == "" {
		kind = "build"
		if pr {
			kind = "revise"
		}
	}
	switch {
	case kind == "review":
		return "reviewing"
	case kind == "revise" && source == "ci":
		return "fixing-ci"
	case kind == "revise" && source == "conflict":
		return "resolving"
	case kind == "revise":
		return "revising"
	case action == "plan":
		return "planning"
	case mode == "fullsend" && reconcile:
		return "reconciling"
	case mode == "fullsend" || strings.HasPrefix(mode, "split"):
		return "splitting"
	}
	return "building"
}

// LabelMerging is a bot pull request whose auto-merge the bot turned on (#2748): approved, and in
// CI or `main`'s merge queue. It comes off when auto-merge goes off (`queue.HoldMerge`: a
// revision, a hold, a stop; a block), when the pull request merges or closes, and in the sweep
// once nothing keeps it merging (`sweep.Merging`). A pull request a person merges (`ready for
// merge`, `person merges`) never carries it.
var LabelMerging = LabelPrefix + "merging"

// LabelWaiting is an item held for the subscription that owns it (#2748, section 1): only its
// owner's 5-hour session keeps that one from taking it, and it can start again within
// `hold_minutes`, so it waits for that owner, not for another subscription. The planner and the
// sweep put it on and take it off (`plan.SyncHolds`, through `queue.SetWaiting`), the planner
// gives a held item to nobody else (`plan.Owning`, `plan.Pairs`), and it comes off with the queue
// label when the item leaves the queue (`queue.SetStateLabel`) or a person hands it off
// (`plan.Handoff`).
var LabelWaiting = LabelPrefix + "waiting"

// LabelWaitingTurn is an approved draft that waits for an overlapping pull request ahead of it to
// merge before it goes ready (#2948, section 2): its own review run approved it, but its test-merge
// after a pull request approved earlier, and conflict-free, conflicted. It is a state label
// (`queue.StateLabels`): the lane keeps it (`waiting_turn`), and when that pull request merges, or
// the wait runs out (`hold_minutes`), it is taken up again as a conflict revision on the same
// subscription and model (`plan.TakeTurn`).
var LabelWaitingTurn = LabelPrefix + "waiting-turn"

// LabelTracking is the type label of a tracker, the production countdown and the status issues
// (#2748): the bot may comment on one and rewrite its own, but never queues, plans, builds or
// splits it, nor adopts it as a fullsend tree: every queue path treats it as `human`
// (`queue.IsHuman`). The status issue gets it when the dashboard finds or opens it, and the open
// `production merge` issue from the sweep.
var LabelTracking string = "tracking"

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
// beside `ready for merge`), and `large patch` and `Info`, which nothing read. #2748 took
// `difficulty:easy` (`Relabelled`) and `bot:working` (`LabelWorking`: each thread that still
// carries it gets its run's live label first).
var RetiredLabels = []string{"method:manual", "method:use-bot", LabelPrefix + "approved",
	LabelPrefix + "needs-review", "large patch", "Info", "difficulty:easy", LabelWorking}

// Relabelled is a retired label whose meaning another label carries now: before the sweep deletes
// it, every open thread that carries it gets the other one in its place (`sweep._labels`), so what
// it said is kept. `difficulty:easy` is `difficulty:medium` (#2748), and a person's easy label
// becomes a person's medium label, which the bot never changes.
var Relabelled = map[string]string{"difficulty:easy": "difficulty:medium"}

// Difficulties: an item's difficulty decides which models may plan, build and review it; no label
// counts as medium, and with several the hardest counts. Like `human`, these match whatever their
// case.
var Difficulties = []string{"medium", "hard"}

// RetiredDifficulties is a difficulty the bot no longer has and the one it reads as: `easy` went
// in #2748, so an item whose label, record, plan or rating still says easy is medium.
var RetiredDifficulties = map[string]string{"easy": "medium"}

// AsDifficulty is the difficulty `name` reads as: itself for one of `Difficulties`, the one a
// retired difficulty became (`RetiredDifficulties`), or "" for any other name.
func AsDifficulty(name string) string {
	if slices.Contains(Difficulties, name) {
		return name
	}
	return RetiredDifficulties[name]
}

// LabelDifficulty is the difficulty a lowercased `difficulty:*` label reads as (`AsDifficulty`:
// `difficulty:easy` is medium), and false for any other label.
func LabelDifficulty(lowered string) (string, bool) {
	name, ok := strings.CutPrefix(lowered, "difficulty:")
	if !ok {
		return "", false
	}
	found := AsDifficulty(name)
	return found, found != ""
}

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
// Nothing builds at weak since `difficulty:easy` went (#2748): a weak model takes no work.
var MinTier = map[string]string{"medium": "medium", "hard": "strong"}

// PartFloor is the weakest tier that may build a fullsend part (#505) of any difficulty once its
// plan meets its plan floor, on a subscription with no seat of the part's own tier (Muse): a part
// lands with no checks or review, and its tree's reconcile, a strong model's, checks and reviews
// them all.
var PartFloor string = "medium"

// PlanFloor is the weakest tier whose plan an item of each difficulty builds from (#317 part 6,
// #695): a medium model plans a medium item, only a strong one a hard one. An item no one has
// rated yet (`UNRATED_PLAN_FLOOR`) may be rated and planned by a medium model, whose plan then
// stands only if it rates the item medium (#317 part 8); one it rates hard goes on to a strong
// planner. The planning lane offers a medium item to the medium planners first (#2748).
var PlanFloor = map[string]string{"medium": "medium", "hard": "strong"}

// UnratedPlanFloor is UNRATED_PLAN_FLOOR.
var UnratedPlanFloor string = "medium"

// StepUpAfter is failures of an item's own (not infra, a usage pause, a halt or a stop) at one
// difficulty before the bot raises it a step, medium to hard; three more at hard block it
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

// ResolverTakes is what a `take` resolver takes: the version the merge brought in (`main`), or
// both sides' lines (`union`, as git's union merge does, #2948): only for a file that is truly an
// append-only list, since union keeps both sides of an edited line too.
var ResolverTakes = []string{"main", "union"}

// Resolver is a file whose merge conflict the harness resolves itself, with no model
// (`resolve.py`): `take` the version `main` brought in or both sides' lines, or `run` a command that
// rewrites the file from the merged tree. One of the two, never both. From `resolvers` in
// `.harness/config.json`.
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
	// medium item gets 3 verdicts from its lane owner, then goes to its final audit; a
	// hard item stays with its Opus owner for up to 10. Only rounds that end in a readable review
	// verdict count: usage pauses, infra failures, halts and stops keep the work and pause the
	// count.
	MaxReviewCycles    *pyjson.Object
	MaxFailures        int
	AutoMerge          bool
	MergeMethod        string
	CIWorkflow         string
	RequiredChecks     []string
	CIReruns           int // `ci_reruns`: CI runs again on a head with no commit at most this often (#2948)
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
	// MigrationPaths and LockFiles are the paths that merge themselves on a condition (#2986,
	// `merge.go`): a migration with no data-losing or access-changing statement that changes no
	// migration `main` has, and a lock that gains no package. The config's `migration_paths` and
	// `lock_files`; none when left out.
	MigrationPaths []string // default: ()
	LockFiles      []string // default: ()
	// DecisionWaitHours: how long a question with a default waits for a trusted answer before the
	// lane that asked builds its default (#2986, `decision.go`); 0, when left out, never goes ahead.
	DecisionWaitHours int // default: 0
	// DecisionAssignees: the people a question assigns while it waits for their answer (#3009,
	// `decision.go`); none when left out.
	DecisionAssignees []string // default: ()
	UploadTranscripts bool
	Install           *Gate
	Gates             []*Gate
	// ResumeSessions: a run on a Claude account keeps its own session and the next run of the
	// item on the same account resumes it (`internal/sessions`, #2748). The config's
	// `resume_sessions`; off when it is left out.
	ResumeSessions bool // default: False
	// HoldMinutes: how long a queued item waits for the subscription that owns it when only that
	// one's 5-hour window keeps it from starting (#2748, section 1; `hold.go`, `plan.Owning`).
	// The config's `hold_minutes`; 0, when it is left out, holds nothing.
	HoldMinutes int // default: 0
	// HoldExempt: the subscriptions an item never waits for (`hold_exempt`): one of them that
	// cannot take its item hands it off at once.
	HoldExempt []string // default: ()
	// SpotCheckEvery: every how many merged medium bot pull requests one gets the after-merge
	// spot-check (#2748, section 5; `internal/spotcheck`). The config's `spot_check_every`; 0,
	// when it is left out, samples none.
	SpotCheckEvery int // default: 0
	// FullsendAdopt: a queued parent with open sub-issues is adopted as a fullsend tree by itself
	// (#2748, section 2; `fullsend.go`). The config's `fullsend_adopt`; off when it is left out.
	FullsendAdopt bool // default: False
	// FullsendSendBacks: how many times a fullsend reconcile may send one part back to the
	// subscription that built it (#2748, section 2). The config's `fullsend_send_backs`; 0, when
	// it is left out, sends none back.
	FullsendSendBacks int // default: 0
	// OwnReviewRun: a run that changes code hands its lane to its own review run on the same
	// subscription, which reviews, fixes and approves it (#2748, 2026-11-02; `ownreview.go`). The
	// config's `own_review_run`; off when it is left out, as Squishy's is: one run builds, checks
	// and reviews.
	OwnReviewRun bool // default: False
	// OwnReviewReserveMinutes: how long the lane stays reserved for that own review run once the
	// build delivered (`own_review_reserve_minutes`, 30 when it is left out).
	OwnReviewReserveMinutes int // default: 30
	// UsageHistory: the status loop records every subscription's usage over time on the usage
	// branch (#2749, `usage.go`, `internal/usage`). The config's `usage_history`; off when it is
	// left out, as Squishy's is.
	UsageHistory bool // default: False
	// Pool is the subscriptions, from `.harness/providers.json`.
	Pool *providers.Pool
	// Resolvers are the conflicted files the harness resolves itself when it merges `main`
	// (`resolve.py`).
	Resolvers []*Resolver // default: ()
	// MaxSelfCheckRounds: self checks a self-checking builder gets before its change goes to
	// review anyway.
	MaxSelfCheckRounds int // default: 3
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
