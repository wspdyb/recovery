@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
cd /d "%~dp0"
title סנכרון פרויקט Recovery לגיטהב
color 0a

echo ========================================================================
echo   סנכרון פרויקט Recovery לגיטהב (GitHub Sync)
echo   תיקייה: %cd%
echo ========================================================================
echo.

where git >nul 2>nul
if errorlevel 1 (
    echo [שגיאה] git לא נמצא ב-PATH.
    pause
    exit /b 1
)

echo דוחף שינויים ל-GitHub (ענף main)...
git add -A
git commit -m "Sync updates %date% %time%" >nul 2>&1
git push origin main

if errorlevel 1 (
    echo   [X] הסנכרון נכשל.
    set FAIL=1
) else (
    echo   [V] הפרויקט סונכרן והועלה לגיטהב בהצלחה!
)

echo.
pause