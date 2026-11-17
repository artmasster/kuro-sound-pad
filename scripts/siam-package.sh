#!/usr/bin/env bash
# Runs ON the Siam host, in the folder release-windows.ps1 unpacked there: signs the app, builds the installer and
# signs it, so that every program file a user ends up with carries the publisher's signature:
#
#   the app                 signed here, before it goes into the installer
#   the uninstaller         signed by makensis as it writes it (the installer script's SIGN_UNINSTALLER)
#   the installer's helpers NSIS unpacks its plug-ins (DLLs) into a temporary folder while it runs; it gets signed
#                           copies of the two this installer uses
#   the installer           signed last, with all of the above inside
#
#   bash siam-package.sh <version> <setup file name>
#
# Leaves <setup file name> and signed/ (the app and the plug-ins, to be checked on Windows) in this folder.
set -euo pipefail
DIR=$(cd "$(dirname "$0")" && pwd)
VERSION=$1
SETUP=$2
SIGN="bash $DIR/siam-sign.sh"
STOCK=${NSIS_STOCK:-/usr/share/nsis}
PLUGINS="System nsDialogs"

$SIGN "$DIR/kuro-sound-pad.exe"

# A private copy of the NSIS data folder holding only the plug-ins this installer uses, signed. A plug-in the script
# starts using later is missing here, so the build stops instead of shipping it unsigned. The signed copies are
# kept between releases (they only change with NSIS itself): signing them again every time would be waste.
KEEP="$HOME/.cache/kuro-sound-pad/nsis-$(makensis -VERSION)-plugins"
mkdir -p "$KEEP" "$DIR/signed"
cp -r "$STOCK" "$DIR/nsis"
find "$DIR/nsis/Plugins" -name '*.dll' -delete
for p in $PLUGINS; do
  if [ ! -f "$KEEP/$p.dll" ]; then
    cp "$STOCK/Plugins/x86-unicode/$p.dll" "$DIR/$p.dll"
    $SIGN "$DIR/$p.dll"
    mv "$DIR/$p.dll" "$KEEP/$p.dll"
  fi
  cp "$KEEP/$p.dll" "$DIR/nsis/Plugins/x86-unicode/$p.dll"
  cp "$KEEP/$p.dll" "$DIR/signed/$p.dll"
done

cd "$DIR/installer"
NSISDIR="$DIR/nsis" makensis -V2 -INPUTCHARSET UTF8 \
  "-DVERSION=$VERSION" \
  "-DEXE_PATH=$DIR/kuro-sound-pad.exe" \
  "-DLIBRARY_PACK=$DIR/library.kpk" \
  "-DART_DIR=$DIR/art" \
  "-DOUTFILE=$DIR/$SETUP" \
  "-DSIGN_UNINSTALLER=$SIGN" \
  installer.nsi
$SIGN "$DIR/$SETUP"
cp "$DIR/kuro-sound-pad.exe" "$DIR/signed/kuro-sound-pad.exe"
