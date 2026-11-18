@echo off
REM Wrapper reproductible : contourne la politique d'exécution PowerShell
REM locale sans modifier la politique de la machine.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1"
exit /b %ERRORLEVEL%
