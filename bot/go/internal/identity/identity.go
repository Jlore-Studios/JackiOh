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
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// RepoRoot is the repository root. The Python's is two directories above
// `bot/harness/identity.py`; a Go binary has no such file, so it is the first directory from the
// working directory up that holds `.harness/config.json` (bot/go/README.md, Read the repository
// from disk), or the working directory when none does.
var RepoRoot = findRepoRoot()

func findRepoRoot() string {
	wd, err := os.Getwd()
	if err != nil {
		return "."
	}
	if resolved, err := filepath.EvalSymlinks(wd); err == nil { // Path.resolve()
		wd = resolved
	}
	for dir := wd; ; {
		if st, err := os.Stat(filepath.Join(dir, ".harness", "config.json")); err == nil && !st.IsDir() {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return wd
		}
		dir = parent
	}
}

// Modes are the modes a bot may have beyond a plain build (#60): `oneshot` builds an issue in one
// run with fullsend, `split` breaks it into sub-issues the bot builds itself, `split-bot` into
// sub-issues for the other bot, and `fullsend` (#505) into parts the bot lands on one branch of
// the issue's own, which a last run then reconciles into one pull request.
var Modes = []string{"oneshot", "split", "split-bot", "fullsend"}

// Other is another bot in the same repository: the issues it owns this one leaves alone (its
// queue labels on them, or assigned to its account), and, with `home`, the status section and
// sweep the night bot's status loop runs for it.
type Other struct {
	Name        string
	Login       string
	LabelPrefix string
	Home        string // default: ""
	// TokenEnv is the environment variable that holds its GitHub token, for a process started as
	// it (`__main__._companion`).
	TokenEnv string // default: ""
	// Workflow is its workflow (`squishy-run.yml`, `bot-night.yml`): its live runs there hold
	// slots of the GitHub runners the bots share, first come first served
	// (`plan.others_on_github`, #1455).
	Workflow string // default: ""
}

// Identity is identity.Identity. Build one with NewIdentity, which sets the night bot's values.
type Identity struct {
	// Name is how it names itself in a sentence ("the night bot", "Squishy"), and Title at the
	// head of a title.
	Name  string // default: "the night bot"
	Title string // default: "Night bot"
	// Command is the slash command, without its slash: `/harness build`, `/squishy build`.
	Command       string // default: "harness"
	LabelPrefix   string // default: "bot:"
	BranchPrefix  string // default: "bot/"
	StateBranch   string // default: "bot-state"
	JournalBranch string // default: "bot-journal"
	// Workflow is the workflow a run is (`bot-night.yml`): dispatched to start one, read to list
	// the runs.
	Workflow string // default: "bot-night.yml"
	Marker   string // default: "<!-- jackioh-bot -->"
	// Trust is where the trust list is: the bots share one.
	Trust  string   // default: ".harness/trust.txt"
	Modes  []string // default: ()
	Others []*Other // default: field(default_factory=tuple)
	// Suggestions is whether it makes suggestions (the config's `suggestions.enabled`): Squishy
	// does not, so it has no labels for them.
	Suggestions bool // default: True
}

// NewIdentity is Identity() with the dataclass's defaults: the night bot's.
func NewIdentity() *Identity {
	return &Identity{
		Name:          "the night bot",
		Title:         "Night bot",
		Command:       "harness",
		LabelPrefix:   "bot:",
		BranchPrefix:  "bot/",
		StateBranch:   "bot-state",
		JournalBranch: "bot-journal",
		Workflow:      "bot-night.yml",
		Marker:        "<!-- jackioh-bot -->",
		Trust:         ".harness/trust.txt",
		Modes:         []string{},
		Others:        []*Other{},
		Suggestions:   true,
	}
}

func (x *Identity) Slash() string {
	return "/" + x.Command
}

// CommitPrefix is what its commits' messages start with: `bot: build pass 1 for #85`.
func (x *Identity) CommitPrefix() string {
	return strings.TrimRight(x.LabelPrefix, ":")
}

// Home is the bot's home directory, relative to the repository root.
func Home() string {
	found := strings.Trim(strings.TrimFunc(os.Getenv("HARNESS_HOME"), py.IsSpace), "/")
	if found == "" {
		return ".harness"
	}
	return found
}

func other(raw any, where string) (*Other, error) {
	o, ok := raw.(*pyjson.Object)
	if !ok || o == nil {
		return nil, fmt.Errorf("%s: expected an object", where)
	}
	for _, key := range []string{"name", "login", "label_prefix"} {
		if !o.Has(key) {
			// str(KeyError('name')) is "'name'"
			return nil, fmt.Errorf("%s: needs name, login and label_prefix (%s)", where, py.Repr(key))
		}
	}
	return &Other{
		Name:        py.Str(o.Get("name")),
		Login:       py.Str(o.Get("login")),
		LabelPrefix: py.Str(o.Get("label_prefix")),
		Home:        py.OrStr(o.Get("home"), ""),
		TokenEnv:    py.OrStr(o.Get("token_env"), ""),
		Workflow:    py.OrStr(o.Get("workflow"), ""),
	}, nil
}

// Keys are the keys an `identity` section may have.
var Keys = []string{"name", "title", "command", "label_prefix", "branch_prefix", "state_branch",
	"journal_branch", "workflow", "marker", "trust", "modes", "others"}

// Parse is an `Identity` from a config's `identity` section (None or {} is the night bot's). Its
// error is Python's ValueError, with its message; config.parse turns it into a ConfigError.
func Parse(raw any, suggestions bool /* = True */) (*Identity, error) {
	if o, ok := raw.(*pyjson.Object); raw == nil || (ok && o == nil) {
		found := NewIdentity()
		found.Suggestions = suggestions
		return found, nil
	}
	section, ok := raw.(*pyjson.Object)
	if !ok {
		return nil, errors.New("identity: expected an object")
	}
	var unknown []string
	for _, k := range section.Keys() {
		if !slices.Contains(Keys, k) {
			unknown = append(unknown, k)
		}
	}
	sort.Strings(unknown)
	if len(unknown) > 0 {
		return nil, fmt.Errorf("identity: unknown keys %s", strings.Join(unknown, ", "))
	}
	modes := []string{}
	for _, m := range py.Each(section.Get("modes")) {
		modes = append(modes, py.Str(m))
	}
	var bad []string
	for _, m := range modes {
		if !slices.Contains(Modes, m) {
			bad = append(bad, m)
		}
	}
	if len(bad) > 0 {
		return nil, fmt.Errorf("identity.modes: %s not one of %s", strings.Join(bad, ", "),
			strings.Join(Modes, ", "))
	}
	found := NewIdentity()
	for _, k := range Keys {
		v, ok := section.Lookup(k)
		if !ok || k == "modes" || k == "others" {
			continue
		}
		s := py.Str(v)
		switch k {
		case "name":
			found.Name = s
		case "title":
			found.Title = s
		case "command":
			found.Command = s
		case "label_prefix":
			found.LabelPrefix = s
		case "branch_prefix":
			found.BranchPrefix = s
		case "state_branch":
			found.StateBranch = s
		case "journal_branch":
			found.JournalBranch = s
		case "workflow":
			found.Workflow = s
		case "marker":
			found.Marker = s
		case "trust":
			found.Trust = s
		}
	}
	others := []*Other{}
	for i, o := range py.Each(section.Get("others")) {
		parsed, err := other(o, fmt.Sprintf("identity.others[%d]", i))
		if err != nil {
			return nil, err
		}
		others = append(others, parsed)
	}
	found.Modes, found.Others, found.Suggestions = modes, others, suggestions
	if !strings.HasSuffix(found.LabelPrefix, ":") || !strings.HasSuffix(found.BranchPrefix, "/") {
		return nil, errors.New("identity: label_prefix ends with `:` and branch_prefix with `/`")
	}
	return found, nil
}

// Load is the identity in `<root>/<home>/config.json`; the night bot's when the file or the
// section is missing (a test's temporary root, say). A section that is there but wrong is an
// error (Parse's). An empty `root` is RepoRoot and an empty `where` is Home().
func Load(root string /* = None */, where string /* = None */) (*Identity, error) {
	if root == "" {
		root = RepoRoot
	}
	if where == "" {
		where = Home()
	}
	path := filepath.Join(joinPath(root, where), "config.json")
	text, err := os.ReadFile(path)
	if err != nil || !utf8.Valid(text) {
		return NewIdentity(), nil
	}
	parsed, err := pyjson.Loads(text)
	if err != nil {
		return NewIdentity(), nil
	}
	raw, ok := parsed.(*pyjson.Object)
	if !ok {
		return NewIdentity(), nil
	}
	wanted := raw.Obj("suggestions")
	return Parse(raw.Get("identity"), pyjson.Truthy(wanted.GetOr("enabled", true)))
}

// joinPath is Path(a) / b: an absolute `b` stands alone.
func joinPath(a, b string) string {
	if filepath.IsAbs(b) {
		return b
	}
	return filepath.Join(a, b)
}

// Home_ is HOME = home(), read once when the process starts.
var Home_ = Home()

// Current is CURRENT = load(), read once when the process starts. Python raises at import when
// the section is wrong, which ends the process with exit code 1 before anything runs; so does
// this.
var Current = loadCurrent()

func loadCurrent() *Identity {
	found, err := Load("", "")
	if err != nil {
		fmt.Fprintf(os.Stderr, "ValueError: %s\n", err)
		os.Exit(1)
	}
	return found
}
