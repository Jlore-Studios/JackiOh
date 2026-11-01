// Package tests is bot/tests/, ported (#467): one <file>_test.go for each test_<file>.py,
// black-box against internal/, with internal/fakes for fakes.py and internal/testsupport for
// support.py. Each Python test became a Go test of the same name (README.md, Rules for a change).
package tests

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"

	_ "time/tzdata" // a runner without a zone database behaves the same (README.md, Read the repository from disk)
)

// TestMain sets up what the Python suite had for every test: it ran from bot/ (ci.yml's bot
// selftest runs `python3 -m unittest discover -s tests -t .` with `working-directory: bot`, as
// test_squishy runs its Squishy process with cwd=BOT_DIR), so a path the harness or a test leaves
// relative resolves where it did. Nothing else was global: bot/tests/__init__.py is empty, and the
// one setUpModule, squishy_cases.py's (skip unless the process is Squishy, HARNESS_HOME=.squishy),
// belongs to squishy_cases_test.go's tests.
func TestMain(m *testing.M) {
	// `go test` starts in the package's directory, bot/go/tests.
	bot, err := filepath.Abs(filepath.Join("..", ".."))
	if err == nil {
		err = os.Chdir(bot)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "tests: cannot work from bot/:", err)
		os.Exit(1)
	}
	os.Exit(m.Run())
}
