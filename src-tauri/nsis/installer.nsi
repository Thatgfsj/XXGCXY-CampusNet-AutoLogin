; Custom welcome page text
!define MUI_WELCOMEPAGE_TITLE "欢迎使用新乡工程学院校园网自动保活程序"
!define MUI_WELCOMEPAGE_TEXT "安装向导将在计算机上安装 xxgcxy-wifi。$\r$\n$\r$\n本程序用于自动检测校园网连接状态，并在断网时自动重连和登录。$\r$\n$\r$\n点击「下一步」继续，或点击「取消」退出安装向导。"

!macro preInit
  ; 检测 PowerShell 7+ (pwsh.exe 是 PowerShell 7+ 的命令，Windows 自带的是 powershell.exe 即 PowerShell 5)
  nsExec::ExecToStack 'pwsh -Command "exit 0"'
  Pop $0
  Pop $1
  
  ; 如果 pwsh 命令执行成功，说明已安装 PowerShell 7+
  StrCmp $0 "0" done
  
  ; PowerShell 7 未安装，提示用户
  MessageBox MB_YESNO|MB_ICONQUESTION "检测到系统未安装 PowerShell 7，本程序需要 PowerShell 7 才能正常运行。$\r$\n$\r$\n是否立即安装 PowerShell 7？$\r$\n$\r$\n（点击「否」将退出安装）" IDYES installPS7
  Abort "安装已取消 - 需要 PowerShell 7"
  
  installPS7:
  DetailPrint "正在安装 PowerShell 7，请稍候..."
  nsExec::ExecToStack 'winget install --id Microsoft.PowerShell --source winget --accept-source-agreements --accept-package-agreements --silent'
  Pop $0
  Pop $1
  
  IntCmp $0 0 ps7success
    MessageBox MB_OK|MB_ICONWARNING "PowerShell 7 安装可能未成功完成。$\r$\n$\r$\n程序将在首次运行时自动检测并安装。"
    Goto done
  
  ps7success:
  DetailPrint "PowerShell 7 安装完成"
  
  done:
!macroend
