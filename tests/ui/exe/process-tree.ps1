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
