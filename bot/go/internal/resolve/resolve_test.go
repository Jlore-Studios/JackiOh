package resolve

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSummary(t *testing.T) {
	done := []*Resolution{
		{Path: "spec/INDEX.md", How: "took `main`'s version", OK: true},
		{Path: "g.jsonl", How: "ran `bless`", WhyNot: "it left no file"},
	}
	want := "`spec/INDEX.md`: took `main`'s version; `g.jsonl`: ran `bless`, not resolved: it left no file; " +
		"2 file(s) left for the builder"
	if got := Summary(done, []string{"a", "b"}); got != want {
		t.Errorf("Summary: %s", got)
	}
	if Summary(nil, nil) != "" {
		t.Error("nothing done says nothing")
	}
	if got := done[1].ToDict(); got.Str("why_not") != "it left no file" || got.Keys()[3] != "why_not" {
		t.Errorf("ToDict: %v", got.Keys())
	}
}

func TestMarkedReadsLinesAsText(t *testing.T) {
	dir := t.TempDir()
	for name, want := range map[string]bool{
		"start":    true,
		"end":      true,
		"middle":   false,
		"cr":       true,
		"indented": false,
	} {
		text := map[string]string{
			"start":    "a\n<<<<<<< HEAD\nb\n",
			"end":      ">>>>>>> main\n",
			"middle":   "Title\n=======\n",
			"cr":       "a\r<<<<<<< HEAD\r",
			"indented": " <<<<<<< HEAD\n",
		}[name]
		path := filepath.Join(dir, name)
		if err := os.WriteFile(path, []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
		if got := marked(path); got != want {
			t.Errorf("marked(%s) = %v", name, got)
		}
	}
	if marked(filepath.Join(dir, "absent")) {
		t.Error("no file holds no marker")
	}
}
