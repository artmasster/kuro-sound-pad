<#
Signed Windows release: builds the exe here, signs it on the Siam host (`ssh siam`, SSL.com eSigner certificate of
Pawat Benjaoun, through siam-sign.sh), builds the installer there with the uninstaller signed as makensis writes it,
signs the installer, brings both back, checks the signatures and zips a portable copy.

    pwsh scripts\release-windows.ps1              # dist\KuroSoundPad-<ver>-win-x64-setup.exe + -portable.zip
    pwsh scripts\release-windows.ps1 -Upload      # also uploads both to the GitHub release v<ver> (created as a draft if missing)

Needs: rustup (stable-x86_64-pc-windows-gnu) + WinLibs on PATH for the build, `ssh siam` with the signing workspace,
and for -Upload the GitHub CLI logged in.
#>
param([switch]$Upload)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$version = (Select-String Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$setup = "KuroSoundPad-$version-win-x64-setup.exe"
$portable = "KuroSoundPad-$version-win-x64-portable.zip"
Write-Host "== Kuro Sound Pad $version"

Write-Host '== build'
cargo build --release --locked
if ($LASTEXITCODE) { throw 'cargo build failed' }

Write-Host '== sign on the Siam host'
# The Siam host rate-limits SSH (about 6 connections per 30 s), so this uses four: upload, sign + package,
# download, clean up.
$id = -join ((48..57) + (97..122) | Get-Random -Count 10 | ForEach-Object { [char]$_ })
$remote = "/tmp/ksp-release-$id"
$stage = Join-Path $root "target\release-stage-$id"
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target\release\kuro-sound-pad.exe "$stage\ksp-$id-kuro-sound-pad.exe"
Copy-Item installer\installer.nsi "$stage\ksp-$id-installer.nsi"
Copy-Item build\icon.ico "$stage\ksp-$id-icon.ico"
Copy-Item scripts\siam-sign.sh "$stage\ksp-$id-siam-sign.sh"
tar -cf "$stage\ksp-$id-library.tar" -C assets library
if ($LASTEXITCODE) { throw 'packing the sound library failed' }
$steps = @(
    'set -e',
    "mkdir -p $remote/installer $remote/build",
    "mv /tmp/ksp-$id-kuro-sound-pad.exe $remote/kuro-sound-pad.exe",
    "mv /tmp/ksp-$id-installer.nsi $remote/installer/installer.nsi",
    "mv /tmp/ksp-$id-icon.ico $remote/build/icon.ico",
    "mv /tmp/ksp-$id-siam-sign.sh $remote/siam-sign.sh",
    "tar -xf /tmp/ksp-$id-library.tar -C $remote",
    "bash $remote/siam-sign.sh $remote/kuro-sound-pad.exe",
    "cd $remote/installer",
    "makensis -V2 -DVERSION=$version -DEXE_PATH=$remote/kuro-sound-pad.exe -DLIBRARY_DIR=$remote/library -DOUTFILE=$remote/$setup '-DSIGN_UNINSTALLER=bash $remote/siam-sign.sh' installer.nsi",
    "bash $remote/siam-sign.sh $remote/$setup",
    ''
) -join "`n"
# LF line endings: the steps run under bash on the host.
[IO.File]::WriteAllText("$stage\ksp-$id-steps.sh", $steps)
try {
    scp -q (Get-ChildItem $stage).FullName 'siam:/tmp/'
    if ($LASTEXITCODE) { throw 'upload failed' }
    ssh siam "bash /tmp/ksp-$id-steps.sh"
    if ($LASTEXITCODE) { throw 'signing or packaging on the Siam host failed' }
    New-Item -ItemType Directory -Force dist | Out-Null
    scp -q "siam:$remote/$setup" "siam:$remote/kuro-sound-pad.exe" $stage
    if ($LASTEXITCODE) { throw 'download failed' }
    Copy-Item "$stage\$setup" "dist\$setup" -Force
    Copy-Item "$stage\kuro-sound-pad.exe" "dist\kuro-sound-pad-$version-signed.exe" -Force
}
finally {
    ssh siam "rm -rf $remote /tmp/ksp-$id-*"
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
}

Write-Host '== check signatures'
foreach ($f in "dist\$setup", "dist\kuro-sound-pad-$version-signed.exe") {
    $s = Get-AuthenticodeSignature $f
    Write-Host ("{0}: {1} ({2})" -f (Split-Path $f -Leaf), $s.Status, $s.SignerCertificate.Subject)
    if ($s.Status -ne 'Valid') { throw "$f is not validly signed" }
}

Write-Host '== portable zip'
# The signed exe with the sounds next to it: unzip anywhere and run.
$folder = "dist\portable\Kuro Sound Pad"
Remove-Item -Recurse -Force dist\portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $folder | Out-Null
Copy-Item "dist\kuro-sound-pad-$version-signed.exe" "$folder\Kuro Sound Pad.exe"
Copy-Item assets\library "$folder\library" -Recurse
Remove-Item "dist\$portable" -ErrorAction SilentlyContinue
Compress-Archive -Path $folder -DestinationPath "dist\$portable" -CompressionLevel Optimal
Remove-Item -Recurse -Force dist\portable
Get-ChildItem dist | ForEach-Object { Write-Host ("{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB)) }

if ($Upload) {
    gh release view "v$version" --repo artmasster/kuro-sound-pad *> $null
    if ($LASTEXITCODE) {
        gh release create "v$version" --draft --title "Kuro Sound Pad $version" --notes '' --repo artmasster/kuro-sound-pad
        if ($LASTEXITCODE) { throw 'creating the release failed' }
    }
    gh release upload "v$version" "dist\$setup" "dist\$portable" --clobber --repo artmasster/kuro-sound-pad
    if ($LASTEXITCODE) { throw 'upload failed' }
}
