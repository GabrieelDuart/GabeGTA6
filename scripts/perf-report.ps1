# Sums up a run's log: frame rate, where a frame's time goes, video memory, what was
# refused or dropped, and how the run ended, with what to try next.
#   scripts\perf-report.ps1 <log>                 the whole run
#   scripts\perf-report.ps1 <log> -LastSeconds 60  only its last minute (the steady part)
#   scripts\perf-report.ps1 <log> -Follow          again every 10 s while the title runs
# The log is what play-1060.ps1 printed (*>&1 | Out-File): its lines may be wrapped,
# so a record is a line with a time stamp and the lines after it without one.
param(
    [Parameter(Mandatory)][string]$Log,
    [int]$LastSeconds = 0,
    [switch]$Follow
)

function Read-Records([string]$path) {
    $records = New-Object System.Collections.Generic.List[string]
    $current = $null
    foreach ($line in [IO.File]::ReadLines((Resolve-Path $path).Path)) {
        if ($line -match '^\d{4}-\d\d-\d\dT[\d:.]+Z ') {
            if ($current) { $records.Add($current) }
            $current = $line
        } elseif ($current) {
            $current += ' ' + $line.Trim()
        }
    }
    if ($current) { $records.Add($current) }
    $records
}

function Get-Median([double[]]$xs) {
    if (-not $xs -or $xs.Count -eq 0) { return $null }
    $s = $xs | Sort-Object
    $s[[int][Math]::Floor(($s.Count - 1) / 2)]
}

function Show-Report {
    $copy = Join-Path ([IO.Path]::GetTempPath()) 'victory-perf-report.txt'
    # The run still writes it: read a copy.
    Copy-Item $Log $copy -Force
    $records = Read-Records $copy
    $time = { param($r) [datetime]::Parse($r.Substring(0, $r.IndexOf('Z') + 1)).ToUniversalTime() }
    $last = if ($records.Count) { & $time $records[$records.Count - 1] } else { [datetime]::UtcNow }
    $from = if ($LastSeconds -gt 0) { $last.AddSeconds(-$LastSeconds) } else { [datetime]::MinValue }

    $frames = @(); $busy = @(); $vram = $null; $refused = 0; $failure = $null; $draws = @()
    foreach ($r in $records) {
        if ($r -match 'host pipeline refused|program refused') { $refused++ }
        if ($r -match 'host GPU failure|device removed|did not close') { $failure = $r }
        if ((& $time $r) -lt $from) { continue }
        if ($r -match 'victory_gpu::host: ([\d.]+) fps, ([\d.]+) ms/frame \(([\d.]+) preparing.*?; ([\d.]+) on the GPU; ([\d.]+) copying back.*?\), (\d+) draws/frame, dropped (\d+) without a host pipeline.*? and (\d+) indirect') {
            $frames += [pscustomobject]@{
                Fps = [double]$Matches[1]; Ms = [double]$Matches[2]; Prepare = [double]$Matches[3]
                Gpu = [double]$Matches[4]; Back = [double]$Matches[5]; Draws = [int]$Matches[6]
                NoPipeline = [int]$Matches[7]; Indirect = [int]$Matches[8]
            }
        }
        if ($r -match "the card's clock per frame: nothing of ours queued ([\d.]+) ms \(the card about (\d+)% busy") {
            $busy += [double]$Matches[2]
        }
        if ($r -match "the card's own: budget (\d+) MB, in use (\d+) MB.*?the PC's shared with the card: budget (\d+) MB, in use (\d+) MB") {
            $vram = [pscustomobject]@{ Budget = [int]$Matches[1]; Used = [int]$Matches[2]; SharedBudget = [int]$Matches[3]; Shared = [int]$Matches[4] }
        }
    }

    Write-Host ("== {0} ({1} of log{2})" -f (Split-Path $Log -Leaf), $last.ToLocalTime().ToString('HH:mm:ss'),
        $(if ($LastSeconds -gt 0) { ", last $LastSeconds s" } else { '' }))
    if (-not $frames) { Write-Host 'No frame lines yet (the title is still starting, or the log has none).'; return }
    $fps = Get-Median ($frames | ForEach-Object Fps)
    $ms = Get-Median ($frames | ForEach-Object Ms)
    $gpu = Get-Median ($frames | ForEach-Object Gpu)
    $prep = Get-Median ($frames | ForEach-Object Prepare)
    $back = Get-Median ($frames | ForEach-Object Back)
    $dr = Get-Median ($frames | ForEach-Object Draws)
    $nop = Get-Median ($frames | ForEach-Object NoPipeline)
    Write-Host ("frames      {0} fps median ({1:N0} ms a frame), best {2} fps, worst {3} fps, {4} samples" -f
        $fps, $ms, ($frames | Measure-Object Fps -Maximum).Maximum, ($frames | Measure-Object Fps -Minimum).Minimum, $frames.Count)
    Write-Host ("a frame     {0:N0} ms on the card, {1:N0} ms preparing on the processor, {2:N0} ms copying back" -f $gpu, $prep, $back)
    if ($busy) { Write-Host ("card busy   about {0}% by our count" -f (Get-Median $busy)) }
    Write-Host ("draws       {0} a frame, {1} dropped without a pipeline" -f $dr, $nop)
    if ($vram) {
        Write-Host ("video mem   {0} of {1} MB on the card; {2} MB of the PC's memory used by the card" -f $vram.Used, $vram.Budget, $vram.Shared)
    }
    Write-Host ("refused     {0} pipelines or programs the driver refused (whole run)" -f $refused)
    if ($failure) { Write-Host ("ended       " + $failure.Substring(0, [Math]::Min(300, $failure.Length))) }

    Write-Host '-- what to try'
    $any = $false
    if ($vram -and $vram.Used -ge 0.93 * $vram.Budget) {
        Write-Host '* The card''s memory is full, so textures and buffers live in the PC''s memory and are read over the bus: -MipDrop 3, -Scale 0.5, close the browser.'
        $any = $true
    }
    if ($gpu -ge 0.6 * $ms) {
        Write-Host ('* The card is the bottleneck ({0:N0} of {1:N0} ms): -Scale 0.5; mesh draws (each a dispatch, barriers and a draw on this card) are the next suspect.' -f $gpu, $ms)
        $any = $true
    } elseif ($prep -ge 0.4 * $ms) {
        Write-Host ('* The processor is the bottleneck ({0:N0} ms preparing): keep -Safe off; VICTORY_THIN_THREADS.' -f $prep)
        $any = $true
    }
    if ($back -ge 20) { Write-Host ('* {0:N0} ms a frame copying what the card wrote back to the title''s memory.' -f $back); $any = $true }
    if ($refused -gt 0) { Write-Host '* Some pipelines are refused: run once with -More @{VICTORY_D3D12_DEBUG=''1''} to see why.'; $any = $true }
    if (-not $any) { Write-Host '* Nothing stands out.' }
}

if ($Follow) {
    while ($true) {
        Clear-Host
        Show-Report
        if (-not (Get-Process victory -ErrorAction SilentlyContinue)) { Write-Host '(the title is not running)'; break }
        Start-Sleep -Seconds 10
    }
} else {
    Show-Report
}
