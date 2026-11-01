package logins

// logins.py's behaviour, against answers taken from the Python (bot/tests/test_logins.py's
// cases among them).

import (
	"encoding/base64"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// needPorted skips a test while a package it leans on is still the generated stub (each package
// is ported on its own; the tests run in full once they are merged).
func needPorted(t *testing.T, calls ...func()) {
	t.Helper()
	for _, call := range calls {
		if herr.Try(call) != nil {
			t.Skip("a package this test needs is not ported in this tree")
		}
	}
}

func TestJSONText(t *testing.T) {
	b64 := func(s string) string { return base64.StdEncoding.EncodeToString([]byte(s)) }
	wrapped := b64(`{"a": 1}`)
	wrapped = wrapped[:4] + "\n" + wrapped[4:]
	for secret, want := range map[string]string{
		`{"a": 1}`: `{"a": 1}`, "  {\"a\": 1}\n": `{"a": 1}`, b64(`{"a": 1}`): `{"a": 1}`,
		wrapped: "", b64("[1]"): "", "[1]": "", "not json": "", "": "",
		b64(`{"a": "é"}`): `{"a": "é"}`, "e31=": "{}", "e30=": "{}",
	} {
		got, ok := jsonText(secret)
		if got != want || ok != (want != "") {
			t.Errorf("_json_text(%q): got %q, %v; want %q", secret, got, ok, want)
		}
	}
}

func TestLeaves(t *testing.T) {
	got := leaves(`{"a": "x", "b": [1, "y", {"c": "z", "d": null}], "e": {"f": "w"}}`)
	if strings.Join(got, ",") != "x,y,z,w" || len(leaves("nope")) != 0 {
		t.Errorf("_leaves: %q", got)
	}
}

func provider(cli, login, secret string, env map[string]string) *providers.Provider {
	return &providers.Provider{ID: cli + "-1", CLI: cli, Login: login, Secret: secret, Env: env}
}

func TestMachineLoginWritesNothing(t *testing.T) {
	home := filepath.Join(t.TempDir(), "logins")
	for _, cli := range []string{"codex", "agy", "muse"} {
		login := Prepare(provider(cli, "machine", "", map[string]string{"X": "1"}), "", "a vault it must not open", home)
		if login.Files.Len() != 0 || login.Home != home || login.Secret != "" {
			t.Errorf("%s: %+v", cli, login)
		}
		if _, err := os.Stat(home); !os.IsNotExist(err) {
			t.Errorf("%s: the home was made", cli)
		}
		want := map[string]string{"X": "1"}
		if cli == "muse" {
			want["MUSE_NO_AUTO_UPDATE"] = "1"
		}
		if len(login.Env) != len(want) || login.Env["X"] != "1" || login.Env["MUSE_NO_AUTO_UPDATE"] != want["MUSE_NO_AUTO_UPDATE"] {
			t.Errorf("%s env: %v", cli, login.Env)
		}
		if Seal(login) != "" {
			t.Errorf("%s: a machine login leaves the machine", cli)
		}
	}
}

const codexAuth = `{"auth_mode": "chatgpt", "OPENAI_API_KEY": null, "tokens": {"refresh_token": "ref-` +
	`cccccccccccccccccccccccccccccccccccccccc", "account_id": "acct-1234567890abcdef"}}`

func TestSecretLogins(t *testing.T) {
	needPorted(t, func() { redact.Remember(nil) }, func() { vault.Open("", "") })
	root := t.TempDir()
	// claude: its token and a config directory of its own.
	claude := Prepare(provider("claude", "secret", "CLAUDE_CODE_OAUTH_TOKEN", nil), "  sk-ant-oat01-x  ", "",
		filepath.Join(root, "c"))
	if claude.Env["CLAUDE_CODE_OAUTH_TOKEN"] != "sk-ant-oat01-x" || claude.Env["CLAUDE_CONFIG_DIR"] != root+"/c/claude" ||
		claude.Files.Len() != 0 || Seal(claude) != "" {
		t.Errorf("claude: %+v", claude)
	}
	if info, err := os.Stat(root + "/c/claude"); err != nil || !info.IsDir() || info.Mode().Perm()&0o077 != 0 {
		t.Errorf("claude's config directory: %v %v", info, err)
	}
	// codex: auth.json as pasted or base64, private, and a config that keeps it in a file.
	codex := provider("codex", "secret", "CODEX_AUTH_JSON", map[string]string{"E": "v"})
	for i, secret := range []string{codexAuth, base64.StdEncoding.EncodeToString([]byte(codexAuth))} {
		home := filepath.Join(root, "x", string(rune('a'+i)))
		login := Prepare(codex, secret, "", home)
		auth := filepath.Join(login.Env["CODEX_HOME"], "auth.json")
		data, err := os.ReadFile(auth)
		info, _ := os.Stat(auth)
		toml, _ := os.ReadFile(filepath.Join(home, "codex", "config.toml"))
		if err != nil || string(data) != codexAuth || info.Mode().Perm() != 0o600 || login.Env["E"] != "v" ||
			string(toml) != "cli_auth_credentials_store = \"file\"\n" {
			t.Errorf("codex %d: %+v %q %v", i, login, data, err)
		}
		if login.Files.Str("auth.json") != auth || login.Initial["auth.json"] != codexAuth {
			t.Errorf("codex %d files: %s %v", i, pyjson.Dumps(login.Files), login.Initial)
		}
	}
	bad, ok := herr.Catch[*herr.LoginError](func() { Prepare(codex, "not json at all", "", filepath.Join(root, "bad")) })
	if !ok || bad.Msg != "`codex-1`'s secret is not the contents of ~/.codex/auth.json after `codex login` "+
		"(Sign in with ChatGPT) (as JSON, or base64 of it)" {
		t.Errorf("a secret that is not a login file: %v", bad)
	}
	// A rotated login is sealed, and used next time; one sealed under another secret is not.
	login := Prepare(codex, codexAuth, "", filepath.Join(root, "one"))
	if Seal(login) != "" {
		t.Error("nothing changed during the job")
	}
	rotated := strings.Replace(codexAuth, "ref-c", "ref-z", 1)
	if err := os.WriteFile(login.Files.Str("auth.json"), []byte(rotated), 0o600); err != nil {
		t.Fatal(err)
	}
	sealed := Seal(login)
	again := Prepare(codex, codexAuth, sealed, filepath.Join(root, "two"))
	data, _ := os.ReadFile(again.Files.Str("auth.json"))
	if sealed == "" || string(data) != rotated {
		t.Errorf("the vault: %q %+v %q", sealed, again, data)
	}
	pasted := strings.Replace(codexAuth, "chatgpt", "other", 1)
	other := Prepare(codex, pasted, sealed, filepath.Join(root, "three"))
	if data, _ := os.ReadFile(other.Files.Str("auth.json")); string(data) != pasted {
		t.Errorf("a vault under another secret: %+v %q", other, data)
	}
	// muse: a login file, or an API key.
	muse := provider("muse", "secret", "MUSE_AUTH", nil)
	file := Prepare(muse, `{"schema_version": 1}`, "", filepath.Join(root, "m"))
	if file.Env["XDG_CONFIG_HOME"] != root+"/m/muse-config" || file.Env["MUSE_NO_AUTO_UPDATE"] != "1" ||
		file.Env["META_API_KEY"] != "" {
		t.Errorf("muse's file: %v", file.Env)
	}
	keyed := Prepare(muse, "meta-key-kkk", "", filepath.Join(root, "k"))
	if keyed.Env["META_API_KEY"] != "meta-key-kkk" || keyed.Files.Len() != 0 {
		t.Errorf("muse's key: %+v", keyed)
	}
	// agy has no secret login, and an empty secret is an error.
	if e, ok := herr.Catch[*herr.LoginError](func() {
		Prepare(provider("agy", "secret", "MUSE_AUTH", nil), "{}", "", filepath.Join(root, "agy"))
	}); !ok || e.Msg != "the agy CLI has no secret login; it logs in on the machine" {
		t.Errorf("agy: %v", e)
	}
	if e, ok := herr.Catch[*herr.LoginError](func() {
		Prepare(provider("claude", "secret", "CLAUDE_CODE_OAUTH_TOKEN", nil), "  ", "", root)
	}); !ok || e.Msg != "`claude-1`'s secret `CLAUDE_CODE_OAUTH_TOKEN` is empty in this job" {
		t.Errorf("an empty secret: %v", e)
	}
}
