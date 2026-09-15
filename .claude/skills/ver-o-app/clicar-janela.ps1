# Clica num ponto da janela real do M/OS, pelas coordenadas lidas na FOTO do
# `capturar-janela.ps1`: somadas a origem da JANELA (GetWindowRect), elas entram
# direto, sem descontar a barra de titulo.
#
# O `orca computer` nao enumera a janela principal do M/OS; o que funciona e
# achar a janela pelo titulo exato + PID + maior area, traze-la a frente e
# clicar por coordenada. Uma invocacao = um clique; entre dois cliques, capture
# e confira.
param(
  [Parameter(Mandatory = $true)][int]$X,
  [Parameter(Mandatory = $true)][int]$Y,
  [string]$Titulo = "M/OS",
  [string]$Processo = "mos-desktop",
  [int]$ProcessoId = 0
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class JanelaClique {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static IntPtr Achar(string titulo, uint pid) {
    IntPtr melhor = IntPtr.Zero; long area = -1;
    EnumWindows(delegate(IntPtr h, IntPtr l) {
      uint p; GetWindowThreadProcessId(h, out p);
      if (pid != 0 && p != pid) return true;
      if (!IsWindowVisible(h)) return true;
      var sb = new StringBuilder(400); GetWindowText(h, sb, 400);
      if (sb.ToString() != titulo) return true;
      RECT r; GetWindowRect(h, out r);
      long a = (long)(r.R - r.L) * (r.B - r.T);
      if (a > area) { area = a; melhor = h; }
      return true;
    }, IntPtr.Zero);
    return melhor;
  }
}
"@

$pid2 = 0
if ($ProcessoId -gt 0) { $pid2 = $ProcessoId }
elseif ($Processo) {
  $p = Get-Process -Name $Processo -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($p) { $pid2 = $p.Id }
}
$h = [JanelaClique]::Achar($Titulo, [uint32]$pid2)
if ($h -eq [IntPtr]::Zero) { Write-Error "nenhuma janela '$Titulo'"; exit 1 }
if ([JanelaClique]::IsIconic($h)) { [JanelaClique]::ShowWindow($h, 9) | Out-Null }
[JanelaClique]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 250
$r = New-Object JanelaClique+RECT
[JanelaClique]::GetWindowRect($h, [ref]$r) | Out-Null
$sx = $r.L + $X
$sy = $r.T + $Y
[JanelaClique]::SetCursorPos($sx, $sy) | Out-Null
Start-Sleep -Milliseconds 60
[JanelaClique]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
[JanelaClique]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
Write-Output "clique em ($X,$Y) da foto -> tela ($sx,$sy); janela $($r.R-$r.L)x$($r.B-$r.T) @ $($r.L),$($r.T)"
