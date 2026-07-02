<#
.SYNOPSIS
    Builds Theonix OS ISO using a Docker container (Windows host).

.DESCRIPTION
    Builds the Arch Linux Docker image containing mkarchiso and all build tools,
    then runs the build_iso.sh script inside the container while mounting the
    project directory so the resulting ISO is saved to the Windows host.

    Security: Uses --cap-add instead of --privileged to follow least-privilege
    principle. Only SYS_ADMIN (loopback mounts) and MKNOD (device creation)
    capabilities are granted — the minimum required by mkarchiso.

.PARAMETER Clean
    If specified, removes any existing 'theonix-builder' Docker image first.

.EXAMPLE
    .\scripts\build_in_docker.ps1
    .\scripts\build_in_docker.ps1 -Clean
#>

[CmdletBinding()]
param(
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# ---------------------------------------------------------------------------
# Locate project root (two levels up from scripts/)
# ---------------------------------------------------------------------------
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

Write-Host "=== Theonix OS Docker Build ===" -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot"     -ForegroundColor Gray

# ---------------------------------------------------------------------------
# Check Docker availability
# ---------------------------------------------------------------------------
try {
    $null = docker info 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Docker returned non-zero exit code." }
}
catch {
    Write-Host "ERROR: Docker Desktop is not running or not installed." -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again."             -ForegroundColor Yellow
    exit 1
}

# ---------------------------------------------------------------------------
# Optional clean rebuild
# ---------------------------------------------------------------------------
if ($Clean) {
    Write-Host "[0/3] Removing existing Docker image..." -ForegroundColor Yellow
    docker rmi --force theonix-builder 2>$null
    Write-Host "      Existing image removed."
}

# ---------------------------------------------------------------------------
# Step 1 — Build the Docker image
# ---------------------------------------------------------------------------
$ImageExists = (docker images -q theonix-builder)
if ($ImageExists) {
    Write-Host "[1/3] Found existing 'theonix-builder' image. Skipping build." -ForegroundColor Green
} else {
    Write-Host "[1/3] Building Theonix build image (this may take a few minutes)..." -ForegroundColor Green
    docker build `
        --tag theonix-builder `
        --label "org.theonix.version=1.0" `
        --label "org.theonix.description=Theonix OS ISO Builder" `
        .
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Docker image build failed." -ForegroundColor Red
        exit 1
    }
}

# ---------------------------------------------------------------------------
# Step 2 — Run mkarchiso inside the container
#
# Security: --privileged is intentionally NOT used.
# mkarchiso requires:
#   SYS_ADMIN  — for loop device mounts (squashfs, FAT images)
#   MKNOD      — to create block device nodes inside the chroot
#
# We also pass /dev/loop-control and a few loop devices rather than
# the entire /dev tree.
# ---------------------------------------------------------------------------
Write-Host "[2/3] Running mkarchiso inside container (least-privilege mode)..." -ForegroundColor Green
Write-Host "      Capabilities granted: SYS_ADMIN, MKNOD"                      -ForegroundColor Gray
Write-Host "      --privileged: NOT used"                                       -ForegroundColor Gray

# Resolve Windows path to a Docker-compatible format
$MountPath = $ProjectRoot -replace '\\', '/'
if ($MountPath -match '^([A-Za-z]):(.*)') {
    $MountPath = "//$($Matches[1].ToLower())$($Matches[2])"
}

docker run `
    --rm `
    --cap-add SYS_ADMIN `
    --cap-add MKNOD `
    --device /dev/loop0 `
    --device /dev/loop1 `
    --device /dev/loop2 `
    --device /dev/loop-control `
    --security-opt apparmor:unconfined `
    --tmpfs /tmp:exec,size=4g `
    --volume "theonix-build-cache:/build-cache" `
    --volume "theonix-pacman-cache:/var/cache/pacman/pkg" `
    --volume "${MountPath}:/workdir" `
    theonix-builder

# ISO is written directly to /workdir/out (mounted from the project root)
if ($LASTEXITCODE -eq 0) {
    Write-Host "[3/3] Build finished — checking for ISO in 'out/'..." -ForegroundColor Green
    New-Item -ItemType Directory -Path "$ProjectRoot\out" -Force | Out-Null
}

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "=== Build Successful! ===" -ForegroundColor Green
    Write-Host "Check the 'out/' directory for your Theonix OS ISO." -ForegroundColor Cyan
    $IsoFiles = Get-ChildItem -Path "$ProjectRoot\out" -Filter "*.iso" -ErrorAction SilentlyContinue
    if ($IsoFiles) {
        foreach ($iso in $IsoFiles) {
            $sizeMB = [math]::Round($iso.Length / 1MB, 1)
            Write-Host "  ISO: $($iso.Name)  ($sizeMB MB)" -ForegroundColor White
        }
    }
}
else {
    Write-Host ""
    Write-Host "=== Build Failed! ===" -ForegroundColor Red
    Write-Host "Check the output above for error details." -ForegroundColor Yellow
    Write-Host "[3/3] Tip: run with -Clean flag to force a fresh image build." -ForegroundColor Yellow
    exit 1
}
