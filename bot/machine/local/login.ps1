<#
  The local runner (bot/machine/README.md, The local runner).
  One-time login of a machine subscription on this PC, into that subscription's own volume
  (jackioh-pc-home-<sub>), the way bot/machine/README.md step 3 logs in on the night box.
  This is a second login of the same account, alongside the night box's; the box's stays valid.

    .\login.ps1 muse     # muse login
    .\login.ps1 gpt      # codex login --device-auth  (Sign in with ChatGPT)
    .\login.ps1 agy      # agy: sign in with Google, then quit
    .\login.ps1 devin    # devin auth login --force-manual-token-flow
    .\login.ps1 muse -Shell   # a shell in that volume, to look around
#>
param(
  [Parameter(Mandatory)][ValidateSet('muse', 'gpt', 'agy', 'devin')][string]$Sub,
  [switch]$Shell
)
$image = (Get-Content (Join-Path $PSScriptRoot 'config.json') -Raw | ConvertFrom-Json).image
$cmd = if ($Shell) { @('bash') } else {
  switch ($Sub) {
    'muse'  { @('muse', 'login') }
    'gpt'   { @('codex', 'login', '--device-auth') }
    'agy'   { @('agy') }
    'devin' { @('devin', 'auth', 'login', '--force-manual-token-flow') }
  }
}
docker run -it --rm -v "jackioh-pc-home-${Sub}:/home/runner" $image @cmd
