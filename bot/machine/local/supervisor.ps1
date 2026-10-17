<#
  The local runner's supervisor (bot/machine/README.md, The local runner): lends this PC to the
  repository's workflows while, and only while, the PC is otherwise idle.

  Every sampleSeconds it reads this PC's CPU and memory, leaving out what its own runner
  containers use. "Calm" means every sample of the last calmSeconds had CPU under
  cpuPercentBelow and used memory under memoryGBBelow (config.json).

  Each entry of config.json's `serve` is one runner label. A runner here is one ephemeral
  container: it registers with that one label (a single-use JIT config), takes one job and is
  gone. While it is calm and fewer than maxJobs jobs run here:
  - a label with a job queued for it (in one of the entry's `workflows`) gets a runner;
  - a `standby` label (jackioh-selfplay-local) keeps one idle runner registered, so it shows Idle
    under Settings -> Actions -> Runners and a dispatch starts at once;
  - a label with a `home` (a night box subscription's, its login's volume) is served only once
    login.ps1 has made that volume.
  An idle runner no label wants any more is retired after a minute. When it is not calm, every
  idle runner is retired at once, and a runner in the middle of a job is throttled to
  throttledCpus (the job keeps going, slowly, and nothing is lost) until it is calm again.

  No container is ever given a GPU. Runs in the foreground; install-task.ps1 starts it at logon.
  Log: logs\supervisor.log.
#>
param(
  [string]$ConfigPath = (Join-Path $PSScriptRoot 'config.json')
)
$ErrorActionPreference = 'Continue'
$cfg = Get-Content $ConfigPath -Raw | ConvertFrom-Json
$repo = $cfg.repo
$logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
$logDir = Join-Path $PSScriptRoot 'logs'
New-Item -ItemType Directory -Force $logDir | Out-Null
$logFile = Join-Path $logDir 'supervisor.log'
$windowSize = [math]::Ceiling($cfg.calmSeconds / $cfg.sampleSeconds)
$samples = New-Object System.Collections.Generic.Queue[object]
$throttled = @{}      # container id -> $true while throttled
$unwanted = @{}       # container id -> when it was first idle with no label wanting it
$lastQueueCheck = [datetime]::MinValue
$lastSweep = [datetime]::MinValue
$nextStart = [datetime]::MinValue
$queued = @()
$startCooldownSeconds = 15
$retireGraceMinutes = 1

function Log([string]$msg) {
  $line = '{0:yyyy-MM-dd HH:mm:ss} {1}' -f (Get-Date), $msg
  Write-Host $line
  if ((Test-Path $logFile) -and (Get-Item $logFile).Length -gt 5MB) {
    Move-Item -Force $logFile "$logFile.1"
  }
  Add-Content -Path $logFile -Value $line -Encoding utf8
}

# A native command's output as text and its exit code, without PowerShell 5.1 turning stderr into
# errors.
function Run([string]$exe, [string[]]$argv) {
  $out = & $exe @argv 2>&1 | ForEach-Object { "$_" }
  [pscustomobject]@{ Code = $LASTEXITCODE; Out = ($out -join "`n") }
}

# `gh api` as JSON. (Not named Gh: PowerShell's names are case-blind, so a function Gh would shadow
# gh.exe for `& 'gh'`.)
function GhJson([string[]]$argv) {
  $r = Run 'gh' $argv
  if ($r.Code -ne 0) { throw "gh $($argv -join ' '): $($r.Out)" }
  if ($r.Out) { $r.Out | ConvertFrom-Json } else { $null }
}

function To-GB([string]$text) {
  if ($text -notmatch '^\s*([\d.]+)\s*([KMGT]?i?B)') { return 0 }
  $n = [double]$Matches[1]
  switch -regex ($Matches[2]) {
    '^K' { return $n / 1MB } '^M' { return $n / 1KB } '^G' { return $n } '^T' { return $n * 1KB }
    default { return $n / 1GB }
  }
}

# The serve entry of a label.
function Entry([string]$label) { @($cfg.serve | Where-Object { $_.label -eq $label })[0] }

# The labels this PC may serve now: every entry without a home, and those whose home volume exists.
function Servable {
  $volumes = @((Run 'docker' @('volume', 'ls', '--format', '{{.Name}}')).Out -split "`n" | ForEach-Object { $_.Trim() })
  @($cfg.serve | Where-Object { -not $_.home -or $_.home -in $volumes } | ForEach-Object { $_.label })
}

# A container's labels (`docker ps`'s `k=v,k=v`) as a table.
function Labels-Of([string]$text) {
  $table = @{}
  foreach ($pair in ($text -split ',')) {
    $kv = $pair -split '=', 2
    if ($kv.Count -eq 2) { $table[$kv[0].Trim()] = $kv[1].Trim() }
  }
  $table
}

# The runner containers: id, label, GitHub runner id, name, busy (in a job). The format holds no
# double quotes: PowerShell 5.1 drops them from a native command's arguments.
function Get-Runners {
  $r = Run 'docker' @('ps', '--filter', 'label=jackioh.pc-runner=1', '--format', '{{.ID}}|{{.Names}}|{{.Labels}}')
  if ($r.Code -ne 0 -or -not $r.Out) { return @() }
  foreach ($line in ($r.Out -split "`n")) {
    $f = $line.Trim() -split '\|', 3
    if ($f.Count -lt 3) { continue }
    $labels = Labels-Of $f[2]
    $busy = (Run 'docker' @('exec', $f[0], 'pgrep', '-f', 'Runner.Worker')).Code -eq 0
    [pscustomobject]@{ Id = $f[0]; Label = $labels['jackioh.label']; RunnerId = $labels['jackioh.runner-id']; Name = $f[1]; Busy = $busy }
  }
}

# This PC's load without its runner containers: CPU % of all logical CPUs, memory used in GB.
function Get-Sample($runners) {
  $cpu = $null
  # CPU time, as `docker stats` measures the runners' share, so the two subtract like for like.
  # (Processor Utility scales by clock speed: at turbo it reads the runners' cores as far more than
  # their share of the CPUs, and the rest of the PC as busier than it is.)
  foreach ($path in '\Processor(_Total)\% Processor Time', '\Processor Information(_Total)\% Processor Time') {
    try {
      $cpu = (Get-Counter $path -SampleInterval $cfg.sampleSeconds -MaxSamples 1 -ErrorAction Stop).CounterSamples[0].CookedValue
      break
    } catch { }
  }
  if ($null -eq $cpu) { Start-Sleep -Seconds $cfg.sampleSeconds; $cpu = 100 }
  $os = Get-CimInstance Win32_OperatingSystem
  $memGB = ($os.TotalVisibleMemorySize - $os.FreePhysicalMemory) / 1MB
  $oursCpu = 0.0; $oursMem = 0.0
  if (@($runners).Count) {
    $s = Run 'docker' (@('stats', '--no-stream', '--format', '{{.CPUPerc}}|{{.MemUsage}}') + @($runners | ForEach-Object Id))
    if ($s.Code -eq 0) {
      foreach ($line in ($s.Out -split "`n")) {
        $f = $line -split '\|'
        if ($f.Count -lt 2) { continue }
        $oursCpu += [double]($f[0].TrimEnd('%').Trim()) / $logical
        $oursMem += To-GB (($f[1] -split '/')[0])
      }
    }
  }
  [pscustomobject]@{
    Cpu = [math]::Max(0, [math]::Min(100, $cpu) - $oursCpu)
    Mem = [math]::Max(0, $memGB - $oursMem)
  }
}

# The labels of the jobs queued now for `servable`, one per job, in the order GitHub lists them
# (as bot/machine/starter.py reads them). A workflow that does not exist yet is skipped.
function Get-QueuedLabels([string[]]$servable) {
  $labels = @()
  $seen = @{}
  $workflows = @($cfg.serve | Where-Object { $_.label -in $servable } | ForEach-Object { $_.workflows } | Select-Object -Unique)
  foreach ($wf in $workflows) {
    foreach ($status in 'queued', 'in_progress') {
      try {
        $runs = (GhJson @('api', "repos/$repo/actions/workflows/$wf/runs?status=$status&per_page=30")).workflow_runs
      } catch { continue }
      foreach ($run in @($runs)) {
        if (-not $run -or $seen[$run.id]) { continue }
        $seen[$run.id] = $true
        $jobs = (GhJson @('api', "repos/$repo/actions/runs/$($run.id)/jobs?filter=latest&per_page=100")).jobs
        foreach ($job in @($jobs)) {
          if ($job.status -notin 'queued', 'pending', 'requested') { continue }
          $labels += @($job.labels | Where-Object { $_ -in $servable })
        }
      }
    }
  }
  $labels
}

# A short name for a label's runners: `selfplay` for jackioh-selfplay-local, the subscription for
# night-vm-<sub>.
function Short([string]$label) {
  if ($label -eq 'jackioh-selfplay-local') { return 'selfplay' }
  $label -replace '^night-vm-', ''
}

function Start-Runner([string]$label, [string]$why) {
  $entry = Entry $label
  $name = 'pc-{0}-{1:MMddHHmmss}' -f (Short $label), (Get-Date)
  # One label of its own, as register-runners.sh gives the night box's (#1008).
  $jit = GhJson @('api', '-X', 'POST', "repos/$repo/actions/runners/generate-jitconfig",
    '-f', "name=$name", '-F', 'runner_group_id=1', '-f', "labels[]=$label", '-f', 'work_folder=_work')
  # The single-use config goes in through a file, not the command line.
  $envFile = Join-Path $env:TEMP "$name.env"
  Set-Content -Path $envFile -Value "JIT_CONFIG=$($jit.encoded_jit_config)" -Encoding ascii -NoNewline
  $cpus = [math]::Max(1, [math]::Floor($cfg.containerCpus))
  $argv = @('run', '-d', '--rm', '--name', $name,
    '--label', 'jackioh.pc-runner=1', '--label', "jackioh.label=$label", '--label', "jackioh.runner-id=$($jit.runner.id)",
    '--cpus', "$($cfg.containerCpus)", '--memory', $cfg.containerMemory, '--memory-swap', $cfg.containerMemory,
    '--pids-limit', '8192', '--security-opt', 'no-new-privileges',
    '-e', "CARGO_BUILD_JOBS=$cpus", '-e', "RAYON_NUM_THREADS=$cpus", '--env-file', $envFile)
  if ($entry.home) { $argv += @('-v', "$($entry.home):/home/runner") }
  # Codex sandboxes its commands with bubblewrap, which needs the user namespaces Docker's default
  # seccomp profile refuses.
  if ($entry.seccompUnconfined) { $argv += @('--security-opt', 'seccomp=unconfined') }
  $r = Run 'docker' ($argv + @($cfg.image))
  Remove-Item -Force $envFile -ErrorAction SilentlyContinue
  if ($r.Code -ne 0) {
    Log "could not start $name : $($r.Out)"
    Run 'gh' @('api', '-X', 'DELETE', "repos/$repo/actions/runners/$($jit.runner.id)") | Out-Null
  } else {
    Log "started $name ($why)"
  }
}

# Unregisters an idle runner and removes its container. GitHub refuses to remove a runner that is
# running a job, so a job that landed a moment ago is never cut off.
function Stop-IdleRunner($runner, [string]$why) {
  $r = Run 'gh' @('api', '-X', 'DELETE', "repos/$repo/actions/runners/$($runner.RunnerId)")
  if ($r.Code -ne 0 -and $r.Out -notmatch '404|Not Found') { return }
  Run 'docker' @('rm', '-f', $runner.Id) | Out-Null
  $unwanted.Remove($runner.Id)
  Log "retired idle $($runner.Name) ($why)"
}

function Set-Throttle($runner, [bool]$on) {
  if ([bool]$throttled[$runner.Id] -eq $on) { return }
  $cpus = if ($on) { $cfg.throttledCpus } else { $cfg.containerCpus }
  if ((Run 'docker' @('update', '--cpus', "$cpus", $runner.Id)).Code -eq 0) {
    $throttled[$runner.Id] = $on
    Log ("{0} {1} to {2} CPUs" -f ($(if ($on) { 'throttled' } else { 'restored' })), $runner.Name, $cpus)
  }
}

# GitHub keeps a JIT runner that never ran (its container died first) as offline; drop those.
function Remove-StaleRunners($runners) {
  $live = @($runners | ForEach-Object { "$($_.RunnerId)" })
  try {
    $all = (GhJson @('api', "repos/$repo/actions/runners?per_page=100")).runners
    foreach ($gr in @($all)) {
      if ($gr.name -like 'pc-*' -and $gr.status -eq 'offline' -and "$($gr.id)" -notin $live) {
        Run 'gh' @('api', '-X', 'DELETE', "repos/$repo/actions/runners/$($gr.id)") | Out-Null
        Log "removed stale runner $($gr.name)"
      }
    }
  } catch { Log "stale-runner sweep failed: $_" }
}

# The labels wanted now, at most `room` of them: one per queued job, then each standby label not
# already among them.
function Wanted([string[]]$queuedLabels, [string[]]$servable, [int]$room) {
  $want = New-Object System.Collections.ArrayList
  foreach ($label in $queuedLabels) { if ($want.Count -lt $room) { [void]$want.Add($label) } }
  foreach ($entry in @($cfg.serve | Where-Object { $_.standby -and $_.label -in $servable })) {
    if ($want.Count -lt $room -and $entry.label -notin $want) { [void]$want.Add($entry.label) }
  }
  , $want
}

Log ("supervisor up on $repo, serving {0}; calm = CPU < {1}% and memory < {2} GB for {3}s; at most {4} job(s); no GPU" -f `
  (@($cfg.serve | ForEach-Object { $_.label }) -join ', '), $cfg.cpuPercentBelow, $cfg.memoryGBBelow, $cfg.calmSeconds, $cfg.maxJobs)
$wasCalm = $null
while ($true) {
  if ((Run 'docker' @('info', '--format', '{{.ServerVersion}}')).Code -ne 0) {
    if ($wasCalm -ne 'nodocker') { Log 'Docker is not running; waiting'; $wasCalm = 'nodocker' }
    Start-Sleep -Seconds 30
    continue
  }
  $runners = @(Get-Runners)
  $sample = Get-Sample $runners
  $samples.Enqueue($sample)
  while ($samples.Count -gt $windowSize) { [void]$samples.Dequeue() }
  $over = @($samples | Where-Object { $_.Cpu -ge $cfg.cpuPercentBelow -or $_.Mem -ge $cfg.memoryGBBelow })
  $calm = ($samples.Count -ge $windowSize) -and ($over.Count -eq 0)
  if ($calm -ne $wasCalm) {
    Log ("{0}: CPU {1:N0}%, memory {2:N1} GB (without the runners)" -f $(if ($calm) { 'calm' } else { 'busy' }), $sample.Cpu, $sample.Mem)
    $wasCalm = $calm
  }

  if (((Get-Date) - $lastSweep).TotalMinutes -ge 10) { Remove-StaleRunners $runners; $lastSweep = Get-Date }

  $busy = @($runners | Where-Object Busy)
  $idle = @($runners | Where-Object { -not $_.Busy })
  foreach ($r in $busy) { Set-Throttle $r (-not $calm); $unwanted.Remove($r.Id) }

  $room = [int]$cfg.maxJobs - $busy.Count
  if (-not $calm -or $room -le 0) {
    $why = if (-not $calm) { 'the PC is busy' } else { 'no room for another job' }
    foreach ($r in $idle) { Stop-IdleRunner $r $why }
    continue
  }

  $servable = @(Servable)
  if (((Get-Date) - $lastQueueCheck).TotalSeconds -ge 20) {
    try { $queued = @(Get-QueuedLabels $servable) } catch { Log "queue check failed: $_"; $queued = @() }
    $lastQueueCheck = Get-Date
  }
  $want = Wanted $queued $servable $room

  # Match the idle runners to what is wanted; an unmatched one is retired after a grace period, in
  # case GitHub has just handed it a job the queue check has not seen yet.
  $missing = [System.Collections.ArrayList]@($want)
  foreach ($r in $idle) {
    if ($r.Label -in $missing) {
      $missing.Remove($r.Label)
      $unwanted.Remove($r.Id)
      continue
    }
    if (-not $unwanted[$r.Id]) { $unwanted[$r.Id] = Get-Date }
    elseif (((Get-Date) - $unwanted[$r.Id]).TotalMinutes -ge $retireGraceMinutes) { Stop-IdleRunner $r 'nothing wants it' }
  }
  # Idle runners count against maxJobs too, so two jobs landing at once never run past it.
  if ($missing.Count -and ($busy.Count + $idle.Count) -lt [int]$cfg.maxJobs -and (Get-Date) -ge $nextStart) {
    $label = $missing[0]
    $why = if ($label -in $queued) { "a queued $label job" } else { "standby for $label" }
    try { Start-Runner $label $why } catch { Log "start failed: $_" }
    $nextStart = (Get-Date).AddSeconds($startCooldownSeconds)
    $lastQueueCheck = [datetime]::MinValue
  }
}
