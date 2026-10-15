Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WE {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public struct RECT { public int L, T, R, B; }
  public static List<string> Run(uint target) {
    var output = new List<string>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target) {
        var sb = new StringBuilder(256); GetWindowText(h, sb, 256);
        var r = new RECT(); GetWindowRect(h, out r);
        output.Add(h.ToInt64() + "|" + IsWindowVisible(h) + "|" + r.L + "," + r.T + "-" + r.R + "," + r.B + "|" + sb.ToString());
      }
      return true;
    }, IntPtr.Zero);
    return output;
  }
}
'@
$procs = Get-Process stocker -ErrorAction SilentlyContinue
if (-not $procs) { Write-Output "no stocker process"; exit }
foreach ($p in $procs) {
  Write-Output "PID $($p.Id):"
  [WE]::Run($p.Id)
}
