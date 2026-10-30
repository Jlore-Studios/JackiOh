// Package identity is bot/harness/identity.py.
//
// Which bot this process is (#60): the night bot, or Squishy, from the one harness.
//
// Both bots run this code. Each workflow names its bot's home directory in `HARNESS_HOME`:
// `.harness` (the default) for the night bot, `.squishy` for Squishy. The home holds that bot's
// `config.json` and `providers.json`, and the config's `identity` section says what sets the bot
// apart from the other: its labels' prefix, its slash command, its branches, its state and journal
// branches, its workflow, the marker on everything it writes, and the modes it has beyond building.
// A config without the section is the night bot's, as it always was.
//
// The rest of the harness reads these through `config.py`'s constants (`LABEL_BUILD`, `SLASH`,
// `STATE_BRANCH`, …), which are fixed when the process starts: one process is one bot. Code that
// needs the other bot (the night bot's status issue carries Squishy's section, and its status loop
// sweeps for Squishy too) starts a second process with the other home (`__main__._companion`).
//
// This module and `config.py` are the only ones that read `os.environ`.
package identity

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
)

// REPO_ROOT = Path(__file__).resolve().parents[2]
var RepoRoot any // unported

// MODES = ("oneshot", "split", "split-bot", "fullsend")
var Modes = []string{"oneshot", "split", "split-bot", "fullsend"}

// Another bot in the same repository: the issues it owns this one leaves alone (its queue
// labels on them, or assigned to its account), and, with `home`, the status section and sweep
// the night bot's status loop runs for it.

type Other struct {
	Name        string
	Login       string
	LabelPrefix string
	Home        string // default: ""
	TokenEnv    string // default: ""
	Workflow    string // default: ""
}

// Identity is identity.Identity.

type Identity struct {
	Name          string   // default: "the night bot"
	Title         string   // default: "Night bot"
	Command       string   // default: "harness"
	LabelPrefix   string   // default: "bot:"
	BranchPrefix  string   // default: "bot/"
	StateBranch   string   // default: "bot-state"
	JournalBranch string   // default: "bot-journal"
	Workflow      string   // default: "bot-night.yml"
	Marker        string   // default: "<!-- jackioh-bot -->"
	Trust         string   // default: ".harness/trust.txt"
	Modes         []string // default: ()
	Others        []*Other // default: field(default_factory=tuple)
	Suggestions   bool     // default: True
}

func (x *Identity) Slash() string {
	panic("unported: identity.Identity.slash")
}

// What its commits' messages start with: `bot: build pass 1 for #85`.

func (x *Identity) CommitPrefix() string {
	panic("unported: identity.Identity.commit_prefix")
}

// The bot's home directory, relative to the repository root.

func Home() string {
	panic("unported: identity.home")
}

// KEYS = ("name", "title", "command", "label_prefix", "branch_prefix", "state_branch", "journal_branch", "workflow", "marker", "trust", "modes", "others")
var Keys = []string{"name", "title", "command", "label_prefix", "branch_prefix", "state_branch", "journal_branch", "workflow", "marker", "trust", "modes", "others"}

// An `Identity` from a config's `identity` section (None or {} is the night bot's).

func Parse(raw any, suggestions bool /* = True */) *git.Identity {
	panic("unported: identity.parse")
}

// The identity in `<root>/<home>/config.json`; the night bot's when the file or the section
// is missing (a test's temporary root, say). A section that is there but wrong is an error.

func Load(root string /* = None */, where string /* = None */) *git.Identity {
	panic("unported: identity.load")
}

// HOME = home()
var Home_ any // unported

// CURRENT = load()
var Current any // unported
