package resolve

// Mergiraf, the syntax-aware merge driver (#2948, section 3).
//
// The repository's root `.gitattributes` gives TypeScript, TSX, JavaScript, JSON and YAML files
// `merge=mergiraf`, lockfiles and generated files excepted. Git runs a driver of that name only
// where one is configured: a clone with none (a person's) merges those files with git's own text
// merge, as before. The bot's work job registers it in a worktree it is about to merge `main` into,
// once a test-merge shows a conflict in a file the driver takes (`Takes`), so a run with no such
// conflict spends nothing on it:
//
//   - the binary is the pinned release (`MergirafVersion`): the one on the job's PATH when it is that
//     version, else the one an earlier job built in the machine user's cache (`CacheDir`), else built
//     now from that release's source on crates.io, its checksum held to `MergirafSHA256` (cargo then
//     holds every dependency to the release's `Cargo.lock`). GitHub's runners and the bot's machine
//     get it the same way, since both run the harness's work job; the machine keeps the build in each
//     user's cache for its later jobs.
//   - `Register` puts it in that worktree's own config (`git config --worktree`, which goes with the
//     worktree), never the checkout's, so no later job on the machine finds a driver it did not set.
//   - anything that fails (no download, a checksum that does not match, a build that fails or runs out
//     of time, a config git refuses) is logged in the run's steps, and the merge goes on with git's
//     own, whose conflicts the resolvers and then a model settle as before.
//
// Mergiraf exits non-zero on a conflict it cannot settle and leaves conflict markers then, which git
// counts as a conflict like its own: the rest of the run treats them alike.

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// MergirafVersion is the release the work job uses, and MergirafSHA256 the SHA-256 of its source
// (`MergirafSource`, the `.crate` crates.io serves, whose checksum its index records as `cksum`).
var (
	MergirafVersion = "0.20.0"
	MergirafSource  = "https://static.crates.io/crates/mergiraf/mergiraf-" + MergirafVersion + ".crate"
	MergirafSHA256  = "4a7be1108bbb7d636476c790bb512f80351433fabbc481fefe78a2e36eaacb0e"
)

// MergirafName is the driver's name in `.gitattributes` (`merge=mergiraf`).
var MergirafName = "mergiraf"

// MergirafArgs is the driver's command line after the binary, as Mergiraf documents it for git.
var MergirafArgs = "merge --git %O %A %B -s %S -x %X -y %Y -p %P -l %L"

// MergirafInstallMinutes is the most a build of the pinned release may take; MergirafSourceBytes the
// most of its source the job downloads; MergirafCheckSeconds how long `mergiraf --version` may take.
var (
	MergirafInstallMinutes = 20
	MergirafSourceBytes    = 64 << 20
	MergirafCheckSeconds   = 30
)

// CacheDir is where the pinned release is built for the machine user `env` runs as (its HOME), or
// the job's scratch directory (`fallback`) when it has none.
func CacheDir(env map[string]string, fallback string) string {
	home := env["HOME"]
	if home == "" {
		home = fallback
	}
	return filepath.Join(home, ".cache", "jackioh-harness", "mergiraf-"+MergirafVersion)
}

// FindMergiraf is the pinned Mergiraf's binary for `env`: on its PATH when that one is the pinned
// version, else the one built in `cache` (`CacheDir`), or "" and why there is none. A variable so the
// tests can stand a fake driver in.
var FindMergiraf = findMergiraf

func findMergiraf(env map[string]string, cache string) (string, string) {
	for _, dir := range filepath.SplitList(env["PATH"]) {
		if dir == "" {
			continue
		}
		if bin := filepath.Join(dir, MergirafName); isPinned(bin, env) {
			return bin, ""
		}
	}
	if bin := filepath.Join(cache, "bin", MergirafName); isPinned(bin, env) {
		return bin, ""
	}
	return "", fmt.Sprintf("Mergiraf %s is not on PATH nor built in %s", MergirafVersion, cache)
}

// isPinned is: `bin` runs and says it is the pinned version (`mergiraf --version`).
func isPinned(bin string, env map[string]string) bool {
	if info, err := os.Stat(bin); err != nil || info.IsDir() {
		return false
	}
	out, err := command(bin, []string{"--version"}, "", env, time.Duration(MergirafCheckSeconds)*time.Second)
	return err == nil && strings.Contains(out, " "+MergirafVersion)
}

// InstallMergiraf builds the pinned release into `cache` and returns its binary, or "" and why not.
// It gives up past `seconds`. A variable so the tests never download or build.
var InstallMergiraf = installMergiraf

func installMergiraf(env map[string]string, cache string, seconds int) (string, string) {
	limit := min(seconds, MergirafInstallMinutes*60)
	if limit <= 0 {
		return "", "the run has no time left to build it"
	}
	deadline := time.Now().Add(time.Duration(limit) * time.Second)
	crate, why := download(MergirafSource, time.Until(deadline))
	if why != "" {
		return "", why
	}
	sum := sha256.Sum256(crate)
	if got := hex.EncodeToString(sum[:]); got != MergirafSHA256 {
		return "", fmt.Sprintf("its source's checksum is %s, not the pinned %s", got, MergirafSHA256)
	}
	src := filepath.Join(cache, "src")
	if err := os.RemoveAll(src); err != nil {
		return "", "could not clear " + src + ": " + err.Error()
	}
	if err := untar(crate, src); err != nil {
		return "", "could not unpack its source: " + err.Error()
	}
	build := map[string]string{}
	for k, v := range env {
		build[k] = v
	}
	build["CARGO_TARGET_DIR"] = filepath.Join(cache, "target")
	build["CARGO_INCREMENTAL"] = "0"
	out, err := command("cargo", []string{"install", "--locked", "--quiet", "--path",
		filepath.Join(src, "mergiraf-"+MergirafVersion), "--root", cache}, cache, build, time.Until(deadline))
	_ = os.RemoveAll(filepath.Join(cache, "target")) // the binary is all a later job needs
	if err != nil {
		return "", fmt.Sprintf("`cargo install` failed (%s): %s", err.Error(), py.SliceFrom(py.Strip(out), -300))
	}
	bin := filepath.Join(cache, "bin", MergirafName)
	if !isPinned(bin, env) {
		return "", "the build left no working " + bin
	}
	return bin, ""
}

// download is the body at `url`, read within `limit`, or why not.
func download(url string, limit time.Duration) ([]byte, string) {
	if limit <= 0 {
		return nil, "no time left to download it"
	}
	client := &http.Client{Timeout: limit}
	response, err := client.Get(url)
	if err != nil {
		return nil, "could not download " + url + ": " + err.Error()
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return nil, fmt.Sprintf("could not download %s: %s", url, response.Status)
	}
	body, err := io.ReadAll(io.LimitReader(response.Body, int64(MergirafSourceBytes)+1))
	if err != nil {
		return nil, "could not download " + url + ": " + err.Error()
	}
	if len(body) > MergirafSourceBytes {
		return nil, fmt.Sprintf("%s is larger than %d bytes", url, MergirafSourceBytes)
	}
	return body, ""
}

// untar unpacks a gzipped tarball's files and directories into `dir`, refusing any entry that would
// land outside it.
func untar(data []byte, dir string) error {
	zipped, err := gzip.NewReader(bytes.NewReader(data))
	if err != nil {
		return err
	}
	archive := tar.NewReader(zipped)
	root := filepath.Clean(dir) + string(filepath.Separator)
	for {
		header, err := archive.Next()
		if errors.Is(err, io.EOF) {
			return nil
		}
		if err != nil {
			return err
		}
		target := filepath.Join(dir, header.Name)
		if !strings.HasPrefix(target, root) {
			return fmt.Errorf("%s is outside the archive", header.Name)
		}
		switch header.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, 0o755); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return err
			}
			file, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(header.Mode)&0o755|0o644)
			if err != nil {
				return err
			}
			_, copyErr := io.Copy(file, archive)
			if closeErr := file.Close(); copyErr == nil {
				copyErr = closeErr
			}
			if copyErr != nil {
				return copyErr
			}
		}
	}
}

// command runs `name args...` in `cwd` with exactly `env`, for at most `limit`: its output, and an
// error when it could not start, failed or ran out of time.
func command(name string, args []string, cwd string, env map[string]string, limit time.Duration) (string, error) {
	if limit <= 0 {
		return "", errors.New("no time left")
	}
	ctx, cancel := context.WithTimeout(context.Background(), limit)
	defer cancel()
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = cwd
	cmd.Env = []string{}
	for k, v := range env {
		cmd.Env = append(cmd.Env, k+"="+v)
	}
	if path := env["PATH"]; path != "" && !strings.Contains(name, "/") {
		// exec looks a bare name up on this process's PATH: look it up on the job's instead.
		for _, dir := range filepath.SplitList(path) {
			if found := filepath.Join(dir, name); dir != "" && isExecutable(found) {
				cmd.Path = found
				break
			}
		}
	}
	cmd.WaitDelay = time.Second
	out, err := cmd.CombinedOutput()
	if ctx.Err() == context.DeadlineExceeded {
		return string(out), fmt.Errorf("it ran past %s", limit.Round(time.Second))
	}
	return string(out), err
}

// isExecutable is: `path` is a file someone may run.
func isExecutable(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir() && info.Mode()&0o111 != 0
}

// Takes is the `paths` whose merge driver in `wt` is Mergiraf (`git check-attr merge`, the root
// `.gitattributes`), in their order.
func Takes(wt *git.Git, paths []string) []string {
	found := []string{}
	if len(paths) == 0 {
		return found
	}
	proc := wt.Run(false, 600, nil, append([]string{"check-attr", "merge", "--"}, paths...)...)
	if proc.Returncode != 0 {
		return found
	}
	for _, line := range py.Splitlines(proc.Stdout) {
		path, value, ok := strings.Cut(line, ": merge: ")
		if ok && py.Strip(value) == MergirafName {
			found = append(found, path)
		}
	}
	return found
}

// DriverCommand is the `merge.mergiraf.driver` line for `bin`, as Mergiraf documents it.
func DriverCommand(bin string) string {
	return "'" + strings.ReplaceAll(bin, "'", `'\''`) + "' " + MergirafArgs
}

// Register makes `bin` the Mergiraf merge driver of worktree `wt` alone, in that worktree's own
// config (`extensions.worktreeConfig` on `repo`, the checkout it was added to). "" when git took it,
// else why not; then nothing of it is left, and the merge is git's own.
func Register(repo, wt *git.Git, bin string) string {
	steps := [][]string{
		{"config", "extensions.worktreeConfig", "true"},
		{"config", "--worktree", "merge." + MergirafName + ".name", "Mergiraf " + MergirafVersion},
		{"config", "--worktree", "merge." + MergirafName + ".driver", DriverCommand(bin)},
	}
	for i, args := range steps {
		on := wt
		if i == 0 {
			on = repo
		}
		if proc := on.Run(false, 600, nil, args...); proc.Returncode != 0 {
			Unregister(wt)
			return fmt.Sprintf("git would not take `%s` (%s)", strings.Join(args, " "),
				py.SliceTo(py.Strip(proc.Stderr), 200))
		}
	}
	if wt.Out(false, "config", "--get", "merge."+MergirafName+".driver") != DriverCommand(bin) {
		Unregister(wt)
		return "the driver did not read back from the worktree's config"
	}
	return ""
}

// Unregister takes the driver out of worktree `wt`'s own config.
func Unregister(wt *git.Git) {
	wt.Run(false, 600, nil, "config", "--worktree", "--remove-section", "merge."+MergirafName)
}

// Mergiraf registers the pinned Mergiraf as worktree `wt`'s merge driver, finding it or building it
// first (`FindMergiraf`, `InstallMergiraf`, within `seconds`). Returns what it did, for the run's
// steps, and whether the driver is in place; on false the merge is git's own.
func Mergiraf(repo, wt *git.Git, env map[string]string, scratch string, seconds int) (string, bool) {
	cache := CacheDir(env, scratch)
	bin, missing := FindMergiraf(env, cache)
	how := "found `" + bin + "`"
	if bin == "" {
		var why string
		if bin, why = InstallMergiraf(env, cache, seconds); bin == "" {
			return fmt.Sprintf("not used: %s; and it could not be built: %s. Git's own merge instead",
				missing, why), false
		}
		how = "built `" + bin + "`"
	}
	if why := Register(repo, wt, bin); why != "" {
		return fmt.Sprintf("not used: %s. Git's own merge instead", why), false
	}
	return fmt.Sprintf("Mergiraf %s is the merge driver here (%s)", MergirafVersion, how), true
}
