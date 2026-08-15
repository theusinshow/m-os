@echo off
echo Limpando cache do servidor...
if exist .next rmdir /s /q .next
start "" cmd /k "npm run dev -- -p 5000"
echo Aguardando compilacao inicial...
timeout /t 8 /nobreak >nul
start "" "http://localhost:5000"
