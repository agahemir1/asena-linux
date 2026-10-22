$ErrorActionPreference = 'Stop'
$repoDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location -LiteralPath $repoDirectory
try {
    & "$PSScriptRoot/start-dev-vm.ps1"
    $sshArguments = @('-i', 'target/vm/dev-key', '-p', '22310', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=3', '-o', 'UserKnownHostsFile=target/vm/known_hosts', 'asena@127.0.0.1')
    $connected = $false
    for ($attempt = 0; $attempt -lt 45; $attempt++) {
        & ssh @sshArguments true 2>$null
        if ($LASTEXITCODE -eq 0) { $connected = $true; break }
        Start-Sleep -Seconds 2
    }
    if (-not $connected) { throw 'The Debian VM did not become ready. Check target/vm/serial.log.' }
    # Add the loopback HTTP forward to a VM started by an older launcher.
    $listener = Get-NetTCPConnection -State Listen -LocalPort 22380 -ErrorAction SilentlyContinue
    if (-not $listener) {
        $monitorClient = [System.Net.Sockets.TcpClient]::new('127.0.0.1', 22311)
        try {
            $monitorStream = $monitorClient.GetStream()
            $monitorBytes = [System.Text.Encoding]::ASCII.GetBytes("hostfwd_add net0 tcp:127.0.0.1:22380-:6080`n")
            $monitorStream.Write($monitorBytes, 0, $monitorBytes.Length)
            Start-Sleep -Milliseconds 200
        } finally { $monitorClient.Dispose() }
    }
    $archivePath = 'target/vm/preview-source.tar.gz'
    & tar -czf $archivePath --exclude=./target --exclude=./.git -C $repoDirectory .
    if ($LASTEXITCODE -ne 0) { throw 'Could not archive the source files.' }
    & scp -i target/vm/dev-key -P 22310 -o BatchMode=yes -o UserKnownHostsFile=target/vm/known_hosts $archivePath asena@127.0.0.1:/home/asena/preview-source.tar.gz
    if ($LASTEXITCODE -ne 0) { throw 'Could not copy source files to Debian.' }
    & ssh @sshArguments 'cd /home/asena/asena && tar xzf ../preview-source.tar.gz && bash scripts/preview-desktop.sh stop && bash scripts/preview-desktop.sh start'
    if ($LASTEXITCODE -ne 0) { throw 'Desktop preview failed. Check target/preview logs in Debian.' }
    Write-Output 'Open: http://127.0.0.1:22380/vnc.html?autoconnect=true&resize=scale'
} finally { Pop-Location }
