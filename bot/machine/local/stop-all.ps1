<#
  The local runner (bot/machine/README.md, The local runner).
  Retires every runner of this PC now: unregisters it from GitHub and removes its container.
  A runner in the middle of a job is left alone unless -Force (that job then fails, and the bot
  treats it like a lost runner). Stop the supervisor first (install-task.ps1 -Remove), or it
  starts new ones.
#>
param([switch]$Force)
$repo = (Get-Content (Join-Path $PSScriptRoot 'config.json') -Raw | ConvertFrom-Json).repo
# No double quotes in the format: PowerShell 5.1 drops them from a native command's arguments.
$lines = docker ps --filter label=jackioh.pc-runner=1 --format '{{.ID}}|{{.Names}}|{{.Labels}}'
foreach ($line in @($lines)) {
  if (-not $line) { continue }
  $id, $name, $labels = $line -split '\|', 3
  $runnerId = ([regex]::Match($labels, 'jackioh\.runner-id=(\d+)')).Groups[1].Value
  gh api -X DELETE "repos/$repo/actions/runners/$runnerId" 2>&1 | Out-Null
  if ($LASTEXITCODE -eq 0 -or $Force) {
    docker rm -f $id | Out-Null
    "removed $name"
  } else {
    "$name is running a job; left alone (-Force to kill it)"
  }
}
