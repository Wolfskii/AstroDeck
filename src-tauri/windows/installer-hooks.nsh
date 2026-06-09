; AstroDeck Windows installer hooks.
; Upgrade in place and avoid duplicate shortcuts after renames or reinstalls.

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

  ; If a desktop shortcut already exists, refresh its target instead of adding another.
  ${If} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
    !insertmacro SetShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
!macroend
