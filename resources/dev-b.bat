@echo off
cd /d E:\dev\ContextSend\contextsend
set "CONTEXTSEND_DATA_DIR=E:/tmp/cs-b"
set "CARGO_TARGET_DIR=E:/tmp/cs-target-b"
pnpm tauri dev --config "{\"build\":{\"beforeDevCommand\":\"\"}}"
pause
