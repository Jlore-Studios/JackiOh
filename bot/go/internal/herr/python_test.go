package herr

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"syscall"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// type(exc).__name__ and str(exc) as Python 3.13 on Linux gives them for the same exceptions.
func TestNameAndStrArePythons(t *testing.T) {
	_, notFound := exec.LookPath("no-such-program-anywhere")
	var m map[string]int
	for _, c := range []struct {
		err        error
		name, text string
	}{
		{&py.Error{Type: "ValueError", Msg: "bad"}, "ValueError", "bad"},
		{py.KeyError("k"), "KeyError", "'k'"},
		{&GitHubError{Msg: "no", Status: 500}, "GitHubError", "no"},
		{&StateUnreadable{Msg: "x"}, "StateUnreadable", "x"},
		{&fs.PathError{Op: "open", Path: "plan.json", Err: syscall.ENOENT}, "FileNotFoundError",
			"[Errno 2] No such file or directory: 'plan.json'"},
		{&fs.PathError{Op: "mkdir", Path: "/x", Err: syscall.EACCES}, "PermissionError",
			"[Errno 13] Permission denied: '/x'"},
		{&os.LinkError{Op: "rename", Old: "a", New: "b", Err: syscall.EEXIST}, "FileExistsError",
			"[Errno 17] File exists: 'a' -> 'b'"},
		{syscall.ECONNREFUSED, "ConnectionRefusedError", "[Errno 111] Connection refused"},
		{os.NewSyscallError("write", syscall.EPIPE), "BrokenPipeError", "[Errno 32] Broken pipe"},
		{notFound, "FileNotFoundError", "[Errno 2] No such file or directory: 'no-such-program-anywhere'"},
		{Try(func() { panic("boom") }), "Exception", "boom"},
		{Try(func() { m["x"]++ }), "RuntimeError", "assignment to entry in nil map"},
		{errors.New("plain"), "Exception", "plain"},
		{fmt.Errorf("wrapped: %w", errors.New("x")), "Exception", "wrapped: x"},
	} {
		if Name(c.err) != c.name || Str(c.err) != c.text {
			t.Errorf("%T: %s: %s, want %s: %s", c.err, Name(c.err), Str(c.err), c.name, c.text)
		}
	}
	if !IsOSError(syscall.ENOENT) || !IsOSError(&fs.PathError{Err: syscall.EIO}) || IsOSError(errors.New("x")) {
		t.Error("IsOSError")
	}
}
