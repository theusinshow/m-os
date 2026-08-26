# Tunel do sync - mantem 127.0.0.1:9120 apontando para o hub da VPS.
#
# Irmao gemeo do `hermes-tunnel.ps1`, e de proposito: o hub escuta somente em
# localhost na VPS pela mesma razao que o dashboard do Hermes — um banco pessoal
# nao precisa de porta publica. O M/OS fala com 127.0.0.1:9120, e sem tunel a
# sincronizacao simplesmente nao alcanca (e diz isso, em vez de fingir).
#
# Precisa de uma chave SEM passphrase: a tarefa roda sem ninguem olhando.
#
# NAO roda como servico e nao pede admin: e uma tarefa agendada no logon, no
# contexto do proprio usuario. Mesma forma do tunel do Hermes, porque manter os
# dois iguais e o que faz consertar um ensinar a consertar o outro.
#
# LIMITE CONHECIDO: isto serve o DESKTOP. O iPhone nao mantem tunel SSH, e para
# ele o hub vai precisar de endereco publico com TLS — outra decisao, e nao uma
# extensao desta.

$candidatas = @("hermes_work", "id_ed25519", "hermes_home")

function Find-Key {
    foreach ($nome in $candidatas) {
        $caminho = Join-Path $env:USERPROFILE ".ssh\$nome"
        if (-not (Test-Path $caminho)) { continue }
        if (((Get-Content $caminho -TotalCount 3) -join "`n") -match "ENCRYPTED") { continue }
        return $caminho
    }
    return $null
}

$key = Find-Key
if (-not $key) {
    Write-Error ("Nenhuma chave SSH sem passphrase encontrada em ~/.ssh (procurei: " +
        ($candidatas -join ", ") + "). Sem ela o tunel nao sobe e a sincronizacao nao alcanca o hub.")
    exit 1
}
Write-Host "chave: $key"
$target = "hermes@167.233.43.1"
$port = 9120
$delay = 5

function Test-Port {
    param([int]$Number)
    $client = New-Object Net.Sockets.TcpClient
    try {
        $client.Connect("127.0.0.1", $Number)
        return $true
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

while ($true) {
    # Alguem ja abriu o tunel na mao. Subir um segundo faria
    # ExitOnForwardFailure derrubar este na hora, e o laco giraria sem parar.
    if (Test-Port -Number $port) {
        Start-Sleep -Seconds 15
        continue
    }

    $started = Get-Date
    & ssh -i $key -N -T `
        -o BatchMode=yes `
        -o ExitOnForwardFailure=yes `
        -o ServerAliveInterval=30 `
        -o ServerAliveCountMax=3 `
        -o StrictHostKeyChecking=accept-new `
        -L "${port}:127.0.0.1:${port}" $target
    $lasted = (Get-Date) - $started

    # Conexao que durou e sinal de rede boa: o proximo erro recomeca do inicio.
    # Queda imediata e repetida vai afastando as tentativas ate cinco minutos,
    # para nao martelar a VPS quando ela estiver fora.
    if ($lasted.TotalSeconds -ge 60) {
        $delay = 5
    } else {
        $delay = [Math]::Min($delay * 2, 300)
    }
    Start-Sleep -Seconds $delay
}
