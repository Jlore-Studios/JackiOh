package config

import (
	"bufio"
	"errors"
	"os"
	"sort"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json and identities.jsonl are the Python harness's answers to the same calls
// (bot/harness/config.py, run once with each bot's HARNESS_HOME).

func load(t *testing.T, path string) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

// outcome is what the Python's helpers printed: {"ok": value} or {"error": "Class: message"}.
func outcome(fn func() any) *pyjson.Object {
	var got any
	err := herr.Try(func() { got = fn() })
	if err == nil {
		return pyjson.New("ok", got)
	}
	var cfg *herr.ConfigError
	var exc *py.Error
	switch {
	case errors.As(err, &cfg):
		return pyjson.New("error", "ConfigError: "+cfg.Msg)
	case errors.As(err, &exc):
		return pyjson.New("error", exc.Type+": "+exc.Msg)
	}
	panic(err)
}

func TestLabelsAsEachBotHasThem(t *testing.T) {
	night := &identity.Identity{Name: "the night bot", LabelPrefix: "bot:", Modes: []string{"fullsend"},
		Suggestions: true}
	squishy := &identity.Identity{Name: "Squishy", LabelPrefix: "squishy:",
		Modes: []string{"oneshot", "split", "split-bot", "fullsend"}, Suggestions: false}
	f, err := os.Open("testdata/identities.jsonl")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	scanner.Buffer(nil, 1<<20)
	for _, id := range []*identity.Identity{night, squishy} {
		if !scanner.Scan() {
			t.Fatal("identities.jsonl is short")
		}
		v, err := pyjson.LoadsString(scanner.Text())
		if err != nil {
			t.Fatal(err)
		}
		want := v.(*pyjson.Object)
		got := labels(id, id.LabelPrefix, id.Modes)
		if !pyjson.Equal(pyjson.ListOf(got.Keys()), want.Get("order")) {
			t.Errorf("%s: labels in the order %v, want %v", id.Name, got.Keys(), want.Get("order"))
		}
		keys := got.Keys()
		sort.Strings(keys)
		items := []any{}
		for _, k := range keys {
			items = append(items, []any{k, got.Get(k)})
		}
		retired := []any{"method:manual", "method:use-bot", id.LabelPrefix + "approved",
			id.LabelPrefix + "needs-review", "large patch", "Info"}
		if dump := pyjson.Dumps([]any{items, retired}); dump != want.Str("labels_dump") {
			t.Errorf("%s: labels dump\n%s\nwant\n%s", id.Name, dump, want.Str("labels_dump"))
		}
		if s := said(id.Name, "The night bot and the night bot; a night bot"); s != want.Str("said") {
			t.Errorf("%s: said %q, want %q", id.Name, s, want.Str("said"))
		}
	}
}

func TestTheNightBotsConstantsWithoutIdentity(t *testing.T) {
	// identity is a stub in this build: the constants fall back to the night bot's.
	if Slash != "/harness" || CommitPrefix != "bot" || LabelBuild != "bot:build" ||
		ConfigPath != ".harness/config.json" || HaltPath != ".harness/HALT" || TrustPath != ".harness/trust.txt" {
		t.Errorf("night bot constants: %q %q %q %q %q %q", Slash, CommitPrefix, LabelBuild, ConfigPath, HaltPath, TrustPath)
	}
	if got := pyjson.Dumps(DifficultyLabels); got != `{"difficulty:easy": "easy", "difficulty:medium": "medium", "difficulty:hard": "hard"}` {
		t.Errorf("DifficultyLabels: %s", got)
	}
}

func TestSecretKeysInOrder(t *testing.T) {
	want := load(t, "testdata/python.json").List("secret_keys")
	if !pyjson.Equal(pyjson.ListOf(SecretKeys), want) {
		t.Errorf("SecretKeys %v, want %v", SecretKeys, want)
	}
}

func gateJSON(g *Gate) any { return []any{g.Name, g.Run, g.TimeoutMinutes, g.Machine} }

func TestParseHelpersAsPython(t *testing.T) {
	cases := load(t, "testdata/python.json").Obj("cases")
	show := map[string]func(raw any) any{
		"easy": func(raw any) any {
			r := easyRule(raw)
			return []any{r.MaxFiles, r.MaxLines, pyjson.ListOf(r.OffLimits), pyjson.ListOf(r.Generated)}
		},
		"gate": func(raw any) any { return gateJSON(gate(raw, "gates[0]")) },
		"resolvers": func(raw any) any {
			out := []any{}
			for _, r := range resolvers(raw) {
				out = append(out, []any{r.Path, r.Take, r.Run, r.TimeoutMinutes})
			}
			return out
		},
		"priorities": func(raw any) any {
			out := pyjson.New()
			for k, v := range priorities(raw) {
				out.Set(k, v)
			}
			return out
		},
		"quiet": func(raw any) any {
			q := quiet(raw)
			partners := []any{}
			for _, p := range q.Partners {
				partners = append(partners, []any{p.Repo, p.Workflows})
			}
			return []any{q.Enabled, q.IntervalMinutes, q.MaxWaitMinutes, q.PingModel, partners}
		},
		"review_cycles": func(raw any) any {
			c := reviewCycles(raw)
			out := []any{}
			for _, k := range c.Keys() {
				out = append(out, []any{k, c.Get(k)})
			}
			return out
		},
	}
	for _, kind := range cases.Keys() {
		for _, pair := range cases.List(kind) {
			raw, want := pair.([]any)[0], pair.([]any)[1]
			got := outcome(func() any { return show[kind](raw) })
			if !pyjson.Equal(got, want) {
				t.Errorf("%s(%s) = %s, want %s", kind, pyjson.Dumps(raw), pyjson.Dumps(got), pyjson.Dumps(want))
			}
		}
	}
}

func TestParseErrorsAsPython(t *testing.T) {
	for _, pair := range load(t, "testdata/python.json").List("parse_errors") {
		raw, want := pair.([]any)[0].(*pyjson.Object), pair.([]any)[1].(string)
		got := outcome(func() any { return Parse(raw, RepoRoot, map[string]string{}, nil) })
		if got.Str("error") != want {
			t.Errorf("parse: %q, want %q", got.Str("error"), want)
		}
	}
}

func TestEnvironmentHelpers(t *testing.T) {
	env := map[string]string{"GH_TOKEN": "short", "GITHUB_TOKEN": "a-long-token", "PATH": "/bin",
		"CODEX_AUTH_JSON": "{}", "HARNESS_PROVIDER_SECRET": "provider-secret"}
	if got := strings.Join(SecretValues(env), ","); got != "a-long-token,provider-secret" {
		t.Errorf("SecretValues: %s", got)
	}
	child := ChildEnv(env, []string{"CODEX_AUTH_JSON"})
	if len(child) != 2 || child["PATH"] != "/bin" || child["CODEX_AUTH_JSON"] != "{}" {
		t.Errorf("ChildEnv: %v", child)
	}
	other := &identity.Other{Home: ".squishy", TokenEnv: "SQUISHY_GITHUB_TOKEN"}
	got := CompanionEnv(other, map[string]string{"BOT_GITHUB_TOKEN": "mine", "X": "1"})
	if got["HARNESS_HOME"] != ".squishy" || got["X"] != "1" || len(got) != 2 {
		t.Errorf("CompanionEnv without the token: %v", got)
	}
	got = CompanionEnv(other, map[string]string{"BOT_GITHUB_TOKEN": "mine", "SQUISHY_GITHUB_TOKEN": "theirs"})
	if got["BOT_GITHUB_TOKEN"] != "theirs" {
		t.Errorf("CompanionEnv with the token: %v", got)
	}
}

func TestConfigMethods(t *testing.T) {
	c := NewConfig()
	c.Repo, c.RunID = "o/r", "42"
	if c.RunURL() != "https://github.com/o/r/actions/runs/42" {
		t.Errorf("RunURL: %s", c.RunURL())
	}
	c.RunID = "pending:7"
	if c.RunURL() != "" {
		t.Errorf("a pending run has no page: %s", c.RunURL())
	}
	c.CLIBins["claude"] = "/x/claude"
	if c.ClaudeBin() != "/x/claude" || c.Bin("codex") != "codex" {
		t.Errorf("Bin: %s %s", c.ClaudeBin(), c.Bin("codex"))
	}
	c.ProviderSecrets = map[string]string{"MUSE_AUTH": "m"}
	if c.SecretFor("MUSE_AUTH") != "m" || c.SecretFor("CODEX_AUTH_JSON") != "" {
		t.Error("SecretFor by name")
	}
	c.ProviderSecrets[ProviderSecretKey] = "p"
	if c.SecretFor("MUSE_AUTH") != "p" {
		t.Error("SecretFor prefers the job's one secret")
	}
	c.ActionsToken = "a"
	if c.WriteToken() != "a" {
		t.Error("WriteToken falls back to the job's token")
	}
}
