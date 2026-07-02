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

echo "Updating Arch Linux mirrors to prevent download errors..."
reflector --latest 10 --protocol https --sort rate --save /etc/pacman.d/mirrorlist

echo "Copying clean releng profile..."
rm -rf "$PROFILE_DIR"
cp -r /usr/share/archiso/configs/releng/ "$PROFILE_DIR"

echo "Overlaying custom profile configurations..."
cp -aT /workdir/profile/ "$PROFILE_DIR"

# Deduplicate package list (profile may contain duplicates from manual edits)
awk '!/^#/ && NF { if ($1 in seen) next; seen[$1]=1 } { print }' \
    "$PROFILE_DIR/packages.x86_64" > "$PROFILE_DIR/packages.x86_64.tmp"
mv "$PROFILE_DIR/packages.x86_64.tmp" "$PROFILE_DIR/packages.x86_64"

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
MODULES=(vboxvideo)
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

# Removed manual copy to etc/theonix as it is provided by theonix-config package
echo "Installing custom design themes..."
mkdir -p "$PROFILE_DIR/airootfs/usr/share/sddm/themes"
cp -a /workdir/design/themes/sddm/theonix "$PROFILE_DIR/airootfs/usr/share/sddm/themes/" 2>/dev/null || true

mkdir -p "$PROFILE_DIR/airootfs/usr/share/plymouth/themes"
cp -a /workdir/design/themes/plymouth/theonix "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/" 2>/dev/null || true
# Ensure background.png exists for the Plymouth script theme
if [ ! -f "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/theonix/background.png" ]; then
    cp /workdir/design/themes/kde/theonix/contents/images/3840x2160.png \
        "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/theonix/background.png" 2>/dev/null || \
    cp "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/theonix/logo.png" \
        "$PROFILE_DIR/airootfs/usr/share/plymouth/themes/theonix/background.png" 2>/dev/null || true
fi

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

echo "Installing Calamares custom configurations (staged, applied after pacstrap)..."
# Never place module .conf files under etc/calamares/modules before pacstrap —
# the calamares package owns those paths and pacman will abort on conflicts.
rm -rf "$PROFILE_DIR/airootfs/etc/calamares/modules"

mkdir -p "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules"
cp -a /workdir/calamares/modules/* "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules/" 2>/dev/null || true
chmod +x "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/modules"/*.sh 2>/dev/null || true

[ -f /workdir/calamares/settings.conf ] && \
    cp /workdir/calamares/settings.conf "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/settings.conf"
[ -d /workdir/calamares/branding ] && \
    cp -a /workdir/calamares/branding "$PROFILE_DIR/airootfs/usr/local/share/calamares_custom/"

mkdir -p "$PROFILE_DIR/airootfs/etc/pacman.d/hooks"
cat <<'EOF' > "$PROFILE_DIR/airootfs/etc/pacman.d/hooks/99-calamares-custom.hook"
[Trigger]
Operation = Install
Operation = Upgrade
Type = Package
Target = calamares

[Action]
Description = Applying Theonix Calamares configuration...
When = PostTransaction
NeedsTargets
Exec = /bin/sh -c 'cp -af /usr/local/share/calamares_custom/modules/. /etc/calamares/modules/; [ -f /usr/local/share/calamares_custom/settings.conf ] && cp -f /usr/local/share/calamares_custom/settings.conf /etc/calamares/settings.conf; [ -d /usr/local/share/calamares_custom/branding ] && cp -a /usr/local/share/calamares_custom/branding/. /etc/calamares/branding/'
EOF

echo ""
echo "=== Calamares Module Validation ==="
CALAMARES_SETTINGS="/workdir/calamares/settings.conf"
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
chmod +x "$PROFILE_DIR/airootfs/usr/local/bin/theonix-installer" 2>/dev/null || true
chmod +x "$PROFILE_DIR/airootfs/usr/local/bin/theonix-recovery" 2>/dev/null || true

echo "Generating shadow/gshadow for live ISO..."
/workdir/scripts/generate_shadow.sh "$PROFILE_DIR"

echo "Starting ISO build..."
mkarchiso -v -w "$WORKDIR" -o "$OUTDIR" "$PROFILE_DIR" || { echo "mkarchiso failed!"; exit 1; }

echo "=== Build Complete ==="
