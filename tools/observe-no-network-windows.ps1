param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,
    [int]$Seconds = 10
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$resolvedExecutable = (Resolve-Path -LiteralPath $Executable -ErrorAction Stop).Path
$process = Start-Process -FilePath $resolvedExecutable -WindowStyle Hidden -PassThru
$observed = [System.Collections.Generic.List[string]]::new()
$observedProcesses = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

try {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($Seconds)
    while ([DateTimeOffset]::UtcNow -lt $deadline -and -not $process.HasExited) {
        $allProcesses = Get-CimInstance Win32_Process
        $ids = [System.Collections.Generic.HashSet[uint32]]::new()
        [void]$ids.Add([uint32]$process.Id)
        do {
            $before = $ids.Count
            foreach ($candidate in $allProcesses) {
                if ($ids.Contains([uint32]$candidate.ParentProcessId)) {
                    [void]$ids.Add([uint32]$candidate.ProcessId)
                }
            }
        } while ($ids.Count -ne $before)

        foreach ($candidate in $allProcesses) {
            if ($ids.Contains([uint32]$candidate.ProcessId)) {
                [void]$observedProcesses.Add("$($candidate.Name) (PID $($candidate.ProcessId))")
            }
        }

        foreach ($connection in Get-NetTCPConnection -ErrorAction Stop) {
            if ($ids.Contains([uint32]$connection.OwningProcess)) {
                $owner = $allProcesses | Where-Object ProcessId -eq $connection.OwningProcess | Select-Object -First 1
                $observed.Add("PID $($connection.OwningProcess) $($owner.Name) | TCP $($connection.LocalAddress):$($connection.LocalPort) -> $($connection.RemoteAddress):$($connection.RemotePort) [$($connection.State)] | $($owner.CommandLine)")
            }
        }
        foreach ($endpoint in Get-NetUDPEndpoint -ErrorAction Stop) {
            if ($ids.Contains([uint32]$endpoint.OwningProcess)) {
                $owner = $allProcesses | Where-Object ProcessId -eq $endpoint.OwningProcess | Select-Object -First 1
                $observed.Add("PID $($endpoint.OwningProcess) $($owner.Name) | UDP $($endpoint.LocalAddress):$($endpoint.LocalPort) | $($owner.CommandLine)")
            }
        }
        Start-Sleep -Milliseconds 100
    }
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}

if ($observed.Count -gt 0) {
    $observed | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
    throw "SeqQC process family opened a TCP or UDP endpoint during observation."
}

if ($observedProcesses.Count -eq 0) {
    throw 'Process-family observation returned no processes; the gate cannot pass.'
}

$processNames = [string[]]($observedProcesses | Sort-Object)
Write-Output "Observed process family: $([string]::Join(', ', $processNames))"
Write-Output "No TCP or UDP endpoints were observed in the SeqQC process family for $Seconds seconds."
