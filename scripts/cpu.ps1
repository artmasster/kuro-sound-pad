<#
How much processor the app uses: idle, and while a sound plays (the progress strip, the clock and the meter move).
Percent of one core, measured over a few seconds after the start-up work is done.

    pwsh scripts\cpu.ps1 [-Exe target\release\kuro-sound-pad.exe]
#>
param([string]$Exe = 'target\debug\kuro-sound-pad.exe')
$root = Split-Path $PSScriptRoot -Parent
function Measure-App([string]$name, [string]$autoplay) {
    $psi = [Diagnostics.ProcessStartInfo]::new((Join-Path $root $Exe))
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.Environment['KURO_DATA_DIR'] = Join-Path $root 'target\live-data'
    $psi.Environment['KURO_ALLOW_SECOND'] = '1'
    if ($autoplay) { $psi.Environment['KURO_AUTOPLAY'] = $autoplay }
    $p = [Diagnostics.Process]::Start($psi)
    try {
        Start-Sleep -Seconds 4
        $p.Refresh(); $a = $p.TotalProcessorTime.TotalMilliseconds
        Start-Sleep -Seconds 6
        $p.Refresh(); $b = $p.TotalProcessorTime.TotalMilliseconds
        Write-Host ("{0,-8} {1,5:N1}% of one core, {2:N0} MB" -f $name, (($b - $a) / 60), ($p.WorkingSet64 / 1MB))
    }
    finally { $p.Kill(); $p.WaitForExit() }
}
Measure-App 'idle' ''
Measure-App 'playing' '4'
