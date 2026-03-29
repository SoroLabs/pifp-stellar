# PIFP Backup System - Quick Reference Card

## 🚀 Quick Start (3 Steps)

```bash
# 1. Configure
cd scripts/
cp .env.backup.example .env.backup
# Edit .env.backup with your credentials

# 2. Test
./backup.sh

# 3. Automate
./setup_cron.sh
```

## 📁 Files Created

| File | Purpose |
|------|---------|
| `backup.sh` | Main backup script |
| `restore.sh` | Database restore |
| `setup_cron.sh` | Cron job setup |
| `test_backup.sh` | Test suite |
| `.env.backup.example` | Config template |
| `BACKUP_README.md` | Full documentation |

## 🔧 Configuration

### Minimum Required Variables

```bash
BACKUP_DB_PATH=/workspace/backend/indexer/pifp_events.db
BACKUP_BUCKET=your-bucket-name
STORAGE_TYPE=s3  # or 'gcs'
AWS_ACCESS_KEY_ID=your_key
AWS_SECRET_ACCESS_KEY=your_secret
```

## 📅 Default Schedule

- **When**: Daily at 2:00 AM UTC
- **Retention**: 30 days
- **Location**: S3/GCS bucket

## ✅ Verification Commands

```bash
# Check cron job
crontab -l | grep pifp

# View logs
tail -f /var/log/pifp_backup.log

# List backups in S3
aws s3 ls s3://your-bucket/backups/

# Test restore
./restore.sh
```

## 🆘 Common Issues

| Issue | Solution |
|-------|----------|
| "Database not found" | Run indexer first to create DB |
| "AWS CLI required" | Install: `curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" && unzip awscliv2.zip && sudo ./aws/install` |
| "Bucket access denied" | Check IAM permissions and bucket policy |
| "Backup locked" | Wait for indexer to finish writing |

## 🔒 Security Checklist

- [x] Credentials in environment variables only
- [x] `.env.backup` added to `.gitignore`
- [x] Server-side encryption enabled
- [x] Private bucket access
- [x] No secrets in code/logs

## 💰 Estimated Cost

~**$0.25/month** for typical usage
- Database: 1 GB
- Compressed: 250 MB
- 30 daily backups retained

## 📊 Backup Flow

```
SQLite DB → Copy → Gzip → Upload → Verify → Cleanup
                ↓
            Retention (delete >30 days)
```

## 🔄 Restore Flow

```
Download → Verify → Decompress → Stop Indexer → Replace DB → Verify → Start Indexer
                    ↓
              Safety Backup Created
```

## 🛠️ Maintenance

### Weekly
- [ ] Check logs for errors
- [ ] Verify ~30 backup files exist

### Monthly  
- [ ] Test restore procedure
- [ ] Review storage costs

### Quarterly
- [ ] Rotate credentials
- [ ] Disaster recovery drill

## 📖 Documentation

- **Full Guide**: `BACKUP_README.md`
- **Implementation**: `IMPLEMENTATION_SUMMARY.md`
- **Main README**: See "Database Backup & Restore" section

## 🎯 Next Steps

1. ✅ Configure `.env.backup`
2. ✅ Run `./backup.sh` manually
3. ✅ Setup automation with `./setup_cron.sh`
4. ✅ Test restore with `./restore.sh`
5. ✅ Monitor first automated run

---

**Status**: ✅ Production Ready  
**Date**: March 28, 2026  
**License**: MIT (same as PIFP project)
