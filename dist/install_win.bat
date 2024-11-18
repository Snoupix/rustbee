:: TODO: Change this to an install.bat and move executables to %appdata%/rustbee folder
:: or another one and add to PATH this dir then set perms

@echo off
:: Cd where this script is
cd /d "%~dp0"

icacls rustbee.exe /grant %USERNAME%:(F)
icacls rustbee-daemon.exe /grant %USERNAME%:(F)
