# PIFP Database Backup System

Automated backup and restore system for the PIFP SQLite indexer database.

## Overview

The backup system provides automated, secure backups of the PIFP indexer database with:
- **Daily automated backups** via cron job
- **Compression** using gzip for efficient storage
- **Cloud storage** integration (AWS S3 or Google Cloud Storage)
- **30-day retention policy** with automatic cleanup
- **Point-in-time recovery** capability
- **Comprehensive logging** and error handling

## Architecture

```
┌─────────────────┐
│  Indexer DB     │
│  (SQLite)       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  backup.sh      │
│  - Copy DB      │
│  - Compress     │
│  - Upload       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  S3 / GCS       │
│  (Encrypted)    │
└─────────────────┘
```

## Quick Start

### 1. Configure Environment

```bash
cd scripts/
cp .env.backup.example .env.backup
# Edit .env.backup with your credentials
```

### 2. Test Backup Manually

```bash
./backup.sh
```

### 3. Setup Automated Backups

```bash
./setup_cron.sh
```

## Configuration

### Environment Variables

Create a `.env.backup` file in the `scripts/` directory:

```bash
# Database path
BACKUP_DB_PATH=/workspace/backend/indexer/pifp_events.db

# Storage provider: 's3' or 'gcs'
STORAGE_TYPE=s3

# AWS S3 configuration
BACKUP_BUCKET=pifp-database-backups
BACKUP_REGION=us-east-1
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key

# Retention (days)
BACKUP_RETENTION_DAYS=30

# Logging
LOG_LEVEL=INFO
LOG_FILE=/var/log/pifp_backup.log
```

### Storage Options

#### AWS S3

```bash
STORAGE_TYPE=s3
BACKUP_BUCKET=your-bucket-name
BACKUP_REGION=us-east-1
AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

**IAM Policy Requirements:**
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::your-bucket-name",
        "arn:aws:s3:::your-bucket-name/backups/*"
      ]
    }
  ]
}
```

#### Google Cloud Storage

```bash
STORAGE_TYPE=gcs
BACKUP_BUCKET=your-bucket-name
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

**Service Account Permissions:**
- `storage.objects.create`
- `storage.objects.delete`
- `storage.objects.get`
- `storage.objects.list`

## Usage

### Manual Backup

Run a one-time backup:

```bash
cd scripts/
source .env.backup  # Load environment variables
./backup.sh
```

**Expected Output:**
```
[2025-01-28 02:00:00] [INFO] =========================================
[2025-01-28 02:00:00] [INFO] Starting PIFP Database Backup
[2025-01-28 02:00:00] [INFO] =========================================
[2025-01-28 02:00:01] [INFO] Validating environment configuration...
[2025-01-28 02:00:01] [INFO] Environment validation passed
[2025-01-28 02:00:02] [INFO] Creating backup directory: /tmp/pifp_backups
[2025-01-28 02:00:02] [INFO] Copying database from: /workspace/backend/indexer/pifp_events.db
[2025-01-28 02:00:03] [INFO] Database copied successfully. Size: 1048576 bytes
[2025-01-28 02:00:04] [INFO] Compressing backup file...
[2025-01-28 02:00:05] [INFO] Compression complete. Compressed size: 262144 bytes
[2025-01-28 02:00:06] [INFO] Uploading backup to S3 bucket: pifp-database-backups
[2025-01-28 02:00:15] [INFO] Successfully uploaded to S3: s3://pifp-database-backups/backups/pifp_backup_20250128_020000.db.gz
[2025-01-28 02:00:16] [INFO] Verifying backup upload...
[2025-01-28 02:00:17] [INFO] Upload verification successful
[2025-01-28 02:00:18] [INFO] Applying retention policy: keeping backups for 30 days
[2025-01-28 02:00:20] [INFO] Retention policy applied successfully
[2025-01-28 02:00:21] [INFO] Cleaning up local temporary files...
[2025-01-28 02:00:21] [INFO] Cleanup complete
[2025-01-28 02:00:21] [INFO] =========================================
[2025-01-28 02:00:21] [INFO] Backup completed successfully!
[2025-01-28 02:00:21] [INFO] Backup file: pifp_backup_20250128_020000.db.gz
[2025-01-28 02:00:21] [INFO] Location: s3://pifp-database-backups/backups/
[2025-01-28 02:00:21] [INFO] =========================================
```

### Restore from Backup

#### Restore Latest Backup

```bash
cd scripts/
source .env.backup
./restore.sh
```

#### Restore Specific Backup

```bash
./restore.sh pifp_backup_20250101_020000.db.gz
```

**Restore Process:**
1. Downloads backup from cloud storage
2. Verifies backup integrity
3. Stops indexer service (if running)
4. Creates safety backup of current database
5. Restores database from backup
6. Verifies restored database integrity
7. Restarts indexer service

**Expected Output:**
```
[2025-01-28 10:00:00] [INFO] =========================================
[2025-01-28 10:00:00] [INFO] Starting PIFP Database Restore
[2025-01-28 10:00:00] [INFO] =========================================
[2025-01-28 10:00:01] [INFO] Validating environment configuration...
[2025-01-28 10:00:01] [INFO] Environment validation passed
[2025-01-28 10:00:02] [INFO] Searching for latest backup in S3 bucket: pifp-database-backups
[2025-01-28 10:00:05] [INFO] Found latest backup: pifp_backup_20250128_020000.db.gz
[2025-01-28 10:00:06] [INFO] Downloading backup from S3: pifp_backup_20250128_020000.db.gz
[2025-01-28 10:00:15] [INFO] Download complete: /tmp/pifp_restore/pifp_backup_20250128_020000.db.gz
[2025-01-28 10:00:16] [INFO] Verifying backup integrity...
[2025-01-28 10:00:17] [INFO] Backup integrity verified. Size: 262144 bytes
[2025-01-28 10:00:18] [INFO] Decompressing backup file...
[2025-01-28 10:00:20] [INFO] Decompression complete. Database size: 1048576 bytes
[2025-01-28 10:00:21] [INFO] Stopping indexer service...
[2025-01-28 10:00:23] [INFO] Indexer process terminated (PID: 12345)
[2025-01-28 10:00:24] [INFO] Replacing database at: /workspace/backend/indexer/pifp_events.db
[2025-01-28 10:00:25] [INFO] Creating safety backup: /workspace/backend/indexer/pifp_events.db.pre_restore_20250128_100024
[2025-01-28 10:00:26] [INFO] Database replaced successfully
[2025-01-28 10:00:27] [INFO] Verifying restored database integrity...
[2025-01-28 10:00:28] [INFO] Database integrity verified. Tables: 4
[2025-01-28 10:00:29] [INFO] Starting indexer service...
[2025-01-28 10:00:30] [INFO] Please start the indexer manually if needed
[2025-01-28 10:00:31] [INFO] Cleaning up restore temporary files...
[2025-01-28 10:00:31] [INFO] Cleanup complete
[2025-01-28 10:00:31] [INFO] =========================================
[2025-01-28 10:00:31] [INFO] Restore completed successfully!
[2025-01-28 10:00:31] [INFO] =========================================
```

### Automated Scheduling

Setup daily automated backups:

```bash
./setup_cron.sh
```

This will:
- Install a cron job to run backup daily at 2:00 AM UTC
- Log output to `/var/log/pifp_backup_cron.log`
- Load environment from `.env.backup`

**Verify Installation:**
```bash
crontab -l | grep pifp
```

**Expected Output:**
```
# PIFP Database Backup - Daily automated backup
0 2 * * * /workspace/scripts/backup.sh >> /var/log/pifp_backup_cron.log 2>&1
```

### Modify Backup Schedule

Edit crontab:
```bash
crontab -e
```

**Common Schedules:**
- Every 6 hours: `0 */6 * * *`
- Every hour: `0 * * * *`
- Twice daily (midnight & noon): `0 0,12 * * *`
- Weekly (Sunday 3 AM): `0 3 * * 0`

## Retention Policy

Backups are automatically managed with a 30-day retention policy:

- **Daily backups**: One backup per day
- **Retention period**: 30 days
- **Automatic deletion**: Backups older than 30 days are deleted
- **Cleanup timing**: Applied during each backup run

**Example Timeline:**
```
Jan 1   Jan 5   Jan 10  Jan 15  Jan 20  Jan 25  Jan 28  Feb 1
  |-------|-------|-------|-------|-------|-------|-------|
  ✓       ✓       ✓       ✓       ✓       ✓       ✓       
                                                  ↓
                                        Delete Jan 1 backup
```

To change retention period, set in `.env.backup`:
```bash
BACKUP_RETENTION_DAYS=60  # Keep 2 months
```

## Security

### Credentials Management

✅ **DO:**
- Store credentials in `.env.backup` file
- Add `.env.backup` to `.gitignore` (already configured)
- Use IAM roles when possible (EC2, GCE)
- Rotate access keys regularly
- Use least-privilege permissions

❌ **DON'T:**
- Commit `.env.backup` to version control
- Hardcode credentials in scripts
- Share credentials via email/chat
- Use root/admin credentials

### Backup Encryption

**At Rest:**
- S3: Server-side encryption (AES-256) enabled by default
- GCS: Automatic encryption at rest

**In Transit:**
- All uploads use HTTPS/TLS
- No unencrypted data transmission

### Access Control

**Bucket Policy Recommendations:**
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "DenyPublicAccess",
      "Effect": "Deny",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::your-bucket-name/backups/*",
      "Condition": {
        "Bool": {
          "aws:SecureTransport": "false"
        }
      }
    }
  ]
}
```

## Monitoring & Logging

### Log Files

**Backup Logs:**
- Default location: `/var/log/pifp_backup.log`
- Cron logs: `/var/log/pifp_backup_cron.log`

**View Recent Activity:**
```bash
tail -f /var/log/pifp_backup.log
```

**Search Logs:**
```bash
grep "ERROR" /var/log/pifp_backup.log
grep "Successfully uploaded" /var/log/pifp_backup.log
```

### Log Levels

Configure verbosity in `.env.backup`:

```bash
LOG_LEVEL=DEBUG  # Maximum detail
LOG_LEVEL=INFO   # Normal operation (default)
LOG_LEVEL=WARN   # Warnings only
LOG_LEVEL=ERROR  # Errors only
```

### Alerting

For production environments, consider adding alerting:

**Example: Email on Failure**
```bash
# In setup_cron.sh or crontab
0 2 * * * /workspace/scripts/backup.sh || mail -s "Backup Failed" admin@example.com
```

**Example: Slack Notification**
Add to `backup.sh` after error detection:
```bash
curl -X POST -H 'Content-type: application/json' \
  --data '{"text":"PIFP Backup Failed!"}' \
  https://hooks.slack.com/services/YOUR/WEBHOOK/URL
```

## Troubleshooting

### Common Issues

#### 1. "Database file not found"

**Cause:** Indexer hasn't created the database yet

**Solution:**
```bash
# Run the indexer first
cd backend/indexer
cargo run
# Let it create the database, then stop it
# Now run backup
```

#### 2. "Cannot access database. It may be locked"

**Cause:** Indexer is actively writing to database

**Solution:**
- Stop indexer before backup
- Or wait for write operations to complete
- Consider using WAL mode for SQLite

#### 3. "AWS CLI is required but not installed"

**Solution:**
```bash
# Install AWS CLI
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Verify installation
aws --version
```

#### 4. "Failed to upload to S3"

**Causes:**
- Invalid credentials
- Bucket doesn't exist
- Network issues
- Permissions issue

**Solution:**
```bash
# Test credentials
aws sts get-caller-identity

# Test bucket access
aws s3 ls s3://your-bucket-name

# Check IAM permissions
aws iam list-attached-user-policies --user-name your-user
```

#### 5. "No backups found in bucket"

**Causes:**
- First backup hasn't run yet
- Wrong bucket name
- Wrong region

**Solution:**
```bash
# List all objects in bucket
aws s3 ls s3://your-bucket-name/backups/ --region your-region

# Verify bucket exists
aws s3api head-bucket --bucket your-bucket-name
```

### Debug Mode

Enable detailed debugging:

```bash
LOG_LEVEL=DEBUG ./backup.sh
```

This will show:
- Environment variable values
- Each step's detailed output
- API calls and responses
- File operation details

## Testing

### Manual Backup Test

```bash
# 1. Create test database
mkdir -p /tmp/test_indexer
sqlite3 /tmp/test_indexer/pifp_events.db "CREATE TABLE events (id INTEGER); INSERT INTO events VALUES (1);"

# 2. Set environment
export BACKUP_DB_PATH=/tmp/test_indexer/pifp_events.db
export BACKUP_BUCKET=test-backup-bucket
export STORAGE_TYPE=s3

# 3. Run backup
./backup.sh

# 4. Verify
ls -lh /tmp/pifp_backups/
```

### Restore Test

```bash
# 1. Backup current database
./backup.sh

# 2. Delete original database
rm /workspace/backend/indexer/pifp_events.db

# 3. Restore
./restore.sh

# 4. Verify restoration
sqlite3 /workspace/backend/indexer/pifp_events.db "SELECT COUNT(*) FROM events;"
```

### Disaster Recovery Drill

Practice full recovery quarterly:

1. Simulate complete database loss
2. Restore from latest backup
3. Verify data integrity
4. Document recovery time
5. Update procedures based on learnings

## Performance Impact

### Backup Operation

- **Duration**: Typically 1-5 minutes depending on database size
- **CPU**: Minimal (compression uses ~10-20% CPU)
- **Memory**: < 100MB
- **Network**: Upload bandwidth dependent
- **Database**: Brief read lock during copy (< 1 second)

### Best Practices

1. **Schedule during low-traffic periods**: 2:00 AM UTC recommended
2. **Monitor backup duration**: Alert if > 10 minutes
3. **Test restore regularly**: Ensure backups are valid
4. **Rotate credentials**: Every 90 days
5. **Review logs weekly**: Catch issues early

## Cost Estimation

### AWS S3 Example

**Assumptions:**
- Database size: 1 GB
- Compressed size: 250 MB (75% compression)
- Daily backups
- 30-day retention

**Monthly Storage:**
```
250 MB × 30 backups = 7.5 GB
Cost: 7.5 GB × $0.023/GB = $0.17/month
```

**API Calls:**
- 3 PUT requests per backup (upload, verify, lifecycle)
- 90 PUT requests/month = negligible cost

**Total Estimated Cost**: < $0.25/month

### Google Cloud Storage Example

Similar pricing structure:
- Standard storage: $0.020/GB/month
- Operations: Similar minimal cost

## Maintenance

### Monthly Tasks

- [ ] Review backup logs for errors
- [ ] Verify backup file count matches expected (30)
- [ ] Test restore procedure
- [ ] Check storage costs

### Quarterly Tasks

- [ ] Rotate AWS/GCS credentials
- [ ] Perform disaster recovery drill
- [ ] Review and update retention policy
- [ ] Document any infrastructure changes

### Annual Tasks

- [ ] Review backup strategy effectiveness
- [ ] Evaluate new storage options
- [ ] Update documentation
- [ ] Train team members on procedures

## Support

For issues or questions:
1. Check logs: `/var/log/pifp_backup.log`
2. Review troubleshooting section above
3. Enable DEBUG logging
4. Contact project maintainers

## License

Part of the PIFP project. See main repository LICENSE file.
