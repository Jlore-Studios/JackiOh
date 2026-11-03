// mergiraf_cache_test.go: #2948 section 3. Mergiraf, the merge driver a conflict's merge uses, is
// built once from main by bot-cache.yml into its own cache entry, keyed on the version
// `resolve/mergiraf.go` pins, and the model jobs only restore it, as they do the Rust cache: nothing
// a model ran reaches a later job through it. `harness mergiraf` says the entry's version and
// directory, and builds it with `--install`.
package tests

import (
	"path/filepath"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/resolve"
)

// mergirafCacheKey is the entry's key, alike in the job that saves it and the jobs that restore it.
const mergirafCacheKey = "key: mergiraf-${{ runner.os }}-${{ runner.arch }}-${{ steps.mergiraf.outputs.version }}"

// mergirafCachePath is the entry's path, `harness mergiraf`'s `dir=` line's.
const mergirafCachePath = "path: ${{ steps.mergiraf.outputs.dir }}/bin"

func TestMergirafCache_MainAloneBuildsAndSavesIt(t *testing.T) {
	cache := workflowsRead(t, "bot-cache.yml")
	job := workflowsJob(t, cache, "mergiraf")
	assertIn(t, "bot/go/internal/resolve/mergiraf.go", strings.SplitN(cache, "\njobs:", 2)[0]) // a new pin builds anew
	assertIn(t, "run: harness mergiraf | tee -a \"$GITHUB_OUTPUT\"", job)
	assertIn(t, mergirafCacheKey, job)
	assertIn(t, mergirafCachePath, job)
	assertIn(t, "lookup-only: true", workflowsStep(t, job, "uses: actions/cache/restore@v4"))
	build := workflowsStep(t, job, "name: Build it from its pinned source")
	assertIn(t, "if: steps.cache.outputs.cache-hit != 'true'", build)
	assertIn(t, "run: harness mergiraf --install", build)
	save := workflowsStep(t, job, "uses: actions/cache/save@v4")
	assertIn(t, "if: steps.cache.outputs.cache-hit != 'true' && github.ref == 'refs/heads/main'", save)
	assertIn(t, mergirafCachePath, save)
	assertLess(t, strIndex(t, job, "name: Build it from its pinned source"), strIndex(t, job, "uses: actions/cache/save@v4"))
	assertTrue(t, workflowsLinted(t, "bot-cache.yml"))
}

// A model ran in those jobs: they restore the entry and never save it.
func TestMergirafCache_TheModelJobsRestoreItAndNeverSave(t *testing.T) {
	night := workflowsJob(t, workflowsNight(t), "work")
	squishy := workflowsJob(t, workflowsRead(t, "squishy-run.yml"), "work")
	for name, work := range map[string]string{"bot-night": night, "squishy-run": squishy} {
		assertIn(t, mergirafCacheKey, work, name)
		assertIn(t, mergirafCachePath, work, name)
		assertIn(t, "run: harness mergiraf | tee -a \"$GITHUB_OUTPUT\"", workflowsStep(t, work, "name: The pinned Mergiraf"), name)
		assertNotRegex(t, work, `uses: actions/cache(/save)?@`)
		assertNotIn(t, "harness mergiraf --install", work, name) // a miss builds it in the run, when a merge needs it
		assertLess(t, strIndex(t, work, "name: The pinned Mergiraf"), strIndex(t, work, "secrets["), name)
	}
	// On the machine each user's build stays in that user's cache: GitHub's runners alone restore it.
	assertIn(t, "if: startsWith(env.PLAN_RUNS_ON, 'ubuntu-')", workflowsStep(t, night, "name: The pinned Mergiraf"))
}

// `harness mergiraf` says the version and the directory, for GITHUB_OUTPUT.
func TestMergirafCache_TheCommandSaysTheVersionAndTheDirectory(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	out, _ := capture(t, &cli.Stdout)
	assertEqual(t, cli.Main([]string{"mergiraf"}), 0)
	dir := filepath.Join(home, ".cache", "jackioh-harness", "mergiraf-"+resolve.MergirafVersion)
	assertEqual(t, out.String(), "version="+resolve.MergirafVersion+"\ndir="+dir+"\n")
}

// `--install` finds it there, or builds it; it fails when it can do neither.
func TestMergirafCache_InstallFindsOrBuildsIt(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	dir := filepath.Join(home, ".cache", "jackioh-harness", "mergiraf-"+resolve.MergirafVersion)
	find, install := resolve.FindMergiraf, resolve.InstallMergiraf
	t.Cleanup(func() { resolve.FindMergiraf, resolve.InstallMergiraf = find, install })
	var built []string
	resolve.FindMergiraf = func(env map[string]string, cache string) (string, string) { return "", "not here" }
	resolve.InstallMergiraf = func(env map[string]string, cache string, seconds int) (string, string) {
		built = append(built, cache)
		assertEqual(t, seconds, resolve.MergirafInstallMinutes*60)
		return filepath.Join(cache, "bin", "mergiraf"), ""
	}
	out, _ := capture(t, &cli.Stdout)
	assertEqual(t, cli.Main([]string{"mergiraf", "--install"}), 0)
	assertEqual(t, built, []string{dir})
	assertIn(t, "bin="+filepath.Join(dir, "bin", "mergiraf")+"\n", out.String())

	resolve.FindMergiraf = func(env map[string]string, cache string) (string, string) {
		return filepath.Join(cache, "bin", "mergiraf"), ""
	}
	built = nil
	assertEqual(t, cli.Main([]string{"mergiraf", "--install"}), 0)
	assertEqual(t, len(built), 0) // found: nothing is built

	resolve.FindMergiraf = func(env map[string]string, cache string) (string, string) { return "", "not here" }
	resolve.InstallMergiraf = func(env map[string]string, cache string, seconds int) (string, string) {
		return "", "no network here"
	}
	errs, _ := capture(t, &cli.Stderr)
	assertEqual(t, cli.Main([]string{"mergiraf", "--install"}), 1)
	assertIn(t, "not here; and it could not be built: no network here", errs.String())
}
