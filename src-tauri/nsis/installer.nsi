; Custom welcome page text
!define MUI_WELCOMEPAGE_TITLE "欢迎使用新乡工程学院校园网自动保活程序"
!define MUI_WELCOMEPAGE_TEXT "安装向导将在计算机上安装「新乡工程校园网保活」程序。$\r$\n$\r$\n本程序用于自动检测校园网连接状态，并在断网时自动重连和登录。$\r$\n$\r$\n点击「下一步」继续，或点击「取消」退出安装向导。"

!macro NSIS_HOOK_POSTINSTALL
  ; 1. 复制生成中文主执行文件：新乡工程校园网保活.exe
  CopyFiles /SILENT "$INSTDIR\xxgcxy-wifi.exe" "$INSTDIR\新乡工程校园网保活.exe"

  ; 2. 清理安装向导默认生成的英文快捷方式
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
  Delete "$INSTDIR\新乡工程校园网保活.exe"
  Delete "$DESKTOP\新乡工程校园网保活.lnk"
  Delete "$SMPROGRAMS\新乡工程校园网保活\新乡工程校园网保活.lnk"
  Delete "$SMPROGRAMS\新乡工程校园网保活\卸载.lnk"
  RMDir "$SMPROGRAMS\新乡工程校园网保活"
  Delete "$DESKTOP\xxgcxy-wifi.lnk"
  Delete "$SMPROGRAMS\xxgcxy-wifi.lnk"
  RMDir "$SMPROGRAMS\xxgcxy-wifi"
!macroend
