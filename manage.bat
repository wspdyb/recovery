@echo off
chcp 65001 >nul
:menu
cls
echo ====================================
echo   PhoenixRecover - לוהינ תכרעמ- 2026
echo ====================================
echo [0] תוילת תנקתה (npm install)
echo [1] חותיפ תרש תצרה (Dev Mode)
echo [2] טנרטניא תסרג תיינב (Web Build)
echo [3] (Build EXE) הנקתה ץבוק תריצי
echo [4] (Git Push) באהטיגל הפיחדו ןוכדע
echo [5] האיצי
echo ====================================
set /p choice=": (5-0) תושרפא רחב "

if "%choice%"=="0" goto install_dependencies
if "%choice%"=="1" goto run_dev
if "%choice%"=="2" goto build_web
if "%choice%"=="3" goto build_exe
if "%choice%"=="4" goto push_to_github
if "%choice%"=="5" goto exit
goto menu

:install_dependencies
echo.
echo ...תסרג יביכר תנקתה
call npm install
echo.
echo .םייתסה
pause
goto menu

:run_dev
echo.
echo ...חוטיפה תיביבס תא ליעפמ
call npm run tauri dev
pause
goto menu

:build_web
echo.
echo ...טנרטניא תסרג תיינב ליחתמ
call npm run build
echo.
echo .החלבצב המייתסה היינבה
pause
goto menu

:build_exe
echo.
echo .תוש דסמ תחקל יושע הז תליחת ... (EXE) הנקתה טבוק רצוי
call npm run tauri build
echo.
echo .release תייקיתבן כומה הנקתהה טבוק
pause
goto menu

:push_to_github
echo.
echo ...באהטיגל פחדו םיינוישי דחעמ
call git add .
set /p commit_msg=": (טלופדיל Enter ץלח וא) נוכדע תעדה סנכה"
if "%commit_msg%"=="" set commit_msg="Update"
call git commit -m %commit_msg%
call git push
echo.
echo .םלשהו נוכדעה
pause
goto menu

:exit
exit
