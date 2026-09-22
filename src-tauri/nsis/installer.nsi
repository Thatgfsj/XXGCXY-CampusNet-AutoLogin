; Custom welcome page text
!define MUI_WELCOMEPAGE_TITLE "欢迎使用新乡工程学院校园网自动保活程序"
!define MUI_WELCOMEPAGE_TEXT "安装向导将在计算机上安装「新乡工程校园网保活」程序。$\r$\n$\r$\n本程序用于自动检测校园网连接状态，并在断网时自动重连和登录。$\r$\n$\r$\n点击「下一步」继续，或点击「取消」退出安装向导。"

!macro NSIS_HOOK_PREINSTALL
  ; 关键: 升级安装时旧版中文 exe 常驻托盘 (自启守护是常态), 而 Tauri 模板的 CheckIfAppIsRunning
  ; 只检测英文名 exe —— 不先结束进程, POSTINSTALL 的 CopyFiles 会静默失败, 用户永远跑旧版
  nsExec::Exec 'taskkill /F /IM "新乡工程校园网保活.exe"'
  nsExec::Exec 'taskkill /F /IM "xxgcxy-wifi.exe"'
  Sleep 500
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 1. 复制生成中文主执行文件：新乡工程校园网保活.exe
  CopyFiles /SILENT "$INSTDIR\xxgcxy-wifi.exe" "$INSTDIR\新乡工程校园网保活.exe"

  ; 2. 关键防御：将 NoShortcutMode 设为 1，阻断完成页 (FinishPage) 自动回调 CreateOrUpdateDesktopShortcut
  ;    彻底杜绝点击“完成”时再次生成英文的 $DESKTOP\xxgcxy-wifi.lnk
  StrCpy $NoShortcutMode 1

  ; 3. 清理安装向导默认生成的英文快捷方式（静默模式或旧版本残留）
  Delete "$DESKTOP\xxgcxy-wifi.lnk"
  Delete "$SMPROGRAMS\xxgcxy-wifi.lnk"
  Delete "$SMPROGRAMS\xxgcxy-wifi\xxgcxy-wifi.lnk"
  RMDir "$SMPROGRAMS\xxgcxy-wifi"

  ; 3. 创建指向中文可执行文件的桌面和开始菜单快捷方式
  CreateShortcut "$DESKTOP\新乡工程校园网保活.lnk" "$INSTDIR\新乡工程校园网保活.exe" "" "$INSTDIR\新乡工程校园网保活.exe" 0
  CreateDirectory "$SMPROGRAMS\新乡工程校园网保活"
  CreateShortcut "$SMPROGRAMS\新乡工程校园网保活\新乡工程校园网保活.lnk" "$INSTDIR\新乡工程校园网保活.exe" "" "$INSTDIR\新乡工程校园网保活.exe" 0
  CreateShortcut "$SMPROGRAMS\新乡工程校园网保活\卸载.lnk" "$INSTDIR\uninstall.exe"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; 卸载前先结束可能正在运行的进程
  nsExec::Exec 'taskkill /F /IM "新乡工程校园网保活.exe"'
  nsExec::Exec 'taskkill /F /IM "xxgcxy-wifi.exe"'
  Sleep 500

  ; 清理开机自启注册表项: 应用写入的值名是中文名 (另有历史旧名),
  ; Tauri 模板卸载逻辑只删英文 PRODUCTNAME, 不清理会留下指向已删除 exe 的死自启条目
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "新乡工程校园网保活"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CampusWifiHelper"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "XXGCXY_WiFi"

  Delete "$INSTDIR\新乡工程校园网保活.exe"
  Delete "$DESKTOP\新乡工程校园网保活.lnk"
  Delete "$SMPROGRAMS\新乡工程校园网保活\新乡工程校园网保活.lnk"
  Delete "$SMPROGRAMS\新乡工程校园网保活\卸载.lnk"
  RMDir "$SMPROGRAMS\新乡工程校园网保活"
  Delete "$DESKTOP\xxgcxy-wifi.lnk"
  Delete "$SMPROGRAMS\xxgcxy-wifi.lnk"
  RMDir "$SMPROGRAMS\xxgcxy-wifi"
!macroend
