#!/usr/bin/env bash
# =============================================================================
# Theonix OS — scripts/generate_shadow.sh
# =============================================================================
# Generates the /etc/shadow file for the live ISO at build time.
# This script is called by build_iso.sh immediately before mkarchiso.
#
# The live ISO root account is locked (password login disabled).
# Users can only login via the auto-login configured in getty@ override.
#
# Usage: ./scripts/generate_shadow.sh <profile_dir>
#   profile_dir: path to the ArchISO profile directory (default: profile/)
# =============================================================================

set -euo pipefail

PROFILE_DIR="${1:-profile}"
SHADOW_PATH="${PROFILE_DIR}/airootfs/etc/shadow"
GSHADOW_PATH="${PROFILE_DIR}/airootfs/etc/gshadow"

# Validate profile dir exists
if [[ ! -d "${PROFILE_DIR}" ]]; then
    echo "ERROR: Profile directory not found: ${PROFILE_DIR}" >&2
    exit 1
fi

echo "==> Generating shadow file at: ${SHADOW_PATH}"

# Create the shadow file with a locked root account.
# Format: username:password:last_changed:min:max:warn:inactive:expire:reserved
#
# '!' prefix on password hash = account locked (no password login).
# Last changed = days since epoch 0 (Jan 1 1970) — forces no expiry logic.
# All other fields empty = system defaults apply.

# Get current days since epoch for the password last-changed field
DAYS_SINCE_EPOCH=$(( $(date +%s) / 86400 ))

cat > "${SHADOW_PATH}" << EOF
root:!:${DAYS_SINCE_EPOCH}::::::
EOF

# Set strict permissions — shadow must only be readable by root
chmod 000 "${SHADOW_PATH}"

cat > "${GSHADOW_PATH}" << EOF
root:!::
EOF
chmod 000 "${GSHADOW_PATH}"

echo "==> shadow file generated (root account locked, password login disabled)"
echo "==> gshadow file generated"
echo "==> Permissions set to 000 (root only)"
