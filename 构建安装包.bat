@echo off
rem TICK 打包脚本（产出 Windows 安装包）
where cargo >nul 2>nul
if errorlevel 1 (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>nul
cd /d "%~dp0"
echo [TICK] 构建安装包...
call npm run tauri build
if errorlevel 1 (
  echo 构建失败
  pause
  exit /b 1
)
echo.
echo 构建完成，安装包位置：
dir /b src-tauri\target\release\bundle\nsis\*.exe 2>nul
dir /b src-tauri\target\release\stocker.exe 2>nul
pause
