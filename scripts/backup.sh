#!/bin/bash
# =============================================================================
# PIFP Database Backup Script
# =============================================================================
# Purpose: Creates compressed backups of the SQLite indexer database and 
#          uploads them to secure object storage (S3/GCS)
# 
# Backup Flow:
#   1. Verify database file exists and is not locked
#   2. Create timestamped copy of SQLite database
#   3. Compress backup using gzip
#   4. Upload to S3/GCS bucket
#   5. Verify upload success
#   6. Clean up local temporary files
#
# Retention Policy:
#   - Backups are retained for 30 days
#   - Older backups are automatically deleted from storage
#   - Deletion uses storage lifecycle policies or script-based cleanup
#
# Security Considerations:
#   - Credentials loaded from environment variables only
#   - Backup files are never stored locally after upload
#   - Bucket must be private (no public access)
#   - Uses IAM roles or access keys with minimal permissions
#
# Usage: ./backup.sh
# Required Environment Variables:
#   - BACKUP_DB_PATH: Path to SQLite database file
#   - BACKUP_BUCKET: S3/GCS bucket name
#   - BACKUP_REGION: AWS region (for S3)
#   - STORAGE_TYPE: 's3' or 'gcs'
#   - AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY (if using S3)
#   - GOOGLE_APPLICATION_CREDENTIALS (if using GCS)
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration - Loaded from environment variables
# -----------------------------------------------------------------------------
DB_PATH="${BACKUP_DB_PATH:-/workspace/backend/indexer/pifp_events.db}"
BUCKET_NAME="${BACKUP_BUCKET:-}"
REGION="${BACKUP_REGION:-us-east-1}"
STORAGE_TYPE="${STORAGE_TYPE:-s3}"
BACKUP_RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-30}"

# Local backup settings
LOCAL_BACKUP_DIR="/tmp/pifp_backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILENAME="pifp_backup_${TIMESTAMP}.db"
COMPRESSED_FILENAME="${BACKUP_FILENAME}.gz"

# Logging configuration
LOG_LEVEL="${LOG_LEVEL:-INFO}"
LOG_FILE="${LOG_FILE:-/var/log/pifp_backup.log}"

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
        log_error "Backup failed with exit code: $exit_code"
        # Clean up any partial files
        rm -f "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}" 2>/dev/null || true
        rm -f "${LOCAL_BACKUP_DIR}/${COMPRESSED_FILENAME}" 2>/dev/null || true
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
    
    if [[ ! -f "$DB_PATH" ]]; then
        log_error "Database file not found at: $DB_PATH"
        log_error "Please ensure the indexer has been run and database exists"
        exit 1
    fi
    
    # Check if database is accessible and not locked
    if ! sqlite3 "$DB_PATH" "SELECT 1;" >/dev/null 2>&1; then
        log_error "Cannot access database file. It may be locked or corrupted."
        exit 1
    fi
    
    # Validate storage type and credentials
    case "$STORAGE_TYPE" in
        s3)
            if [[ -z "${AWS_ACCESS_KEY_ID:-}" ]] || [[ -z "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
                log_warn "AWS credentials not set. Attempting to use IAM role."
            fi
            # Verify AWS CLI is available
            if ! command -v aws &> /dev/null; then
                log_error "AWS CLI is required for S3 backups but not installed"
                exit 1
            fi
            ;;
        gcs)
            if [[ -z "${GOOGLE_APPLICATION_CREDENTIALS:-}" ]]; then
                log_warn "GOOGLE_APPLICATION_CREDENTIALS not set. Attempting to use default credentials."
            fi
            # Verify gsutil is available
            if ! command -v gsutil &> /dev/null; then
                log_error "Google Cloud SDK (gsutil) is required for GCS backups but not installed"
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
# Backup Functions
# -----------------------------------------------------------------------------
create_backup_directory() {
    log_info "Creating backup directory: $LOCAL_BACKUP_DIR"
    mkdir -p "$LOCAL_BACKUP_DIR"
    chmod 700 "$LOCAL_BACKUP_DIR"
}

copy_database() {
    log_info "Copying database from: $DB_PATH"
    
    # Use cp with --preserve to maintain file attributes
    # Add a small delay to ensure file is not locked during copy
    cp --preserve=mode,timestamps "$DB_PATH" "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}"
    
    # Verify the copy was successful
    if [[ ! -f "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}" ]]; then
        log_error "Failed to create database copy"
        exit 1
    fi
    
    local original_size=$(stat -c%s "$DB_PATH" 2>/dev/null || stat -f%z "$DB_PATH" 2>/dev/null)
    local backup_size=$(stat -c%s "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}" 2>/dev/null || stat -f%z "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}" 2>/dev/null)
    
    if [[ "$original_size" != "$backup_size" ]]; then
        log_error "Backup file size mismatch. Original: $original_size, Backup: $backup_size"
        exit 1
    fi
    
    log_info "Database copied successfully. Size: $backup_size bytes"
}

compress_backup() {
    log_info "Compressing backup file..."
    
    # Compress using gzip with best compression ratio
    gzip -9 "${LOCAL_BACKUP_DIR}/${BACKUP_FILENAME}"
    
    if [[ ! -f "${LOCAL_BACKUP_DIR}/${COMPRESSED_FILENAME}" ]]; then
        log_error "Failed to create compressed backup"
        exit 1
    fi
    
    local compressed_size=$(stat -c%s "${LOCAL_BACKUP_DIR}/${COMPRESSED_FILENAME}" 2>/dev/null || stat -f%z "${LOCAL_BACKUP_DIR}/${COMPRESSED_FILENAME}" 2>/dev/null)
    log_info "Compression complete. Compressed size: $compressed_size bytes"
}

upload_to_storage() {
    log_info "Uploading backup to ${STORAGE_TYPE^^} bucket: $BUCKET_NAME"
    
    local source_file="${LOCAL_BACKUP_DIR}/${COMPRESSED_FILENAME}"
    local destination_path="backups/${COMPRESSED_FILENAME}"
    
    case "$STORAGE_TYPE" in
        s3)
            # Upload to S3 with server-side encryption
            aws s3 cp "$source_file" "s3://${BUCKET_NAME}/${destination_path}" \
                --region "$REGION" \
                --storage-class STANDARD \
                --server-side-encryption AES256 \
                --metadata "backup-type=database,created-by=pifp-backup-script" \
                --quiet
            
            if [[ $? -eq 0 ]]; then
                log_info "Successfully uploaded to S3: s3://${BUCKET_NAME}/${destination_path}"
            else
                log_error "Failed to upload to S3"
                exit 1
            fi
            ;;
        gcs)
            # Upload to GCS with encryption
            gsutil -h "x-goog-meta-backup-type: database" \
                   -h "x-goog-meta-created-by: pifp-backup-script" \
                   cp "$source_file" "gs://${BUCKET_NAME}/${destination_path}"
            
            if [[ $? -eq 0 ]]; then
                log_info "Successfully uploaded to GCS: gs://${BUCKET_NAME}/${destination_path}"
            else
                log_error "Failed to upload to GCS"
                exit 1
            fi
            ;;
    esac
    
    # Verify upload by checking file exists in bucket
    verify_upload "$destination_path"
}

verify_upload() {
    local remote_path="$1"
    log_info "Verifying backup upload..."
    
    case "$STORAGE_TYPE" in
        s3)
            if aws s3 ls "s3://${BUCKET_NAME}/${remote_path}" --region "$REGION" &>/dev/null; then
                log_info "Upload verification successful"
            else
                log_error "Upload verification failed - file not found in bucket"
                exit 1
            fi
            ;;
        gcs)
            if gsutil ls "gs://${BUCKET_NAME}/${remote_path}" &>/dev/null; then
                log_info "Upload verification successful"
            else
                log_error "Upload verification failed - file not found in bucket"
                exit 1
            fi
            ;;
    esac
}

# -----------------------------------------------------------------------------
# Retention Policy Implementation
# -----------------------------------------------------------------------------
apply_retention_policy() {
    log_info "Applying retention policy: keeping backups for ${BACKUP_RETENTION_DAYS} days"
    
    local cutoff_date=$(date -d "-${BACKUP_RETENTION_DAYS} days" +%Y%m%d_%H%M%S 2>/dev/null || date -v-${BACKUP_RETENTION_DAYS}d +%Y%m%d_%H%M%S)
    log_info "Deleting backups older than: $cutoff_date"
    
    case "$STORAGE_TYPE" in
        s3)
            # List and delete old backups from S3
            aws s3 ls "s3://${BUCKET_NAME}/backups/" --region "$REGION" | \
                grep "pifp_backup_" | \
                while read -r line; do
                    local filename=$(echo "$line" | awk '{print $NF}')
                    local file_date=$(echo "$filename" | grep -oP '\d{8}_\d{6}')
                    
                    if [[ "$file_date" < "$cutoff_date" ]]; then
                        log_info "Deleting expired backup: $filename"
                        aws s3 rm "s3://${BUCKET_NAME}/backups/${filename}" --region "$REGION" --quiet
                    fi
                done
            ;;
        gcs)
            # List and delete old backups from GCS
            gsutil ls "gs://${BUCKET_NAME}/backups/pifp_backup_*.gz" | \
                while read -r url; do
                    local filename=$(basename "$url")
                    local file_date=$(echo "$filename" | grep -oP '\d{8}_\d{6}')
                    
                    if [[ "$file_date" < "$cutoff_date" ]]; then
                        log_info "Deleting expired backup: $filename"
                        gsutil rm "$url"
                    fi
                done
            ;;
    esac
    
    log_info "Retention policy applied successfully"
}

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------
cleanup_local_files() {
    log_info "Cleaning up local temporary files..."
    rm -rf "${LOCAL_BACKUP_DIR:?}"/*
    log_info "Cleanup complete"
}

# -----------------------------------------------------------------------------
# Main Execution
# -----------------------------------------------------------------------------
main() {
    log_info "========================================="
    log_info "Starting PIFP Database Backup"
    log_info "========================================="
    
    validate_environment
    create_backup_directory
    copy_database
    compress_backup
    upload_to_storage
    apply_retention_policy
    cleanup_local_files
    
    log_info "========================================="
    log_info "Backup completed successfully!"
    log_info "Backup file: $COMPRESSED_FILENAME"
    log_info "Location: ${STORAGE_TYPE}://${BUCKET_NAME}/backups/"
    log_info "========================================="
}

# Run main function
main "$@"
