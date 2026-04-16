Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap 256, 256
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::FromArgb(102, 126, 234))

$pen = New-Object System.Drawing.Pen([System.Drawing.Color]::White, 12)
$center = 128

# 画WiFi弧线
$g.DrawArc($pen, 50, 50, 156, 156, 225, 90)
$g.DrawArc($pen, 80, 80, 96, 96, 225, 90)
$g.DrawArc($pen, 110, 110, 36, 36, 225, 90)

# 画圆点
$brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$g.FillEllipse($brush, 118, 118, 20, 20)

$bmp.Save("I:\openclaw\wifi\src-tauri\icons\app-icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
$pen.Dispose()
$brush.Dispose()
Write-Host "图标创建成功"
