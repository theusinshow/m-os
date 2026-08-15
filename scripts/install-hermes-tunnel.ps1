# Instala o tunel do Hermes como tarefa agendada no logon.
#
# Roda uma vez, no contexto do proprio usuario — nao precisa de admin, porque a
# tarefa nao usa RunLevel Highest nem escreve fora do perfil.
#
# Existe como script para tirar a citacao do caminho: registrar a tarefa numa
# linha unica exige aspas dentro de aspas dentro de aspas, e foi exatamente ali
# que a tentativa anterior se perdeu.
#
# Idempotente: rodar de novo apenas atualiza a copia do script e recria a
# tarefa.

$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "hermes-tunnel.ps1"
$dir = Join-Path $env:LOCALAPPDATA "M-OS-Tunnel"
$target = Join-Path $dir "hermes-tunnel.ps1"
$taskName = "M-OS Hermes Tunnel"

if (-not (Test-Path $source)) {
    throw "Nao encontrei $source. Rode este script de dentro de scripts/ do repositorio."
}

New-Item -ItemType Directory -Force -Path $dir | Out-Null
Copy-Item $source $target -Force
Write-Host "script instalado em: $target"

$argument = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $target + '"'
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument $argument
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -ExecutionTimeLimit ([TimeSpan]::Zero)

Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Description "Mantem 127.0.0.1:9119 apontando para o dashboard do Hermes na VPS." `
    -Force | Out-Null

Write-Host "tarefa registrada: $taskName"

Start-ScheduledTask -TaskName $taskName
Start-Sleep -Seconds 3

$state = (Get-ScheduledTask -TaskName $taskName).State
Write-Host "estado da tarefa: $state"

$client = New-Object Net.Sockets.TcpClient
try {
    $client.Connect("127.0.0.1", 9119)
    Write-Host "tunel: ABERTO em 127.0.0.1:9119"
} catch {
    Write-Host "tunel: ainda fechado — pode levar alguns segundos, ou a VPS esta fora"
} finally {
    $client.Dispose()
}
