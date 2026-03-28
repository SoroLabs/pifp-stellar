#!/bin/bash
# =============================================================================
# PIFP Backup Cron Job Setup Script
# =============================================================================
# Purpose: Configures automated daily backups via cron
# 
# This script:
#   1. Validates backup script and configuration
#   2. Creates cron job for daily execution
#   3. Logs cron job installation status
#
# Usage: ./setup_cron.sh
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_SCRIPT="${SCRIPT_DIR}/backup.sh"
ENV_FILE="${SCRIPT_DIR}/.env.backup"
CRON_LOG="/var/log/pifp_backup_cron.log"

echo "========================================="
echo "PIFP Backup Cron Job Setup"
echo "========================================="

# Validate backup script exists
if [[ ! -f "$BACKUP_SCRIPT" ]]; then
    echo "ERROR: Backup script not found at: $BACKUP_SCRIPT"
    exit 1
fi

# Check if .env.backup exists
if [[ ! -f "$ENV_FILE" ]]; then
    echo "WARNING: Configuration file not found: $ENV_FILE"
    echo "Please copy .env.backup.example to .env.backup and configure it first"
    echo ""
    read -p "Do you want to continue without loading environment variables? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Make backup script executable
chmod +x "$BACKUP_SCRIPT"

# Define cron schedule (daily at 2:00 AM)
CRON_SCHEDULE="0 2 * * *"

# Create cron command
CRON_COMMAND="$BACKUP_SCRIPT >> $CRON_LOG 2>&1"

# Check if cron job already exists
EXISTING_CRON=$(crontab -l 2>/dev/null | grep -c "pifp.*backup" || true)

if [[ "$EXISTING_CRON" -gt 0 ]]; then
    echo "WARNING: Existing PIFP backup cron job found"
    echo "Current cron jobs:"
    crontab -l 2>/dev/null | grep "pifp.*backup" || true
    echo ""
    read -p "Do you want to replace it? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted"
        exit 0
    fi
fi

# Install new cron job
{
    crontab -l 2>/dev/null || true
    echo "# PIFP Database Backup - Daily automated backup"
    echo "$CRON_SCHEDULE $CRON_COMMAND"
} | crontab -

echo ""
echo "✓ Cron job installed successfully!"
echo ""
echo "Schedule: Daily at 2:00 AM UTC"
echo "Log file: $CRON_LOG"
echo ""
echo "To verify installation:"
echo "  crontab -l | grep pifp"
echo ""
echo "To view logs after first run:"
echo "  tail -f $CRON_LOG"
echo ""
echo "========================================="
