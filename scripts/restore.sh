#!/bin/bash
# =============================================================================
# PIFP Database Restore Script
# =============================================================================
# Purpose: Downloads and restores the SQLite indexer database from a backup
#          stored in S3/GCS object storage
# 
# Restore Flow:
#   1. Validate environment and credentials
#   2. Download backup file from S3/GCS
#   3. Verify backup integrity
#   4. Decompress backup file
#   5. Stop indexer service (if running)
#   6. Replace current database with backup
#   7. Verify restored database integrity
#   8. Restart indexer service
#
# Security Considerations:
#   - Credentials loaded from environment variables only
#   - Backup files are downloaded to secure temporary location
#   - Temporary files are cleaned up after restore
#   - Requires write permissions to database directory
#
# Usage: ./restore.sh [backup_filename]
#        If no filename specified, uses latest backup
#
# Examples:
#   ./restore.sh                                    # Restore from latest backup
#   ./restore.sh pifp_backup_20250101_020000.db.gz  # Restore from specific backup
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration - Loaded from environment variables
# -----------------------------------------------------------------------------
DB_PATH="${BACKUP_DB_PATH:-/workspace/backend/indexer/pifp_events.db}"
BUCKET_NAME="${BACKUP_BUCKET:-}"
REGION="${BACKUP_REGION:-us-east-1}"
STORAGE_TYPE="${STORAGE_TYPE:-s3}"

# Local restore settings
RESTORE_DIR="/tmp/pifp_restore"
BACKUP_FILENAME="${1:-}"

# Logging configuration
LOG_LEVEL="${LOG_LEVEL:-INFO}"
LOG_FILE="${LOG_FILE:-/var/log/pifp_backup.log}"

# Indexer service management (adjust based on your deployment)
INDEXER_PID_FILE="${INDEXER_PID_FILE:-}"
INDEXER_SERVICE_NAME="${INDEXER_SERVICE_NAME:-}"

# -----------------------------------------------------------------------------
# Logging Functions
# -----------------------------------------------------------------------------
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] [${level}] ${message}" | tee -a "$LOG_FILE" 2>/dev/null || echo "[${timestamp}] [${level}] ${message}"
}

log_info() { log "INFO" "$@"; }
log_warn() { log "WARN" "$@"; }
log_error() { log "ERROR" "$@"; }
log_debug() { [[ "$LOG_LEVEL" == "DEBUG" ]] && log "DEBUG" "$@" || true; }

# -----------------------------------------------------------------------------
# Error Handling
# -----------------------------------------------------------------------------
cleanup_on_error() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        log_error "Restore failed with exit code: $exit_code"
        # Clean up restore directory
        rm -rf "${RESTORE_DIR:?}"/* 2>/dev/null || true
        
        if [[ -n "$DB_BACKUP_PATH" ]] && [[ -f "$DB_BACKUP_PATH" ]]; then
            log_warn "Database may be in inconsistent state"
            log_warn "Manual intervention may be required"
        fi
    fi
    exit $exit_code
}

trap cleanup_on_error EXIT

# -----------------------------------------------------------------------------
# Validation Functions
# -----------------------------------------------------------------------------
validate_environment() {
    log_info "Validating environment configuration..."
    
    if [[ -z "$BUCKET_NAME" ]]; then
        log_error "BACKUP_BUCKET environment variable is required"
        exit 1
    fi
    
    # Check storage CLI availability
    case "$STORAGE_TYPE" in
        s3)
            if ! command -v aws &> /dev/null; then
                log_error "AWS CLI is required for S3 restores but not installed"
                exit 1
            fi
            ;;
        gcs)
            if ! command -v gsutil &> /dev/null; then
                log_error "Google Cloud SDK (gsutil) is required for GCS restores but not installed"
                exit 1
            fi
            ;;
        *)
            log_error "Invalid STORAGE_TYPE: $STORAGE_TYPE. Must be 's3' or 'gcs'"
            exit 1
            ;;
    esac
    
    log_info "Environment validation passed"
}

# -----------------------------------------------------------------------------
# Backup Selection
# -----------------------------------------------------------------------------
find_latest_backup() {
    log_info "Searching for latest backup in ${STORAGE_TYPE^^} bucket: $BUCKET_NAME"
    
    local latest_backup=""
    
    case "$STORAGE_TYPE" in
        s3)
            latest_backup=$(aws s3 ls "s3://${BUCKET_NAME}/backups/" --region "$REGION" | \
                grep "pifp_backup_" | \
                sort -k2 | \
                tail -1 | \
                awk '{print $NF}')
            ;;
        gcs)
            latest_backup=$(gsutil ls "gs://${BUCKET_NAME}/backups/pifp_backup_*.gz" 2>/dev/null | \
                sort | \
                tail -1 | \
                xargs -n1 basename)
            ;;
    esac
    
    if [[ -z "$latest_backup" ]]; then
        log_error "No backups found in bucket"
        exit 1
    fi
    
    log_info "Found latest backup: $latest_backup"
    echo "$latest_backup"
}

# -----------------------------------------------------------------------------
# Download Functions
# -----------------------------------------------------------------------------
download_backup() {
    local backup_file="$1"
    log_info "Downloading backup from ${STORAGE_TYPE^^}: $backup_file"
    
    mkdir -p "$RESTORE_DIR"
    chmod 700 "$RESTORE_DIR"
    
    local destination="${RESTORE_DIR}/${backup_file}"
    
    case "$STORAGE_TYPE" in
        s3)
            aws s3 cp "s3://${BUCKET_NAME}/backups/${backup_file}" "$destination" --region "$REGION"
            ;;
        gcs)
            gsutil cp "gs://${BUCKET_NAME}/backups/${backup_file}" "$destination"
            ;;
    esac
    
    if [[ ! -f "$destination" ]]; then
        log_error "Failed to download backup file"
        exit 1
    fi
    
    log_info "Download complete: $destination"
}

verify_backup_integrity() {
    local backup_file="$1"
    log_info "Verifying backup integrity..."
    
    local filepath="${RESTORE_DIR}/${backup_file}"
    
    # Check file exists and has content
    if [[ ! -f "$filepath" ]]; then
        log_error "Backup file not found: $filepath"
        exit 1
    fi
    
    local file_size=$(stat -c%s "$filepath" 2>/dev/null || stat -f%z "$filepath" 2>/dev/null)
    if [[ "$file_size" -eq 0 ]]; then
        log_error "Backup file is empty"
        exit 1
    fi
    
    # Test gzip integrity
    if ! gzip -t "$filepath" 2>/dev/null; then
        log_error "Backup file is corrupted (gzip test failed)"
        exit 1
    fi
    
    log_info "Backup integrity verified. Size: $file_size bytes"
}

# -----------------------------------------------------------------------------
# Restore Functions
# -----------------------------------------------------------------------------
decompress_backup() {
    local backup_file="$1"
    log_info "Decompressing backup file..."
    
    local compressed_path="${RESTORE_DIR}/${backup_file}"
    local decompressed_path="${RESTORE_DIR}/${backup_file%.gz}"
    
    gunzip -c "$compressed_path" > "$decompressed_path"
    
    if [[ ! -f "$decompressed_path" ]]; then
        log_error "Failed to decompress backup"
        exit 1
    fi
    
    local size=$(stat -c%s "$decompressed_path" 2>/dev/null || stat -f%z "$decompressed_path" 2>/dev/null)
    log_info "Decompression complete. Database size: $size bytes"
}

stop_indexer() {
    log_info "Stopping indexer service..."
    
    # Method 1: Stop via systemd service
    if [[ -n "$INDEXER_SERVICE_NAME" ]] && systemctl is-active --quiet "$INDEXER_SERVICE_NAME" 2>/dev/null; then
        systemctl stop "$INDEXER_SERVICE_NAME"
        log_info "Indexer service stopped"
        return
    fi
    
    # Method 2: Kill via PID file
    if [[ -n "$INDEXER_PID_FILE" ]] && [[ -f "$INDEXER_PID_FILE" ]]; then
        local pid=$(cat "$INDEXER_PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid"
            sleep 2
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid"
            fi
            log_info "Indexer process terminated (PID: $pid)"
        fi
        rm -f "$INDEXER_PID_FILE"
        return
    fi
    
    # Method 3: Try to find and kill indexer process
    local indexer_pid=$(pgrep -f "indexer" | head -1 || true)
    if [[ -n "$indexer_pid" ]]; then
        kill -TERM "$indexer_pid"
        sleep 2
        if pgrep -f "indexer" >/dev/null; then
            kill -9 "$indexer_pid"
        fi
        log_info "Indexer process terminated (PID: $indexer_pid)"
    else
        log_info "No running indexer process found"
    fi
}

replace_database() {
    local backup_file="$1"
    local db_file="${RESTORE_DIR}/${backup_file%.gz}"
    
    log_info "Replacing database at: $DB_PATH"
    
    # Create backup of current database before replacing
    if [[ -f "$DB_PATH" ]]; then
        local pre_restore_backup="${DB_PATH}.pre_restore_$(date +%Y%m%d_%H%M%S)"
        log_info "Creating safety backup: $pre_restore_backup"
        cp "$DB_PATH" "$pre_restore_backup"
        DB_BACKUP_PATH="$pre_restore_backup"
    fi
    
    # Ensure parent directory exists
    mkdir -p "$(dirname "$DB_PATH")"
    
    # Replace database
    cp "$db_file" "$DB_PATH"
    chmod 644 "$DB_PATH"
    
    log_info "Database replaced successfully"
}

verify_restored_database() {
    log_info "Verifying restored database integrity..."
    
    # Check file exists
    if [[ ! -f "$DB_PATH" ]]; then
        log_error "Restored database file not found"
        exit 1
    fi
    
    # Test SQLite integrity
    if ! sqlite3 "$DB_PATH" "PRAGMA integrity_check;" | grep -q "ok"; then
        log_error "Database integrity check failed"
        exit 1
    fi
    
    # Verify tables exist
    local table_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';")
    if [[ "$table_count" -lt 3 ]]; then
        log_error "Database appears incomplete. Found $table_count tables"
        exit 1
    fi
    
    log_info "Database integrity verified. Tables: $table_count"
}

start_indexer() {
    log_info "Starting indexer service..."
    
    # Method 1: Start via systemd service
    if [[ -n "$INDEXER_SERVICE_NAME" ]]; then
        systemctl start "$INDEXER_SERVICE_NAME"
        log_info "Indexer service started"
        return
    fi
    
    # Method 2: Manual start (user needs to configure this)
    log_info "Please start the indexer manually if needed"
    log_info "Command: cd /workspace/backend/indexer && cargo run"
}

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------
cleanup_restore_files() {
    log_info "Cleaning up restore temporary files..."
    rm -rf "${RESTORE_DIR:?}"/*
    log_info "Cleanup complete"
}

# -----------------------------------------------------------------------------
# Main Execution
# -----------------------------------------------------------------------------
main() {
    log_info "========================================="
    log_info "Starting PIFP Database Restore"
    log_info "========================================="
    
    validate_environment
    
    # Determine which backup to restore
    if [[ -z "$BACKUP_FILENAME" ]]; then
        BACKUP_FILENAME=$(find_latest_backup)
    else
        log_info "Using specified backup: $BACKUP_FILENAME"
    fi
    
    download_backup "$BACKUP_FILENAME"
    verify_backup_integrity "$BACKUP_FILENAME"
    decompress_backup "$BACKUP_FILENAME"
    stop_indexer
    replace_database "$BACKUP_FILENAME"
    verify_restored_database
    start_indexer
    cleanup_restore_files
    
    log_info "========================================="
    log_info "Restore completed successfully!"
    log_info "========================================="
}

# Run main function
main "$@"
