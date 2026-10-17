<#
  The local runner (bot/machine/README.md, The local runner).
  Starts supervisor.ps1 at every logon, hidden, as you (no admin needed).
    .\install-task.ps1            # install (and start it now)
    .\install-task.ps1 -Remove    # remove it; then run .\stop-all.ps1 to retire any runner left
#>
param([switch]$Remove)
$name = 'JackiOh PC runner'
if ($Remove) {
  Stop-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
  Unregister-ScheduledTask -TaskName $name -Confirm:$false
  return
}
$script = Join-Path $PSScriptRoot 'supervisor.ps1'
$action = New-ScheduledTaskAction -Execute 'powershell.exe' `
  -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$script`"" `
  -WorkingDirectory $PSScriptRoot
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
  -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) `
  -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName $name
"'$name' installed and started; log: $(Join-Path $PSScriptRoot 'logs\supervisor.log')"
