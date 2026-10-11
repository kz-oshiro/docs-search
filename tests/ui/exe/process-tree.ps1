function Get-OwnedProcesses {
    param([object[]]$Processes, [int]$RootId)
    $root = $Processes | Where-Object { $_.ProcessId -eq $RootId }
    if (-not $root) { return }
    $owned = @{ $RootId = $root }
    do {
        $added = $false
        foreach ($candidate in $Processes) {
            $candidateId = [int]$candidate.ProcessId
            $parent = $owned[[int]$candidate.ParentProcessId]
            # ParentProcessId can refer to an exited process whose PID has since
            # been reused. A real child cannot predate its current parent.
            if (-not $owned.ContainsKey($candidateId) -and $parent -and
                $candidate.CreationDate -ge $parent.CreationDate) {
                $owned[$candidateId] = $candidate
                $added = $true
            }
        }
    } while ($added)
    $Processes | Where-Object { $owned.ContainsKey([int]$_.ProcessId) }
}

function Wait-OwnedProcessesExit {
    param([object[]]$Saved, [scriptblock]$ReadProcesses = { Get-CimInstance Win32_Process }, [int]$TimeoutMs = 5000)
    $timer = [Diagnostics.Stopwatch]::StartNew()
    do {
        $processes = @(& $ReadProcesses)
        $remaining = @(foreach ($entry in $Saved) {
            $processes | Where-Object { $_.ProcessId -eq $entry.id -and $_.CreationDate.ToString('o') -eq $entry.created }
        })
        if (-not $remaining.Count) { return }
        # CIM can briefly retain a process after its handle signals termination.
        # Re-observe until absent, with one deadline for the entire owned tree.
    } while ($timer.ElapsedMilliseconds -lt $TimeoutMs)
    throw "Owned EXE/WebView2 process could not be terminated: $($remaining.ProcessId -join ',')"
}
