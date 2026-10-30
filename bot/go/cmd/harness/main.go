// Command harness is the night bot's command line, `harness`'s twin
// (bot/harness/__main__.py).
package main

import (
	"os"
	_ "time/tzdata"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
)

func main() { os.Exit(cli.Main(os.Args[1:])) }
