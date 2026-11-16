<#
Pictures of the window in its main states, for checking the layout without looking at the screen.

    pwsh scripts\shots.ps1                                  # every state, into target\shots
    pwsh -Command "& scripts\shots.ps1 -States 'drag','playing,select'"

A state is one or more of the names `App::boot` knows (select, empty, playing, preview, search, capture, drag, swap,
error, welcome, help, clear), joined by commas. Uses the settings in target\live-data (so the real ones are never
touched) and the debug build.
#>
param([string[]]$States = @('main', 'playing,select', 'empty', 'drag', 'swap', 'capture', 'search,preview', 'error', 'welcome', 'help', 'clear'))
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\debug\kuro-sound-pad.exe'
$out = Join-Path $root 'target\shots'
New-Item -ItemType Directory -Force $out | Out-Null
foreach ($s in $States) {
    $name = $s -replace ',', '-'
    $png = Join-Path $out "$name.png"
    Remove-Item $png -ErrorAction SilentlyContinue
    $psi = [Diagnostics.ProcessStartInfo]::new($exe)
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.Environment['KURO_DATA_DIR'] = Join-Path $root 'target\live-data'
    $psi.Environment['KURO_SHOT'] = $png
    $psi.Environment['KURO_SHOT_STATE'] = $s
    $psi.Environment['KURO_ALLOW_SECOND'] = '1'
    $p = [Diagnostics.Process]::Start($psi)
    if (-not $p.WaitForExit(20000)) { $p.Kill(); Write-Host "${name}: timed out" } else { Write-Host "${name}: $(Test-Path $png)" }
}
