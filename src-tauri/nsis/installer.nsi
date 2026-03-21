; Custom welcome page text
!define MUI_WELCOMEPAGE_TITLE "欢迎使用新乡工程学院校园网自动保活程序"
!define MUI_WELCOMEPAGE_TEXT "安装向导将在计算机上安装 xxgcxy-wifi。$\r$\n$\r$\n本程序用于自动检测校园网连接状态，并在断网时自动重连和登录。$\r$\n$\r$\n点击「下一步」继续，或点击「取消」退出安装向导。"

!macro preInit
  ; Check PowerShell 7 before installation starts
  nsExec::ExecToStack 'pwsh -Command "exit 0"'
  Pop $0
  Pop $1
  StrCmp $0 "0" done
  
  MessageBox MB_YESNO|MB_ICONQUESTION "PowerShell 7 is required for this application. Install now?" IDYES installPS7
  Abort "Installation cancelled - PowerShell 7 required"
  
  installPS7:
  nsExec::ExecToStack 'winget install --id Microsoft.PowerShell --source winget --accept-source-agreements --accept-package-agreements --silent'
  Pop $0
  Pop $1
  
  done:
!macroend
