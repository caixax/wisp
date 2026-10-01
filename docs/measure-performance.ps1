# Measures cold-start time and idle memory for the Wisp release binary.
# Startup is measured from process start to the main window handle existing.
# Memory sums wisp.exe plus every WebView2 process descended from it.

param(
  [string]$Exe = 'D:\Projects\wisp\src-tauri\target\release\wisp.exe',
  [int]$SettleSeconds = 10
)

if (-not (Test-Path $Exe)) { throw "not found: $Exe" }

function Get-Descendants([int]$RootPid) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
  $result = @()
  $frontier = @($RootPid)
  while ($frontier.Count -gt 0) {
    $next = @()
    foreach ($p in $frontier) {
      foreach ($c in $all | Where-Object { $_.ParentProcessId -eq $p }) {
        $result += $c
        $next += $c.ProcessId
      }
    }
    $frontier = $next
  }
  return $result
}

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath $Exe -PassThru
$handleMs = $null
while ($sw.Elapsed.TotalSeconds -lt 30) {
  $proc.Refresh()
  if ($proc.MainWindowHandle -ne 0) { $handleMs = $sw.Elapsed.TotalMilliseconds; break }
  Start-Sleep -Milliseconds 5
}
$sw.Stop()

Start-Sleep -Seconds $SettleSeconds

$proc.Refresh()

# Private working set is what Task Manager reports per app. Summed plain
# working set over WebView2's process tree double-counts shared Edge pages,
# so both numbers are reported.
$perf = Get-CimInstance Win32_PerfRawData_PerfProc_Process |
  Select-Object IDProcess, WorkingSetPrivate
function Get-PrivateWsMB([int]$TargetPid) {
  $row = $perf | Where-Object { $_.IDProcess -eq $TargetPid } | Select-Object -First 1
  if ($null -eq $row) { return 0 }
  return [math]::Round($row.WorkingSetPrivate / 1MB, 1)
}

$rows = @()
$rows += [pscustomobject]@{
  Name = 'wisp.exe'; Pid = $proc.Id
  WorkingSetMB = [math]::Round($proc.WorkingSet64 / 1MB, 1)
  PrivateWsMB  = Get-PrivateWsMB $proc.Id
}

foreach ($d in Get-Descendants $proc.Id) {
  try {
    $dp = Get-Process -Id $d.ProcessId -ErrorAction Stop
    $rows += [pscustomobject]@{
      Name = $d.Name; Pid = $d.ProcessId
      WorkingSetMB = [math]::Round($dp.WorkingSet64 / 1MB, 1)
      PrivateWsMB  = Get-PrivateWsMB $d.ProcessId
    }
  } catch {}
}

$rows | Format-Table -AutoSize
$totalWs = ($rows | Measure-Object WorkingSetMB -Sum).Sum
$totalPriv = ($rows | Measure-Object PrivateWsMB -Sum).Sum

Write-Output ''
Write-Output ("startup_to_window_ms = {0}" -f ([math]::Round($handleMs, 0)))
Write-Output ("processes            = {0}" -f $rows.Count)
Write-Output ("total_working_set_MB = {0}" -f [math]::Round($totalWs, 1))
Write-Output ("total_private_ws_MB  = {0}" -f [math]::Round($totalPriv, 1))
Write-Output ("exe_size_MB          = {0}" -f [math]::Round((Get-Item $Exe).Length / 1MB, 2))

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Get-Process -Name wisp -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
