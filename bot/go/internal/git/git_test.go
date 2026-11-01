package git

import (
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds the Python's answers (harness.git's normalise and matches, with the
// committed config's forbidden and review paths, Python 3.13).
func python(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("testdata", "python.json"))
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return pyjson.Obj(v)
}

func TestNormaliseAndMatchesAsPython(t *testing.T) {
	data := python(t)
	for _, c := range data.List("normalise") {
		row := pyjson.List(c)
		if got := Normalise(row[0].(string)); got != row[1] {
			t.Errorf("Normalise(%q) = %q; Python %q", row[0], got, row[1])
		}
	}
	cases := data.List("matches")
	if len(cases) == 0 {
		t.Fatal("no cases")
	}
	for _, c := range cases {
		row := pyjson.List(c)
		patterns := pyjson.Strings(data.Obj("patterns").Get(row[0].(string)))
		if got := Matches(row[1].(string), patterns); got != row[2] {
			t.Errorf("Matches(%q, %s) = %v; Python %v", row[1], row[0], got, row[2])
		}
	}
}

func TestTextIsPythonsTextMode(t *testing.T) {
	if got := text([]byte("a\r\nb\rc\xe2\x82\n")); got != "a\nb\nc�\n" {
		t.Fatalf("text = %q", got)
	}
	err := &TimeoutExpired{Cmd: []string{"git", "-c", "x"}, Timeout: 600}
	if err.Error() != "Command '['git', '-c', 'x']' timed out after 600 seconds" {
		t.Fatal(err.Error())
	}
}

func TestRunCommitsAndReads(t *testing.T) {
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("no git")
	}
	dir := t.TempDir()
	env := map[string]string{"PATH": os.Getenv("PATH"), "HOME": dir}
	repo := NewGit(dir, env)
	if repo.Env["GIT_TERMINAL_PROMPT"] != "0" || len(env) != 2 {
		t.Fatal("the env is a copy, with no prompt")
	}
	repo.Run(true, 600, nil, "init", "-q", "-b", "main")
	who := IdentityBot("jgoetzmann-bot", 7)
	if who.Email != "7+jgoetzmann-bot@users.noreply.github.com" {
		t.Fatal(who.Email)
	}
	if repo.CommitAll("nothing", who) {
		t.Fatal("nothing to commit")
	}
	os.WriteFile(filepath.Join(dir, "a.txt"), []byte("one\r\ntwo\n"), 0o644)
	if !repo.CommitAll("first", who) {
		t.Fatal("a commit")
	}
	base := repo.Head()
	os.WriteFile(filepath.Join(dir, "a.txt"), []byte("one\n"), 0o644)
	os.WriteFile(filepath.Join(dir, "b.bin"), []byte{0, 1, 2}, 0o644)
	repo.CommitAll("second", who)
	if got := repo.Numstat(base, "HEAD"); pyjson.Dumps(got) != `{"a.txt": 3, "b.bin": 0}` {
		t.Fatalf("numstat %s", pyjson.Dumps(got))
	}
	if got := repo.ChangedPaths(base, "HEAD"); !reflect.DeepEqual(got, []string{"a.txt", "b.bin"}) {
		t.Fatalf("changed %v", got)
	}
	if repo.Rev("nope") != "" || repo.Blob(base, "b.bin") != "" || repo.Blob("HEAD", "b.bin") == "" {
		t.Fatal("rev and blob")
	}
	if got := repo.Log(base, "HEAD"); !strings.HasSuffix(got, " second") {
		t.Fatalf("log %q", got)
	}
	if author := repo.Out(true, "log", "-1", "--format=%an <%ae>"); author != "jgoetzmann-bot <7+jgoetzmann-bot@users.noreply.github.com>" {
		t.Fatal(author)
	}
	if got := repo.Unsanctioned("HEAD", []string{base}, []string{"*.bin"}); !reflect.DeepEqual(got, []string{"b.bin"}) {
		t.Fatalf("unsanctioned %v", got)
	}
	repo.RestoreFrom([]string{base}, []string{"a.txt", "b.bin"})
	if _, err := os.Stat(filepath.Join(dir, "b.bin")); !os.IsNotExist(err) {
		t.Fatal("a path no anchor has is deleted")
	}
	if got, _ := os.ReadFile(filepath.Join(dir, "a.txt")); string(got) != "one\r\ntwo\n" {
		t.Fatalf("a path is put back as the anchor holds it: %q", got)
	}
	if diff, cut := repo.Diff(base, "HEAD", 10); !cut || len([]rune(diff)) != 10 {
		t.Fatal("a diff cut short")
	}
	exc, ok := herr.Catch[*herr.GitError](func() { repo.Run(true, 600, nil, "rev-parse", "--verify", "nope") })
	if !ok || !strings.HasPrefix(exc.Msg, "git rev-parse --verify nope failed (128): ") {
		t.Fatalf("a failure raises GitError: %v", exc)
	}
}
