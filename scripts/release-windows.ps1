<#
Signed Windows release: builds the app here, then on the Siam host (`ssh siam`, SSL.com eSigner certificate of
Pawat Benjaoun) signs it, builds the installer around it and signs that too (scripts/siam-package.sh says what
exactly gets signed). Brings the installer back, checks every signature and writes the update feed for it.

    pwsh scripts\release-windows.ps1              # dist\KuroSoundPad-<ver>-win-x64-setup.exe + dist\latest.yml
    pwsh scripts\release-windows.ps1 -Upload      # also uploads both to the GitHub release v<ver> (a draft if new)
    pwsh scripts\release-windows.ps1 -Publish     # uploads, then publishes the release as the latest

Installed copies find a new version through the latest published release: nothing reaches them before -Publish.

Needs: rustup (stable-x86_64-pc-windows-gnu) + WinLibs on PATH for the build, python with Pillow for the installer's
picture, `ssh siam` with the signing workspace, and for -Upload / -Publish the GitHub CLI logged in.
#>
param([switch]$Upload, [switch]$Publish)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$repo = 'artmasster/kuro-sound-pad'
$publisher = 'Pawat Benjaoun'
$version = (Select-String Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$setup = "KuroSoundPad-$version-win-x64-setup.exe"
Write-Host "== Kuro Sound Pad $version"

Write-Host '== build'
cargo build --release --locked
if ($LASTEXITCODE) { throw 'cargo build failed' }

Write-Host '== everything the Siam host needs, in one file'
$id = -join ((48..57) + (97..122) | Get-Random -Count 10 | ForEach-Object { [char]$_ })
$stage = Join-Path $root "target\release-stage-$id"
$pack = Join-Path $root "target\ksp-$id.tar"
$remote = "/tmp/ksp-release-$id"
New-Item -ItemType Directory -Force "$stage\installer", "$stage\build" | Out-Null
try {
    Copy-Item target\release\kuro-sound-pad.exe $stage
    Copy-Item installer\installer.nsi "$stage\installer"
    Copy-Item build\icon.ico "$stage\build"
    Copy-Item scripts\siam-sign.sh, scripts\siam-package.sh $stage
    Copy-Item assets\library "$stage\library" -Recurse
    python -I scripts\installer-art.py "$stage\art"
    if ($LASTEXITCODE) { throw 'drawing the installer picture failed' }
    tar -cf $pack -C $stage .
    if ($LASTEXITCODE) { throw 'packing failed' }

    Write-Host '== sign and package on the Siam host'
    # The Siam host rate-limits SSH (about 6 connections per 30 s), so this uses four: upload, sign + package,
    # download, clean up.
    scp -q $pack "siam:/tmp/ksp-$id.tar"
    if ($LASTEXITCODE) { throw 'upload failed' }
    ssh siam "mkdir -p $remote && tar -xf /tmp/ksp-$id.tar -C $remote && bash $remote/siam-package.sh $version $setup"
    if ($LASTEXITCODE) { throw 'signing or packaging on the Siam host failed' }
    scp -q -r "siam:$remote/$setup" "siam:$remote/signed" $stage
    if ($LASTEXITCODE) { throw 'download failed' }

    Write-Host '== check signatures'
    # The uninstaller is not among these: it only exists once installed. makensis stops the build if signing it fails.
    $files = @("$stage\$setup") + (Get-ChildItem "$stage\signed" -File).FullName
    foreach ($f in $files) {
        $s = Get-AuthenticodeSignature $f
        Write-Host ("{0}: {1} ({2})" -f (Split-Path $f -Leaf), $s.Status, $s.SignerCertificate.Subject)
        if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch "CN=$publisher(,|$)") { throw "$f is not validly signed by $publisher" }
    }
    New-Item -ItemType Directory -Force dist | Out-Null
    Copy-Item "$stage\$setup" "dist\$setup" -Force
}
finally {
    ssh siam "rm -rf $remote /tmp/ksp-$id.tar"
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
    Remove-Item -Force $pack -ErrorAction SilentlyContinue
}

Write-Host '== latest.yml'
python -I scripts\latest-yml.py $version dist\latest.yml "dist\$setup"
if ($LASTEXITCODE) { throw 'writing latest.yml failed' }
Get-Item "dist\$setup" | ForEach-Object { Write-Host ("{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB)) }

if ($Upload -or $Publish) {
    gh release view "v$version" --repo $repo *> $null
    if ($LASTEXITCODE) {
        gh release create "v$version" --draft --title "Kuro Sound Pad $version" --notes '' --repo $repo
        if ($LASTEXITCODE) { throw 'creating the release failed' }
    }
    gh release upload "v$version" "dist\$setup" dist\latest.yml --clobber --repo $repo
    if ($LASTEXITCODE) { throw 'upload failed' }
}
if ($Publish) {
    gh release edit "v$version" --draft=false --latest --repo $repo
    if ($LASTEXITCODE) { throw 'publish failed' }
}
