<#
  The local runner's supervisor (bot/machine/README.md, The local runner): lends this PC to the
  repository's workflows while, and only while, the PC is otherwise idle.

  Every sampleSeconds it reads this PC's CPU and memory, leaving out what its own runner
  containers use. "Calm" means every sample of the last calmSeconds had CPU under
  cpuPercentBelow and used memory under memoryGBBelow (config.json). "Quiet" is calm with every
  sample's CPU under startCpuPercentBelow too: a new runner starts only then, while one already
  up stays as long as it is calm.

  Each entry of config.json's `serve` is one runner label. A runner here is one ephemeral
  container: it registers with that one label (a single-use JIT config), takes one job and is
  gone. While it is quiet and fewer than maxJobs jobs run here:
  - a label with a job queued for it (in one of the entry's `workflows`) gets a runner;
  - a `standby` label (jackioh-selfplay-local) keeps one idle runner registered, so it shows Idle
    under Settings -> Actions -> Runners and a dispatch starts at once;
  - a label with a `home` (a night box subscription's, its login's volume) is served only once
    login.ps1 has made that volume.
  An `ownSlot` label (jackioh-selfplay-local-gather, gather.yml) is outside maxJobs: it gets one
  runner of its own while a job is queued for it, whatever the other labels' runners hold, and
  keeps no standby. An entry's own `containerCpus` and `containerMemory` size its containers in
  place of the global ones. A `priority` entry's runner (gather's, #2134) comes before the others'
  in the Docker VM: it takes the CPU ahead of them while both want it, and under the VM's memory
  pressure the kernel kills their job processes first (Start-Runner). The throttle below still
  holds every runner alike, so each gives way to the owner's own programs.
  An idle runner no label wants any more is retired after a minute. When it is not calm, every
  idle runner is retired at once, and a runner in the middle of a job is throttled to
  throttledCpus (the job keeps going, slowly, and nothing is lost) until it is calm again.

  Every heartbeatSeconds it writes a heartbeat (Send-Heartbeat): this PC's load, with and without
  its runners, the size of its memory, whether it is calm, and each runner container with the
  CPUs it uses, whether it is throttled and, while it plays games, how many of how many
  (Get-Progress), as pc.json in a secret gist of the signed-in account.
  The status issue's runners section and learn lane read it (bot/go/internal/dashboard), so it
  says how loaded the PC is and what it and its gather job are doing. The gist is
  made once; its id is kept in pc-gist.txt beside the scripts and in the repository variable
  JACKIOH_PC_GIST, which bot-status.yml hands the status loop. A heartbeat that fails is logged
  and tried again at the next beat: it never holds a runner up.

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
$memTotalGB = (Get-CimInstance Win32_OperatingSystem).TotalVisibleMemorySize / 1MB
$logDir = Join-Path $PSScriptRoot 'logs'
New-Item -ItemType Directory -Force $logDir | Out-Null
$logFile = Join-Path $logDir 'supervisor.log'
$windowSize = [math]::Ceiling($cfg.calmSeconds / $cfg.sampleSeconds)
$startCpuBelow = if ($null -ne $cfg.startCpuPercentBelow) { $cfg.startCpuPercentBelow } else { $cfg.cpuPercentBelow }
$samples = New-Object System.Collections.Generic.Queue[object]
$throttled = @{}      # container id -> $true while throttled
$unwanted = @{}       # container id -> when it was first idle with no label wanting it
$lastQueueCheck = [datetime]::MinValue
$lastSweep = [datetime]::MinValue
$nextStart = [datetime]::MinValue
$queued = @()
$startCooldownSeconds = 15
$retireGraceMinutes = 1
$heartbeatSeconds = if ($null -ne $cfg.heartbeatSeconds) { [int]$cfg.heartbeatSeconds } else { 120 }
$lastHeartbeat = [datetime]::MinValue
$gistFile = Join-Path $PSScriptRoot 'pc-gist.txt'

function Log([string]$msg) {
  $line = '{0:yyyy-MM-dd HH:mm:ss} {1}' -f (Get-Date), $msg
  Write-Host $line
  if ((Test-Path $logFile) -and (Get-Item $logFile).Length -gt 5MB) {
    Move-Item -Force $logFile "$logFile.1"
  }
  Add-Content -Path $logFile -Value $line -Encoding utf8
}

# One argument as Windows' command line quotes it (CommandLineToArgvW's rules).
function Quote-Arg([string]$a) {
  if ($a -and $a -notmatch '[\s"]') { return $a }
  '"' + (($a -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"'
}

# A native command's output as text (stdout, then stderr) and its exit code, without PowerShell 5.1
# turning stderr into errors. One still running after $timeoutSeconds is killed and fails (code -1):
# a `docker info` that a stuck Docker engine never answered once held the supervisor for 19 hours,
# its runner offline the while (#466).
function Run([string]$exe, [string[]]$argv, [int]$timeoutSeconds = 120) {
  $path = (Get-Command $exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source
  if (-not $path) { return [pscustomobject]@{ Code = -1; Out = "$exe is not on PATH" } }
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $path
  $psi.Arguments = (@($argv) | ForEach-Object { Quote-Arg $_ }) -join ' '
  $psi.UseShellExecute = $false
  $psi.CreateNoWindow = $true
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
  $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
  $p = [System.Diagnostics.Process]::Start($psi)
  $stdout = $p.StandardOutput.ReadToEndAsync()
  $stderr = $p.StandardError.ReadToEndAsync()
  if (-not $p.WaitForExit($timeoutSeconds * 1000)) {
    try { $p.Kill() } catch { }
    return [pscustomobject]@{ Code = -1; Out = "$exe $($argv[0]) gave no answer in $timeoutSeconds s" }
  }
  $p.WaitForExit()
  $lines = @(($stdout.Result.TrimEnd() -split "\r?\n") + ($stderr.Result.TrimEnd() -split "\r?\n") | Where-Object { $_ -ne '' })
  [pscustomobject]@{ Code = $p.ExitCode; Out = ($lines -join "`n") }
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

# Whether a label's runner has a slot of its own, outside maxJobs (its entry's `ownSlot`).
function Own([string]$label) { [bool](Entry $label).ownSlot }

# A label's container size: its entry's containerCpus and containerMemory, else the global ones.
function Cpus-Of([string]$label) {
  $entry = Entry $label
  if ($entry -and $null -ne $entry.containerCpus) { $entry.containerCpus } else { $cfg.containerCpus }
}
function Memory-Of([string]$label) {
  $entry = Entry $label
  if ($entry -and $entry.containerMemory) { $entry.containerMemory } else { $cfg.containerMemory }
}

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

# This PC's load without its runner containers and its yielding ones: CPU % of all logical CPUs,
# memory used in GB; and the whole PC's, with them, for the heartbeat.
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
  $ours = @($runners | ForEach-Object Id)
  # A container that a gate of its own holds to this same rule, and that gives way while a runner
  # works (`yieldingContainers`: a long dev export), is this machine's work too, not the person's.
  # Counted as the person's, it would read the PC as busy the moment it got its CPUs back.
  foreach ($name in @($cfg.yieldingContainers)) {
    if ($name -and (Run 'docker' @('ps', '-q', '--filter', "name=^$name`$")).Out) { $ours += $name }
  }
  if ($ours.Count) {
    $s = Run 'docker' (@('stats', '--no-stream', '--format', '{{.CPUPerc}}|{{.MemUsage}}') + $ours)
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
    CpuAll = [math]::Min(100, $cpu)
    MemAll = $memGB
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

# A short name for a label's runners: `selfplay` for jackioh-selfplay-local, `gather` for
# jackioh-selfplay-local-gather, the subscription for night-vm-<sub>.
function Short([string]$label) {
  if ($label -eq 'jackioh-selfplay-local') { return 'selfplay' }
  if ($label -eq 'jackioh-selfplay-local-gather') { return 'gather' }
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
  $containerCpus = Cpus-Of $label
  $memory = Memory-Of $label
  $cpus = [math]::Max(1, [math]::Floor($containerCpus))
  $argv = @('run', '-d', '--rm', '--name', $name,
    '--label', 'jackioh.pc-runner=1', '--label', "jackioh.label=$label", '--label', "jackioh.runner-id=$($jit.runner.id)",
    '--cpus', "$containerCpus", '--memory', $memory, '--memory-swap', $memory,
    '--pids-limit', '8192', '--security-opt', 'no-new-privileges',
    '-e', "CARGO_BUILD_JOBS=$cpus", '-e', "RAYON_NUM_THREADS=$cpus", '--env-file', $envFile)
  if ($entry.home) { $argv += @('-v', "$($entry.home):/home/runner") }
  # Codex sandboxes its commands with bubblewrap, which needs the user namespaces Docker's default
  # seccomp profile refuses.
  if ($entry.seccompUnconfined) { $argv += @('--security-opt', 'seccomp=unconfined') }
  # A priority runner's cgroup weighs about five times another's (Docker 29 makes --cpu-shares 8192
  # cpu.weight 532, against the default 100), so while both want the VM's 8 CPUs it gets its own 6
  # first and the other the rest; alone, either has all of its --cpus. The runner
  # writes PIPELINE_JOB_OOMSCOREADJ into every step's oom_score_adj (500 when it is unset,
  # actions/runner's ProcessInvoker). The kernel kills the process with the highest score, its share
  # of the VM's memory in thousandths plus that value: 1000 for the rest puts any training step
  # above a priority step's 200 (at most 200 plus its own 6 GB's share), so when the VM runs out of
  # memory a training step goes first; the owner's own containers (0) score below either at the
  # same size. Gather takes the global size: its build of jackioh-server outgrew 3 GB (#2134).
  if ($entry.priority) { $argv += @('--cpu-shares', '8192', '-e', 'PIPELINE_JOB_OOMSCOREADJ=200') }
  else { $argv += @('-e', 'PIPELINE_JOB_OOMSCOREADJ=1000') }
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
  $cpus = if ($on) { $cfg.throttledCpus } else { Cpus-Of $runner.Label }
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

# A request body for `gh api --input`, as UTF-8 without the byte-order mark PowerShell 5.1's
# Set-Content writes, which GitHub's JSON parser refuses.
function Write-Body([string]$path, $body) {
  [System.IO.File]::WriteAllText($path, ($body | ConvertTo-Json -Depth 5 -Compress), (New-Object System.Text.UTF8Encoding $false))
}

# How far a busy runner's games have got, for the status issue's top chart: the `jackioh selfplay`
# (a learn shard's) or `jackioh stats` (gather's) it runs, its --games, --from and --patch, and the
# games its --out holds so far. Each writes one line a game as it finishes, a shard into a file of
# its range in the folder --out names, gather into the file itself. $null while it plays none (a
# build, the retrain, the gate). The path goes to sh as an argument, never as script.
function Get-Progress($runner) {
  $p = Run 'docker' @('exec', $runner.Id, 'pgrep', '-a', '-x', 'jackioh') 20
  if ($p.Code -ne 0 -or -not $p.Out) { return $null }
  $argv = @((@($p.Out -split "`n")[0]).Trim() -split '\s+')
  if ($argv.Count -lt 3 -or $argv[2] -notin 'selfplay', 'stats') { return $null }
  $opt = @{}
  for ($i = 3; $i -lt $argv.Count - 1; $i++) { if ($argv[$i] -like '--*') { $opt[$argv[$i]] = $argv[$i + 1] } }
  $games = 0; $done = 0; $from = 0
  if (-not $opt['--out'] -or -not [int]::TryParse("$($opt['--games'])", [ref]$games)) { return $null }
  $c = Run 'docker' @('exec', $runner.Id, 'sh', '-c', 'cat $1 $1/*.jsonl 2>/dev/null | wc -l', 'sh', $opt['--out']) 20
  if ($c.Code -ne 0 -or -not [int]::TryParse((@($c.Out -split "`n")[0]).Trim(), [ref]$done)) { return $null }
  $progress = [ordered]@{ kind = $argv[2]; games = $games; done = $done }
  if ([int]::TryParse("$($opt['--from'])", [ref]$from)) { $progress.from = $from }
  if ($opt['--patch']) { $progress.patch = $opt['--patch'] }
  $progress
}

# The heartbeat (see the top): pc.json in the gist pc-gist.txt names, made, and named to the
# repository as JACKIOH_PC_GIST, the first time.
function Send-Heartbeat($sample, [bool]$calm, $runners) {
  $cpus = @{}
  if ($runners.Count) {
    $s = Run 'docker' (@('stats', '--no-stream', '--format', '{{.ID}}|{{.CPUPerc}}') + @($runners | ForEach-Object Id))
    if ($s.Code -eq 0) {
      foreach ($line in ($s.Out -split "`n")) {
        $f = $line.Trim() -split '\|'
        if ($f.Count -ge 2) { $cpus[$f[0]] = [math]::Round([double]($f[1].TrimEnd('%').Trim()) / 100, 1) }
      }
    }
  }
  $beat = [ordered]@{
    at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    calm = $calm
    cpu = [math]::Round($sample.Cpu, 1)
    memGB = [math]::Round($sample.Mem, 1)
    cpuAll = [math]::Round($sample.CpuAll, 1)
    memAllGB = [math]::Round($sample.MemAll, 1)
    memTotalGB = [math]::Round($memTotalGB, 1)
    logical = $logical
    runners = @($runners | ForEach-Object {
      $progress = if ($_.Busy) { try { Get-Progress $_ } catch { $null } } else { $null }
      [ordered]@{ name = $_.Name; label = $_.Label; busy = [bool]$_.Busy; cpus = $cpus[$_.Id]; throttled = [bool]$throttled[$_.Id]; progress = $progress }
    })
  }
  $files = @{ 'pc.json' = @{ content = ($beat | ConvertTo-Json -Depth 5 -Compress) } }
  $bodyFile = Join-Path $env:TEMP 'jackioh-pc-heartbeat.json'
  $id = if (Test-Path $gistFile) { (Get-Content $gistFile -Raw).Trim() } else { '' }
  if ($id) {
    Write-Body $bodyFile @{ files = $files }
    $r = Run 'gh' @('api', '-X', 'PATCH', "gists/$id", '--input', $bodyFile)
    if ($r.Code -eq 0) { return }
    if ($r.Out -notmatch '404|Not Found') { Log "heartbeat not sent: $(@($r.Out -split "`n")[0])"; return }
    Log "the heartbeat's gist $id is gone; making another"
  }
  Write-Body $bodyFile @{ description = "JackiOh: this PC's heartbeat, for $repo's status issue"; public = $false; files = $files }
  $made = GhJson @('api', '-X', 'POST', 'gists', '--input', $bodyFile)
  Set-Content -Path $gistFile -Value $made.id -Encoding ascii -NoNewline
  $v = Run 'gh' @('variable', 'set', 'JACKIOH_PC_GIST', '--repo', $repo, '--body', $made.id)
  if ($v.Code -eq 0) { Log "heartbeat gist $($made.id) made, and named to $repo as JACKIOH_PC_GIST" }
  else { Log "heartbeat gist $($made.id) made, but not named to $repo ($(@($v.Out -split "`n")[0])): set the variable JACKIOH_PC_GIST to it" }
}

# The labels wanted now: of the shared ones (not `ownSlot`) at most `room`, one per queued job,
# then each standby label not already among them; and each ownSlot label with a job queued, once.
function Wanted([string[]]$queuedLabels, [string[]]$servable, [int]$room) {
  $want = New-Object System.Collections.ArrayList
  $shared = 0
  foreach ($label in $queuedLabels) {
    if (Own $label) { if ($label -notin $want) { [void]$want.Add($label) } }
    elseif ($shared -lt $room) { [void]$want.Add($label); $shared++ }
  }
  foreach ($entry in @($cfg.serve | Where-Object { $_.standby -and -not $_.ownSlot -and $_.label -in $servable })) {
    if ($shared -lt $room -and $entry.label -notin $want) { [void]$want.Add($entry.label); $shared++ }
  }
  , $want
}

$ownLabels = @($cfg.serve | Where-Object { $_.ownSlot } | ForEach-Object { $_.label })
$ownWords = if ($ownLabels.Count) { ", and one more each for $($ownLabels -join ', ')" } else { '' }
Log ("supervisor up on $repo, serving {0}; calm = CPU < {1}% and memory < {2} GB for {3}s, and a runner starts only at CPU < {5}%; at most {4} job(s){6}; no GPU" -f `
  (@($cfg.serve | ForEach-Object { $_.label }) -join ', '), $cfg.cpuPercentBelow, $cfg.memoryGBBelow, $cfg.calmSeconds, $cfg.maxJobs, $startCpuBelow, $ownWords)
$wasState = $null
while ($true) {
  $d = Run 'docker' @('info', '--format', '{{.ServerVersion}}') 30
  if ($d.Code -ne 0) {
    if ($wasState -ne 'nodocker') {
      Log "Docker is not answering ($(@($d.Out -split "`n")[0])); waiting"
      $wasState = 'nodocker'
    }
    Start-Sleep -Seconds 30
    continue
  }
  $runners = @(Get-Runners)
  $sample = Get-Sample $runners
  $samples.Enqueue($sample)
  while ($samples.Count -gt $windowSize) { [void]$samples.Dequeue() }
  $over = @($samples | Where-Object { $_.Cpu -ge $cfg.cpuPercentBelow -or $_.Mem -ge $cfg.memoryGBBelow })
  $calm = ($samples.Count -ge $windowSize) -and ($over.Count -eq 0)
  $quiet = $calm -and -not @($samples | Where-Object { $_.Cpu -ge $startCpuBelow }).Count
  # quiet: a runner may start; calm: one already up keeps going at full speed; busy: neither.
  $state = if ($quiet) { 'quiet' } elseif ($calm) { 'calm' } else { 'busy' }
  if ($state -ne $wasState) {
    Log ("{0}: CPU {1:N0}%, memory {2:N1} GB (without the runners and yielding containers)" -f $state, $sample.Cpu, $sample.Mem)
    $wasState = $state
  }

  if (((Get-Date) - $lastSweep).TotalMinutes -ge 10) { Remove-StaleRunners $runners; $lastSweep = Get-Date }

  $busy = @($runners | Where-Object Busy)
  $idle = @($runners | Where-Object { -not $_.Busy })
  foreach ($r in $busy) { Set-Throttle $r (-not $calm); $unwanted.Remove($r.Id) }

  if ($heartbeatSeconds -gt 0 -and ((Get-Date) - $lastHeartbeat).TotalSeconds -ge $heartbeatSeconds) {
    $lastHeartbeat = Get-Date
    try { Send-Heartbeat $sample $calm $runners } catch { Log "heartbeat failed: $_" }
  }

  if (-not $calm) {
    foreach ($r in $idle) { Stop-IdleRunner $r 'the PC is busy' }
    continue
  }
  # maxJobs holds the shared labels' runners alone; an ownSlot label's is outside it.
  $shared = @($runners | Where-Object { -not (Own $_.Label) })
  $room = [int]$cfg.maxJobs - @($shared | Where-Object Busy).Count
  if ($room -le 0) {
    foreach ($r in @($idle | Where-Object { -not (Own $_.Label) })) { Stop-IdleRunner $r 'no room for another job' }
    $idle = @($idle | Where-Object { Own $_.Label })
    if (-not $ownLabels.Count) { continue }
  }

  # With no room, only the ownSlot labels' workflows are read, and once a minute, not every 20 s: the
  # learn lane holds the shared slot nearly all day, and each read costs `gh` calls on the owner's
  # rate limit.
  $servable = @(Servable)
  $asked = if ($room -gt 0) { $servable } else { @($servable | Where-Object { Own $_ }) }
  $every = if ($room -gt 0) { 20 } else { 60 }
  if (((Get-Date) - $lastQueueCheck).TotalSeconds -ge $every) {
    try { $queued = @(Get-QueuedLabels $asked) } catch { Log "queue check failed: $_"; $queued = @() }
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
  # Idle runners count against maxJobs too, so two jobs landing at once never run past it; an
  # ownSlot label gets a runner only while it has none, busy or idle.
  $held = @($runners | ForEach-Object { $_.Label })
  $startable = @($missing | Where-Object {
      if (Own $_) { $_ -notin $held } else { $shared.Count -lt [int]$cfg.maxJobs } })
  if ($quiet -and $startable.Count -and (Get-Date) -ge $nextStart) {
    $label = $startable[0]
    $why = if ($label -in $queued) { "a queued $label job" } else { "standby for $label" }
    try { Start-Runner $label $why } catch { Log "start failed: $_" }
    $nextStart = (Get-Date).AddSeconds($startCooldownSeconds)
    $lastQueueCheck = [datetime]::MinValue
  }
}
