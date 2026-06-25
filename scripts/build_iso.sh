#!/bin/bash
set -e

WORKDIR="/workdir/work"
OUTDIR="/workdir/out"
PROFILE_DIR="/workdir/profile_build"

# Ensure directories exist
mkdir -p "$WORKDIR"
mkdir -p "$OUTDIR"

echo "=== Theonix OS ISO Builder ==="

[ "$EUID" -eq 0 ] || { echo "Error: Please run as root."; exit 1; }
command -v mkarchiso &>/dev/null || { echo "Error: archiso not installed."; exit 1; }

echo "Copying clean releng profile..."
rm -rf "$PROFILE_DIR"
cp -r /usr/share/archiso/configs/releng/ "$PROFILE_DIR"

echo "Overlaying custom profile configurations..."
cp -aT /workdir/profile/ "$PROFILE_DIR"

echo "Customizing packages..."
cat <<EOF >> "$PROFILE_DIR/packages.x86_64"
# Theonix OS Core Packages
wayland
plasma-meta
sddm
ollama
networkmanager
pipewire
wireplumber
apparmor
plymouth
konsole
# Explicit providers to avoid pacman interactive prompts
pipewire-jack
noto-fonts
iptables-nft
qt6-multimedia-ffmpeg
tesseract-data-eng
EOF

# The releng profile defaults to the -nox version (no X11/Wayland support)
# We need the full version for KDE Plasma to support dynamic resolution
sed -i 's/virtualbox-guest-utils-nox/virtualbox-guest-utils/g' "$PROFILE_DIR/packages.x86_64"

echo "Applying Theonix OS Branding..."
find "$PROFILE_DIR/syslinux" -type f -name "*.cfg" -exec sed -i 's/Arch Linux/Theonix OS/g' {} +
find "$PROFILE_DIR/efiboot/loader/entries" -type f -name "*.conf" -exec sed -i 's/Arch Linux/Theonix OS/g' {} +
find "$PROFILE_DIR/grub" -type f -name "*.cfg" -exec sed -i 's/Arch Linux/Theonix OS/g' {} +
sed -i 's/Arch Linux/Theonix OS/g' "$PROFILE_DIR/airootfs/etc/motd"

echo "Patching profiledef.sh..."
sed -i "s/iso_name=\"archlinux\"/iso_name=\"theonix-os\"/g" "$PROFILE_DIR/profiledef.sh"
sed -i "s/iso_label=\"ARCH_\$(date +%Y%m)\"/iso_label=\"THEONIX_$(date +%Y%m)\"/g" "$PROFILE_DIR/profiledef.sh"
sed -i "s/iso_publisher=\"Arch Linux <https:\/\/archlinux.org>\"/iso_publisher=\"Theonix OS <https:\/\/theonix.org>\"/g" "$PROFILE_DIR/profiledef.sh"
sed -i "s/iso_application=\"Arch Linux live\/rescue disk\"/iso_application=\"Theonix OS Live\"/g" "$PROFILE_DIR/profiledef.sh"

cat <<EOF > "$PROFILE_DIR/airootfs/etc/os-release"
NAME="Theonix OS"
PRETTY_NAME="Theonix OS"
ID=theonix
BUILD_ID=rolling
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://theonix.org/"
DOCUMENTATION_URL="https://theonix.org/docs"
SUPPORT_URL="https://theonix.org/support"
BUG_REPORT_URL="https://github.com/theonix-os/issues"
LOGO=theonix-logo
AUTHOR="Kelvin Benny Koshy"
THEONIX_CREATOR="Kelvin Benny Koshy"
EOF

echo "Writing archiso.conf with zstd compression..."
mkdir -p "$PROFILE_DIR/airootfs/etc/mkinitcpio.conf.d"
cat <<EOF > "$PROFILE_DIR/airootfs/etc/mkinitcpio.conf.d/archiso.conf"
HOOKS=(base udev plymouth microcode modconf kms memdisk archiso archiso_loop_mnt archiso_pxe_common archiso_pxe_nbd archiso_pxe_http archiso_pxe_nfs block filesystems keyboard)
COMPRESSION="zstd"
COMPRESSION_OPTIONS=(-19 --long)
EOF

mkdir -p "$PROFILE_DIR/airootfs/etc/systemd/system/multi-user.target.wants"
cat <<EOF > "$PROFILE_DIR/airootfs/etc/systemd/system/live-user.service"
[Unit]
Description=Create Live User
Before=sddm.service display-manager.service

[Service]
Type=oneshot
ExecStart=-/usr/bin/useradd -m -g users -G wheel,video,audio,input,storage -s /bin/bash theonix
ExecStart=-/usr/bin/passwd -d theonix
ExecStart=/bin/sh -c "mkdir -p /etc/sudoers.d && echo '%wheel ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/10-wheel-nopasswd"
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF
ln -sf /etc/systemd/system/live-user.service "$PROFILE_DIR/airootfs/etc/systemd/system/multi-user.target.wants/live-user.service"

mkdir -p "$PROFILE_DIR/airootfs/etc/sddm.conf.d"
cat <<EOF > "$PROFILE_DIR/airootfs/etc/sddm.conf.d/autologin.conf"
[Autologin]
User=theonix
Session=plasma
EOF

mkdir -p "$WORKDIR" "$OUTDIR"

echo "Ensuring required packages are included..."
for pkg in plymouth calamares; do
    grep -q "^$pkg$" "$PROFILE_DIR/packages.x86_64" || echo "$pkg" >> "$PROFILE_DIR/packages.x86_64"
done

sed -i '/^snapper$/d' "$PROFILE_DIR/packages.x86_64"
sed -i '/^grub-btrfs$/d' "$PROFILE_DIR/packages.x86_64"

echo "Injecting UI themes..."
mkdir -p "$PROFILE_DIR/airootfs/etc/theonix"
[ -f "design/ui_mockups/theme.qss" ] && cp "design/ui_mockups/theme.qss" "$PROFILE_DIR/airootfs/etc/theonix/"

echo "Installing custom design themes..."
mkdir -p "$PROFILE_DIR/airootfs/usr/share/sddm/themes"
cp -a /workdir/design/themes/sddm/theonix "$PROFILE_DIR/airootfs/usr/share/sddm/themes/" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/plymouth/themes"
cp -a /workdir/design/themes/plymouth/theonix "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/plasma/look-and-feel"
cp -a /workdir/design/themes/kde/theonix "$PROFILE_DIR/airootfs/usr/share/plasma/look-and-feel/org.theonix.desktop" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/color-schemes"
cp -a /workdir/design/themes/kde/theonix/colors/Theonix.colors "$PROFILE_DIR/airootfs/usr/share/color-schemes/" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/wallpapers/Theonix/contents/images"
cp -a /workdir/design/themes/kde/theonix/contents/images/3840x2160.png "$PROFILE_DIR/airootfs/usr/share/wallpapers/Theonix/contents/images/" 2>/dev/null || true
cp -a /workdir/design/themes/kde/wallpaper_metadata.desktop "$PROFILE_DIR/airootfs/usr/share/wallpapers/Theonix/metadata.desktop" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/grub/themes"
cp -a /workdir/design/themes/grub/theonix "$PROFILE_DIR/airootfs/usr/share/grub/themes/" 2>/dev/null || true

echo "Applying dark theme to root user for Calamares pkexec compatibility..."
mkdir -p "$PROFILE_DIR/airootfs/root/.config"
cat <<EOF > "$PROFILE_DIR/airootfs/root/.config/kdeglobals"
[General]
ColorScheme=Theonix
[KDE]
widgetStyle=Breeze
EOF

echo "Installing Calamares custom configurations (only custom/extra modules, not package defaults)..."
mkdir -p "$PROFILE_DIR/airootfs/etc/calamares"
[ -f /workdir/calamares/settings.conf ] && cp /workdir/calamares/settings.conf "$PROFILE_DIR/airootfs/etc/calamares/" 2>/dev/null || true
[ -d /workdir/calamares/branding ] && cp -a /workdir/calamares/branding "$PROFILE_DIR/airootfs/etc/calamares/" 2>/dev/null || true

# To prevent pacman file conflicts during mkarchiso pacstrap, stage modules in a safe location
mkdir -p "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules"
cp -a /workdir/calamares/modules/* "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules/" 2>/dev/null || true
chmod +x "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules"/*.sh 2>/dev/null || true

# Create a pacman hook to copy the custom modules over the package defaults after installation
mkdir -p "$PROFILE_DIR/airootfs/etc/pacman.d/hooks"
cat <<EOF > "$PROFILE_DIR/airootfs/etc/pacman.d/hooks/99-calamares-custom.hook"
[Trigger]
Operation = Install
Operation = Upgrade
Type = Package
Target = calamares

[Action]
Description = Applying custom Calamares module configurations...
When = PostTransaction
Exec = /usr/bin/cp -af /usr/local/share/calamares_custom/modules/. /etc/calamares/modules/
EOF

echo ""
echo "=== Calamares Module Validation ==="
CALAMARES_SETTINGS="$PROFILE_DIR/airootfs/etc/calamares/settings.conf"
CALAMARES_LOCAL="$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules"

MODULES=$(grep -E '^\s+- ' "$CALAMARES_SETTINGS" 2>/dev/null | sed 's/#.*//; s/^\s*- //; s/\s*$//' | grep -v -E '^(local|/usr/lib/calamares/modules|show:|exec:)$' | grep -v '^\s*$' | sort -u)


echo "Module validation report:"
echo "-------------------------------------------"

while IFS= read -r module; do
    [ -z "$module" ] && continue
    if [[ "$module" == *"@"* ]]; then
        base="${module%%@*}"
        instance="${module#*@}"
    else
        base="$module"
        instance=""
    fi

    found=0
    [ -f "$CALAMARES_LOCAL/${base}.conf" ] && found=1
    if [ "$base" = "shellprocess" ] && [ -n "$instance" ]; then
        [ -f "$CALAMARES_LOCAL/shellprocess_${instance}.conf" ] && found=1
    fi

    if [ $found -eq 1 ]; then
        echo "  OK:   $module"
    else
        echo "  FAIL: $module (will use package default)"
    fi
done <<< "$MODULES"
echo "-------------------------------------------"
echo "All modules will be installed (standard from package, custom where provided)!"
echo ""

echo "Configuring systemd targets for graphical boot..."
mkdir -p "$PROFILE_DIR/airootfs/etc/systemd/system/multi-user.target.wants"
ln -sf /usr/lib/systemd/system/sddm.service "$PROFILE_DIR/airootfs/etc/systemd/system/display-manager.service" 2>/dev/null || true
ln -sf /usr/lib/systemd/system/graphical.target "$PROFILE_DIR/airootfs/etc/systemd/system/default.target" 2>/dev/null || true
ln -sf /usr/lib/systemd/system/NetworkManager.service "$PROFILE_DIR/airootfs/etc/systemd/system/multi-user.target.wants/NetworkManager.service" 2>/dev/null || true
ln -sf /usr/lib/systemd/system/systemd-resolved.service "$PROFILE_DIR/airootfs/etc/systemd/system/multi-user.target.wants/systemd-resolved.service" 2>/dev/null || true

chmod +x "$PROFILE_DIR/airootfs/usr/local/bin/force-resolution.sh" 2>/dev/null || true

echo "Starting ISO build..."
mkarchiso -v -w "$WORKDIR" -o "$OUTDIR" "$PROFILE_DIR" 2>&1 | grep -v "WARNING: Cannot change permissions" || true

echo "=== Build Complete ==="
