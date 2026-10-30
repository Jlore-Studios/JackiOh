package cli

import (
	"bytes"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

func loadTestdata(t *testing.T, name string) *pyjson.Object {
	t.Helper()
	raw, err := os.ReadFile("testdata/" + name)
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	return pyjson.Obj(v)
}

// testdata/argparse.json is what Python 3.13 did with each argv, run through
// `harness.__main__.parser().parse_args(argv)` off a terminal (no COLUMNS): its stdout and stderr,
// its SystemExit code (null when it parsed), and the namespace it parsed (`vars(ns)`).
func TestParserMatchesArgparse(t *testing.T) {
	raw, err := os.ReadFile("testdata/argparse.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range pyjson.Objs(v) {
		argv := pyjson.Strings(c.Get("argv"))
		ns, exit := Parser().parse(argv)
		name := strings.Join(argv, " ")
		if c.Get("exit") == nil {
			if exit != nil {
				t.Errorf("%q: exited %d (%q), Python parsed it", name, exit.code, exit.stderr)
				continue
			}
			want := c.Obj("ns")
			got := pyjson.New()
			for _, k := range want.Keys() {
				got.Set(k, nsValue(ns[k]))
			}
			if len(ns) != want.Len() {
				t.Errorf("%q: namespace %v, Python's has keys %v", name, ns, want.Keys())
			}
			if !pyjson.Equal(got, want) {
				t.Errorf("%q: namespace\n%s\nPython's\n%s", name, pyjson.Dumps(got), pyjson.Dumps(want))
			}
			continue
		}
		if exit == nil {
			t.Errorf("%q: parsed, Python exited %v", name, c.Get("exit"))
			continue
		}
		if exit.code != c.Int("exit") {
			t.Errorf("%q: exit %d, Python's %d", name, exit.code, c.Int("exit"))
		}
		if exit.stdout != c.Str("stdout") {
			t.Errorf("%q: stdout\n%s\nPython's\n%s", name, exit.stdout, c.Str("stdout"))
		}
		if exit.stderr != c.Str("stderr") {
			t.Errorf("%q: stderr\n%s\nPython's\n%s", name, exit.stderr, c.Str("stderr"))
		}
	}
}

func nsValue(v any) any {
	if l, ok := v.([]string); ok {
		return pyjson.ListOf(l)
	}
	return v
}

func TestArgsOf(t *testing.T) {
	ns, exit := Parser().parse([]string{"dashboard", "--every", "600", "--for=1200", "--sweep", "--se"})
	if exit != nil {
		t.Fatal(exit.stderr)
	}
	got := argsOf(ns)
	want := &Args{Command: "dashboard", Every: 600, ForSeconds: 1200, Sweep: true, SectionOnly: true}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("got %+v, want %+v", got, want)
	}
	ns, _ = Parser().parse([]string{"plan", "--out", "o", "--quiet-ok", ""})
	if a := argsOf(ns); a.QuietOK == nil || *a.QuietOK != "" || a.Mode != "auto" || a.Out != "o" {
		t.Errorf("plan: %+v", a)
	}
	ns, _ = Parser().parse([]string{"plan", "--out", "o"})
	if a := argsOf(ns); a.QuietOK != nil {
		t.Errorf("--quiet-ok left out is None, not %q", *a.QuietOK)
	}
	ns, _ = Parser().parse([]string{"halt", "away", "today"})
	if a := argsOf(ns); !reflect.DeepEqual(a.Reason, []string{"away", "today"}) {
		t.Errorf("halt: %+v", a.Reason)
	}
}

// testdata/textwrap.json is what Python 3.13's textwrap.wrap and str(timedelta) gave.
func TestWrapAndTimedeltaMatchPython(t *testing.T) {
	data := loadTestdata(t, "textwrap.json")
	for _, c := range pyjson.Objs(data.Get("wrap")) {
		got := wrap(c.Str("text"), c.Int("width"))
		want := pyjson.Strings(c.Get("lines"))
		if !reflect.DeepEqual(got, want) && !(len(got) == 0 && len(want) == 0) {
			t.Errorf("wrap(%q, %d):\n%q\nPython's\n%q", c.Str("text"), c.Int("width"), got, want)
		}
	}
	for _, c := range pyjson.Objs(data.Get("timedelta")) {
		d := time.Duration(c.Int("us")) * time.Microsecond
		if got := timedelta(d); got != c.Str("text") {
			t.Errorf("timedelta(%d µs) = %q, Python's %q", c.Int("us"), got, c.Str("text"))
		}
	}
}

// run is Main with its output captured.
func run(t *testing.T, argv ...string) (int, string, string) {
	t.Helper()
	var out, errOut bytes.Buffer
	oldOut, oldErr := Stdout, Stderr
	Stdout, Stderr = &out, &errOut
	t.Cleanup(func() { Stdout, Stderr = oldOut, oldErr })
	code := Main(argv)
	return code, out.String(), errOut.String()
}

func TestMainExitCodes(t *testing.T) {
	code, out, errOut := run(t, "--help")
	if code != 0 || !strings.HasPrefix(out, "usage: harness [-h]\n") || errOut != "" {
		t.Errorf("--help: %d %q %q", code, out, errOut)
	}
	code, out, errOut = run(t, "plan", "-h")
	if code != 0 || !strings.HasPrefix(out, "usage: harness plan [-h] --out OUT") {
		t.Errorf("plan -h: %d %q %q", code, out, errOut)
	}
	code, out, errOut = run(t, "bogus")
	if code != 2 || out != "" || !strings.Contains(errOut, "harness: error: argument command: invalid choice: 'bogus'") {
		t.Errorf("bogus: %d %q %q", code, out, errOut)
	}
	code, _, errOut = run(t, "work", "--plan", "p")
	if code != 2 || !strings.HasSuffix(errOut, "harness work: error: the following arguments are required: --out, --work-dir\n") {
		t.Errorf("work: %d %q", code, errOut)
	}
	code, _, errOut = run(t, "dispatch", "--mode", "nightly")
	if code != 2 || !strings.Contains(errOut, "error: argument --mode: invalid choice: 'nightly' (choose from 'auto', 'build', 'revise', 'review', 'suggest')") {
		t.Errorf("dispatch: %d %q", code, errOut)
	}
	code, _, errOut = run(t, "pr-title")
	if code != 2 || !strings.Contains(errOut, "harness pr-title: error: the following arguments are required: --title") {
		t.Errorf("pr-title: %d %q", code, errOut)
	}
}

// main's own except clause (ConfigError, HarnessError) answers `error: …` and 1; anything else
// leaves a traceback's last line and 1, as an uncaught exception exits Python.
func TestMainCatchesAsPythonDoes(t *testing.T) {
	oldLoad, oldStatus := loadConfig, Commands["status"]
	t.Cleanup(func() { loadConfig, Commands["status"] = oldLoad, oldStatus })
	loadConfig = func() *config.Config { return &config.Config{} }

	cases := []struct {
		name   string
		cmd    func(*config.Config, *Args) int
		code   int
		stderr string
	}{
		{"returns", func(*config.Config, *Args) int { return 3 }, 3, ""},
		{"harness", func(*config.Config, *Args) int { herr.Raisef("the state is gone"); return 0 }, 1,
			"error: the state is gone\n"},
		{"config", func(*config.Config, *Args) int { herr.Raise(&herr.ConfigError{Msg: "no repo"}); return 0 }, 1,
			"error: no repo\n"},
		{"value", func(*config.Config, *Args) int { py.MustInt("#5"); return 0 }, 1,
			"ValueError: invalid literal for int() with base 10: '#5'\n"},
		{"oserror", func(*config.Config, *Args) int { readJSON("/nonexistent/plan.json"); return 0 }, 1,
			"FileNotFoundError: [Errno 2] No such file or directory: '/nonexistent/plan.json'\n"},
		{"bug", func(*config.Config, *Args) int { panic(errors.New("boom")) }, 1, "errorString: boom\n"},
	}
	// main's answer goes through redact, a generated stub until its own port lands in the tree.
	redacts := herr.Try(func() { redact.Redact("x", nil) }) == nil
	for _, c := range cases {
		Commands["status"] = c.cmd
		code, _, errOut := run(t, "status")
		if code != c.code {
			t.Errorf("%s: exit %d, want %d", c.name, code, c.code)
		}
		if strings.HasPrefix(c.stderr, "error:") && !redacts {
			continue
		}
		if c.code == 1 && c.stderr != "" && !strings.HasSuffix(errOut, c.stderr) {
			t.Errorf("%s: stderr %q, want it to end %q", c.name, errOut, c.stderr)
		}
		if strings.HasPrefix(c.stderr, "error:") != !strings.HasPrefix(errOut, "Traceback") && c.code == 1 {
			t.Errorf("%s: stderr %q", c.name, errOut)
		}
	}
}

func TestItemArgKeepsPythonsQuirk(t *testing.T) {
	if n := itemArg(" 12 "); n == nil || *n != 12 {
		t.Errorf("' 12 ': %v", n)
	}
	if itemArg("") != nil || itemArg("next") != nil {
		t.Error("no number is None")
	}
	// `#5` passes the isdigit() test once stripped, and int() then refuses it.
	err := herr.Try(func() { itemArg("#5") })
	var pe *py.Error
	if !errors.As(err, &pe) || pe.Type != "ValueError" {
		t.Errorf("#5: %v", err)
	}
}
