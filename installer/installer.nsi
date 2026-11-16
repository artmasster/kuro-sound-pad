; Kuro Sound Pad: Windows installer (per user, no administrator prompt).
;
;   makensis -DVERSION=2.0.0 -DEXE_PATH=<kuro-sound-pad.exe> -DLIBRARY_DIR=<assets/library> -DOUTFILE=<setup.exe> installer/installer.nsi
;
; Installs the app and its bundled sounds into %LOCALAPPDATA%\Programs\Kuro Sound Pad. The user's pads and sounds
; live in %APPDATA%\Kuro Sound Pad and are never touched, by an update or by the uninstaller.

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "pass -DVERSION=x.y.z"
!endif
!ifndef EXE_PATH
  !error "pass -DEXE_PATH=<path to kuro-sound-pad.exe>"
!endif
!ifndef LIBRARY_DIR
  !error "pass -DLIBRARY_DIR=<path to assets/library>"
!endif
!ifndef OUTFILE
  !define OUTFILE "KuroSoundPad-${VERSION}-win-x64-setup.exe"
!endif

; Signed releases (scripts/release-windows.ps1) sign the uninstaller as makensis writes it (NSIS 3.08+).
!ifdef SIGN_UNINSTALLER
  !uninstfinalize '${SIGN_UNINSTALLER} "%1"' = 0
!endif

!define PRODUCT "Kuro Sound Pad"
!define EXE "Kuro Sound Pad.exe"
!define UNINST "Uninstall Kuro Sound Pad.exe"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\KuroSoundPad"
!define INSTALL_KEY "Software\KuroSoundPad"

Name "${PRODUCT}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${PRODUCT}"
InstallDirRegKey HKCU "${INSTALL_KEY}" "InstallLocation"
BrandingText "${PRODUCT} ${VERSION}"
ShowInstDetails nevershow
ShowUninstDetails nevershow

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=0 "ProductName" "${PRODUCT}"
VIAddVersionKey /LANG=0 "FileDescription" "${PRODUCT} Setup"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "CompanyName" "Pawat Benjaoun"
VIAddVersionKey /LANG=0 "LegalCopyright" "Pawat Benjaoun"

!define MUI_ICON "../build/icon.ico"
!define MUI_UNICON "../build/icon.ico"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchApp
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Thai"

LangString AppRunning ${LANG_ENGLISH} "${PRODUCT} is running. Close it and click Retry, or Cancel to quit setup."
LangString AppRunning ${LANG_THAI} "${PRODUCT} ยังเปิดอยู่ ปิดแอปแล้วกด Retry หรือกด Cancel เพื่อออก"

; A running copy keeps its exe locked: ask the user to close it (a silent install gives up after half a minute).
!macro WaitForApp
  StrCpy $R2 0
  ${Do}
    ${IfNot} ${FileExists} "$INSTDIR\${EXE}"
      ${Break}
    ${EndIf}
    ClearErrors
    FileOpen $R3 "$INSTDIR\${EXE}" a
    ${IfNot} ${Errors}
      FileClose $R3
      ${Break}
    ${EndIf}
    IntOp $R2 $R2 + 1
    ${If} ${Silent}
      ${If} $R2 >= 60
        Abort
      ${EndIf}
    ${ElseIf} $R2 >= 4
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "$(AppRunning)" IDRETRY +2
      Abort
      StrCpy $R2 0
    ${EndIf}
    Sleep 500
  ${Loop}
!macroend

Section "Kuro Sound Pad" Main
  SetOutPath "$INSTDIR"
  !insertmacro WaitForApp
  File "/oname=${EXE}" "${EXE_PATH}"
  ; The bundled sounds are replaced as a whole, so one dropped from a later version does not linger.
  RMDir /r "$INSTDIR\library"
  SetOutPath "$INSTDIR\library"
  File "${LIBRARY_DIR}/*.*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\${UNINST}"

  CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\${EXE}"
  CreateShortcut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\${EXE}"

  WriteRegStr HKCU "${INSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${PRODUCT} ${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Pawat Benjaoun"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\${UNINST}" /currentuser'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\${UNINST}" /currentuser /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $R4 $R5 $R6
  IntFmt $R4 "0x%08X" $R4
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$R4"
SectionEnd

Function LaunchApp
  Exec '"$INSTDIR\${EXE}"'
FunctionEnd

Section "Uninstall"
  !insertmacro WaitForApp
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\${UNINST}"
  RMDir /r "$INSTDIR\library"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  Delete "$DESKTOP\${PRODUCT}.lnk"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "${INSTALL_KEY}"
SectionEnd
