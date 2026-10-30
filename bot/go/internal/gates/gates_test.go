package gates

import (
	"os"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

type configGate = config.Gate

// testdata/python.json is bot/harness/gates.py's answers to the same calls.
func expected(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

func results() []*GateResult {
	return []*GateResult{
		{Name: "unit", Run: "pnpm test", OK: true, ExitCode: 0, Seconds: 12.5, Tail: "fine"},
		{Name: "lint", Run: "eslint", ExitCode: 1, Seconds: 2.5, Tail: "boom", PreExisting: true},
		{Name: "e2e", Run: "cypress", ExitCode: 124, Seconds: 3.5, Tail: "slow", Inconclusive: true},
		{Name: "fmt", Run: "cargo fmt", ExitCode: 2, Seconds: 0.4},
		{Name: "late", Run: "x", ExitCode: -1, Seconds: 0.0, Skipped: "no time left"},
	}
}

func TestReportsAsPython(t *testing.T) {
	data := expected(t)
	rs := results()
	if got := Table(rs); got != data.Str("table") {
		t.Errorf("Table\n%s\nwant\n%s", got, data.Str("table"))
	}
	if got := FailuresText(rs); got != data.Str("failures") {
		t.Errorf("FailuresText\n%s\nwant\n%s", got, data.Str("failures"))
	}
	if Table(nil) != data.Str("empty_table") {
		t.Errorf("Table of none: %s", Table(nil))
	}
	for i, want := range data.List("dicts") {
		if got := rs[i].ToDict(); pyjson.Dumps(got) != pyjson.Dumps(want) {
			t.Errorf("ToDict %s, want %s", pyjson.Dumps(got), pyjson.Dumps(want))
		}
	}
	if Green(rs) {
		t.Error("a gate this change turned red is not green")
	}
	if !Green(rs[:3]) {
		t.Error("passed, pre-existing and inconclusive are green")
	}
}

func TestMarkInconclusiveOnlyTheGatesThatTimedOut(t *testing.T) {
	rs := []*GateResult{
		{Name: "unit", ExitCode: TimedOut},
		{Name: "install", ExitCode: TimedOut},
		{Name: "lint", ExitCode: 1},
		{Name: "late", ExitCode: TimedOut, Skipped: "no time left"},
	}
	MarkInconclusive(rs, map[string]bool{"unit": true, "lint": true, "late": true})
	if !rs[0].Inconclusive || rs[1].Inconclusive || rs[2].Inconclusive || rs[3].Inconclusive {
		t.Errorf("inconclusive: %v %v %v %v", rs[0].Inconclusive, rs[1].Inconclusive, rs[2].Inconclusive, rs[3].Inconclusive)
	}
}

func TestRunAllSkipsWithNoTimeLeft(t *testing.T) {
	ran := 0
	saved := RunGate
	t.Cleanup(func() { RunGate = saved })
	var timeouts []any
	RunGate = func(_ *configGate, _ string, _ map[string]string, timeoutS *int) *GateResult {
		ran++
		if timeoutS == nil {
			timeouts = append(timeouts, nil)
		} else {
			timeouts = append(timeouts, *timeoutS)
		}
		return &GateResult{OK: true}
	}
	gates := []*configGate{{Name: "a", Run: "x"}, {Name: "b", Run: "y"}}
	left := []float64{120.9, 59.9}
	out := RunAll(gates, ".", nil, func() float64 { l := left[0]; left = left[1:]; return l })
	if ran != 1 || out[1].Skipped != "no time left" || out[1].ExitCode != -1 || out[1].OK {
		t.Errorf("skipped: %+v", out[1])
	}
	RunAll(gates, ".", nil, nil)
	if !pyjson.Equal(timeouts, []any{120, nil, nil}) {
		t.Errorf("timeouts %v", timeouts)
	}
}

func TestRunIsSubprocessRun(t *testing.T) {
	dir := t.TempDir()
	env := map[string]string{"PATH": os.Getenv("PATH"), "ONLY": "this"}
	p := run([]string{"bash", "-c", `printf 'a\r\nb\rc'; printf 'err' >&2; echo "$ONLY ${HOME:-nohome}" > f; exit 3`},
		dir, env, 30*time.Second)
	if p.timedOut || p.code != 3 || py.Universal(py.DecodeReplace(p.stdout)) != "a\nb\nc" || string(p.stderr) != "err" {
		t.Errorf("run: %+v", p)
	}
	if got, _ := os.ReadFile(dir + "/f"); string(got) != "this nohome\n" {
		t.Errorf("the child has exactly env, in cwd: %q", got)
	}
	p = run([]string{"bash", "-c", "kill -TERM $$"}, dir, env, 30*time.Second)
	if p.code != -15 {
		t.Errorf("a signal is minus its number: %d", p.code)
	}
	started := time.Now()
	p = run([]string{"bash", "-c", "printf partial; sleep 30"}, dir, env, time.Second)
	if !p.timedOut || string(p.stdout) != "partial" || time.Since(started) > 10*time.Second {
		t.Errorf("timeout: %+v after %s", p, time.Since(started))
	}
	// A background process that keeps the pipe open holds the run until the timeout, as
	// communicate() does; then it returns without waiting for that process.
	started = time.Now()
	p = run([]string{"bash", "-c", "sleep 30 & echo started"}, dir, env, 2*time.Second)
	if !p.timedOut || !strings.HasPrefix(string(p.stdout), "started") || time.Since(started) > 10*time.Second {
		t.Errorf("held pipe: %+v after %s", p, time.Since(started))
	}
}
