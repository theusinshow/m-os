# Tunel do Hermes - mantem 127.0.0.1:9119 apontando para o dashboard da VPS.
#
# Por que existe: o dashboard do Hermes escuta somente em localhost na VPS, por
# decisao do proprio projeto upstream (docker-compose.yml:75 do Hermes-Agent).
# O M/OS fala com 127.0.0.1:9119, entao sem tunel nao ha transporte - e o app
# mostra Offline, corretamente. Ate agora o tunel era um atalho de Desktop
# disparado a mao; este script o torna permanente.
#
# Precisa de uma chave SEM passphrase: a tarefa roda sem ninguem olhando, e uma
# chave protegida exigiria ssh-agent, que exigiria elevacao para habilitar.
#
# O nome da chave varia por maquina — esta versao procurava um `id_ed25519` fixo
# que nao existia aqui, e o efeito era o pior possivel: `ssh` falhava, o laco
# tentava de novo para sempre, e a unica pista era o Hermes aparecer Offline no
# M/OS. Agora procura entre os nomes conhecidos e RECLAMA ALTO se nao achar,
# em vez de girar calado.
#
# Nao roda como servico e nao pede admin: e uma tarefa agendada no logon, no
# contexto do proprio usuario.

$candidatas = @("hermes_work", "id_ed25519", "hermes_home")

function Find-Key {
    foreach ($nome in $candidatas) {
        $caminho = Join-Path $env:USERPROFILE ".ssh\$nome"
        if (-not (Test-Path $caminho)) { continue }
        # Chave com passphrase nao serve aqui: o ssh ficaria esperando alguem
        # digitar, e ninguem esta olhando.
        if (((Get-Content $caminho -TotalCount 3) -join "`n") -match "ENCRYPTED") { continue }
        return $caminho
    }
    return $null
}

$key = Find-Key
if (-not $key) {
    Write-Error ("Nenhuma chave SSH sem passphrase encontrada em ~/.ssh (procurei: " +
        ($candidatas -join ", ") + "). Sem ela o tunel nao sobe e o Hermes fica Offline.")
    exit 1
}
Write-Host "chave: $key"
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
    # Alguem ja abriu o tunel na mao - o atalho de Desktop ainda existe. Subir um
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
