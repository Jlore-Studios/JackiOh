package work

import (
	"encoding/json"
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// testdata/python.json is what work.py's own helpers and string.Template give for the same
// inputs (written by Python 3).
type pythonCases struct {
	Sub      [][3]string          `json:"sub"`
	Safe     [][2]string          `json:"safe"`
	Prio     [][2]json.RawMessage `json:"prio"`
	Manifest [][2]json.RawMessage `json:"manifest"`
}

func loadCases(t *testing.T) pythonCases {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases pythonCases
	if err := json.Unmarshal(data, &cases); err != nil {
		t.Fatal(err)
	}
	return cases
}

func TestTemplateIsStringTemplate(t *testing.T) {
	cases := loadCases(t)
	values := map[string]string{"a": "A", "b": "B", "a_1": "Z"}
	for _, c := range cases.Sub {
		got, err := pyTemplate(c[0]).substitute(values)
		failed := ""
		if err != nil {
			failed = herr.Name(err) + ": " + herr.Str(err)
			got = ""
		}
		if got != c[1] || failed != c[2] {
			t.Errorf("substitute(%q) = %q, %q; Python: %q, %q", c[0], got, failed, c[1], c[2])
		}
	}
	for _, c := range cases.Safe {
		if got := pyTemplate(c[0]).safeSubstitute(values); got != c[1] {
			t.Errorf("safe_substitute(%q) = %q; Python: %q", c[0], got, c[1])
		}
	}
}

func TestClassPriorities(t *testing.T) {
	for _, c := range loadCases(t).Prio {
		var priorities map[string]string
		var want string
		if err := json.Unmarshal(c[0], &priorities); err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal(c[1], &want); err != nil {
			t.Fatal(err)
		}
		if got := ClassPriorities(priorities); got != want {
			t.Errorf("ClassPriorities(%v) =\n%s\nPython:\n%s", priorities, got, want)
		}
	}
}

func TestIsManifest(t *testing.T) {
	for _, c := range loadCases(t).Manifest {
		var path string
		var want bool
		if err := json.Unmarshal(c[0], &path); err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal(c[1], &want); err != nil {
			t.Fatal(err)
		}
		if got := IsManifest(path); got != want {
			t.Errorf("IsManifest(%q) = %v; Python: %v", path, got, want)
		}
	}
}

func TestFormatAndErrors(t *testing.T) {
	got := pyFormat(SelfCheckContext, map[string]any{"n": 2, "cap": 3})
	if want := "Self check 2 of at most 3."; got[len(got)-len(want):] != want {
		t.Errorf("pyFormat: %q", got)
	}
	if got := pyFormat("{{x}} {a}", map[string]any{"a": nil}); got != "{x} None" {
		t.Errorf("pyFormat braces: %q", got)
	}
	err := herr.Try(func() { pyFormat("{missing}", nil) })
	if err == nil || herr.Name(err)+": "+herr.Str(err) != "KeyError: 'missing'" {
		t.Errorf("pyFormat missing key: %v", err)
	}
	err = herr.Try(func() { readText("testdata/no such file", true) })
	if want := "FileNotFoundError: [Errno 2] No such file or directory: 'testdata/no such file'"; err == nil ||
		herr.Name(err)+": "+herr.Str(err) != want {
		t.Errorf("OSError: %q", herr.Name(err)+": "+herr.Str(err))
	}
	if err := herr.Try(func() { py.MustInt("x") }); err == nil ||
		herr.Str(err) != "invalid literal for int() with base 10: 'x'" {
		t.Errorf("pyInt: %v", err)
	}
}
