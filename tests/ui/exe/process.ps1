param([ValidateSet('owner', 'snapshot', 'close', 'cleanup')][string]$Action)
$ErrorActionPreference = 'Stop'
$processes = @(Get-CimInstance Win32_Process)
$rootId = [int]$env:DOCS_SEARCH_CHILD_PID
$root = $processes | Where-Object { $_.ProcessId -eq $rootId }
if ($root -and $env:DOCS_SEARCH_CHILD_CREATED -and $root.CreationDate.ToString('o') -ne $env:DOCS_SEARCH_CHILD_CREATED) { throw 'EXE PID was reused; refusing process operations' }
if ($Action -in @('cleanup', 'close')) {
    # Windows PowerShell 5.1 emits a JSON array as one pipeline object. Wrapping
    # ConvertFrom-Json in @() would nest it and prevent per-PID identity matching.
    $saved = Get-Content -LiteralPath $env:DOCS_SEARCH_CHILD_SNAPSHOT -Raw | ConvertFrom-Json
    foreach ($entry in $saved) {
        $current = $processes | Where-Object { $_.ProcessId -eq $entry.id }
        if ($current -and $current.CreationDate.ToString('o') -eq $entry.created) {
            if ($Action -eq 'close') {
                if ($entry.id -eq $rootId) {
                    $native = Get-Process -Id $entry.id
                    if ($native.MainWindowTitle -ne 'docs-search' -or -not $native.CloseMainWindow()) { throw 'The launched EXE has no closeable product window' }
                    @{ pid = $rootId; title = $native.MainWindowTitle } | ConvertTo-Json -Compress
                }
            } else {
                Stop-Process -Id $entry.id -Force -ErrorAction SilentlyContinue
            }
        }
    }
    if ($Action -eq 'cleanup') {
        foreach ($entry in $saved) {
            Wait-Process -Id $entry.id -Timeout 5 -ErrorAction SilentlyContinue
        }
        $remaining = @(Get-CimInstance Win32_Process)
        foreach ($entry in $saved) {
            if ($remaining | Where-Object { $_.ProcessId -eq $entry.id -and $_.CreationDate.ToString('o') -eq $entry.created }) { throw 'Owned EXE/WebView2 process could not be terminated' }
        }
    }
    exit 0
}
$owned = @($rootId)
do {
    $children = @($processes | Where-Object { $owned -contains [int]$_.ParentProcessId -and $owned -notcontains [int]$_.ProcessId } | ForEach-Object { [int]$_.ProcessId })
    $owned += $children
} while ($children.Count)
if ($Action -eq 'owner') {
    $listeners = @(Get-NetTCPConnection -State Listen -LocalPort ([int]$env:DOCS_SEARCH_CHILD_PORT) -ErrorAction SilentlyContinue)
    $valid = @($listeners | Where-Object { $owned -contains [int]$_.OwningProcess })
    if (-not $listeners.Count -or $valid.Count -ne $listeners.Count) { throw 'CDP port is not owned by the launched EXE process tree' }
    foreach ($listener in $valid) {
        $owner = $processes | Where-Object { $_.ProcessId -eq $listener.OwningProcess }
        if (-not $owner.CommandLine -or $owner.CommandLine.IndexOf($env:DOCS_SEARCH_CHILD_PROFILE, [StringComparison]::OrdinalIgnoreCase) -lt 0) { throw 'CDP WebView2 is not using the isolated test profile' }
    }
    @{ rootCreated = $root.CreationDate.ToString('o'); listeners = @($valid | Select-Object LocalAddress, LocalPort, OwningProcess) } | ConvertTo-Json -Compress
} else {
    $result = @($processes | Where-Object { $owned -contains [int]$_.ProcessId } | ForEach-Object { @{ id = [int]$_.ProcessId; created = $_.CreationDate.ToString('o') } })
    ConvertTo-Json -InputObject $result -Compress
}
