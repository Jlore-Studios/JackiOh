// Package logins is bot/harness/logins.py.
//
// A provider's login, for one model job.
//
// A **machine** login (`"login": "machine"` in providers.json) was made once, by hand, on the
// bot's own machine as that provider's Linux user (`agent-<id>`, whose runner alone runs its jobs;
// `bot/machine/README.md`): the CLI finds it in its usual place in that home, so `prepare` writes
// nothing and only sets the CLI's environment. agy logs in only this way.
//
// A **secret** login: the workflow hands the model job one secret, its provider's, as
// `HARNESS_PROVIDER_SECRET`. `prepare` turns it into what that CLI reads, under a private directory
// outside every worktree, and returns the environment the CLI needs; the CLI's own home stays
// untouched.
//
// | CLI | The secret holds | Written as |
// |---|---|---|
// | claude | the token `claude setup-token` prints | `CLAUDE_CODE_OAUTH_TOKEN`, with a `CLAUDE_CONFIG_DIR` of its own |
// | codex | `~/.codex/auth.json` after `codex login` (ChatGPT) | `$CODEX_HOME/auth.json`, stored as a file |
// | muse | `~/.config/muse/auth.json` after `muse login`, or an API key | `$XDG_CONFIG_HOME/muse/auth.json`, or `META_API_KEY` (billed per token) |
//
// A JSON secret may be pasted as it is or base64-encoded. A login refreshed by an earlier run (the
// vault, `vault.py`) wins over the pasted one when it opens under the same secret. After the job,
// `seal` reads the files back; if the CLI rotated them, the new ones go into the vault.
package logins

import (
	"encoding/base64"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// FileHints is FILE_HINTS: what a secret must look like for each CLI that takes a login file, for
// the error a person reads.
var FileHints = map[string]string{
	"codex": "the contents of ~/.codex/auth.json after `codex login` (Sign in with ChatGPT)",
	"muse":  "the contents of ~/.config/muse/auth.json after `muse login`, or a META_API_KEY",
}

// CLIEnv is CLI_ENV: environment each CLI gets on top of its login, either way.
var CLIEnv = map[string]map[string]string{
	"muse": {"MUSE_NO_AUTO_UPDATE": "1"},
}

// Login is logins.Login.
type Login struct {
	Provider string
	CLI      string
	Home     string
	Env      map[string]string
	// Files are the files that make the login, by name (each value a path string): what `seal`
	// keeps when the CLI changes them.
	Files   *pyjson.Object    // default: field(default_factory=dict)
	Initial map[string]string // default: field(default_factory=dict, repr=False)
	Secret  string            // default: field(default="", repr=False)
}

// newLogin is Login(provider, cli, home, env, secret=secret), its other fields at their defaults.
func newLogin(provider, cli, home string, env map[string]string, secret string) *Login {
	return &Login{Provider: provider, CLI: cli, Home: home, Env: env, Files: pyjson.New(),
		Initial: map[string]string{}, Secret: secret}
}

// jsonText is _json_text: the secret as JSON text: as pasted, or base64-decoded. False when it is
// neither.
func jsonText(secret string) (string, bool) {
	text := strings.TrimSpace(secret)
	candidates := []string{text}
	if decoded, ok := b64decode(text); ok {
		candidates = append(candidates, decoded)
	}
	for _, candidate := range candidates {
		if v, err := pyjson.LoadsString(candidate); err == nil {
			if _, isDict := v.(*pyjson.Object); isDict {
				return candidate, true
			}
		}
	}
	return "", false
}

// b64decode is base64.b64decode(text, validate=True).decode("utf-8"): the standard alphabet with
// its padding and nothing else (Go's decoder would skip a line break; Python's refuses it), and
// the bytes valid UTF-8.
func b64decode(text string) (string, bool) {
	if strings.ContainsAny(text, "\r\n") {
		return "", false
	}
	data, err := base64.StdEncoding.DecodeString(text)
	if err != nil || !utf8.Valid(data) {
		return "", false
	}
	return string(data), true
}

// leaves is _leaves: every string inside a JSON document: the tokens a transcript must never show.
func leaves(text string) []string {
	found := []string{}
	var walk func(node any)
	walk = func(node any) {
		switch x := node.(type) {
		case string:
			found = append(found, x)
		case *pyjson.Object:
			x.Items(func(_ string, value any) bool {
				walk(value)
				return true
			})
		case []any:
			for _, value := range x {
				walk(value)
			}
		}
	}
	if v, err := pyjson.LoadsString(text); err == nil {
		walk(v)
	}
	return found
}

// write is _write.
func write(path string, text string) {
	mkdir(filepath.Dir(path), 0o700)
	herr.Must(os.WriteFile(path, []byte(text), 0o666))
	herr.Must(os.Chmod(path, 0o600))
}

// mkdir is Path.mkdir(parents=True, exist_ok=True, mode=mode): the missing parents with the
// default permissions (as `mkdir -p` makes them), only the directory itself with `mode`.
func mkdir(dir string, mode os.FileMode) {
	herr.Must(os.MkdirAll(filepath.Dir(dir), 0o777))
	if err := os.Mkdir(dir, mode); err != nil {
		if info, statErr := os.Stat(dir); !errors.Is(err, fs.ErrExist) || statErr != nil || !info.IsDir() {
			panic(err)
		}
	}
}

// readText is Path.read_text(encoding="utf-8"): strict UTF-8, with universal newlines.
func readText(path string) string {
	data, err := os.ReadFile(path)
	herr.Must(err)
	if !utf8.Valid(data) {
		panic(fmt.Errorf("UnicodeDecodeError: 'utf-8' codec can't decode %s", path))
	}
	return py.Universal(string(data))
}

// isFile is Path.is_file().
func isFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.Mode().IsRegular()
}

// Prepare writes `provider`'s login under `home`, or for a machine login, only its environment.
// Raises `LoginError` when the secret is unusable.
func Prepare(provider *providers.Provider, secret string, vaultText string, home string) *Login {
	if provider.Login == "machine" {
		env := map[string]string{}
		for k, v := range CLIEnv[provider.CLI] {
			env[k] = v
		}
		for k, v := range provider.Env {
			env[k] = v
		}
		return newLogin(provider.ID, provider.CLI, purePath(home), env, "")
	}
	secret = strings.TrimSpace(secret)
	if secret == "" {
		herr.Raise(&herr.LoginError{Msg: fmt.Sprintf("`%s`'s secret `%s` is empty in this job",
			provider.ID, provider.Secret)})
	}
	home = purePath(home)
	mkdir(home, 0o700)
	redact.Remember([]string{secret})
	env := map[string]string{}
	for k, v := range provider.Env {
		env[k] = v
	}
	login := newLogin(provider.ID, provider.CLI, home, env, secret)
	saved := map[string]string{}
	var opened *pyjson.Object
	if vaultText != "" {
		opened = vault.Open(vaultText, secret)
	}
	if owner, ok := opened.Get("provider").(string); opened.Len() > 0 && ok && owner == provider.ID {
		if files := opened.Obj("files"); files != nil {
			files.Items(func(k string, v any) bool {
				saved[k] = py.Str(v)
				return true
			})
		}
	}
	if provider.CLI == "claude" {
		configDir := purePath(home, "claude")
		mkdir(configDir, 0o700)
		env["CLAUDE_CODE_OAUTH_TOKEN"] = secret
		env["CLAUDE_CONFIG_DIR"] = configDir
		return login
	}
	if provider.CLI == "codex" {
		codexHome := purePath(home, "codex")
		Install(login, "auth.json", purePath(codexHome, "auth.json"), saved, secret)
		write(purePath(codexHome, "config.toml"), "cli_auth_credentials_store = \"file\"\n")
		env["CODEX_HOME"] = codexHome
	} else if provider.CLI == "muse" {
		for k, v := range CLIEnv["muse"] {
			env[k] = v
		}
		_, isJSON := jsonText(secret)
		if saved["auth.json"] != "" || isJSON {
			configHome := purePath(home, "muse-config")
			Install(login, "auth.json", purePath(configHome, "muse", "auth.json"), saved, secret)
			env["XDG_CONFIG_HOME"] = configHome
		} else {
			// Not a login file: an API key, which Meta bills per token, not on the subscription.
			env["META_API_KEY"] = secret
		}
	} else {
		herr.Raise(&herr.LoginError{Msg: fmt.Sprintf(
			"the %s CLI has no secret login; it logs in on the machine", provider.CLI)})
	}
	login.Files.Items(func(_ string, path any) bool {
		redact.Remember(leaves(readText(path.(string))))
		return true
	})
	return login
}

// Install is _install.
func Install(login *Login, name string, path string, saved map[string]string, secret string) {
	text := saved[name]
	if text == "" {
		var ok bool
		if text, ok = jsonText(secret); !ok {
			herr.Raise(&herr.LoginError{Msg: fmt.Sprintf("`%s`'s secret is not %s (as JSON, or base64 of it)",
				login.Provider, fileHint(login.CLI))})
		}
	}
	write(path, text)
	login.Files.Set(name, path)
	login.Initial[name] = text
}

// fileHint is FILE_HINTS[cli], which raises KeyError for a CLI with none.
func fileHint(cli string) string {
	hint, ok := FileHints[cli]
	if !ok {
		panic(fmt.Errorf("KeyError: %s", py.Repr(cli)))
	}
	return hint
}

// Seal is the login's files sealed for the vault when the CLI changed them during the job, else
// None ("": nothing new to keep).
func Seal(login *Login) string {
	current := pyjson.New()
	login.Files.Items(func(name string, path any) bool {
		if isFile(path.(string)) {
			current.Set(name, readText(path.(string)))
		}
		return true
	})
	if current.Len() == 0 || same(current, login.Initial) {
		return ""
	}
	var found []string
	current.Items(func(_ string, text any) bool {
		found = append(found, leaves(text.(string))...)
		return true
	})
	redact.Remember(found)
	sealed, err := vault.Seal(pyjson.New("provider", login.Provider, "at", clock.ISO(clock.Now("")),
		"files", current), login.Secret)
	herr.Must(err)
	return sealed
}

// same is `current == initial` between two dicts of text.
func same(current *pyjson.Object, initial map[string]string) bool {
	if current.Len() != len(initial) {
		return false
	}
	for _, name := range current.Keys() {
		if text, ok := initial[name]; !ok || text != current.Str(name) {
			return false
		}
	}
	return true
}

// purePath is str(PurePosixPath(*parts)): joined with "/", empty and "." parts dropped, ".."
// kept, an absolute part starting afresh (two leading slashes kept, as POSIX allows).
func purePath(parts ...string) string {
	root := ""
	var segments []string
	for _, part := range parts {
		if strings.HasPrefix(part, "/") {
			segments = nil
			root = "/"
			if strings.HasPrefix(part, "//") && !strings.HasPrefix(part, "///") {
				root = "//"
			}
		}
		for _, s := range strings.Split(part, "/") {
			if s != "" && s != "." {
				segments = append(segments, s)
			}
		}
	}
	if path := root + strings.Join(segments, "/"); path != "" {
		return path
	}
	return "."
}
