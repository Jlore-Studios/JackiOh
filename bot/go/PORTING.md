# Porting the harness to Go (#467)

The rules every port of `bot/harness/<module>.py` to `bot/go/internal/<pkg>/` follows. The goal is
the same bot in another language: no behaviour changes, no new features, no clean-ups that change
what a run does. When a rule here and the Python disagree, the Python's behaviour wins.

## 1. Faithful first

- **Same behaviour, byte for byte where it is text.** Every string the harness writes (comments,
  issue and pull request bodies, Markdown, labels, `<!-- … -->` markers, commit messages, log and
  error lines, `print` output, `$GITHUB_OUTPUT` and step-summary lines) is the Python's exactly.
  So are the order of GitHub calls, retries, sleeps, timeouts, exit codes and file formats.
- **Same structure.** One Go package per Python module, the functions in the Python's order, each
  with its Python docstring and `#:` comments as Go comments (the README links to them). A reviewer
  reads the two side by side.
- **No improvements.** A Python quirk is ported as a quirk, with a comment if it looks like a bug.
  Report it rather than fixing it.

## 2. Layout and names

- `bot/harness/<module>.py` is `bot/go/internal/<module>/`, except `errors.py` → `herr`,
  `context.py` → `hctx`, `review_rule.py` → `reviewrule`, `__main__.py` → `cli` (the command line,
  with `bot/go/cmd/harness/main.go` a few lines around it). The one import cycle in the Python
  (`providers` imports `runner.DEFAULT_PARK` lazily) is broken by defining `DefaultPark` in
  `providers` and having `runner` use it.
- `internal/<pkg>/api.go` is the generated stub of the module's surface: every public name, every
  private name another module or a test uses, with the Python's signature mapped to Go types.
  Replace it with the real code (split into files by the module's own sections if it is long) and
  delete `api.go` when the package is done. **Keep the exported signatures** unless one is wrong;
  narrowing an `any` to the concrete type is right. List every exported signature you change in
  your report, since other packages were written against the stub.
- Names: `snake_case` → `CamelCase` (exported); `_private` → `lowerCamel` (unexported) unless
  another module or a test names it, then exported. These words are written in capitals:
  id, url, json, pr, ci, api, sha, http, cli, iso, utc, ui, pid, tty, gh, html, uid, ok, usd, sql,
  aws, vm, pc, ai (`parse_iso` → `ParseISO`, `pr_url` → `PRURL`). `UPPER_CASE` constants →
  `CamelCase` vars (`MAX_ATTEMPTS` → `MaxAttempts`). Classes → structs; dataclass fields →
  exported fields; methods → methods on the pointer; `__init__` → `NewX(...)`; a classmethod or
  staticmethod → a function `XName`; a property → a method. `providers.SECRETS` is `SecretNames`
  (the class `Secrets` keeps its name).
- Python default arguments: Go has none. The caller passes every argument; the stub notes each
  default as `/* = default */` so callers pass the same value. A dataclass whose defaults are not
  Go's zero values gets a `NewX()` constructor that sets them, and every construction goes through
  it.

## 3. Values

- JSON from GitHub, the state file, plans and every `dict[str, Any]` is `*pyjson.Object` (keys in
  insertion order, as a Python dict), lists are `[]any`, numbers are `int` or `float64`.
  `pyjson.Loads` and `pyjson.Dumps`/`DumpsIndent`/`DumpsWith` are `json.loads` and `json.dumps`
  byte for byte (`ensure_ascii`, separators, `sort_keys`, float repr). Never use `encoding/json`'s
  Marshal for anything the harness writes or hashes.
- Python truthiness: `pyjson.Truthy(v)`, `o.Truthy("key")`. `d.get("k")` is `o.Get("k")`;
  `d.get("k") or {}` is `o.Obj("k")` (a nil object reads as empty); `str(v)` is `py.Str(v)`.
- `dict[str, str]` (environments) is `map[string]string`; sets are `map[T]bool`. A Go map has no
  order: wherever the Python iterates a dict or set and the order shows (output, GitHub writes,
  "the first"), keep an order: a `*pyjson.Object`, a slice, or sort as the Python does.
- Strings are indexed and measured by code point in Python: `py.Len`, `py.Slice`, `py.SliceTo`,
  `py.SliceFrom`, `py.Ljust`, `py.Rjust`, never `s[:n]` or `len(s)` on text that may hold
  non-ASCII. `str.splitlines()` is `py.Splitlines`; `str.split()` is `strings.Fields`;
  `str.strip()` is `strings.TrimSpace`.
- Numbers: `round()` is `py.Round` (half to even), `round(x, n)` is `py.RoundN`, `//` and `%` on
  ints are `py.FloorDiv` and `py.Mod`, `f"{n:,}"` is `py.Thousands`, `repr(float)` and a float in an
  f-string is `pyjson.FloatRepr`. `f"{x:.1f}"` is `fmt.Sprintf("%.1f", x)` (both round correctly).
- Sorting: Python's sort is stable: always `sort.SliceStable` / `slices.SortStableFunc`. `min` and
  `max` with a key return the first of the equal ones.
- `datetime` is `time.Time` (in UTC unless the Python localised it), `timedelta` is
  `time.Duration`, `datetime.time` is `clock.TimeOfDay`. `fnmatch.fnmatch` is `py.Fnmatch`.
  `html.escape` writes `&#x27;` for `'` (Go's `html.EscapeString` writes `&#39;`).
  `urllib.parse.quote` escapes differently from Go's `url.PathEscape`: write the Python's rule.

## 4. Exceptions

`herr` holds `errors.py`'s classes and Python's raise-and-catch:

- `raise GitHubError(msg, status)` → `herr.Raise(&herr.GitHubError{Msg: msg, Status: status})`.
- `try: … except GitHubError as e: …` →
  `if e, ok := herr.Catch[*herr.GitHubError](func() { … }); ok { … }`.
- `except HarnessError` → `herr.CatchHarness`; `except Exception` → `err := herr.Try(func() {…})`.
- `finally` → `defer` inside the closure.
- Python's own exceptions (`ValueError` from `int()`, `OSError`, `json.JSONDecodeError`,
  `KeyError` caught on purpose) become Go's error checks where they arise; nothing else changes.
- A goroutine that stands for a Python thread recovers its own panics: an exception in a Python
  thread does not end the process.
- The command line catches what is left exactly as `__main__.main` does (the same message, the
  same exit code; an uncaught exception exits 1 as Python's does, not Go's 2).

## 5. The outside world

- Only `config` and `identity` read the environment (`os.Getenv`, `os.Environ`), as in Python.
- The repository root (Python's `Path(__file__).parents[2]`) is the first directory, from the
  working directory up, that holds `.harness/config.json`; `bot/prompts/` is under it. Prompts are
  read from disk at run time as the Python does (an `embed` cannot reach `bot/prompts/` from
  `bot/go/`, and the workflows run from a checkout).
- Time zones load with `time.LoadLocation`; `cmd/harness` and the tests import `time/tzdata` so a
  runner without a zone database behaves the same.
- Subprocesses go through `os/exec` with the Python's arguments, environment, timeout and text
  handling. HTTP goes through `net/http` with the Python's headers, retries and backoff.

## 6. Test seams

Python tests replace module attributes with `mock.patch.object`. Where a test does that, the Go
package exposes the name as a package-level variable that the package itself calls through, so a
test can swap it and restore it with `t.Cleanup`:

```go
// Update is dashboard.update; a variable so the tests can replace it.
var Update = update

func update(ctx *hctx.Context, ...) { ... }
```

The attributes the Python suite patches: `dashboard.update`, `dashboard.refresh_runners`,
`dashboard.MAX_BODY`, `dashboard.TRAINING_LANES`, `dashboard.BOX_SLOTS`, `dashboard.BOXES_NOTE`,
`stats.collect`, `disk.clean`, `runner.WATCH_SECONDS`, `runner.POLL_SECONDS`, `controller.deliver`,
`controller`'s `time.sleep`, `sweep._night_run`, `plan.take_planning_lock`, `plan.OTHERS`, and in
`__main__`: `_ctx`, `_summary`, `_output`, `LOCK_TRIES`, `time.sleep`, `time.monotonic`,
`subprocess.run`, `sweep.sweep` and `config.github_env`. `gh`'s HTTP opener is replaceable too
(`urllib.request.urlopen` and `build_opener` in the Python's tests).

## 7. Tests

`bot/tests/test_<file>.py` becomes `bot/go/tests/<file>_test.go` (package `tests`, black-box, with
`internal/fakes` for `fakes.py` and `internal/testsupport` for `support.py`). Each Python test
`test_<name>` becomes `func Test<File>_<Name>` (`test_flow.py`'s `test_build_opens_pr` →
`TestFlow_BuildOpensPR`, the same capitals as §2); when two classes in one file share a method
name, the class goes between: `TestProviders_<Class>_<Name>`. `squishy_cases.py` is
`squishy_cases_test.go`, which runs only as Squishy (`HARNESS_HOME=.squishy`) and is started that
way by the test that stands for `SquishyProcessTests`. `tests/PARITY.md` lists every Python test
beside its Go twin, and the port is not done while one has none.

`test_machine.py` and `test_hygiene.py` stay in Python: their subjects (`bot/machine/starter.py`,
`credits.py`, the shell scripts, `.github/hygiene/branches.py`) are not the harness and stay as
they are.

## 8. Checks

Before you report a package done: `gofmt -l .` prints nothing, `go vet ./...` and
`go build ./...` pass in `bot/go/`, and the package has no `unported` panic left. A focused Go
test for tricky pure logic (parsers, formatters, regexes) is welcome; the full suite is ported
separately.
