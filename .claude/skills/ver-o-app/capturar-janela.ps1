# Captura UMA JANELA do Windows pelo conteudo dela, e nao pela regiao da tela.
#
# O `orca computer` fotografa as coordenadas da janela na tela, entao qualquer
# coisa por cima aparece no lugar do app. `PrintWindow` pede a propria janela
# que se desenhe num contexto grafico — janela coberta, minimizada ou atras de
# outra sai igual.
#
# A flag 2 e `PW_RENDERFULLCONTENT`, e ela existe justamente para janelas
# compostas por GPU: sem ela, WebView2 e Chromium devolvem um retangulo preto.
#
# Roda no Windows PowerShell 5.1 (`powershell.exe`), e nao no 7: o System.Drawing
# saiu do conjunto padrao do .NET moderno.
param(
  [Parameter(Mandatory = $true)][string]$Titulo,
  [Parameter(Mandatory = $true)][string]$Saida,
  [string]$Processo = "",
  [uint32]$Flags = 2,
  # Altura temporaria, para caber a pagina inteira numa foto so. A janela volta
  # ao tamanho e a posicao de antes; ela nunca aparece grande na tela, porque e
  # movida para fora dela durante a captura.
  [int]$Altura = 0
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;

public class JanelaCap {
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc cb, IntPtr p);
  delegate bool EnumWindowsProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }

  /// Restaura uma janela minimizada SEM ativar.
  ///
  /// `PrintWindow` pede a janela que se desenhe, e uma janela minimizada nao
  /// tem area de cliente para desenhar — ela reporta 160x28 e devolve a barra
  /// de titulo. Restaurar e obrigatorio; roubar o foco de quem esta trabalhando
  /// nao e. `SW_SHOWNOACTIVATE` faz exatamente a metade que interessa.
  public static bool Desminimizar(IntPtr h) {
    if (!IsIconic(h)) return false;
    ShowWindow(h, 4);
    System.Threading.Thread.Sleep(350);
    return true;
  }

  /// A maior janela com aquele titulo, minimizada ou nao. "Maior" resolve o
  /// caso do M/OS, que mantem uma janela "M/OS" oculta de 436x261 ao lado da de
  /// verdade; a minimizada entra na conta porque e justamente a que se quer.
  public static IntPtr Achar(string titulo, int pid) {
    IntPtr melhor = IntPtr.Zero; long area = -1;
    EnumWindows(delegate(IntPtr h, IntPtr p) {
      uint dono; GetWindowThreadProcessId(h, out dono);
      if (pid > 0 && dono != (uint)pid) return true;
      if (!IsWindowVisible(h)) return true;
      var sb = new StringBuilder(400);
      GetWindowText(h, sb, 400);
      if (sb.ToString() != titulo) return true;
      // Minimizada ganha de qualquer outra: ela reporta 160x28 e perderia a
      // comparacao por area justamente por estar no estado que se quer corrigir.
      if (IsIconic(h)) { melhor = h; area = long.MaxValue; return false; }
      RECT r; GetWindowRect(h, out r);
      long a = (long)(r.R - r.L) * (r.B - r.T);
      if (a > area) { area = a; melhor = h; }
      return true;
    }, IntPtr.Zero);
    return melhor;
  }

  /// Estica a janela para caber a pagina inteira, FORA da tela.
  ///
  /// `PrintWindow` pede a janela que se desenhe no proprio buffer, e isso nao
  /// depende de ela estar visivel — entao da para leva-la para x=-8000, deixa-la
  /// do tamanho do conteudo e fotografar sem que nada disso apareca para quem
  /// esta usando o computador. O tamanho e a posicao voltam ao fim.
  ///
  /// O risco e o WebView2 suspender o desenho ao se julgar oculto; por isso a
  /// funcao devolve a taxa de pixels vivos, e quem chama compara.
  public static string CapturarAlto(IntPtr h, string destino, uint flags, int altura) {
    RECT antes; GetWindowRect(h, out antes);
    int w = antes.R - antes.L;
    const uint SEM_ATIVAR = 0x0010, SEM_ZORDER = 0x0004;
    SetWindowPos(h, IntPtr.Zero, -8000, 0, w, altura, SEM_ATIVAR | SEM_ZORDER);
    System.Threading.Thread.Sleep(700);
    string saida;
    try { saida = Desenhar(h, destino, flags); }
    finally {
      SetWindowPos(h, IntPtr.Zero, antes.L, antes.T, w, antes.B - antes.T, SEM_ATIVAR | SEM_ZORDER);
    }
    return saida;
  }

  public static string Capturar(IntPtr h, string destino, uint flags) {
    bool restaurada = Desminimizar(h);
    return Desenhar(h, destino, flags) + (restaurada ? " (estava minimizada; restaurada sem ativar)" : "");
  }

  static string Desenhar(IntPtr h, string destino, uint flags) {
    RECT r; GetWindowRect(h, out r);
    int w = r.R - r.L, ht = r.B - r.T;
    if (w <= 0 || ht <= 0) return "janela sem tamanho";
    using (var bmp = new Bitmap(w, ht, PixelFormat.Format32bppArgb)) {
      using (var g = Graphics.FromImage(bmp)) {
        IntPtr dc = g.GetHdc();
        bool ok = PrintWindow(h, dc, flags);
        g.ReleaseHdc(dc);
        if (!ok) return "PrintWindow recusou";
      }
      // Quantos pixels nao sao pretos: a forma barata de detectar a captura
      // vazia que uma janela composta por GPU devolve quando a flag esta errada.
      long vivos = 0, total = 0;
      for (int y = 0; y < ht; y += 7) for (int x = 0; x < w; x += 7) {
        total++;
        Color c = bmp.GetPixel(x, y);
        if (c.R > 8 || c.G > 8 || c.B > 8) vivos++;
      }
      bmp.Save(destino, ImageFormat.Png);
      return string.Format("{0}x{1} conteudo={2:P1} -> {3}", w, ht,
        total == 0 ? 0 : (double)vivos / total, destino);
    }
  }
}
"@ -ReferencedAssemblies System.Drawing

$pid2 = 0
if ($Processo) {
  $p = Get-Process -Name $Processo -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $p) { Write-Error "processo '$Processo' nao esta rodando"; exit 1 }
  $pid2 = $p.Id
}
$h = [JanelaCap]::Achar($Titulo, $pid2)
if ($h -eq [IntPtr]::Zero) { Write-Error "nenhuma janela visivel com titulo '$Titulo'"; exit 1 }
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Saida) | Out-Null
if ($Altura -gt 0) {
  [void][JanelaCap]::Capturar($h, "$env:TEMP\__aquece.png", $Flags)  # desminimiza antes de esticar
  [JanelaCap]::CapturarAlto($h, $Saida, $Flags, $Altura)
} else {
  [JanelaCap]::Capturar($h, $Saida, $Flags)
}
