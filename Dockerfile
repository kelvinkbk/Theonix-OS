FROM archlinux:latest

# =============================================================================
# Theonix OS — Docker Build Image
# =============================================================================
# Provides mkarchiso and all tools needed to build the Theonix OS ISO.
# This image runs WITHOUT --privileged; only SYS_ADMIN + MKNOD caps are used.
# =============================================================================

# Update keyring first to avoid signature errors
# hadolint ignore=DL3018
RUN pacman -Sy --noconfirm archlinux-keyring && \
    pacman -Syu --noconfirm && \
    pacman -S --noconfirm --needed \
        archiso \
        mkinitcpio \
        squashfs-tools \
        dosfstools \
        mtools \
        libisoburn \
        erofs-utils \
        shellcheck \
        gnupg \
        reflector \
        curl && \
    # Clean package cache to reduce image size
    pacman -Scc --noconfirm && \
    rm -rf /var/cache/pacman/pkg/*

# Create a dedicated build group (mkarchiso still requires root to run)
RUN groupadd -r theonix-build

WORKDIR /workdir

# Default command: run the build script
CMD ["/bin/bash", "scripts/build_iso.sh"]
