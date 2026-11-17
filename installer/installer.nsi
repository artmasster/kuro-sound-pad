; Kuro Sound Pad: Windows installer (per user, no administrator prompt).
;
;   makensis -INPUTCHARSET UTF8 -DVERSION=2.1.1 -DEXE_PATH=<kuro-sound-pad.exe> -DLIBRARY_PACK=<assets/library.kpk>
;            -DART_DIR=<output of scripts/installer-art.py> -DOUTFILE=<setup.exe> installer/installer.nsi
;
; Three steps: a first page with one button, the copying, a last page that starts the app. It installs the app and
; its bundled sounds (one sealed file, library.kpk) into %LOCALAPPDATA%\Programs\Kuro Sound Pad. The user's pads and sounds live in
; %APPDATA%\Kuro Sound Pad and are never touched, by an update or by the uninstaller.
;
; The app updates itself by running a newer copy of this installer as `setup.exe /S --updated /D=<its folder>`:
; no pages, wait for the app to close, replace it, start it again.

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
!ifndef LIBRARY_PACK
  !error "pass -DLIBRARY_PACK=<path to assets/library.kpk>"
!endif
!ifndef ART_DIR
  !error "pass -DART_DIR=<folder written by scripts/installer-art.py>"
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
BrandingText " "
ShowInstDetails nevershow
ShowUninstDetails nevershow

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=0 "ProductName" "${PRODUCT}"
VIAddVersionKey /LANG=0 "FileDescription" "${PRODUCT} Setup"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "CompanyName" "Pawat Benjaoun"
VIAddVersionKey /LANG=0 "LegalCopyright" "Pawat Benjaoun"

; ---- the look: the app's own black, its logo and pads on the left of the first and last page ----------------------

!define MUI_ICON "..\build\icon.ico"
!define MUI_UNICON "..\build\icon.ico"
!define MUI_BGCOLOR "0B0B0D"
!define MUI_TEXTCOLOR "ECECEF"
; A themed check box ignores the text colour (it would be black on black): the one on the last page is drawn plain.
!define MUI_FORCECLASSICCONTROLS
; The whole window is that black, not only the first and last page (DarkWindow and DarkCopying below).
!define MUI_CUSTOMFUNCTION_GUIINIT DarkWindow
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.DarkWindow
; The picture is shown as drawn, never stretched: WizardArt below swaps in the one made for the screen's scale.
!define MUI_WELCOMEFINISHPAGE_BITMAP "${ART_DIR}\wizard-100.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP_STRETCH NoStretchNoCropNoAlign
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${ART_DIR}\wizard-100.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP_STRETCH NoStretchNoCropNoAlign

!define MUI_PAGE_CUSTOMFUNCTION_PRE WizardArt
!define MUI_WELCOMEPAGE_TITLE "${PRODUCT} ${VERSION}"
!define MUI_WELCOMEPAGE_TEXT "แผงเสียง 16 ปุ่ม กดที่แผงหรือกดปุ่มลัดแล้วเสียงออกทันที แม้กำลังอยู่ในเกมหรือแอปอื่น$\r$\n$\r$\nติดตั้งให้ผู้ใช้นี้เท่านั้น ไม่ต้องใช้สิทธิ์ผู้ดูแลระบบ ถ้าเคยใช้มาก่อน แผงเสียงและการตั้งค่าเดิมยังอยู่ครบ$\r$\n$\r$\nกด ติดตั้ง เพื่อเริ่ม"
!insertmacro MUI_PAGE_WELCOME

!define MUI_PAGE_CUSTOMFUNCTION_SHOW DarkCopying
!insertmacro MUI_PAGE_INSTFILES

!define MUI_PAGE_CUSTOMFUNCTION_PRE WizardArt
!define MUI_FINISHPAGE_NOREBOOTSUPPORT
!define MUI_FINISHPAGE_TEXT_LARGE
!define MUI_FINISHPAGE_TITLE "ติดตั้งเสร็จแล้ว"
!define MUI_FINISHPAGE_TEXT "${PRODUCT} พร้อมใช้งาน เปิดได้จากเมนู Start หรือไอคอนบนเดสก์ท็อป$\r$\n$\r$\nเมื่อมีรุ่นใหม่ แอปจะแจ้งและอัปเดตให้เองเมื่อกดยืนยัน"
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "เปิด ${PRODUCT}"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchApp
!insertmacro MUI_PAGE_FINISH

!define MUI_PAGE_CUSTOMFUNCTION_PRE un.WizardArt
!define MUI_WELCOMEPAGE_TITLE "ถอนการติดตั้ง ${PRODUCT}"
!define MUI_WELCOMEPAGE_TEXT "ตัวแอปและคลังเสียงที่มากับแอปจะถูกลบออกจากเครื่อง$\r$\n$\r$\nแผงเสียง ไฟล์เสียงที่เพิ่มเอง และการตั้งค่าของคุณจะไม่ถูกลบ ติดตั้งใหม่เมื่อไรก็กลับมาเหมือนเดิม$\r$\n$\r$\nกด ถอนการติดตั้ง เพื่อเริ่ม"
!insertmacro MUI_UNPAGE_WELCOME

!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.DarkCopying
!insertmacro MUI_UNPAGE_INSTFILES

!define MUI_PAGE_CUSTOMFUNCTION_PRE un.WizardArt
!define MUI_FINISHPAGE_TITLE "ถอนการติดตั้งแล้ว"
!define MUI_FINISHPAGE_TEXT "${PRODUCT} ถูกลบออกจากเครื่องแล้ว$\r$\n$\r$\nแผงเสียงและการตั้งค่าของคุณยังอยู่ที่ $APPDATA\${PRODUCT} ลบโฟลเดอร์นั้นเองได้ถ้าไม่ต้องการแล้ว"
!insertmacro MUI_UNPAGE_FINISH

; The app speaks Thai only, and so does its installer.
!insertmacro MUI_LANGUAGE "Thai"

; Puts the picture drawn for this screen's scale in place of the 100 % one the page would load. On a Windows too old
; to say what the scale is, the call fails and the 100 % picture stays.
!macro WizardArtFunction UN
  Function ${UN}WizardArt
    Push $0
    System::Call 'user32::GetDpiForWindow(p $HWNDPARENT) i .r0'
    ${If} $0 >= 192
      File "/oname=$PLUGINSDIR\modern-wizard.bmp" "${ART_DIR}\wizard-200.bmp"
    ${ElseIf} $0 >= 168
      File "/oname=$PLUGINSDIR\modern-wizard.bmp" "${ART_DIR}\wizard-175.bmp"
    ${ElseIf} $0 >= 144
      File "/oname=$PLUGINSDIR\modern-wizard.bmp" "${ART_DIR}\wizard-150.bmp"
    ${ElseIf} $0 >= 120
      File "/oname=$PLUGINSDIR\modern-wizard.bmp" "${ART_DIR}\wizard-125.bmp"
    ${EndIf}
    Pop $0
  FunctionEnd
!macroend
!insertmacro WizardArtFunction ""
!insertmacro WizardArtFunction "un."

!macro DarkFunctions UN
  Function ${UN}DarkWindow
    SetCtlColors $HWNDPARENT "" "${MUI_BGCOLOR}"
    SetCtlColors $mui.Branding.Background "" "${MUI_BGCOLOR}"
    SetCtlColors $mui.Branding.Text "${MUI_BGCOLOR}" "${MUI_BGCOLOR}"
    ; The rules across the window are drawn in the system's light colours, and there is never a page to go back
    ; to. The pages show and hide these as they come and go, so they are given no size instead of being hidden.
    Push $0
    System::Call 'user32::MoveWindow(p $mui.Line.Standard, i 0, i 0, i 0, i 0, i 0)'
    System::Call 'user32::MoveWindow(p $mui.Line.FullWindow, i 0, i 0, i 0, i 0, i 0)'
    System::Call 'user32::MoveWindow(p $mui.Button.Back, i 0, i 0, i 0, i 0, i 0)'
    GetDlgItem $0 $HWNDPARENT 1036 ; the rule under the header
    System::Call 'user32::MoveWindow(p $0, i 0, i 0, i 0, i 0, i 0)'
    Pop $0
  FunctionEnd

  ; The copying page: its own background and the line that says what is being copied.
  Function ${UN}DarkCopying
    SetCtlColors $mui.InstFilesPage "" "${MUI_BGCOLOR}"
    SetCtlColors $mui.InstFilesPage.Text "${MUI_TEXTCOLOR}" "${MUI_BGCOLOR}"
    ; A themed progress bar is the system's green: drawn plain, it takes the app's amber (colours are 0xBBGGRR).
    System::Call 'uxtheme::SetWindowTheme(p $mui.InstFilesPage.ProgressBar, w " ", w " ")'
    SendMessage $mui.InstFilesPage.ProgressBar 0x0409 0 0x27B6FF
    SendMessage $mui.InstFilesPage.ProgressBar 0x2001 0 0x201C1C
  FunctionEnd
!macroend
!insertmacro DarkFunctions ""
!insertmacro DarkFunctions "un."

; ---- installing ---------------------------------------------------------------------------------------------------

; 1 when the app itself started this installer to update: no pages, and the app is started again at the end.
Var Updated

Function .onInit
  StrCpy $Updated 0
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "--updated" $R1
  ${IfNot} ${Errors}
    StrCpy $Updated 1
  ${EndIf}
FunctionEnd

; A running copy keeps its exe locked. An update waits for the app that started it to close (up to half a minute);
; someone installing by hand is asked to close it.
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
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "${PRODUCT} ยังเปิดอยู่ ปิดแอปแล้วกด Retry หรือกด Cancel เพื่อออก" IDRETRY +2
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
  File "/oname=library.kpk" "${LIBRARY_PACK}"
  ; Versions before 2.1.1 kept the bundled sounds as a folder of audio files.
  RMDir /r "$INSTDIR\library"
  WriteUninstaller "$INSTDIR\${UNINST}"

  CreateShortcut "$SMPROGRAMS\${PRODUCT}.lnk" "$INSTDIR\${EXE}"
  ; An update leaves the desktop alone: the user may have removed the icon there.
  ${If} $Updated == 0
    CreateShortcut "$DESKTOP\${PRODUCT}.lnk" "$INSTDIR\${EXE}"
  ${EndIf}

  WriteRegStr HKCU "${INSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${PRODUCT}"
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

  ${If} $Updated == 1
    Call LaunchApp
  ${EndIf}
SectionEnd

Function LaunchApp
  Exec '"$INSTDIR\${EXE}"'
FunctionEnd

Section "Uninstall"
  !insertmacro WaitForApp
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\${UNINST}"
  Delete "$INSTDIR\library.kpk"
  RMDir /r "$INSTDIR\library"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${PRODUCT}.lnk"
  Delete "$DESKTOP\${PRODUCT}.lnk"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "${INSTALL_KEY}"
  ; A downloaded update that never got installed.
  RMDir /r "$APPDATA\${PRODUCT}\update"
SectionEnd
