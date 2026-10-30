package controller

import (
	"archive/zip"
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"syscall"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func TestReferencesAndTitles(t *testing.T) {
	for ref, ok := range map[string]bool{"0123abcd-1760000000": true, "0123abcd-1760000000\n": true,
		"0123abcd-17600000000": true, "../state": false, "plans/x": false, "abcd-1": false, "": false,
		"0123ABCD-1760000000": false} {
		if Ref.MatchString(ref) != ok {
			t.Errorf("REF.match(%q) should be %v", ref, ok)
		}
	}
	ref := NewRef(time.Unix(1760000000, 0))
	if !Ref.MatchString(ref) || ref[9:] != "1760000000" {
		t.Errorf("new_ref = %q", ref)
	}
	if TitlePrefix("x") != "bot-night x " {
		t.Errorf("title_prefix = %q", TitlePrefix("x"))
	}
	for _, c := range []struct {
		planned *pyjson.Object
		want    string
	}{{pyjson.New("number", 3, "action", "build", "provider", "muse"), "build #3 on muse"},
		{pyjson.New("number", 0, "action", "suggest", "provider", "gpt"), "suggest on gpt"},
		{pyjson.New(), "None on None"}} {
		if got := What(c.planned); got != c.want {
			t.Errorf("_what = %q, want %q", got, c.want)
		}
	}
}

func zipOf(t *testing.T, names ...string) []byte {
	t.Helper()
	var buffer bytes.Buffer
	archive := zip.NewWriter(&buffer)
	for _, name := range names {
		w, err := archive.Create(name)
		if err != nil {
			t.Fatal(err)
		}
		if name[len(name)-1] == '/' {
			continue
		}
		if _, err := w.Write([]byte(name)); err != nil {
			t.Fatal(err)
		}
	}
	if err := archive.Close(); err != nil {
		t.Fatal(err)
	}
	return buffer.Bytes()
}

func TestAnUploadUnzipsAsZipfileDid(t *testing.T) {
	root := t.TempDir()
	for name, ok := range map[string]bool{"a/b.txt": true, "a/../b": true, "../escaped.txt": false,
		"/etc/passwd": false, root + "/inside": true, "": true} {
		if inside(root, name) != ok {
			t.Errorf("inside(%q) should be %v", name, ok)
		}
	}
	data := zipOf(t, "a/b.txt", "dir/", "./c.txt", "x/../y.txt", "x//z.txt")
	archive, err := zip.NewReader(bytes.NewReader(data), int64(len(data)))
	if err != nil {
		t.Fatal(err)
	}
	extractAll(archive, root)
	for path, body := range map[string]string{"a/b.txt": "a/b.txt", "c.txt": "./c.txt", "x/y.txt": "x/../y.txt",
		"x/z.txt": "x//z.txt"} {
		got, err := os.ReadFile(filepath.Join(root, path))
		if err != nil || string(got) != body {
			t.Errorf("%s: %q %v", path, got, err)
		}
	}
	if info, err := os.Stat(filepath.Join(root, "dir")); err != nil || !info.IsDir() {
		t.Errorf("dir/: %v", err)
	}
	exc, ok := herr.Catch[*ValueError](func() {
		data := zipOf(t, "..")
		archive, _ := zip.NewReader(bytes.NewReader(data), int64(len(data)))
		extractAll(archive, root)
	})
	if !ok || exc.Msg != "Empty filename." {
		t.Errorf("an empty name: %+v", exc)
	}
	if err := zipError(zip.ErrFormat, ""); herr.Name(err) != "BadZipFile" || err.Error() != "File is not a zip file" {
		t.Errorf("not a zip: %v", err)
	}
}

func TestADeliverysErrorIsNotedByItsPythonName(t *testing.T) {
	denied := &fs.PathError{Op: "mkdir", Path: "/x", Err: syscall.EACCES}
	for _, c := range []struct {
		err        error
		name, text string
	}{{&ValueError{Type: "ValueError", Msg: "no plan x on bot-state"}, "ValueError", "no plan x on bot-state"},
		{&herr.GitHubError{Msg: "no", Status: 500}, "GitHubError", "no"},
		{&herr.GitError{Msg: "clone failed"}, "GitError", "clone failed"},
		{denied, "PermissionError", "[Errno 13] Permission denied: '/x'"},
		{herr.Try(func() { panic("boom") }), "Exception", "boom"}} {
		if herr.Name(c.err) != c.name || herr.Str(c.err) != c.text {
			t.Errorf("%T: %s: %s", c.err, herr.Name(c.err), herr.Str(c.err))
		}
	}
}
