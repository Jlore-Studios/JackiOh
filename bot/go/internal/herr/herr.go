// Package herr is bot/harness/errors.py: the exceptions the harness raises on purpose.
//
// The harness keeps Python's control flow: an error is raised with Raise (a panic carrying an
// error value) and caught where the Python caught it, with Catch for one type (`except
// GitHubError as e:`) or Try for any (`except Exception:`). The command line turns whatever is
// left into the message and exit code the Python printed. So each `try`/`except` of the Python
// is one Catch or Try here, and a function's signature is the Python's, without an error result
// threaded through every caller. Go's own errors (a file that will not open, a number that will
// not parse) are checked where they arise, as the Python's narrow excepts did.
package herr

import (
	"errors"
	"fmt"
	"runtime/debug"
)

// Harness is every error the harness raises on purpose (Python's HarnessError and subclasses).
type Harness interface {
	error
	harness()
}

// HarnessError is the base class itself, raised plain.
type HarnessError struct{ Msg string }

// ConfigError: the committed config or the environment is unusable.
type ConfigError struct{ Msg string }

// GitHubError: a GitHub request failed. Status is the HTTP status, or 0 when none arrived.
type GitHubError struct {
	Msg    string
	Status int
	Body   any
}

// GitError: a git command failed.
type GitError struct{ Msg string }

// StateConflict: the state file changed under an update more times than the retry allows.
type StateConflict struct{ Msg string }

// StateUnreadable: the state file exists but is not JSON, so nothing reads or writes over it.
type StateUnreadable struct{ Msg string }

// LoginError: a provider's secret could not be turned into a login its CLI reads.
type LoginError struct{ Msg string }

func (e *HarnessError) Error() string    { return e.Msg }
func (e *ConfigError) Error() string     { return e.Msg }
func (e *GitHubError) Error() string     { return e.Msg }
func (e *GitError) Error() string        { return e.Msg }
func (e *StateConflict) Error() string   { return e.Msg }
func (e *StateUnreadable) Error() string { return e.Msg }
func (e *LoginError) Error() string      { return e.Msg }

func (*HarnessError) harness()    {}
func (*ConfigError) harness()     {}
func (*GitHubError) harness()     {}
func (*GitError) harness()        {}
func (*StateConflict) harness()   {}
func (*StateUnreadable) harness() {}
func (*LoginError) harness()      {}

// Raise is Python's raise: it panics with err, for a Catch or Try up the stack.
func Raise(err error) { panic(err) }

// Raisef raises a HarnessError with a formatted message.
func Raisef(format string, args ...any) { panic(&HarnessError{Msg: fmt.Sprintf(format, args...)}) }

// Panic is a panic that was not an error value (Python's exceptions are all errors here), kept
// with the stack where it happened.
type Panic struct {
	Value any
	Stack []byte
}

func (p *Panic) Error() string { return fmt.Sprint(p.Value) }

// Try runs fn and returns what it raised, as `try: fn() except Exception as e:` does: any panic
// comes back as an error (Go's runtime errors, Python's AttributeError and KeyError alike).
func Try(fn func()) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = asError(r)
		}
	}()
	fn()
	return nil
}

// Catch runs fn and returns the error of type E it raised, as `except E as e:` does; any other
// panic goes on up.
func Catch[E error](fn func()) (caught E, ok bool) {
	defer func() {
		if r := recover(); r != nil {
			err := asError(r)
			var target E
			if errors.As(err, &target) {
				caught, ok = target, true
				return
			}
			panic(r)
		}
	}()
	fn()
	return caught, false
}

// CatchHarness runs fn and returns the HarnessError (of any subclass) it raised.
func CatchHarness(fn func()) (Harness, bool) { return Catch[Harness](fn) }

// Must raises err when it is not nil.
func Must(err error) {
	if err != nil {
		panic(err)
	}
}

// IsHarness is isinstance(err, HarnessError).
func IsHarness(err error) bool {
	var h Harness
	return errors.As(err, &h)
}

func asError(r any) error {
	if err, ok := r.(error); ok {
		return err
	}
	return &Panic{Value: r, Stack: debug.Stack()}
}
