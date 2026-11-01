package py

import (
	"fmt"
	"strings"
	"syscall"
)

// ErrnoText is str(OSError(errno, os.strerror(errno))) with no file name: "[Errno 2] No such
// file or directory" (Go's text of an errno is C's, but for its capital).
func ErrnoText(errno syscall.Errno) string {
	text := errno.Error()
	if text != "" {
		text = strings.ToUpper(text[:1]) + text[1:]
	}
	return fmt.Sprintf("[Errno %d] %s", int(errno), text)
}

// OSErrorName is the OSError subclass Python raises for errno (PEP 3151), "OSError" for the rest.
func OSErrorName(errno syscall.Errno) string {
	switch errno {
	case syscall.EAGAIN, syscall.EALREADY, syscall.EINPROGRESS:
		return "BlockingIOError"
	case syscall.ECHILD:
		return "ChildProcessError"
	case syscall.EPIPE, syscall.ESHUTDOWN:
		return "BrokenPipeError"
	case syscall.ECONNABORTED:
		return "ConnectionAbortedError"
	case syscall.ECONNREFUSED:
		return "ConnectionRefusedError"
	case syscall.ECONNRESET:
		return "ConnectionResetError"
	case syscall.EEXIST:
		return "FileExistsError"
	case syscall.ENOENT:
		return "FileNotFoundError"
	case syscall.EISDIR:
		return "IsADirectoryError"
	case syscall.ENOTDIR:
		return "NotADirectoryError"
	case syscall.EINTR:
		return "InterruptedError"
	case syscall.EACCES, syscall.EPERM:
		return "PermissionError"
	case syscall.ESRCH:
		return "ProcessLookupError"
	case syscall.ETIMEDOUT:
		return "TimeoutError"
	}
	return "OSError"
}
