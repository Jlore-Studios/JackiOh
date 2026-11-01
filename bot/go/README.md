# The harness in Go (`bot/go/`)

The night bot's harness and Squishy's: one Go module, one binary (`harness`), standard library
only. It was `bot/harness/`, Python, until #467. The port changed the language and nothing the bot
does: the same subcommands, flags, exit codes and output, the same labels, markers, comments and
pinned issues, the same state, journal and plan files byte for byte. This file is the module's
contract (CLAUDE.md: each crate's and app's README): how it is laid out, how to build, run and test
it, the rules a change to it keeps, and why it is built the way it is.

The port is finished, and the Python is gone, so a change here answers to this file and the suite.
The port's own rules (`PORTING.md`: the Python's behaviour won any disagreement, and a quirk was
ported as a quirk, not fixed) and its list of every Python test beside its Go twin
(`tests/PARITY.md`) bound only the port; #2580 removed them, and the rules that still hold are
[below](#rules-for-a-change). `git show 5ffe4ecc9:bot/go/PORTING.md` and
`git show 5ffe4ecc9:bot/go/tests/PARITY.md` print them.

## Build, run, test

```sh
cd bot/go
go build -o "$HOME/.local/bin/harness" ./cmd/harness   # the binary (seconds; no dependency to fetch)
gofmt -l .                                             # prints nothing
go vet ./...
go test ./...                                          # the suite, Squishy's cases in a Squishy process
cd .. && harness --help                                # every command; run it from bot/ or anywhere in the checkout
```

The workflows get the binary from `.github/actions/harness`, which builds it once per version of
`bot/go/` and runner platform and caches it, so a job whose binary is cached sets up no
toolchain at all. `bot selftest` (a job of `ci.yml`, the required check) runs `gofmt`, `go vet`,
`go test`, then the binary under both bots' configs, actionlint, and the Python suites of what
stays Python (`bot/machine/`'s Lambdas and scripts, `.github/hygiene/`'s branch rule).

## Layout

| Path | What |
|---|---|
| `cmd/harness/` | `main`: the binary, a few lines around `internal/cli` |
| `internal/cli/` | the command line (`__main__.py`): every subcommand, the companion process, `$GITHUB_OUTPUT` and the step summary |
| `internal/<module>/` | one package per Python module, named after it (`queue`, `plan`, `work`, `deliver`, `dashboard`, …), except `herr` (`errors.py`), `hctx` (`context.py`) and `reviewrule` (`review_rule.py`) |
| `internal/pyjson/` | JSON as the Python held it, written byte for byte as `json.dumps` writes it |
| `internal/py/` | the Python behaviours the harness depends on, one tested copy each: text (`strip`, `split`, `lower`, slicing by code point, `splitlines`, `fnmatch`, re's Unicode classes), bytes as text, numbers (`round` to even, floor division, `sum`, the `%` format), the built-ins over JSON values (`int()`, `float()`, `x or default`, iteration) and Python's own exceptions |
| `internal/fakes/`, `internal/testsupport/` | the tests' in-memory GitHub and git helpers (`fakes.py`), and their configs, contexts and clocks (`support.py`) |
| `tests/` | the suite, black-box against `internal/`: `bot/tests/test_<file>.py` became `tests/<file>_test.go`, each Python test a Go test of the same name |

The packages keep the Python modules' boundaries, order and docstrings, so the bot's README and
issues that name a module (`dashboard.lanes_table`, `plan.peek`, `waiting.py`) still point at the
right code: `bot/harness/<module>.py`'s `name` is `bot/go/internal/<module>`'s `Name`.

## Design decisions

### Go, standard library only

#467 weighed Go against Rust and strict-typed Python. The harness is I/O and process glue (GitHub's
REST and GraphQL API, git, the model CLIs, JSON state, Markdown), not computation, and Go's standard
library covers all of it (`net/http`, `encoding/json`'s tokenizer, `os/exec`, `regexp`,
`text/template`'s cousins, `testing`). No third-party module means nothing to vet, pin or fetch,
and a cold build in seconds, which is what lets every job build its own binary when the cache
misses instead of depending on a release step.

### One package per Python module, generated stubs first

The port kept the Python's structure so it could be reviewed side by side and so the bot's docs
stay true. Before any body was ported, every module's surface (its public names, and the private
ones another module or a test reaches) was generated from the Python's type hints into
`internal/<pkg>/api.go` stubs that compiled together. With signatures fixed up front, each module
was ported in parallel against the same API and the pieces fitted when they met.

The Python had one import cycle (`config` → `providers` → `runner` → `config`, through a lazy import
of `runner.DEFAULT_PARK`). Go forbids cycles, so `DefaultPark` lives in `providers`, and `runner` uses
it from there.

### Ordered JSON, written as Python writes it (`pyjson`)

A Python dict keeps insertion order, and the harness depends on it: the state file is read, changed
and written back; dicts are iterated to draw the status issue; `json.dumps` output is hashed (the
sweep's label check) and written to the state branch (`indent=2, sort_keys=True`), the plans and
`result.json`. Go's maps have no order and `encoding/json` writes a different format (no spaces,
HTML-escaped, no `ensure_ascii`, its own float format). So every JSON value is a `pyjson` value:
`*pyjson.Object` keeps key order, integers stay `int` and floats `float64`, and `Dumps` writes
`json.dumps`'s bytes exactly, including `\uXXXX` escapes, separators and Python's float `repr`.
Its tests compare against output Python 3.13 wrote. The state branch therefore reads the same
whichever harness wrote it last, and the cut-over changed no file on it.

GitHub payloads stay dynamic (`*pyjson.Object`) rather than typed structs: the harness reads a few
fields of each and passes the rest through, and the fake GitHub builds them freely in tests, as the
Python did. Config, providers, plans and every dataclass are typed structs.

### Python's exceptions as typed panics (`herr`)

The Python raises `GitHubError`, `ConfigError` and the rest deep in a call and catches them several
frames up, in about two hundred `try`/`except` blocks. Threading `error` results through every
function between would have changed most signatures and made each `except` a judgement call. So a
raise is `herr.Raise(err)` (a panic carrying a typed `error`), `except GitHubError as e` is
`herr.Catch[*herr.GitHubError](…)`, and `except Exception` is `herr.Try(…)`: each `try`/`except`
of the Python is one call in the Go, and the control flow is the same. Go's own errors (a file that
will not open, a number that will not parse) are checked where they arise, as the Python's narrow
excepts were. The command line catches what is left exactly as `__main__.main` did, and an
uncaught error exits 1 (Python's code), not Go's 2. Goroutines standing for Python threads recover
their own panics, since a Python thread's exception never ended the process.

This is the pattern the standard library uses inside `encoding/json` and `text/template`: panics
within a component, turned into a result at its edge. Here the edge is the command line.

### Python's text and number rules (`py`)

Python slices strings by code point, rounds half to even, floors integer division, splits lines
at more characters than `\n` and counts more characters as whitespace; its int() reads any
script's digits and its `\w` any letter. Go does none of these by default. Every place the
harness measures, slices, rounds, strips or splits text, or reads a number out of JSON, uses
`py`'s twin of the Python call, one copy for the whole harness, each tested against Python 3.13's
own answers (`internal/py/testdata.json`). Regular expressions are Go's
RE2, which has no lookaround: the four Python patterns with one (`commands.py`'s mention and slash
matching, `runner.py`'s reset time, `triage.py`'s form answers) are matched in code with the same
results, and patterns over text people write keep Python's Unicode `\w` and `\b`.

### Test seams are package variables

Where a Python test replaced a module attribute with `mock.patch.object`, the Go package calls
that function through a package-level variable the test swaps and restores (`dashboard.Update`,
`stats.Collect`, `cli`'s `Ctx`, `Summary`, `Output`, its clock and subprocess runner, …). Nothing
else in the design exists for the tests.

### The suite is the oracle, ported test for test

#467's first milestone asked for language-neutral fixtures proven against the Python. The Python
suite was already that oracle: about a thousand tests over an in-memory GitHub, real git
repositories and fixed clocks, asserting on every write. Each was ported one to one, with the same
name, inputs and assertions, so the Go passed exactly the checks the Python passed (989 of 989).
`test_machine.py` and `test_hygiene.py` stayed Python, beside their Python subjects.

### Read the repository from disk, as the Python did

The Python found the repository two directories above its own file. A binary has no source file,
so the root is the first directory, from the working directory up, that holds
`.harness/config.json`; the workflows keep `working-directory: bot`. The prompts in `bot/prompts/`
are read from disk at run time as before, rather than embedded: `go:embed` cannot reach above
the module, and reading them keeps a prompt change a text change with no rebuild. Time zones load
from the embedded `time/tzdata`, so a runner without a zone database behaves the same.

### One binary per commit, cached, for every runner

`.github/actions/harness` restores the binary for this version of `bot/go/` and runner platform
from the Actions cache; on a miss it sets up the Go that `go.mod` pins, builds with
`CGO_ENABLED=0` (a static binary) and saves it. The bot's jobs used to spend a `setup-python` step
each; a cached binary needs no setup at all. The machines' self-hosted runners use the same action,
so `bot/machine/setup.sh` is unchanged: the box keeps Python for its training lanes and the
commands models run, but the harness no longer needs it.

### The whole migration in one pull request

#467 planned a shadow period and a cut-over per workflow. The issue's author chose the full
migration in one pull request instead. The Python harness and its tests are deleted in the same
change: keeping them one line away would mean making every harness change twice or letting the
rollback rot within days. Reverting the pull request is the rollback, restoring the Python and
the run lines together.

## What it gained

Measured on one machine with the same checkout, each command run 20 times, the output of each pair
identical:

| Command | Python 3.13 | Go |
|---|--:|--:|
| `harness --help` (start-up alone) | 208 ms | 15 ms |
| `harness providers --offline` (config, providers, the table) | 175 ms | 17 ms |

Every bot job also drops its `setup-python` step: a job whose binary is cached restores one 17 MB
file and sets up no toolchain, and every call of the harness gains the start-up (`bot-commands.yml`
runs on every comment and review; a night run calls it in each of its jobs). A cold miss (a new
version of `bot/go/` on a runner platform) costs `setup-go` and a build of a few tens of seconds,
once.

## Where it differs from the Python

The port kept every behaviour the suite checks, and every one it could see. These are the
differences it knowingly left, all outside what the bot writes to GitHub:

- **The command's name.** argparse's `prog` was `python -m harness` and is `harness`, the command
  a person types (usage lines and `error:` lines say `harness …`). `python -m harness.titles` is
  `harness titles`, and `pr-title.yml`'s inline Python is `harness pr-title`.
- **Error texts from Go's libraries.** Where the Python put an exception's own text into a
  message (a `config.json` that is not JSON, a plan file that will not parse, a socket error in a
  note), the text after the Python's prefix is Go's decoder's or network library's wording. An
  uncaught error prints its last line (`Type: message`) rather than a full Python traceback, and
  still exits 1.
- **Daylight-saving edges.** Python's aware datetimes do wall-clock arithmetic within one zone;
  Go's are instants. On the spring-forward night a wake set for 02:00, which does not exist, prints
  as 03:00 (the same instant), and in the autumn's repeated hour the status issue's timeline can
  place a bar an hour apart.
- **Unicode tables.** Go 1.24 has Unicode 15.0, Python 3.13 has 15.1: 622 newer CJK ideographs
  count as `\w` in Python and not in Go, which matters only to a slash command or mention written
  next to one.
- **Set order.** Where the Python walked a set (label names to drop, bundle excludes), whose order
  is a hash's and changed between processes, the Go walks it sorted.
- **Signals.** The model job's stop on SIGTERM raised inside whatever Python was doing; Go raises
  it at the job's next check. The model's own processes get the signal as before.
- **Values the Python crashed on.** A few inputs that raised in the Python (a superscript digit in
  a usage text, a list item that is not an object, NaN in `providers.json`) are skipped or refused
  with a message instead; integers beyond 64 bits overflow.
- **Help width.** argparse wrapped help to the terminal; the Go wraps it at 78 columns.

The port kept the Python bugs it found, each with a comment (for example `Context.window` reading
fields `Config` no longer has, and `plan.ranked` keeping the last index of a repeated name where
`builder_seat` keeps the first). With the Python gone they are bugs like any other: fix one in a
pull request of its own, with a test that shows the new behaviour.

## Rules for a change

These are the harness's own rules, not a port's: a change may fix a bug or change what the bot
does, with a test that shows it. The rules hold the formats the bot's state branches, comments,
markers and plans already hold, which its later runs and the Python that stays (`bot/machine/`,
`.github/hygiene/branches.py`) read back, and keep the code in one style:

- What something reads back stays the same byte for byte unless the change means it to differ:
  the state, journal and plan files, `result.json`, the `<!-- … -->` markers and labels, and the
  `$GITHUB_OUTPUT` lines the workflows read. A change to one still reads what is already written,
  and changes its other readers in the same pull request.
- Write and hash JSON only through `pyjson`; never `encoding/json`'s `Marshal`.
- Raise with `herr.Raise`, catch with `herr.Catch` or `herr.Try`; a goroutine recovers its own.
- Measure and slice text with `py.Len` and `py.Slice`, strip and split it with `py.Strip` and
  `py.Split` (not `strings`'), round with `py.Round`, sort stably.
- Read a JSON value with `py`'s built-ins (`py.Int`, `py.Float`, `py.Or`, `py.Each`, …), raise
  Python's own exceptions as `*py.Error`, and name what an except caught with `herr.Name` and
  `herr.Str`. A Python behaviour `py` lacks goes into `py` with a test against Python 3.13, not
  into a package's private copy.
- Only `config` and `identity` read the environment.
- A new test seam is a package variable the package calls through.
- Names are Go's, with these words in capitals: id, url, json, pr, ci, api, sha, http, cli, iso,
  utc, ui, pid, tty, gh, html, uid, ok, usd, sql, aws, vm, pc, ai (`ParseISO`, `PRURL`). A test is
  `Test<File>_<Name>` in `tests/<file>_test.go` (`TestEvents_CILeavesAStoppedOrBlockedPRAlone`),
  black-box against `internal/`, with `internal/fakes` and `internal/testsupport`.
- Before a change is done, `gofmt -l .` prints nothing and `go vet ./...` and `go test ./...` pass.
- The bots may not change `bot/` (their `forbidden_paths`): a change here is a person's.
