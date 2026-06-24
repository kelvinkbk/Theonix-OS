#!/bin/bash
set -e

echo "==> Running post-install configurations..."
sudo cp /etc/calamares/modules/theonix-postinstall.sh /mnt/root/
sudo arch-chroot /mnt bash /root/theonix-postinstall.sh

echo "==> Generating GRUB menu..."
sudo arch-chroot /mnt /usr/bin/grub-mkconfig -o /boot/grub/grub.cfg

echo "==> Done! Unmounting and rebooting..."
sudo umount -R /mnt || true
reboot
