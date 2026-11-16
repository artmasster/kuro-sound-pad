<#
A picture of what the window really shows after running for a while, read from the window itself (PrintWindow asks
Windows for this one window's own content: nothing else on the screen can end up in the picture).

shots.ps1 redraws the whole window for its picture, so it cannot show mistakes that only build up when small parts
are repainted (a hover, a progress strip). This can: run a script of plays and hovers, then look.

    pwsh scripts\window-shot.ps1 -Autoplay '4,h1,h2,h5,h6,h1' -After 5 -Out target\shots\real.png
#>
param([string]$Autoplay = '', [double]$After = 4, [string]$Out = 'target\shots\real.png')
$root = Split-Path $PSScriptRoot -Parent
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace W -Name P -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool GetClientRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(System.IntPtr h, System.IntPtr dc, uint flags);
public struct RECT { public int L, T, R, B; }
'@
$psi = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'target\debug\kuro-sound-pad.exe'))
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.Environment['KURO_DATA_DIR'] = Join-Path $root 'target\live-data'
$psi.Environment['KURO_ALLOW_SECOND'] = '1'
if ($Autoplay) { $psi.Environment['KURO_AUTOPLAY'] = $Autoplay }
$p = [Diagnostics.Process]::Start($psi)
try {
    Start-Sleep -Milliseconds ([int]($After * 1000))
    $p.Refresh()
    $h = $p.MainWindowHandle
    if ($h -eq [IntPtr]::Zero) { throw 'the window did not open' }
    $c = New-Object W.P+RECT
    [void][W.P]::GetClientRect($h, [ref]$c)
    $bmp = New-Object Drawing.Bitmap $c.R, $c.B
    $g = [Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    # 1 = client area only, 2 = render the full content (works for windows that draw off-screen).
    $ok = [W.P]::PrintWindow($h, $dc, 3)
    $g.ReleaseHdc($dc)
    $bmp.Save((Join-Path $root $Out))
    Write-Host "$($c.R)x$($c.B) printed=$ok cpu=$([math]::Round($p.TotalProcessorTime.TotalSeconds, 2))s ws=$([math]::Round($p.WorkingSet64 / 1MB))MB"
}
finally { $p.Kill() }
