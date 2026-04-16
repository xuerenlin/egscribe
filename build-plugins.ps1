param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

$TargetDir = Join-Path $ScriptDir "target\$Profile"
$OutputDir = Join-Path $ScriptDir "output"
$PluginsDir = Join-Path $OutputDir "plugins"

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}
if (-not (Test-Path $PluginsDir)) {
    New-Item -ItemType Directory -Path $PluginsDir | Out-Null
}

$IsWindows = $PSVersionTable.Platform -eq "Win32NT" -or $env:OS -eq "Windows_NT"
$PluginExeSuffix = if ($IsWindows) { ".exe" } else { "" }

Write-Host "Copying plugin files to $PluginsDir..."

$PluginFolders = Get-ChildItem -Path (Join-Path $ScriptDir "plugins") -Directory
foreach ($Plugin in $PluginFolders) {
    $PluginName = $Plugin.Name
    if ($PluginName -eq "plugin_sdk") {
        Write-Host "  Skipping internal crate: $PluginName"
        continue
    }
    $PluginDir = $Plugin.FullName
    $PluginDescFile = Join-Path $PluginDir "desc.json"

    $IsPythonPlugin = $false
    if (Test-Path $PluginDescFile) {
        $DescContent = Get-Content -Path $PluginDescFile -Raw
        if ($DescContent -match '"exe_path"\s*:\s*"python') {
            $IsPythonPlugin = $true
        }
    }

    $PluginDstDir = Join-Path $PluginsDir $PluginName
    if (-not (Test-Path $PluginDstDir)) {
        New-Item -ItemType Directory -Path $PluginDstDir | Out-Null
    }

    if ($IsPythonPlugin) {
        Get-ChildItem -Path $PluginDir -File -Filter "*.py" -ErrorAction SilentlyContinue | ForEach-Object {
            Copy-Item -Path $_.FullName -Destination (Join-Path $PluginDstDir $_.Name) -Force
        }

        Get-ChildItem -Path $PluginDir -File -ErrorAction SilentlyContinue | Where-Object {
            ($_.Extension -in ".json", ".md") -and (-not $_.Name.EndsWith(".example"))
        } | ForEach-Object {
            Copy-Item -Path $_.FullName -Destination (Join-Path $PluginDstDir $_.Name) -Force
        }

        if (Test-Path $PluginDescFile) {
            Copy-Item -Path $PluginDescFile -Destination (Join-Path $PluginDstDir "desc.json") -Force
            Write-Host "  [OK] Created desc.json for Python plugin: $PluginName"
        }

        Write-Host "  [OK] Copied Python plugin: $PluginName"
    }
    else {
        Write-Host "  Building Rust plugin: $PluginName"
        if ($Profile -eq "release") {
            & cargo build --release --manifest-path (Join-Path $PluginDir "Cargo.toml")
        }
        else {
            & cargo build --manifest-path (Join-Path $PluginDir "Cargo.toml")
        }

        $PluginExe = "$PluginName$PluginExeSuffix"
        $PluginSrc = Join-Path $TargetDir $PluginExe
        $PluginDst = Join-Path $PluginDstDir $PluginExe

        if (Test-Path $PluginSrc) {
            Copy-Item -Path $PluginSrc -Destination $PluginDst -Force
            Write-Host "  [OK] Copied $PluginExe"
        }
        else {
            Write-Warning "Plugin executable not found: $PluginSrc"
        }

        if (Test-Path $PluginDescFile) {
            Copy-Item -Path $PluginDescFile -Destination (Join-Path $PluginDstDir "desc.json") -Force
            Write-Host "  [OK] Created desc.json for Rust plugin: $PluginName"
        }
    }
}

if ($Profile -eq "release") {
    $MainExe = if ($IsWindows) { "egscribe.exe" } else { "egscribe" }
    $MainSrc = Join-Path $TargetDir $MainExe
    $MainDst = Join-Path $OutputDir $MainExe

    if (Test-Path $MainSrc) {
        Copy-Item -Path $MainSrc -Destination $MainDst -Force
        Write-Host "Copied main binary to $MainDst"
    }
    else {
        Write-Warning "Main executable not found: $MainSrc"
    }
}

Write-Host "Done!"
