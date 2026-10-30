package herr

// What a Python `except Exception as exc:` reads off what it caught: type(exc).__name__, str(exc)
// and isinstance(exc, OSError), for the errors the harness raises and the ones Go returns where
// Python raised.

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"runtime"
	"strings"
	"syscall"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// Name is type(exc).__name__: a *py.Error's Type, the harness's own classes by name, the OSError
// subclass of an errno (FileNotFoundError, PermissionError, …), RuntimeError for a Go fault
// (where Python would have raised AttributeError, KeyError or TypeError), Exception for a panic
// that was not an error and for a Go error of no type of its own, and any other error's Go type
// name (gh's HTTPError, URLError and TimeoutError are named for Python's).
func Name(err error) string {
	var exc *py.Error
	if errors.As(err, &exc) {
		return exc.Type
	}
	switch err.(type) {
	case *HarnessError:
		return "HarnessError"
	case *ConfigError:
		return "ConfigError"
	case *GitHubError:
		return "GitHubError"
	case *GitError:
		return "GitError"
	case *StateConflict:
		return "StateConflict"
	case *StateUnreadable:
		return "StateUnreadable"
	case *LoginError:
		return "LoginError"
	}
	if IsOSError(err) {
		var errno syscall.Errno
		var execErr *exec.Error
		switch {
		case errors.As(err, &errno):
			return py.OSErrorName(errno)
		case errors.As(err, &execErr) && errors.Is(execErr.Err, exec.ErrNotFound):
			return "FileNotFoundError"
		}
		return "OSError"
	}
	var fault runtime.Error
	if errors.As(err, &fault) {
		return "RuntimeError"
	}
	var panicked *Panic
	pkg, name, _ := strings.Cut(strings.TrimLeft(fmt.Sprintf("%T", err), "*"), ".")
	if errors.As(err, &panicked) || pkg == "errors" || pkg == "fmt" {
		return "Exception"
	}
	return name
}

// Str is str(exc): an OSError as Python words it ("[Errno 2] No such file or directory:
// 'plan.json'", the program for one that would not start), anything else its message.
func Str(err error) string {
	var errno syscall.Errno
	switch e := err.(type) {
	case *fs.PathError:
		if errors.As(e.Err, &errno) {
			return py.ErrnoText(errno) + ": " + py.Repr(e.Path)
		}
	case *os.LinkError:
		if errors.As(e.Err, &errno) {
			return py.ErrnoText(errno) + ": " + py.Repr(e.Old) + " -> " + py.Repr(e.New)
		}
	case *os.SyscallError:
		if errors.As(e.Err, &errno) {
			return py.ErrnoText(errno)
		}
	case *exec.Error:
		if !errors.As(e.Err, &errno) {
			errno = syscall.ENOENT
		}
		return py.ErrnoText(errno) + ": " + py.Repr(e.Name)
	case syscall.Errno:
		return py.ErrnoText(e)
	}
	return err.Error()
}

// IsOSError is isinstance(exc, OSError) for the errors Go returns where Python raised one.
func IsOSError(err error) bool {
	var pathErr *fs.PathError
	var linkErr *os.LinkError
	var sysErr *os.SyscallError
	var execErr *exec.Error
	var errno syscall.Errno
	return errors.As(err, &pathErr) || errors.As(err, &linkErr) || errors.As(err, &sysErr) ||
		errors.As(err, &execErr) || errors.As(err, &errno)
}
