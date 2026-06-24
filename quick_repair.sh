#!/bin/bash
set -e

echo "==> Mounting partitions..."
sudo mount /dev/sda2 /mnt || true
sudo mkdir -p /mnt/boot/efi
sudo mount /dev/sda1 /mnt/boot/efi || true

echo "==> Installing GRUB..."
sudo arch-chroot /mnt /usr/bin/grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=theonix --force

echo "==> Creating EFI fallback..."
sudo arch-chroot /mnt mkdir -p /boot/efi/EFI/BOOT
sudo arch-chroot /mnt cp /boot/efi/EFI/theonix/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
sudo arch-chroot /mnt bash -c 'echo "\EFI\BOOT\BOOTX64.EFI" > /boot/efi/startup.nsh'

echo "==> Running post-install configurations..."
sudo cp /etc/calamares/modules/theonix-postinstall.sh /mnt/tmp/
sudo arch-chroot /mnt bash /tmp/theonix-postinstall.sh

echo "==> Generating GRUB menu..."
sudo arch-chroot /mnt /usr/bin/grub-mkconfig -o /boot/grub/grub.cfg

echo "==> Done! Unmounting and rebooting..."
sudo umount -R /mnt
reboot
