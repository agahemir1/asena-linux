param(
    [int]$MemoryMiB = 6144,
    [int]$Cpus = 4
)

$ErrorActionPreference = 'Stop'
$vmDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../target/vm'))
$qemuExecutable = Join-Path $vmDirectory 'msys/ucrt64/bin/qemu-system-x86_64.exe'
foreach ($required in @($qemuExecutable, (Join-Path $vmDirectory 'asena-dev.qcow2'), (Join-Path $vmDirectory 'seed.iso'))) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "Missing VM prerequisite: $required. See docs/DEVELOPMENT.md."
    }
}
$pidFile = Join-Path $vmDirectory 'qemu.pid'
if (Test-Path -LiteralPath $pidFile) {
    $previousVmPid = [int](Get-Content -LiteralPath $pidFile)
    $previousVm = Get-Process -Id $previousVmPid -ErrorAction SilentlyContinue
    if ($previousVm -and $previousVm.Path -eq $qemuExecutable) {
        Write-Output "Asena VM is already running (PID $previousVmPid)."
        exit 0
    }
}
$qemuArguments = @(
    '-name', 'asena-dev', '-machine', 'q35',
    '-accel', 'whpx', '-accel', 'tcg,thread=multi', '-cpu', 'max',
    '-m', $MemoryMiB, '-smp', $Cpus,
    '-L', 'msys/ucrt64/share/qemu',
    '-drive', 'file=asena-dev.qcow2,if=virtio,format=qcow2',
    '-drive', 'file=seed.iso,media=cdrom,readonly=on',
    '-netdev', 'user,id=net0,hostfwd=tcp:127.0.0.1:22310-:22,hostfwd=tcp:127.0.0.1:22380-:6080',
    '-device', 'virtio-net-pci,netdev=net0',
    '-display', 'none', '-serial', 'file:serial.log',
    '-monitor', 'tcp:127.0.0.1:22311,server=on,wait=off', '-no-reboot'
)
$vmProcess = Start-Process -FilePath $qemuExecutable -ArgumentList $qemuArguments `
    -WorkingDirectory $vmDirectory -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput (Join-Path $vmDirectory 'qemu.stdout.log') `
    -RedirectStandardError (Join-Path $vmDirectory 'qemu.stderr.log')
Set-Content -LiteralPath $pidFile -Value $vmProcess.Id
Write-Output "Started Asena VM (PID $($vmProcess.Id)). SSH: asena@127.0.0.1 port 22310."
