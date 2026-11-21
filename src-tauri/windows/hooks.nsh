; XkyHold-wd installer hooks (Tauri NSIS, per-user install).
; Uninstalling removes what the app registered for this user: the hourly
; scheduled task (PLAN.md 4.8) and the notification name. Settings, the log,
; the cache and the keys in the Credential Manager stay, as with any app the
; user may install again; "Remove from this computer" in Settings clears them.

!macro NSIS_HOOK_PREUNINSTALL
  ; Task names belong to the whole computer: each user's carries the user name.
  ReadEnvStr $1 USERNAME
  nsExec::Exec 'schtasks.exe /Delete /TN "\XkyHold\xkyhold-wd-$1" /F'
  Pop $0
  nsExec::Exec 'schtasks.exe /Delete /TN "\XkyHold-wd-$1" /F'
  Pop $0
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\com.xkyriver.xkyhold-wd"
!macroend
