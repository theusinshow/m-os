# M/OS — Preparar uma máquina de desenvolvimento

Windows, sem privilégio de administrador. Escrito em 2026-08-18, a partir do que
foi de fato instalado e configurado na máquina de trabalho.

---

## 1. O que precisa existir

| Ferramenta | Para quê | Onde |
|---|---|---|
| Node 20+ | renderer, M-Finance, scripts | winget ou fnm |
| Rust, toolchain **GNU** | `mos-core`, `mos-storage-sqlite`, `mos-desktop` | rustup |
| **binutils do mingw-w64** | `windres`, exigido pelo build do Tauri | WinLibs, §2 |
| WebView2 Runtime | a janela do M/OS | já vem no Windows 11 |

O ponto não óbvio é o terceiro, e ele custou uma tarde. Está na §2.

---

## 2. `windres` — o que trava o build e não avisa

### O sintoma

```
thread 'main' panicked at tauri-winres-0.3.6/src/lib.rs:543:
called `Result::unwrap()` on an `Err` value: NotAttempted("windres")
error: failed to run custom build command for `mos-desktop`
```

### Por que ele demora a aparecer

O `tauri-build` compila um recurso do Windows — ícone e versão do executável — e
para isso chama `windres`. Mas **build script tem cache**: enquanto o grafo de
dependências do crate não mudar, o Cargo reusa o resultado anterior e nada
acontece.

O efeito é que a máquina parece saudável por semanas. O build quebra no dia em
que alguém adiciona **qualquer** dependência nova ao `apps/desktop/src-tauri` —
e o erro aponta para o Tauri, não para a ferramenta que falta.

Foi exatamente assim que apareceu aqui, ao adicionar o `tauri-plugin-autostart`.

### O que NÃO resolve

- o toolchain GNU do rustup traz `x86_64-w64-mingw32-gcc` e `dlltool` em
  `lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained`, mas **não** o `windres`;
- `rc.exe` do SDK do Windows não serve: o `embed-resource` só o procura no
  caminho MSVC, e este projeto usa GNU;
- copiar só o `windres.exe` para uma pasta no PATH também não serve — ele
  depende de DLLs e do `as.exe` da instalação de origem. Foi tentado e falhou
  com `STATUS_DLL_NOT_FOUND`.

### A instalação, sem admin

```powershell
winget install --id BrechtSanders.WinLibs.POSIX.UCRT --scope user `
  --accept-package-agreements --accept-source-agreements
```

O `--scope user` é o que dispensa administrador. Instala em:

```
%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin
```

O winget já acrescenta essa pasta ao **PATH do usuário**. Vale conferir, porque
ele não cria alias para o `windres`:

```powershell
[Environment]::GetEnvironmentVariable("Path", "User") -split ';' | Select-String WinLibs
```

Se não aparecer, acrescente — **no fim da lista**, e não no começo:

```powershell
$bin = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
$atual = [Environment]::GetEnvironmentVariable("Path", "User")
[Environment]::SetEnvironmentVariable("Path", ($atual.TrimEnd(';') + ";" + $bin), "User")
```

**No fim importa.** O rustup já tem o próprio `gcc` em `self-contained`, e pôr
outro mingw na frente trocaria o linker sem ninguém pedir. Um dos problemas
abertos desta máquina (§4) é justamente incompatibilidade de DLL de runtime
entre mingws — não vale a pena criar mais uma chance disso.

Abra um terminal novo e confirme:

```powershell
windres --version
# GNU windres (Binutils for MinGW-W64 x86_64...) 2.47...
```

---

## 3. `TMP` e `TEMP` — quando o linker falha sem motivo aparente

### O sintoma

```
error: linking with `x86_64-w64-mingw32-gcc` failed
ld: cannot find @C:\WINDOWS\TEMP\ccXXXXXX: Invalid argument
```

### A causa

O `gcc` escreve a lista de argumentos do linker num arquivo temporário e passa
o caminho para o `ld`. Se o processo não consegue **ler** aquele diretório
temporário, o `ld` recebe um caminho que não abre — e reclama do arquivo, não
da permissão.

Acontece em ambientes que restringem `C:\WINDOWS\TEMP`, como o sandbox de um
agente. Não é o toolchain: é o diretório.

### A saída

Aponte `TMP` e `TEMP` para um diretório gravável antes de qualquer comando do
Cargo:

```bash
export TMP="/caminho/gravavel/tmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cargo build
```

Numa máquina de uso normal isso não é necessário.

---

## 4. Problema conhecido, ainda aberto

**`cargo test -p mos-desktop` não roda nesta máquina.**

O binário compila e linka, mas morre ao carregar:

```
process didn't exit successfully: mos_desktop_lib-*.exe (exit code: 0xc0000139,
STATUS_ENTRYPOINT_NOT_FOUND)
```

É incompatibilidade entre as DLLs de runtime do mingw que o binário procura e as
que estão no PATH. Com o WinLibs **na frente** do PATH o erro muda para
`STATUS_DLL_NOT_FOUND` — outra falha, não uma solução.

Consequência prática, e ela guia onde escrever teste: **a lógica precisa morar
em `mos-core` ou `mos-storage-sqlite`, onde os testes rodam.** O crate do desktop
deve ficar com casca fina — comandos, laços e adaptação — porque teste que não
roda não protege nada. O `AttentionService` é o exemplo: a regra está no
domínio, e os testes que a exercitam vivem no crate de storage.

Não foi investigado além de duas tentativas. Se um dia incomodar, o caminho mais
promissor é migrar o projeto para o toolchain **MSVC**, que evita mingw inteiro
— o `stable-x86_64-pc-windows-msvc` já está instalado, e o GNU está fixado só
por override de diretório (`rustup override`).

---

## 5. Verificação final

Numa máquina preparada, tudo abaixo passa:

```bash
cargo test -p mos-core            # 152 testes
cargo test -p mos-storage-sqlite  #  98 testes
cargo build -p mos-desktop        # compila, incluindo o build script

cd apps/desktop  && npm ci && npm test && npm run build
cd ../m-finance  && npm ci && npm test && npm run build
```

E o app sobe com:

```bash
cd apps/desktop && npm run tauri dev
```

---

## 6. Serviços externos

Não são necessários para compilar, só para usar o sistema inteiro.

**M-Finance** (`apps/m-finance`) precisa de `.env` com `DATABASE_URL`,
`NEXT_PUBLIC_SUPABASE_URL`, `NEXT_PUBLIC_SUPABASE_ANON_KEY`, `AUTHORIZED_EMAIL`
e `MOS_ACTION_SECRET`. O deploy vem do próprio monorepo — projeto na Vercel
apontando para `theusinshow/m-os` com **Root Directory** `apps/m-finance`.

**Hermes** exige um túnel SSH mantendo `127.0.0.1:9119` apontando para a VPS. O
`scripts/install-hermes-tunnel.ps1` registra uma tarefa agendada no logon, sem
admin. Ele procura uma chave sem passphrase em `~/.ssh` entre os nomes
conhecidos; sem chave, reclama e sai em vez de tentar em silêncio.

**Antes de confiar em qualquer coisa que escreva no M-Finance**, rode:

```bash
DATABASE_URL=... node scripts/check-db-migrations.mjs
```

O banco já esteve quatro migrations atrás do schema por semanas, e o sintoma era
um erro de runtime dentro de uma request — não um build quebrado.
