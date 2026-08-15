# Tunel do Hermes — mantem 127.0.0.1:9119 apontando para o dashboard da VPS.
#
# Por que existe: o dashboard do Hermes escuta somente em localhost na VPS, por
# decisao do proprio projeto upstream (docker-compose.yml:75 do Hermes-Agent).
# O M/OS fala com 127.0.0.1:9119, entao sem tunel nao ha transporte — e o app
# mostra Offline, corretamente. Ate agora o tunel era um atalho de Desktop
# disparado a mao; este script o torna permanente.
#
# Usa a chave SEM passphrase (id_ed25519), verificada como autorizada na VPS.
# A chave hermes_home tem passphrase e por isso nao serve para execucao sem
# supervisao: exigiria ssh-agent, que exigiria elevacao para habilitar.
#
# Nao roda como servico e nao pede admin: e uma tarefa agendada no logon, no
# contexto do proprio usuario.

$key = Join-Path $env:USERPROFILE ".ssh\id_ed25519"
$target = "hermes@167.233.43.1"
$port = 9119
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
    # Alguem ja abriu o tunel na mao — o atalho de Desktop ainda existe. Subir um
    # segundo faria ExitOnForwardFailure derrubar este na hora, e o laco giraria
    # sem parar. Entao cede a vez e volta a olhar.
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

    # Conexao que durou e sinal de que a rede esta boa: o proximo erro merece
    # recomecar do inicio. Queda imediata e repetida vai afastando as tentativas
    # ate cinco minutos, para nao martelar a VPS quando ela estiver fora.
    if ($lasted.TotalSeconds -ge 60) {
        $delay = 5
    } else {
        $delay = [Math]::Min($delay * 2, 300)
    }
    Start-Sleep -Seconds $delay
}
