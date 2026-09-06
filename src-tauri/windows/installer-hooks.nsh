; AstroDeck Windows installer hooks.
; Upgrade in place and avoid duplicate shortcuts after renames or reinstalls.

; Recreate a shortcut so Explorer re-reads the EXE icon. Deleting is OK for
; desktop/start menu; do not use this on pinned taskbar links (it unpins).
!macro RecreateShortcutIfExists LinkPath
  ${If} ${FileExists} "${LinkPath}"
    Delete "${LinkPath}"
    CreateShortcut "${LinkPath}" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
    !insertmacro SetLnkAppUserModelId "${LinkPath}"
  ${EndIf}
!macroend

; Update path + icon on an existing .lnk without deleting it (keeps taskbar pins).
!macro RefreshShortcutIcon LinkPath
  ${If} ${FileExists} "${LinkPath}"
    !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
    ${If} $0 P<> 0
      ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
      ${If} $1 P<> 0
        ${IPersistFile::Load} $1 '("${LinkPath}", ${STGM_READWRITE})'
        ${IShellLink::SetPath} $0 '(w "$INSTDIR\${MAINBINARYNAME}.exe")'
        ${IShellLink::SetIconLocation} $0 '(w "$INSTDIR\${MAINBINARYNAME}.exe", 0)'
        ${IPersistFile::Save} $1 '("${LinkPath}",1)'
        ${IUnknown::Release} $1 ""
      ${EndIf}
      ${IUnknown::Release} $0 ""
    ${EndIf}
    !insertmacro SetLnkAppUserModelId "${LinkPath}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; Reuse a previous install directory (AstroDeck or legacy TapTapDeck).
  ReadRegStr $R4 SHCTX "${MANUPRODUCTKEY}" ""
  ${If} $R4 == ""
    ReadRegStr $R4 HKCU "Software\taptapdeck\TapTapDeck" ""
  ${EndIf}
  ${If} $R4 != ""
    StrCpy $INSTDIR $R4
  ${EndIf}

  ; Replace legacy TapTapDeck NSIS installs instead of installing side-by-side.
  ReadRegStr $R0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TapTapDeck" "UninstallString"
  ${If} $R0 != ""
    ReadRegStr $R1 HKCU "Software\taptapdeck\TapTapDeck" ""
    ${If} $R1 == ""
      ReadRegStr $R1 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TapTapDeck" "InstallLocation"
    ${EndIf}
    ${If} $R1 != ""
      StrCpy $INSTDIR $R1
      ExecWait '$R0 /S _?=$R1' $2
    ${Else}
      ExecWait '$R0 /S' $2
    ${EndIf}
  ${EndIf}

  ; Remove legacy TapTapDeck MSI installs (different upgrade code after rename).
  StrCpy $R5 0
  legacy_msi_loop:
    EnumRegKey $R6 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $R5
    StrCmp $R6 "" legacy_msi_done
    IntOp $R5 $R5 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$R6" "DisplayName"
    StrCmp $R0 "TapTapDeck" 0 legacy_msi_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$R6" "UninstallString"
    StrCmp $R0 "" legacy_msi_loop
    ExecWait '$R0 /quiet' $2
    Goto legacy_msi_done
  legacy_msi_done:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Remove rename leftovers so desktop/start menu do not keep duplicate shortcuts.
  Delete "$DESKTOP\TapTapDeck.lnk"
  Delete "$SMPROGRAMS\TapTapDeck.lnk"

  ; Tauri skips rewriting shortcuts on upgrade, so Explorer keeps the old icon.
  !insertmacro RecreateShortcutIfExists "$DESKTOP\${PRODUCTNAME}.lnk"
  !insertmacro RecreateShortcutIfExists "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !insertmacro RecreateShortcutIfExists "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
  !insertmacro RecreateShortcutIfExists "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"

  !insertmacro RefreshShortcutIcon "$APPDATA\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\${PRODUCTNAME}.lnk"

  ; SHCNE_ASSOCCHANGED | SHCNF_FLUSH: drop Explorer's icon cache for this app.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, i 0, i 0)'
  ${If} ${FileExists} "$SYSDIR\ie4uinit.exe"
    ExecWait '"$SYSDIR\ie4uinit.exe" -show'
  ${EndIf}
!macroend
