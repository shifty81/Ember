[CmdletBinding()]
param()
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
& (Join-Path $Root "PROJECT_CONTROL_CENTER.cmd") --git-repair
exit $LASTEXITCODE
