# Instala o tunel do SYNC como tarefa agendada no logon.
#
# Irmao gemeo do `install-hermes-tunnel.ps1`, e de proposito: manter os dois
# iguais e o que faz consertar um ensinar a consertar o outro. A unica diferenca
# real e a porta - 9120, do hub, em vez de 9119, do dashboard.
#
# Roda uma vez, no contexto do proprio usuario - nao precisa de admin, porque a
# tarefa nao usa RunLevel Highest nem escreve fora do perfil.
#
# Idempotente: rodar de novo apenas atualiza a copia do script e recria a
# tarefa.
#
# POR QUE UM TUNEL, e nao o endereco publico que o iPhone usa: o hub escuta
# somente em localhost na VPS. O celular precisou de HTTPS porque nao mantem
# tunel SSH; o desktop mantem, e o SSH ja cifra tudo entre o PC e a VPS. Um TLS
# por cima do tunel cifraria duas vezes o mesmo trecho, e abriria uma porta
# publica a mais para o hub - que hoje nao tem nenhuma.

$ErrorActionPreference = "Stop"

$source = Join-Path $PSScriptRoot "sync-tunnel.ps1"
$dir = Join-Path $env:LOCALAPPDATA "M-OS-Tunnel"
$target = Join-Path $dir "sync-tunnel.ps1"
$taskName = "M-OS Sync Tunnel"

if (-not (Test-Path $source)) {
    throw "Nao encontrei $source. Rode este script de dentro de scripts/ do repositorio."
}

New-Item -ItemType Directory -Force -Path $dir | Out-Null
Copy-Item $source $target -Force
Write-Host "script instalado em: $target"

$argument = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $target + '"'
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument $argument
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
# `RestartCount` porque o gatilho e apenas o logon: sem ele, um processo que
# morre fica morto ate o proximo login. O laco interno do `sync-tunnel.ps1` ja
# cuida da VPS fora do ar; isto cuida do processo em si desaparecer.
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 999 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -ExecutionTimeLimit ([TimeSpan]::Zero)

Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Description "Mantem 127.0.0.1:9120 apontando para o hub de sincronizacao na VPS." `
    -Force | Out-Null

Write-Host "tarefa registrada: $taskName"

Start-ScheduledTask -TaskName $taskName
Start-Sleep -Seconds 4

$state = (Get-ScheduledTask -TaskName $taskName).State
Write-Host "estado da tarefa: $state"

# A conferencia vai ate o `/health` do hub, e nao para na porta aberta: o tunel
# pode estar de pe com o servico do outro lado fora, e uma porta que aceita
# conexao sem ninguem atras e o tipo de "esta funcionando" que engana.
try {
    $resposta = Invoke-RestMethod -Uri "http://127.0.0.1:9120/health" -TimeoutSec 5
    if ($resposta.ok) {
        Write-Host "hub: ALCANCADO (contrato $($resposta.contrato))"
    } else {
        Write-Host "hub: respondeu, mas nao com ok=true"
    }
} catch {
    Write-Host "hub: ainda fora de alcance - pode levar alguns segundos, ou a VPS esta fora"
}
