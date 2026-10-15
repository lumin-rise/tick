@echo off
rem TICK 开发模式启动脚本（自动初始化 MSVC 编译环境）
where cargo >nul 2>nul
if errorlevel 1 (
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>nul
cd /d "%~dp0"
echo [TICK] 启动开发模式...
npm run tauri dev
