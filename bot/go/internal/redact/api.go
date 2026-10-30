// Package redact is bot/harness/redact.py.
//
// Secret redaction. Everything the harness writes to GitHub or to an artifact passes through it.
package redact

// REDACTION = "[REDACTED]"
var Redaction string = "[REDACTED]"

// MIN_REMEMBERED = 16
var MinRemembered int = 16

// Redact these values too, from now on in this process.

func Remember(values []string) {
	panic("unported: redact.remember")
}

// `text` with every known token shape and every live secret value replaced.

func Redact(text string, extra []string /* = None */) string {
	panic("unported: redact.redact")
}

// Every string leaf of nested dicts and lists, redacted.

func RedactJSON(obj any) any {
	panic("unported: redact.redact_json")
}
