<#
Starts the debug build on the test settings (target\live-data), plays pads the way hotkeys would (overlapping,
restarting, stop all, one left to end by itself) and prints what the audio thread reported. Sound comes out of the
default device, so keep master_volume low in target\live-data\settings.json.
#>
param([string]$Script = '0,1,0,stop,2,7', [int]$Seconds = 8)
$root = Split-Path $PSScriptRoot -Parent
$psi = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'target\debug\kuro-sound-pad.exe'))
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.RedirectStandardError = $true
$psi.Environment['KURO_DATA_DIR'] = Join-Path $root 'target\live-data'
$psi.Environment['KURO_LOG'] = '1'
$psi.Environment['KURO_AUTOPLAY'] = $Script
$psi.Environment['KURO_ALLOW_SECOND'] = '1'
$p = [Diagnostics.Process]::Start($psi)
$err = $p.StandardError.ReadToEndAsync()
Start-Sleep -Seconds $Seconds
$p.Kill()
$p.WaitForExit()
$err.Result
