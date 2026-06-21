#!/usr/bin/env bash
# shellcheck disable=SC2034
# =============================================================================
# Theonix OS — ArchISO Profile Definition
# =============================================================================
# This file defines the identity and build configuration of the Theonix OS ISO.
# Modified from the stock Arch Linux releng profile.
# =============================================================================

iso_name="theonix-os"
iso_label="THEONIX_$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y%m)"
iso_publisher="Theonix OS <https://theonix.org>"
iso_application="Theonix OS — AI-Powered Linux"
iso_version="$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y.%m.%d)"
install_dir="theonix"

buildmodes=('iso')

bootmodes=(
    'bios.syslinux'
    'uefi.systemd-boot'
)

pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=(
    '-comp' 'zstd'
    '-Xcompression-level' '3'
    '-b' '1M'
)
bootstrap_tarball_compression=(
    'zstd' '-c' '-T0' '--auto-threads=logical' '--long' '-19'
)

file_permissions=(
    # Security-critical files
    ["/etc/shadow"]="0:0:000"
    ["/etc/gshadow"]="0:0:000"
    ["/etc/sudoers.d/11-live-session"]="0:0:0440"

    # Root home directory
    ["/root"]="0:0:750"

    # Executable scripts
    ["/root/.automated_script.sh"]="0:0:700"
    ["/root/.gnupg"]="0:0:700"

    # Theonix helper scripts
    ["/usr/local/bin/theonix-enable-ssh"]="0:0:755"
    ["/usr/local/bin/theonix-recovery"]="0:0:755"

    # Legacy Arch helpers (retained for compatibility)
    ["/usr/local/bin/choose-mirror"]="0:0:755"
    ["/usr/local/bin/Installation_guide"]="0:0:755"
    ["/usr/local/bin/livecd-sound"]="0:0:755"
)
