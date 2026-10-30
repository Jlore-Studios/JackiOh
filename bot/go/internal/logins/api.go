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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// FILE_HINTS = { "codex": "the contents of ~/.codex/auth.json after `codex login` (Sign in with ChatGPT)", "muse": "the contents of ~/.config/muse/auth.json after
var FileHints = map[string]string{"codex": "the contents of ~/.codex/auth.json after `codex login` (Sign in with ChatGPT)", "muse": "the contents of ~/.config/muse/auth.json after `muse login`, or a META_API_KEY"}

// CLI_ENV: dict[str, dict[str, str]] = { "muse": {"MUSE_NO_AUTO_UPDATE": "1"}, }
var CLIEnv *pyjson.Object // unported

// Login is logins.Login.

type Login struct {
	Provider string
	CLI      string
	Home     string
	Env      map[string]string
	Files    *pyjson.Object    // default: field(default_factory=dict)
	Initial  map[string]string // default: field(default_factory=dict, repr=False)
	Secret   string            // default: field(default="", repr=False)
	Source   string            // default: "secret"
}

// Write `provider`'s login under `home`, or for a machine login, only its environment.
// Raises `LoginError` when the secret is unusable.

func Prepare(provider *providers.Provider, secret string, vaultText string, home string) *Login {
	panic("unported: logins.prepare")
}

// _install is logins._install.

func Install(login *Login, name string, path string, saved map[string]string, secret string) {
	panic("unported: logins._install")
}

// The login's files sealed for the vault when the CLI changed them during the job, else
// None (nothing new to keep).

func Seal(login *Login) string {
	panic("unported: logins.seal")
}
