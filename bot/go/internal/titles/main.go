package titles

// The command line, `harness titles shipped|retcon …` (patches-ship.yml,
// title-retcon.yml): `main`, with what its argparse parser does (usage, help, errors and exit
// codes, as Python 3.13's prints them at 80 columns, as on a runner).

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"math"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// stdout and stderr are where `main` prints; variables so the tests can read them.
var (
	stdout io.Writer = os.Stdout
	stderr io.Writer = os.Stderr
)

// Main is titles.main: the exit code. `argv` nil is None (the process's arguments).
func Main(argv []string /* = None */) (code int) {
	if argv == nil {
		argv = os.Args[1:]
	}
	args, exit, ok := parseArgs(argv)
	if !ok {
		return exit
	}
	defer func() {
		if r := recover(); r != nil {
			// An uncaught exception: Python prints its traceback, whose last line this is, and
			// exits 1.
			fmt.Fprintln(stderr, exceptionLine(r))
			code = 1
		}
	}()
	client := gh.NewGitHub(environ("GITHUB_REPOSITORY"), environ("GH_TOKEN"), false, nil, nil, "")
	if args.command == "retcon" {
		threads := client.Paginate("repos/"+client.Repo+"/issues", pyjson.New("state", "all"), "", 100_000, false)
		where := joinPath(py.Strip(runGit("rev-parse", "--show-toplevel")), PatchesJSON)
		data, err := os.ReadFile(where)
		if err != nil {
			herr.Raise(err)
		}
		patches, err := pyjson.Loads(data)
		if err != nil {
			herr.Raise(&jsonDecodeError{err.Error()})
		}
		rows := PlanRetcon(patchList(threads), VersionsByThread(patchList(patches), client, subjectOfCommit))
		if args.numbers != "" {
			wanted := map[int]bool{}
			for _, n := range strings.Split(args.numbers, ",") {
				if py.Strip(n) != "" {
					wanted[pyInt(n)] = true
				}
			}
			kept := []*Retitle{}
			for _, row := range rows {
				if wanted[row.Number] {
					kept = append(kept, row)
				}
			}
			rows = kept
		}
		if args.limit != 0 {
			rows = rows[:sliceEnd(len(rows), args.limit)]
		}
		WriteCsv(rows, args.csv)
		fmt.Fprintf(stdout, "titles: %d of %d threads change; the CSV is %s\n", len(rows), len(threads), args.csv)
		if args.write {
			fmt.Fprintf(stdout, "titles: wrote %d thread(s)\n", ApplyRetcon(client, rows, args.pause, nil))
		}
		return 0
	}

	entries := Shipped(patchesAt(args.before), patchesAt(args.after))
	if len(entries) == 0 {
		fmt.Fprintln(stdout, "titles: nothing shipped in this push")
		return 0
	}
	for _, line := range RetitleShipped(client, entries, nil) {
		fmt.Fprintln(stdout, "titles: "+line)
	}
	return 0
}

// environ is `os.environ[name]`: a KeyError when it is not set. (The Python reads the
// environment here, outside config, and so does this.)
func environ(name string) string {
	value, ok := os.LookupEnv(name)
	if !ok {
		herr.Raise(&keyError{name})
	}
	return value
}

// sliceEnd is where `xs[:end]` stops, with Python's negative end.
func sliceEnd(n, end int) int {
	if end < 0 {
		end += n
	}
	return max(0, min(end, n))
}

// joinPath is os.path.join(a, b).
func joinPath(a, b string) string {
	if strings.HasPrefix(b, "/") || a == "" {
		return b
	}
	if strings.HasSuffix(a, "/") {
		return a + b
	}
	return a + "/" + b
}

func writeFile(path, text string) error { return os.WriteFile(path, []byte(text), 0o666) }

// gitOutput is `_git`.
func gitOutput(args ...string) string {
	cmd := exec.Command("git", args...)
	var out, errOut bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errOut
	err := cmd.Run()
	var exit *exec.ExitError
	if errors.As(err, &exit) {
		herr.Raise(&calledProcessError{cmd: append([]string{"git"}, args...), code: exit.ExitCode()})
	}
	herr.Must(err)
	text := out.String()
	return py.Universal(text)
}

// calledProcessError is subprocess.CalledProcessError.
type calledProcessError struct {
	cmd  []string
	code int
}

func (e *calledProcessError) Error() string {
	return fmt.Sprintf("Command '%s' returned non-zero exit status %d.", py.Repr(e.cmd), e.code)
}

// keyError is KeyError; jsonDecodeError is json.JSONDecodeError.
type keyError struct{ key string }

func (e *keyError) Error() string { return py.Repr(e.key) }

type jsonDecodeError struct{ msg string }

func (e *jsonDecodeError) Error() string { return e.msg }

// exceptionLine is the last line of the traceback Python prints for an uncaught exception.
func exceptionLine(r any) string {
	err, ok := r.(error)
	if !ok {
		return fmt.Sprint(r)
	}
	var (
		ghErr  *herr.GitHubError
		called *calledProcessError
		key    *keyError
		value  *valueError
		typ    *typeError
		decode *jsonDecodeError
		ghVal  *gh.ValueError
		path   *fs.PathError
	)
	switch {
	case errors.As(err, &ghErr):
		return "harness.errors.GitHubError: " + ghErr.Msg
	case errors.As(err, &called):
		return "subprocess.CalledProcessError: " + called.Error()
	case errors.As(err, &key):
		return "KeyError: " + key.Error()
	case errors.As(err, &value):
		return "ValueError: " + value.Error()
	case errors.As(err, &typ):
		return "TypeError: " + typ.Error()
	case errors.As(err, &decode):
		return "json.decoder.JSONDecodeError: " + decode.Error()
	case errors.As(err, &ghVal):
		return "json.decoder.JSONDecodeError: " + ghVal.Error()
	case errors.As(err, &path) && errors.Is(err, fs.ErrNotExist):
		return fmt.Sprintf("FileNotFoundError: [Errno 2] No such file or directory: %s", py.Repr(path.Path))
	case errors.As(err, &path):
		return "OSError: " + err.Error()
	}
	return err.Error()
}

// ------------------------------------------------------------------ argparse

// arguments is the parsed command line.
type arguments struct {
	command       string
	before, after string
	write         bool
	numbers, csv  string
	limit         int
	pause         float64
}

const prog = "harness titles"

const topUsage = "usage: harness titles [-h] {shipped,retcon} ...\n"

const topHelp = topUsage + `
positional arguments:
  {shipped,retcon}
    shipped         retitle what a push to main shipped
    retcon          retitle every issue and pull request (#1372)

options:
  -h, --help        show this help message and exit
`

// option is one of a parser's options; kind is "flag" (store_true), "str", "int" or "float".
type option struct {
	flag, kind string
	required   bool
}

type parser struct {
	prog, usage, help string
	options           []option
}

var subparsers = map[string]*parser{
	"shipped": {
		prog:  prog + " shipped",
		usage: "usage: harness titles shipped [-h] --before BEFORE --after AFTER\n",
		help: `
options:
  -h, --help       show this help message and exit
  --before BEFORE
  --after AFTER
`,
		options: []option{{"--before", "str", true}, {"--after", "str", true}},
	},
	"retcon": {
		prog: prog + " retcon",
		usage: "usage: harness titles retcon [-h] [--write] [--numbers NUMBERS]\n" +
			"                             [--limit LIMIT] [--csv CSV] [--pause PAUSE]\n",
		help: `
options:
  -h, --help         show this help message and exit
  --write            write the titles; without it, only the CSV
  --numbers NUMBERS  only these threads, comma-separated
  --limit LIMIT      at most this many threads
  --csv CSV
  --pause PAUSE      seconds between writes
`,
		options: []option{
			{"--write", "flag", false}, {"--numbers", "str", false}, {"--limit", "int", false},
			{"--csv", "str", false}, {"--pause", "float", false},
		},
	},
}

// fail is parser.error(message): the usage and the message on stderr, exit 2.
func fail(usage, prog, message string) (*arguments, int, bool) {
	fmt.Fprint(stderr, usage)
	fmt.Fprintf(stderr, "%s: error: %s\n", prog, message)
	return nil, 2, false
}

// parseArgs is parser.parse_args(argv): the arguments, or the exit code when it printed help
// (0) or an error (2).
func parseArgs(argv []string) (*arguments, int, bool) {
	var extras []string
	i := 0
	for ; i < len(argv); i++ {
		a := argv[i]
		if a == "--" {
			i++
			break
		}
		kind, _, explicit, hasExplicit := classify(a, nil)
		if kind == "positional" {
			break
		}
		if kind == "help" {
			if hasExplicit {
				return fail(topUsage, prog, "argument -h/--help: ignored explicit argument "+py.Repr(explicit))
			}
			fmt.Fprint(stdout, topHelp)
			return nil, 0, false
		}
		extras = append(extras, a)
	}
	if i >= len(argv) {
		return fail(topUsage, prog, "the following arguments are required: command")
	}
	command := argv[i]
	sub := subparsers[command]
	if sub == nil {
		return fail(topUsage, prog, "argument command: invalid choice: "+py.Repr(command)+" (choose from 'shipped', 'retcon')")
	}
	args := &arguments{command: command, csv: "title-retcon.csv", pause: 2.0}
	seen := map[string]bool{}
	rest := argv[i+1:]
	for j := 0; j < len(rest); {
		a := rest[j]
		if a == "--" {
			extras = append(extras, rest[j:]...)
			break
		}
		kind, opt, explicit, hasExplicit := classify(a, sub.options)
		switch kind {
		case "positional", "unknown":
			extras = append(extras, a)
			j++
			continue
		case "help":
			if hasExplicit {
				return fail(sub.usage, sub.prog, "argument -h/--help: ignored explicit argument "+py.Repr(explicit))
			}
			fmt.Fprint(stdout, sub.usage+sub.help)
			return nil, 0, false
		}
		if opt.kind == "flag" {
			if hasExplicit {
				return fail(sub.usage, sub.prog, "argument "+opt.flag+": ignored explicit argument "+py.Repr(explicit))
			}
			args.write = true
			seen[opt.flag] = true
			j++
			continue
		}
		value := explicit
		switch {
		case hasExplicit:
			j++
		case j+1 < len(rest) && rest[j+1] != "--" && isPositional(rest[j+1], sub.options):
			value = rest[j+1]
			j += 2
		default:
			return fail(sub.usage, sub.prog, "argument "+opt.flag+": expected one argument")
		}
		switch opt.kind {
		case "int":
			n, ok := parseInt(value)
			if !ok {
				return fail(sub.usage, sub.prog, "argument "+opt.flag+": invalid int value: "+py.Repr(value))
			}
			args.limit = n
		case "float":
			f, ok := parseFloat(value)
			if !ok {
				return fail(sub.usage, sub.prog, "argument "+opt.flag+": invalid float value: "+py.Repr(value))
			}
			args.pause = f
		default:
			switch opt.flag {
			case "--before":
				args.before = value
			case "--after":
				args.after = value
			case "--numbers":
				args.numbers = value
			case "--csv":
				args.csv = value
			}
		}
		seen[opt.flag] = true
	}
	var missing []string
	for _, opt := range sub.options {
		if opt.required && !seen[opt.flag] {
			missing = append(missing, opt.flag)
		}
	}
	if len(missing) > 0 {
		return fail(sub.usage, sub.prog, "the following arguments are required: "+strings.Join(missing, ", "))
	}
	if len(extras) > 0 {
		return fail(topUsage, prog, "unrecognized arguments: "+strings.Join(extras, " "))
	}
	return args, 0, true
}

// negativeNumber is argparse's `_negative_number_matcher`.
var negativeNumber = regexp.MustCompile(`^-\p{Nd}+\n?$|^-\p{Nd}*\.\p{Nd}+\n?$`)

// classify is argparse's `_parse_optional`: "positional" (a value), "help", "option" (one of
// options, with the value after its `=`), or "unknown" (an option no parser has). A long option
// may be shortened to any prefix no other option shares (none of these parsers' options share
// one), and `-h` takes anything after it as help does.
func classify(a string, options []option) (kind string, opt option, explicit string, hasExplicit bool) {
	if a == "" || a[0] != '-' || a == "-" {
		return "positional", opt, "", false
	}
	name, value, hasValue := strings.Cut(a, "=")
	if !strings.HasPrefix(a, "--") {
		if strings.HasPrefix(a, "-h") {
			return "help", opt, "", false
		}
	} else if len(name) > 2 {
		if strings.HasPrefix("--help", name) {
			return "help", opt, value, hasValue
		}
		for _, o := range options {
			if strings.HasPrefix(o.flag, name) {
				return "option", o, value, hasValue
			}
		}
	}
	if negativeNumber.MatchString(a) || strings.Contains(a, " ") {
		return "positional", opt, "", false
	}
	return "unknown", opt, "", false
}

// isPositional is whether argparse reads `a` as a value (its 'A').
func isPositional(a string, options []option) bool {
	kind, _, _, _ := classify(a, options)
	return kind == "positional"
}

// floatSyntax is what float() reads, once stripped: a decimal number with digits that may be any
// script's and `_` between two of them, or inf, infinity or nan in any case.
var floatSyntax = regexp.MustCompile(`^[+-]?(?:(?i:inf|infinity|nan)|(?:\p{Nd}(?:_?\p{Nd})*(?:\.(?:\p{Nd}(?:_?\p{Nd})*)?)?|\.\p{Nd}(?:_?\p{Nd})*)(?:[eE][+-]?\p{Nd}(?:_?\p{Nd})*)?)$`)

// parseFloat is float(text), and false where float() raises ValueError.
func parseFloat(text string) (float64, bool) {
	clean := py.Strip(text)
	if !floatSyntax.MatchString(clean) {
		return 0, false
	}
	if strings.EqualFold(strings.TrimLeft(clean, "+-"), "nan") {
		return math.NaN(), true // Go reads no sign on a NaN
	}
	var b strings.Builder
	for _, r := range clean {
		switch {
		case r == '_':
		case r > 0x7f && unicode.IsDigit(r):
			b.WriteByte(byte('0' + py.DigitValue(r)))
		default:
			b.WriteRune(r)
		}
	}
	f, err := strconv.ParseFloat(b.String(), 64)
	if err != nil && !errors.Is(err, strconv.ErrRange) {
		return 0, false
	}
	return f, true
}
