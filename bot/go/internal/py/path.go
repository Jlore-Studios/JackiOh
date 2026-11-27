package py

import (
	"errors"
	"os"
	"strings"
	"syscall"
)

// PurePath is str(PurePosixPath(*parts)): joined with "/", empty and "." parts dropped, ".."
// kept, an absolute part starting afresh (two leading slashes kept, as POSIX allows).
func PurePath(parts ...string) string {
	root := ""
	var segments []string
	for _, part := range parts {
		if strings.HasPrefix(part, "/") {
			segments = nil
			root = "/"
			if strings.HasPrefix(part, "//") && !strings.HasPrefix(part, "///") {
				root = "//"
			}
		}
		for _, s := range strings.Split(part, "/") {
			if s != "" && s != "." {
				segments = append(segments, s)
			}
		}
	}
	if path := root + strings.Join(segments, "/"); path != "" {
		return path
	}
	return "."
}

// ExecPath is the program subprocess.Popen runs for name: name itself when it names a path, else the
// first executable of that name on the PATH of the child's own environment (os.get_exec_path(env)),
// or the errno Popen's FileNotFoundError or PermissionError carries when there is none.
func ExecPath(name string, env map[string]string) (string, syscall.Errno) {
	if strings.Contains(name, "/") {
		return name, 0
	}
	path, ok := env["PATH"]
	if !ok {
		path = "/bin:/usr/bin"
	}
	saved := syscall.Errno(0)
	for _, dir := range strings.Split(path, ":") {
		candidate := name
		if dir != "" {
			candidate = strings.TrimSuffix(dir, "/") + "/" + name
		}
		info, err := os.Stat(candidate)
		if err != nil {
			var errno syscall.Errno
			if errors.As(err, &errno) && errno != syscall.ENOENT && errno != syscall.ENOTDIR && saved == 0 {
				saved = errno
			}
			continue
		}
		if info.Mode().IsRegular() && info.Mode()&0o111 != 0 {
			return candidate, 0
		}
		if saved == 0 {
			saved = syscall.EACCES
		}
	}
	if saved == 0 {
		saved = syscall.ENOENT
	}
	return "", saved
}
