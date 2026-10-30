package tests

// bot/tests/test_logins.py: logins: each CLI's secret written where it looks, and refreshed logins
// kept in the vault.

import (
	"encoding/base64"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/logins"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/vault"
)

// loginsCodexAuth is CODEX_AUTH (a fresh copy each time).
func loginsCodexAuth() *pyjson.Object {
	return pyjson.New("auth_mode", "chatgpt", "OPENAI_API_KEY", nil, "last_refresh", "2026-10-01T00:00:00Z",
		"tokens", pyjson.New("id_token", "id-"+strings.Repeat("a", 40), "access_token", "acc-"+strings.Repeat("b", 40),
			"refresh_token", "ref-"+strings.Repeat("c", 40), "account_id", "acct-1234567890abcdef"))
}

// loginsMuseAuth is MUSE_AUTH (a fresh copy each time).
func loginsMuseAuth() *pyjson.Object {
	return pyjson.New("schema_version", 1, "providers", pyjson.New("meta", pyjson.New("mechanism", "oauth",
		"access_token", "m-"+strings.Repeat("f", 40), "refresh_token", "mr-"+strings.Repeat("g", 40))))
}

// loginsSeal is vault.seal, whose error Python raises.
func loginsSeal(t *testing.T, payload *pyjson.Object, secret string) string {
	t.Helper()
	sealed, err := vault.Seal(payload, secret)
	if err != nil {
		t.Fatal(err)
	}
	return sealed
}

// loginsReadText is Path.read_text().
func loginsReadText(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}

// loginsJSON is json.loads(text), an object.
func loginsJSON(t *testing.T, text string) *pyjson.Object {
	t.Helper()
	v, err := pyjson.LoadsString(text)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

// ---------------------------------------------------------------------------- VaultTests

func TestLogins_RoundTripAndRefusals(t *testing.T) {
	sealed := loginsSeal(t, pyjson.New("provider", "gpt", "files", pyjson.New("auth.json", "{}")), "secret-one")
	providersTrue(t, vault.LooksSealed(sealed))
	providersEq(t, vault.Open(sealed, "secret-one").Get("provider"), "gpt")
	providersTrue(t, vault.Open(sealed, "another-secret") == nil) // a new login wins
	parts := strings.SplitN(sealed, ":", 3)
	version, tag, body := parts[0], parts[1], parts[2]
	tampered := version + ":" + tag + ":" + py.SliceTo(body, -8) + "AAAAAAAA"
	providersTrue(t, vault.Open(tampered, "secret-one") == nil)
	providersFalse(t, vault.LooksSealed("hello"))
	providersFalse(t, vault.LooksSealed("v1:zz:"+strings.Repeat("A", 10)))
	providersTrue(t, vault.Open("", "secret-one") == nil)
}

func TestLogins_TheCiphertextShowsNothing(t *testing.T) {
	sealed := loginsSeal(t, pyjson.New("files", pyjson.New("auth.json", pyjson.Dumps(loginsCodexAuth()))), "secret-one")
	providersNotIn(t, "ref-ccc", sealed)
	providersNotIn(t, "chatgpt", sealed)
}

// ---------------------------------------------------------------------------- PrepareTests

// loginsWorld is PrepareTests' setUp.
type loginsWorld struct {
	pool  *providers.Pool
	home  string
	codex *providers.Provider
	muse  *providers.Provider
}

func newLoginsWorld(t *testing.T) *loginsWorld {
	pool := providers.Load(ts.Root)
	return &loginsWorld{
		pool:  pool,
		home:  filepath.Join(t.TempDir(), "logins"),
		codex: ts.SecretLogin(pool.Get("gpt"), "CODEX_AUTH_JSON"),
		muse:  ts.SecretLogin(pool.Get("muse"), "MUSE_AUTH"),
	}
}

func TestLogins_ClaudeGetsItsTokenAndItsOwnConfigDirectory(t *testing.T) {
	w := newLoginsWorld(t)
	login := logins.Prepare(w.pool.Get("claude-2"), "sk-ant-oat01-"+strings.Repeat("x", 30), "", w.home)
	providersEq(t, login.Env["CLAUDE_CODE_OAUTH_TOKEN"], "sk-ant-oat01-"+strings.Repeat("x", 30))
	info, err := os.Stat(login.Env["CLAUDE_CONFIG_DIR"])
	providersTrue(t, err == nil && info.IsDir())
	providersEq(t, login.Files.Len(), 0)
	providersNone(t, logins.Seal(login))
}

func TestLogins_CodexTakesAuthJSONAsPastedOrBase64(t *testing.T) {
	w := newLoginsWorld(t)
	text := pyjson.Dumps(loginsCodexAuth())
	for _, secret := range []string{text, base64.StdEncoding.EncodeToString([]byte(text))} {
		login := logins.Prepare(w.codex, secret, "", filepath.Join(w.home, py.SliceTo(secret, 4)))
		auth := filepath.Join(login.Env["CODEX_HOME"], "auth.json")
		providersEq(t, loginsJSON(t, loginsReadText(t, auth)).Obj("tokens").Get("account_id"),
			"acct-1234567890abcdef")
		info, err := os.Stat(auth)
		if err != nil {
			t.Fatal(err)
		}
		providersEq(t, info.Mode().Perm(), os.FileMode(0o600))
		providersIn(t, `cli_auth_credentials_store = "file"`,
			loginsReadText(t, filepath.Join(filepath.Dir(auth), "config.toml")))
	}
	providersRaises[*herr.LoginError](t, func() {
		logins.Prepare(w.codex, "not json at all", "", filepath.Join(w.home, "bad"))
	})
}

func TestLogins_ARotatedLoginIsSealedAndUsedNextTime(t *testing.T) {
	w := newLoginsWorld(t)
	secret := pyjson.Dumps(loginsCodexAuth())
	login := logins.Prepare(w.codex, secret, "", filepath.Join(w.home, "one"))
	providersNone(t, logins.Seal(login)) // nothing changed during the job
	rotated := loginsCodexAuth()
	rotated.Obj("tokens").Set("refresh_token", "ref-"+strings.Repeat("z", 40))
	if err := os.WriteFile(login.Files.Str("auth.json"), []byte(pyjson.Dumps(rotated)), 0o666); err != nil {
		t.Fatal(err)
	}
	sealed := logins.Seal(login)
	providersTrue(t, vault.LooksSealed(sealed))
	again := logins.Prepare(w.codex, secret, sealed, filepath.Join(w.home, "two"))
	providersEq(t, again.Source, "vault")
	auth := loginsJSON(t, loginsReadText(t, filepath.Join(again.Env["CODEX_HOME"], "auth.json")))
	providersEq(t, auth.Obj("tokens").Get("refresh_token"), "ref-"+strings.Repeat("z", 40))
	// A vault under another secret is ignored: a freshly pasted login wins.
	other := logins.Prepare(w.codex, pyjson.Dumps(loginsCodexAuth().Set("x", 1)), sealed,
		filepath.Join(w.home, "three"))
	providersEq(t, other.Source, "secret")
	// And so is one sealed for another provider, even under the same secret.
	foreign := loginsSeal(t, pyjson.New("provider", "muse", "files", pyjson.New("auth.json", "{}")), secret)
	mismatch := logins.Prepare(w.codex, secret, foreign, filepath.Join(w.home, "four"))
	providersEq(t, mismatch.Source, "secret")
}

func TestLogins_AMachineLoginWritesNothingAndLeavesTheCLIItsOwnHome(t *testing.T) {
	w := newLoginsWorld(t)
	for _, name := range []string{"gpt", "agy", "muse"} {
		provider := w.pool.Get(name)
		providersEq(t, provider.Login, "machine")
		login := logins.Prepare(provider, "", "a vault it must not open", filepath.Join(w.home, name))
		providersEq(t, []any{login.Source, login.Files.Len()}, []any{"machine", 0})
		_, err := os.Stat(filepath.Join(w.home, name))
		providersTrue(t, os.IsNotExist(err))
		for _, key := range []string{"CODEX_HOME", "XDG_CONFIG_HOME", "CLAUDE_CONFIG_DIR", "META_API_KEY"} {
			providersNotIn(t, key, login.Env)
		}
		providersNone(t, logins.Seal(login)) // nothing of it leaves the machine
	}
	providersEq(t, logins.Prepare(w.pool.Get("muse"), "", "", w.home).Env,
		map[string]string{"MUSE_NO_AUTO_UPDATE": "1"})
}

func TestLogins_AgyHasNoSecretLogin(t *testing.T) {
	w := newLoginsWorld(t)
	providersRaises[*herr.LoginError](t, func() {
		logins.Prepare(ts.SecretLogin(w.pool.Get("agy"), "MUSE_AUTH"), "{}", "", w.home)
	})
}

func TestLogins_MuseTakesALoginFileOrFallsBackToAnAPIKey(t *testing.T) {
	w := newLoginsWorld(t)
	login := logins.Prepare(w.muse, pyjson.Dumps(loginsMuseAuth()), "", filepath.Join(w.home, "a"))
	auth := filepath.Join(login.Env["XDG_CONFIG_HOME"], "muse", "auth.json")
	info, err := os.Stat(auth)
	providersTrue(t, err == nil && info.Mode().IsRegular())
	providersNotIn(t, "META_API_KEY", login.Env)
	providersEq(t, login.Env["MUSE_NO_AUTO_UPDATE"], "1")
	keyed := logins.Prepare(w.muse, "meta-key-"+strings.Repeat("k", 30), "", filepath.Join(w.home, "b"))
	providersEq(t, []any{keyed.Env["META_API_KEY"], keyed.Source},
		[]any{"meta-key-" + strings.Repeat("k", 30), "api key"})
}

func TestLogins_TheTokensInsideALoginFileAreRedacted(t *testing.T) {
	w := newLoginsWorld(t)
	logins.Prepare(w.codex, pyjson.Dumps(loginsCodexAuth()), "", w.home)
	refresh := loginsCodexAuth().Obj("tokens").Str("refresh_token")
	providersNotIn(t, refresh, redact.Redact("the refresh token is "+refresh, nil))
}

func TestLogins_AnEmptySecretIsAnError(t *testing.T) {
	w := newLoginsWorld(t)
	providersRaises[*herr.LoginError](t, func() {
		logins.Prepare(w.pool.Get("claude-1"), "  ", "", w.home)
	})
}
