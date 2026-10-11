$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'process-tree.ps1')
function Process([int]$Id, [int]$Parent, [int]$Created) {
    [pscustomobject]@{ ProcessId = $Id; ParentProcessId = $Parent; CreationDate = [datetime]'2026-10-11T00:00:00Z' + [timespan]::FromSeconds($Created) }
}
# Deliberately put descendants before parents to require transitive discovery.
$processes = @(
    (Process 500 400 50), (Process 300 200 20), (Process 600 999 45),
    (Process 200 400 10), (Process 400 100 40), (Process 700 100 30), (Process 100 999 30)
)
$actual = @((Get-OwnedProcesses $processes 100).ProcessId | Sort-Object)
if (($actual -join ',') -ne '100,400,500,700') {
    throw "PID reuse must exclude older children and their descendants: $actual"
}
if (@(Get-OwnedProcesses $processes 999).Count -ne 0) {
    throw 'An absent root must not claim processes with a stale parent PID'
}
Write-Output 'Owned process creation-order contracts passed'
