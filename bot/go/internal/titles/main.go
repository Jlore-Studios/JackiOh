package titles

// The command line, `harness titles shipped …` (patches-ship.yml): `main`, with what its argparse parser does (usage, help, errors and exit
// codes, as Python 3.13's prints them at 80 columns, as on a runner).

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"os/exec"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
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
		herr.Raise(py.KeyError(name))
	}
	return value
}

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

// exceptionLine is the last line of the traceback Python prints for an uncaught exception.
func exceptionLine(r any) string {
	err, ok := r.(error)
	if !ok {
		return fmt.Sprint(r)
	}
	var (
		ghErr  *herr.GitHubError
		called *calledProcessError
		exc    *py.Error
		ghVal  *gh.ValueError
		path   *fs.PathError
	)
	switch {
	case errors.As(err, &ghErr):
		return "harness.errors.GitHubError: " + ghErr.Msg
	case errors.As(err, &called):
		return "subprocess.CalledProcessError: " + called.Error()
	case errors.As(err, &exc):
		return exc.Type + ": " + exc.Msg
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
}

const prog = "harness titles"

const topUsage = "usage: harness titles [-h] {shipped} ...\n"

const topHelp = topUsage + `
positional arguments:
  {shipped}
    shipped   retitle what a push to main shipped

options:
  -h, --help  show this help message and exit
`

// option is one of a parser's options, each taking one value.
type option struct {
	flag     string
	required bool
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
		options: []option{{"--before", true}, {"--after", true}},
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
		return fail(topUsage, prog, "argument command: invalid choice: "+py.Repr(command)+" (choose from 'shipped')")
	}
	args := &arguments{command: command}
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
		switch opt.flag {
		case "--before":
			args.before = value
		case "--after":
			args.after = value
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
