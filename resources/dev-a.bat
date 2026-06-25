@echo off
cd /d E:\dev\ContextSend\contextsend
set "CONTEXTSEND_DATA_DIR=E:/tmp/cs-a"
pnpm tauri dev
pause
