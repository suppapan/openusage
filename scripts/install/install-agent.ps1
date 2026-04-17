# OpenUsage Agent installer for Windows.
#
# One-liner install (interactive prompts):
#   iwr https://github.com/suppapan/openusage/releases/latest/download/install-agent.ps1 | iex
#
# Non-interactive install:
#   $env:OPENUSAGE_TOKEN = "YOUR_TOKEN"
#   $env:OPENUSAGE_RELAY = "https://relay.example.com:8090"
#   iwr https://github.com/suppapan/openusage/releases/latest/download/install-agent.ps1 | iex
#
# Or invoke directly with params:
#   & ([scriptblock]::Create((iwr https://.../install-agent.ps1))) -Token "X" -Relay "Y"

param(
  [string]$Token = $env:OPENUSAGE_TOKEN,
  [string]$Relay = $env:OPENUSAGE_RELAY,
  [string]$MachineName = $env:OPENUSAGE_MACHINE_NAME,
  [int]$Interval = 300,
  [string]$InstallDir = "$env:LOCALAPPDATA\OpenUsage",
  [switch]$NoService
)

$ErrorActionPreference = 'Stop'
$Repo = 'suppapan/openusage'

Write-Host ">>> OpenUsage Agent installer"

# Detect arch
$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x86_64' } else { Throw 'Only 64-bit Windows is supported' }
$BinaryName = "openusage-agent-windows-$Arch.exe"
Write-Host "    Platform: windows-$Arch"
Write-Host "    Binary:   $BinaryName"

# Resolve latest release
$LatestApi = "https://api.github.com/repos/$Repo/releases/latest"
$Release = Invoke-RestMethod -Uri $LatestApi
$Tag = $Release.tag_name
Write-Host "    Release:  $Tag"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Tag/$BinaryName"

# Prompt for missing values
if (-not $Token) {
  $Token = Read-Host "Sync token"
}
if (-not $Relay) {
  $Relay = Read-Host "Relay URL (e.g. https://relay.example.com:8090)"
}
if (-not $Token -or -not $Relay) {
  Throw "Token and relay URL are required"
}

# Install binary
New-Item -Type Directory -Path $InstallDir -Force | Out-Null
$TargetBin = Join-Path $InstallDir 'openusage-agent.exe'
Write-Host ">>> Downloading from $DownloadUrl"
Invoke-WebRequest -Uri $DownloadUrl -OutFile $TargetBin
Write-Host ">>> Installed to $TargetBin"

if ($NoService) {
  Write-Host ""
  Write-Host ">>> Done. Run manually with:"
  Write-Host "    $TargetBin --token $Token --relay $Relay"
  exit 0
}

# Register as scheduled task (simpler than Windows service for user-level setup)
$TaskName = 'OpenUsageAgent'
$TaskArgs = "--token `"$Token`" --relay `"$Relay`" --interval $Interval"
if ($MachineName) {
  $TaskArgs += " --machine-name `"$MachineName`""
}

Write-Host ">>> Registering scheduled task '$TaskName'"
$Action = New-ScheduledTaskAction -Execute $TargetBin -Argument $TaskArgs
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)

try {
  Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
} catch {}

Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Settings $Settings -Force | Out-Null
Start-ScheduledTask -TaskName $TaskName

Write-Host ""
Write-Host ">>> Agent running as scheduled task '$TaskName'."
Write-Host ">>> Check status: Get-ScheduledTask -TaskName $TaskName"
Write-Host ">>> Stop:         Stop-ScheduledTask -TaskName $TaskName"
Write-Host ">>> Uninstall:    Unregister-ScheduledTask -TaskName $TaskName -Confirm:`$false"
