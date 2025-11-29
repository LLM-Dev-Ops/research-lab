# SPARC Phase 5: Completion - Section 6: Disaster Recovery & Business Continuity

> **LLM-Research-Lab DR/BC Specification**
> Part of SPARC Phase 5 (Completion) - Enterprise-Grade Resilience
> Target: RTO < 4 hours, RPO < 1 hour, 99.9% SLA

---

## Table of Contents

- [Overview](#overview)
- [6.1 Backup Strategy](#61-backup-strategy)
- [6.2 Recovery Procedures](#62-recovery-procedures)
- [6.3 High Availability Architecture](#63-high-availability-architecture)
- [6.4 Disaster Recovery Plan](#64-disaster-recovery-plan)
- [6.5 Business Continuity](#65-business-continuity)
- [Appendix A: Recovery Scripts](#appendix-a-recovery-scripts)
- [Appendix B: Runbook Templates](#appendix-b-runbook-templates)
- [Appendix C: DR Testing Checklist](#appendix-c-dr-testing-checklist)

---

## Overview

This section establishes comprehensive disaster recovery and business continuity capabilities for LLM-Research-Lab, ensuring data protection, rapid recovery, and service resilience across all infrastructure components.

### Recovery Objectives

| Metric | Target | Measurement |
|--------|--------|-------------|
| RTO (Recovery Time Objective) | < 4 hours | Time to restore service functionality |
| RPO (Recovery Point Objective) | < 1 hour | Maximum acceptable data loss |
| Availability SLA | 99.9% | Annual uptime target (8.76h downtime/year) |
| MTTR (Mean Time to Recovery) | < 2 hours | Average incident resolution time |
| Backup Verification | 100% | All backups tested monthly |

### Disaster Categories

1. **Infrastructure Failure**: Single AZ outage, network partition
2. **Data Corruption**: Database corruption, accidental deletion
3. **Security Incident**: Ransomware, data breach, unauthorized access
4. **Application Failure**: Software bugs, deployment issues
5. **Regional Outage**: Multi-AZ failure, cloud provider incident

---

## 6.1 Backup Strategy

### 6.1.1 PostgreSQL Backup Configuration

#### Continuous Archiving (WAL-E/WAL-G)

```yaml
# k8s/postgres-backup-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: postgres-backup-config
  namespace: llm-research-lab-data
data:
  backup-config.yaml: |
    # WAL-G Configuration
    aws:
      region: us-east-1
      s3_bucket: llm-research-lab-postgres-backups
      s3_prefix: wal-archive
      sse: AES256

    postgres:
      host: postgres-primary.llm-research-lab-data.svc.cluster.local
      port: 5432
      database: llm_research_lab
      user: backup_user

    # Backup Schedule
    backup_schedule:
      full_backup: "0 2 * * *"  # Daily at 2 AM UTC
      incremental_backup: "0 */4 * * *"  # Every 4 hours
      wal_archival: continuous

    # Retention Policy
    retention:
      full_backups: 30  # Keep 30 daily backups
      incremental_backups: 7  # Keep 7 days of incrementals
      wal_segments: 7  # Keep 7 days of WAL
      weekly_backups: 12  # Keep 12 weekly backups (90 days)
      monthly_backups: 12  # Keep 12 monthly backups (1 year)

    # Compression & Encryption
    compression: lz4
    encryption:
      enabled: true
      kms_key_id: arn:aws:kms:us-east-1:ACCOUNT:key/backup-encryption

    # Performance Tuning
    upload_concurrency: 4
    download_concurrency: 4
    upload_disk_concurrency: 2
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-full-backup
  namespace: llm-research-lab-data
spec:
  schedule: "0 2 * * *"
  concurrencyPolicy: Forbid
  successfulJobsHistoryLimit: 7
  failedJobsHistoryLimit: 3
  jobTemplate:
    spec:
      backoffLimit: 2
      template:
        metadata:
          labels:
            app: postgres-backup
            type: full
        spec:
          restartPolicy: OnFailure
          serviceAccountName: postgres-backup-sa
          containers:
          - name: wal-g-backup
            image: wal-g/wal-g:latest
            imagePullPolicy: IfNotPresent
            command:
            - /bin/bash
            - -c
            - |
              set -e
              echo "Starting PostgreSQL full backup at $(date)"

              # Pre-backup validation
              pg_isready -h ${PGHOST} -p ${PGPORT} -U ${PGUSER}

              # Create backup with metadata
              BACKUP_NAME="base_$(date +%Y%m%d_%H%M%S)"
              wal-g backup-push /var/lib/postgresql/data \
                --backup-name ${BACKUP_NAME} \
                --full-backup-on-error \
                --delta-from-name LATEST

              # Verify backup
              wal-g backup-list | grep ${BACKUP_NAME}

              # Send metrics
              curl -X POST http://prometheus-pushgateway:9091/metrics/job/postgres_backup \
                -d "postgres_backup_success{type=\"full\"} 1"
              curl -X POST http://prometheus-pushgateway:9091/metrics/job/postgres_backup \
                -d "postgres_backup_duration_seconds{type=\"full\"} $SECONDS"

              echo "Backup completed successfully"
            env:
            - name: PGHOST
              value: postgres-primary.llm-research-lab-data.svc.cluster.local
            - name: PGPORT
              value: "5432"
            - name: PGUSER
              valueFrom:
                secretKeyRef:
                  name: postgres-backup-credentials
                  key: username
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: postgres-backup-credentials
                  key: password
            - name: WALG_S3_PREFIX
              value: s3://llm-research-lab-postgres-backups/wal-archive
            - name: AWS_REGION
              value: us-east-1
            - name: WALG_COMPRESSION_METHOD
              value: lz4
            - name: WALG_DELTA_MAX_STEPS
              value: "7"
            volumeMounts:
            - name: backup-config
              mountPath: /etc/wal-g
              readOnly: true
            resources:
              requests:
                cpu: 500m
                memory: 1Gi
              limits:
                cpu: 2
                memory: 4Gi
          volumes:
          - name: backup-config
            configMap:
              name: postgres-backup-config
```

#### PostgreSQL Logical Backups

```yaml
# k8s/postgres-logical-backup.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-logical-backup
  namespace: llm-research-lab-data
spec:
  schedule: "30 3 * * *"  # Daily at 3:30 AM UTC
  concurrencyPolicy: Forbid
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: pg-dump
            image: postgres:15-alpine
            command:
            - /bin/sh
            - -c
            - |
              set -e
              BACKUP_FILE="llm_research_lab_$(date +%Y%m%d_%H%M%S).dump"

              # Create custom format dump (supports parallel restore)
              pg_dump -h ${PGHOST} \
                      -U ${PGUSER} \
                      -d llm_research_lab \
                      -F c \
                      -Z 6 \
                      -f /tmp/${BACKUP_FILE} \
                      --verbose \
                      --no-owner \
                      --no-acl

              # Upload to S3
              aws s3 cp /tmp/${BACKUP_FILE} \
                s3://llm-research-lab-postgres-backups/logical/${BACKUP_FILE} \
                --storage-class INTELLIGENT_TIERING \
                --server-side-encryption AES256

              # Create metadata
              cat > /tmp/metadata.json <<EOF
              {
                "backup_type": "logical",
                "database": "llm_research_lab",
                "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
                "size_bytes": $(stat -f%z /tmp/${BACKUP_FILE}),
                "pg_version": "$(psql -h ${PGHOST} -U ${PGUSER} -t -c 'SELECT version()')"
              }
              EOF

              aws s3 cp /tmp/metadata.json \
                s3://llm-research-lab-postgres-backups/logical/${BACKUP_FILE}.metadata.json

              echo "Logical backup completed: ${BACKUP_FILE}"
            env:
            - name: PGHOST
              value: postgres-primary.llm-research-lab-data.svc.cluster.local
            - name: PGUSER
              valueFrom:
                secretKeyRef:
                  name: postgres-backup-credentials
                  key: username
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: postgres-backup-credentials
                  key: password
            resources:
              requests:
                cpu: 1
                memory: 2Gi
              limits:
                cpu: 2
                memory: 4Gi
```

### 6.1.2 ClickHouse Backup Configuration

```yaml
# k8s/clickhouse-backup-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: clickhouse-backup-config
  namespace: llm-research-lab-data
data:
  config.yaml: |
    general:
      remote_storage: s3
      disable_progress_bar: false
      backups_to_keep_local: 3
      backups_to_keep_remote: 30
      log_level: info
      allow_empty_backups: false

    clickhouse:
      username: backup_user
      host: clickhouse.llm-research-lab-data.svc.cluster.local
      port: 9000
      data_path: /var/lib/clickhouse
      skip_tables:
        - system.*
        - INFORMATION_SCHEMA.*
      timeout: 5m

    s3:
      access_key: ${AWS_ACCESS_KEY_ID}
      secret_key: ${AWS_SECRET_ACCESS_KEY}
      bucket: llm-research-lab-clickhouse-backups
      region: us-east-1
      acl: private
      endpoint: ""
      force_path_style: false
      path: backups
      disable_ssl: false
      compression_level: 9
      compression_format: gzip
      sse: AES256
      disable_cert_verification: false
      storage_class: STANDARD
      concurrency: 4
      part_size: 104857600  # 100MB
      max_parts_count: 10000
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: clickhouse-backup
  namespace: llm-research-lab-data
spec:
  schedule: "0 4 * * *"  # Daily at 4 AM UTC
  concurrencyPolicy: Forbid
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: clickhouse-backup
            image: altinity/clickhouse-backup:latest
            command:
            - /bin/bash
            - -c
            - |
              set -e
              BACKUP_NAME="auto_$(date +%Y%m%d_%H%M%S)"

              echo "Creating ClickHouse backup: ${BACKUP_NAME}"
              clickhouse-backup create ${BACKUP_NAME}

              echo "Uploading backup to S3"
              clickhouse-backup upload ${BACKUP_NAME}

              echo "Cleaning up old local backups"
              clickhouse-backup delete local --keep-last=3

              echo "Cleaning up old remote backups"
              clickhouse-backup delete remote --keep-last=30

              echo "Backup completed successfully"
            env:
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: clickhouse-backup-credentials
                  key: aws_access_key_id
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: clickhouse-backup-credentials
                  key: aws_secret_access_key
            volumeMounts:
            - name: backup-config
              mountPath: /etc/clickhouse-backup
            - name: clickhouse-data
              mountPath: /var/lib/clickhouse
            resources:
              requests:
                cpu: 1
                memory: 2Gi
              limits:
                cpu: 2
                memory: 4Gi
          volumes:
          - name: backup-config
            configMap:
              name: clickhouse-backup-config
          - name: clickhouse-data
            persistentVolumeClaim:
              claimName: clickhouse-data
```

### 6.1.3 Kafka Backup Strategy

```yaml
# k8s/kafka-backup-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: kafka-backup-config
  namespace: llm-research-lab-data
data:
  backup.sh: |
    #!/bin/bash
    set -e

    BACKUP_DATE=$(date +%Y%m%d_%H%M%S)
    BACKUP_DIR="/tmp/kafka-backup-${BACKUP_DATE}"
    S3_BUCKET="s3://llm-research-lab-kafka-backups"

    mkdir -p ${BACKUP_DIR}

    # Export topic configurations
    echo "Exporting topic configurations..."
    kafka-topics.sh --bootstrap-server kafka:9092 \
      --describe --topics-with-overrides > ${BACKUP_DIR}/topic-configs.txt

    # Export consumer group offsets
    echo "Exporting consumer group offsets..."
    kafka-consumer-groups.sh --bootstrap-server kafka:9092 \
      --all-groups --describe > ${BACKUP_DIR}/consumer-offsets.txt

    # Export ACLs
    echo "Exporting ACLs..."
    kafka-acls.sh --bootstrap-server kafka:9092 \
      --list > ${BACKUP_DIR}/acls.txt

    # Create metadata snapshot
    cat > ${BACKUP_DIR}/metadata.json <<EOF
    {
      "backup_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
      "kafka_version": "$(kafka-broker-api-versions.sh --bootstrap-server kafka:9092 | head -1)",
      "cluster_id": "$(kafka-cluster.sh cluster-id --bootstrap-server kafka:9092)"
    }
    EOF

    # Compress and upload
    tar -czf /tmp/kafka-metadata-${BACKUP_DATE}.tar.gz -C /tmp kafka-backup-${BACKUP_DATE}
    aws s3 cp /tmp/kafka-metadata-${BACKUP_DATE}.tar.gz \
      ${S3_BUCKET}/metadata/kafka-metadata-${BACKUP_DATE}.tar.gz \
      --storage-class STANDARD_IA

    # Cleanup
    rm -rf ${BACKUP_DIR} /tmp/kafka-metadata-${BACKUP_DATE}.tar.gz

    echo "Kafka metadata backup completed"
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: kafka-metadata-backup
  namespace: llm-research-lab-data
spec:
  schedule: "0 */6 * * *"  # Every 6 hours
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: kafka-backup
            image: confluentinc/cp-kafka:7.5.0
            command: ["/bin/bash", "/scripts/backup.sh"]
            volumeMounts:
            - name: backup-script
              mountPath: /scripts
            resources:
              requests:
                cpu: 200m
                memory: 512Mi
          volumes:
          - name: backup-script
            configMap:
              name: kafka-backup-config
              defaultMode: 0755
```

### 6.1.4 Redis Backup Configuration

```yaml
# k8s/redis-backup-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: redis-backup-config
  namespace: llm-research-lab-data
data:
  backup.sh: |
    #!/bin/bash
    set -e

    BACKUP_DATE=$(date +%Y%m%d_%H%M%S)
    S3_BUCKET="s3://llm-research-lab-redis-backups"

    # Trigger BGSAVE
    echo "Triggering Redis BGSAVE..."
    redis-cli -h redis-master BGSAVE

    # Wait for save to complete
    while [ $(redis-cli -h redis-master LASTSAVE) -eq $(redis-cli -h redis-master LASTSAVE) ]; do
      echo "Waiting for BGSAVE to complete..."
      sleep 5
    done

    # Copy RDB file
    RDB_FILE="/data/dump.rdb"
    BACKUP_FILE="redis-dump-${BACKUP_DATE}.rdb"

    cp ${RDB_FILE} /tmp/${BACKUP_FILE}

    # Upload to S3
    aws s3 cp /tmp/${BACKUP_FILE} \
      ${S3_BUCKET}/snapshots/${BACKUP_FILE} \
      --storage-class STANDARD_IA \
      --server-side-encryption AES256

    # Create metadata
    cat > /tmp/metadata.json <<EOF
    {
      "backup_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
      "redis_version": "$(redis-cli -h redis-master INFO server | grep redis_version)",
      "used_memory": "$(redis-cli -h redis-master INFO memory | grep used_memory_human)",
      "db_keys": "$(redis-cli -h redis-master DBSIZE)"
    }
    EOF

    aws s3 cp /tmp/metadata.json \
      ${S3_BUCKET}/snapshots/${BACKUP_FILE}.metadata.json

    # Cleanup
    rm -f /tmp/${BACKUP_FILE} /tmp/metadata.json

    echo "Redis backup completed: ${BACKUP_FILE}"
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: redis-backup
  namespace: llm-research-lab-data
spec:
  schedule: "0 */12 * * *"  # Every 12 hours
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: redis-backup
            image: redis:7-alpine
            command: ["/bin/sh", "/scripts/backup.sh"]
            volumeMounts:
            - name: backup-script
              mountPath: /scripts
            - name: redis-data
              mountPath: /data
            env:
            - name: AWS_REGION
              value: us-east-1
            resources:
              requests:
                cpu: 200m
                memory: 512Mi
          volumes:
          - name: backup-script
            configMap:
              name: redis-backup-config
              defaultMode: 0755
          - name: redis-data
            persistentVolumeClaim:
              claimName: redis-data
```

### 6.1.5 Backup Verification Testing

```yaml
# k8s/backup-verification-job.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: backup-verification
  namespace: llm-research-lab-data
spec:
  schedule: "0 6 * * 0"  # Weekly on Sunday at 6 AM UTC
  concurrencyPolicy: Forbid
  jobTemplate:
    spec:
      template:
        metadata:
          labels:
            app: backup-verification
        spec:
          restartPolicy: OnFailure
          containers:
          - name: verify-backups
            image: amazon/aws-cli:latest
            command:
            - /bin/bash
            - -c
            - |
              set -e

              echo "=== Backup Verification Report ==="
              echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
              echo ""

              # Verify PostgreSQL backups
              echo "PostgreSQL Backups:"
              LATEST_PG_BACKUP=$(aws s3 ls s3://llm-research-lab-postgres-backups/wal-archive/ \
                --recursive | sort | tail -1 | awk '{print $4}')
              BACKUP_AGE=$(($(date +%s) - $(date -d "$(aws s3 ls s3://llm-research-lab-postgres-backups/wal-archive/${LATEST_PG_BACKUP} | awk '{print $1" "$2}')" +%s)))

              if [ ${BACKUP_AGE} -lt 86400 ]; then
                echo "  ✓ Latest backup: ${LATEST_PG_BACKUP} (${BACKUP_AGE}s old)"
              else
                echo "  ✗ Latest backup is older than 24h: ${BACKUP_AGE}s"
                exit 1
              fi

              # Verify ClickHouse backups
              echo "ClickHouse Backups:"
              LATEST_CH_BACKUP=$(aws s3 ls s3://llm-research-lab-clickhouse-backups/backups/ | sort | tail -1 | awk '{print $2}')
              echo "  ✓ Latest backup: ${LATEST_CH_BACKUP}"

              # Verify Kafka metadata backups
              echo "Kafka Metadata Backups:"
              LATEST_KAFKA_BACKUP=$(aws s3 ls s3://llm-research-lab-kafka-backups/metadata/ | sort | tail -1 | awk '{print $4}')
              echo "  ✓ Latest backup: ${LATEST_KAFKA_BACKUP}"

              # Verify Redis backups
              echo "Redis Backups:"
              LATEST_REDIS_BACKUP=$(aws s3 ls s3://llm-research-lab-redis-backups/snapshots/ | grep .rdb | sort | tail -1 | awk '{print $4}')
              echo "  ✓ Latest backup: ${LATEST_REDIS_BACKUP}"

              # Send success metric
              curl -X POST http://prometheus-pushgateway:9091/metrics/job/backup_verification \
                -d "backup_verification_success 1"

              echo ""
              echo "=== Verification Complete ==="
            resources:
              requests:
                cpu: 100m
                memory: 256Mi
```

### 6.1.6 Retention Policy Management

```yaml
# k8s/backup-retention-cleanup.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: backup-retention-cleanup
  namespace: llm-research-lab-data
spec:
  schedule: "0 5 * * 0"  # Weekly on Sunday at 5 AM UTC
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: cleanup
            image: amazon/aws-cli:latest
            command:
            - /bin/bash
            - -c
            - |
              set -e

              # Function to delete old backups
              cleanup_backups() {
                local bucket=$1
                local prefix=$2
                local days=$3

                echo "Cleaning up backups older than ${days} days in ${bucket}/${prefix}"

                aws s3 ls s3://${bucket}/${prefix} --recursive | \
                  while read -r line; do
                    createDate=$(echo $line | awk '{print $1" "$2}')
                    createDate=$(date -d "$createDate" +%s)
                    olderThan=$(date -d "-${days} days" +%s)

                    if [[ $createDate -lt $olderThan ]]; then
                      fileName=$(echo $line | awk '{print $4}')
                      echo "Deleting: ${fileName}"
                      aws s3 rm s3://${bucket}/${fileName}
                    fi
                  done
              }

              # PostgreSQL: Keep daily for 30 days, weekly for 90 days, monthly for 1 year
              echo "=== PostgreSQL Backup Retention ==="
              # Daily backups older than 30 days
              cleanup_backups "llm-research-lab-postgres-backups" "wal-archive/base_" 30
              # Logical backups older than 90 days
              cleanup_backups "llm-research-lab-postgres-backups" "logical/" 90

              # ClickHouse: Keep 30 daily backups
              echo "=== ClickHouse Backup Retention ==="
              cleanup_backups "llm-research-lab-clickhouse-backups" "backups/" 30

              # Kafka: Keep metadata for 90 days
              echo "=== Kafka Backup Retention ==="
              cleanup_backups "llm-research-lab-kafka-backups" "metadata/" 90

              # Redis: Keep snapshots for 30 days
              echo "=== Redis Backup Retention ==="
              cleanup_backups "llm-research-lab-redis-backups" "snapshots/" 30

              echo "Retention cleanup completed"
            resources:
              requests:
                cpu: 100m
                memory: 256Mi
```

---

## 6.2 Recovery Procedures

### 6.2.1 PostgreSQL Point-in-Time Recovery (PITR)

```bash
#!/bin/bash
# scripts/postgres-pitr-recovery.sh
# PostgreSQL Point-in-Time Recovery Script

set -e

RECOVERY_TARGET_TIME=${1:-"latest"}
RECOVERY_TARGET_NAME=${2:-""}
S3_BACKUP_BUCKET="s3://llm-research-lab-postgres-backups/wal-archive"
PGDATA="/var/lib/postgresql/data"
RECOVERY_CONF="${PGDATA}/postgresql.auto.conf"

echo "=== PostgreSQL PITR Recovery ==="
echo "Recovery Target Time: ${RECOVERY_TARGET_TIME}"
echo "Recovery Target Name: ${RECOVERY_TARGET_NAME}"
echo ""

# Step 1: Stop PostgreSQL if running
echo "[1/7] Stopping PostgreSQL..."
pg_ctl stop -D ${PGDATA} -m fast || true
sleep 5

# Step 2: Backup current data directory
echo "[2/7] Backing up current data directory..."
BACKUP_DIR="/var/lib/postgresql/data-backup-$(date +%Y%m%d_%H%M%S)"
mv ${PGDATA} ${BACKUP_DIR}
echo "Existing data backed up to: ${BACKUP_DIR}"

# Step 3: List available base backups
echo "[3/7] Available base backups:"
wal-g backup-list

# Step 4: Restore base backup
if [ "${RECOVERY_TARGET_TIME}" == "latest" ]; then
  BACKUP_NAME="LATEST"
else
  # Find backup closest to target time
  BACKUP_NAME=$(wal-g backup-list --detail --json | \
    jq -r --arg target "${RECOVERY_TARGET_TIME}" \
    '.[] | select(.time <= $target) | .backup_name' | \
    tail -1)
fi

echo "[4/7] Restoring base backup: ${BACKUP_NAME}"
mkdir -p ${PGDATA}
wal-g backup-fetch ${PGDATA} ${BACKUP_NAME}

# Step 5: Configure recovery settings
echo "[5/7] Configuring recovery settings..."
cat > ${PGDATA}/recovery.signal <<EOF
# Recovery configuration
EOF

cat >> ${RECOVERY_CONF} <<EOF
# Recovery Settings
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_action = 'promote'
EOF

if [ "${RECOVERY_TARGET_TIME}" != "latest" ]; then
  echo "recovery_target_time = '${RECOVERY_TARGET_TIME}'" >> ${RECOVERY_CONF}
fi

if [ -n "${RECOVERY_TARGET_NAME}" ]; then
  echo "recovery_target_name = '${RECOVERY_TARGET_NAME}'" >> ${RECOVERY_CONF}
fi

# Step 6: Set permissions
echo "[6/7] Setting permissions..."
chown -R postgres:postgres ${PGDATA}
chmod 700 ${PGDATA}

# Step 7: Start PostgreSQL in recovery mode
echo "[7/7] Starting PostgreSQL in recovery mode..."
pg_ctl start -D ${PGDATA} -l ${PGDATA}/recovery.log

# Wait for recovery to complete
echo "Waiting for recovery to complete..."
until pg_isready -h localhost -p 5432; do
  echo "PostgreSQL is recovering..."
  sleep 5
done

echo ""
echo "=== Recovery Complete ==="
echo "PostgreSQL is now running and has been promoted."
echo "Recovery log: ${PGDATA}/recovery.log"
echo ""

# Verify recovery
echo "=== Post-Recovery Verification ==="
psql -U postgres -c "SELECT pg_is_in_recovery();"
psql -U postgres -c "SELECT pg_last_wal_receive_lsn();"
psql -U postgres -c "SELECT COUNT(*) FROM experiments;"

echo ""
echo "Recovery completed successfully!"
```

### 6.2.2 ClickHouse Full System Restore

```bash
#!/bin/bash
# scripts/clickhouse-full-restore.sh
# ClickHouse Full System Restore Script

set -e

BACKUP_NAME=${1:-""}
RESTORE_SCHEMA=${2:-"all"}
S3_BUCKET="s3://llm-research-lab-clickhouse-backups/backups"

if [ -z "${BACKUP_NAME}" ]; then
  echo "Usage: $0 <backup_name> [schema_name|all]"
  echo ""
  echo "Available backups:"
  clickhouse-backup list remote
  exit 1
fi

echo "=== ClickHouse Full Restore ==="
echo "Backup Name: ${BACKUP_NAME}"
echo "Restore Schema: ${RESTORE_SCHEMA}"
echo ""

# Step 1: Download backup from S3
echo "[1/5] Downloading backup from S3..."
clickhouse-backup download ${BACKUP_NAME}

# Step 2: List backup contents
echo "[2/5] Backup contents:"
clickhouse-backup list local | grep ${BACKUP_NAME}

# Step 3: Stop ClickHouse writes (optional, creates consistency)
echo "[3/5] Setting ClickHouse to read-only mode..."
clickhouse-client --query "SYSTEM STOP MERGES"
clickhouse-client --query "SET readonly = 1"

# Step 4: Restore backup
echo "[4/5] Restoring backup..."
if [ "${RESTORE_SCHEMA}" == "all" ]; then
  clickhouse-backup restore --schema --data ${BACKUP_NAME}
else
  clickhouse-backup restore --schema --data --table ${RESTORE_SCHEMA}.* ${BACKUP_NAME}
fi

# Step 5: Resume normal operations
echo "[5/5] Resuming normal operations..."
clickhouse-client --query "SET readonly = 0"
clickhouse-client --query "SYSTEM START MERGES"

echo ""
echo "=== Restore Complete ==="

# Verification
echo "=== Post-Restore Verification ==="
clickhouse-client --query "SELECT database, name, total_rows FROM system.tables WHERE database NOT IN ('system', 'INFORMATION_SCHEMA')"

echo ""
echo "Restore completed successfully!"
```

### 6.2.3 Data Integrity Verification

```bash
#!/bin/bash
# scripts/verify-data-integrity.sh
# Post-Recovery Data Integrity Verification

set -e

echo "=== Data Integrity Verification ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

ERRORS=0

# PostgreSQL Integrity Checks
echo "[PostgreSQL Checks]"
echo "Running pg_checksums..."
if pg_checksums -D /var/lib/postgresql/data --check; then
  echo "  ✓ Checksums valid"
else
  echo "  ✗ Checksum validation failed"
  ((ERRORS++))
fi

echo "Running VACUUM ANALYZE..."
psql -U postgres -d llm_research_lab -c "VACUUM ANALYZE;" || ((ERRORS++))

echo "Checking table counts..."
EXPERIMENT_COUNT=$(psql -U postgres -d llm_research_lab -t -c "SELECT COUNT(*) FROM experiments;")
echo "  Experiments: ${EXPERIMENT_COUNT}"

echo "Checking referential integrity..."
psql -U postgres -d llm_research_lab -c "
  SELECT conrelid::regclass AS table_name, conname AS constraint_name
  FROM pg_constraint
  WHERE contype = 'f'
  ORDER BY conrelid::regclass::text, conname;
" || ((ERRORS++))

# ClickHouse Integrity Checks
echo ""
echo "[ClickHouse Checks]"
echo "Checking table integrity..."
clickhouse-client --query "CHECK TABLE experiments.runs" || ((ERRORS++))

echo "Running OPTIMIZE..."
clickhouse-client --query "OPTIMIZE TABLE experiments.runs FINAL" || ((ERRORS++))

echo "Verifying row counts..."
RUN_COUNT=$(clickhouse-client --query "SELECT COUNT(*) FROM experiments.runs")
echo "  Experiment Runs: ${RUN_COUNT}"

# Kafka Checks
echo ""
echo "[Kafka Checks]"
echo "Checking topic health..."
kafka-topics.sh --bootstrap-server kafka:9092 --describe | head -20

echo "Verifying consumer groups..."
kafka-consumer-groups.sh --bootstrap-server kafka:9092 --list

# Redis Checks
echo ""
echo "[Redis Checks]"
echo "Running Redis DBSIZE..."
REDIS_KEYS=$(redis-cli -h redis-master DBSIZE)
echo "  Total Keys: ${REDIS_KEYS}"

echo "Checking Redis memory..."
redis-cli -h redis-master INFO memory | grep used_memory_human

echo "Running Redis consistency check..."
redis-cli -h redis-master --rdb /tmp/redis-check.rdb
redis-check-rdb /tmp/redis-check.rdb || ((ERRORS++))
rm -f /tmp/redis-check.rdb

# Summary
echo ""
echo "=== Verification Summary ==="
if [ ${ERRORS} -eq 0 ]; then
  echo "✓ All integrity checks passed"
  exit 0
else
  echo "✗ ${ERRORS} integrity check(s) failed"
  exit 1
fi
```

---

## 6.3 High Availability Architecture

### 6.3.1 Multi-Zone Deployment Configuration

```yaml
# k8s/multi-az-topology.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: topology-spread-constraints
  namespace: llm-research-lab-prod
data:
  constraints.yaml: |
    # Topology Spread Constraints for HA
    topologySpreadConstraints:
    - maxSkew: 1
      topologyKey: topology.kubernetes.io/zone
      whenUnsatisfiable: DoNotSchedule
      labelSelector:
        matchLabels:
          app: llm-research-lab-api
    - maxSkew: 1
      topologyKey: kubernetes.io/hostname
      whenUnsatisfiable: ScheduleAnyway
      labelSelector:
        matchLabels:
          app: llm-research-lab-api
---
# PostgreSQL Primary-Replica Configuration
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres-primary
  namespace: llm-research-lab-data
spec:
  replicas: 1
  serviceName: postgres-primary
  selector:
    matchLabels:
      app: postgres
      role: primary
  template:
    metadata:
      labels:
        app: postgres
        role: primary
    spec:
      affinity:
        nodeAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            nodeSelectorTerms:
            - matchExpressions:
              - key: topology.kubernetes.io/zone
                operator: In
                values:
                - us-east-1a
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - postgres
              - key: role
                operator: In
                values:
                - replica
            topologyKey: kubernetes.io/hostname
      containers:
      - name: postgres
        image: postgres:15-alpine
        ports:
        - containerPort: 5432
          name: postgres
        env:
        - name: POSTGRES_DB
          value: llm_research_lab
        - name: POSTGRES_USER
          valueFrom:
            secretKeyRef:
              name: postgres-credentials
              key: username
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: postgres-credentials
              key: password
        - name: POSTGRES_REPLICATION_USER
          value: replicator
        - name: POSTGRES_REPLICATION_PASSWORD
          valueFrom:
            secretKeyRef:
              name: postgres-replication
              key: password
        volumeMounts:
        - name: postgres-data
          mountPath: /var/lib/postgresql/data
        - name: postgres-config
          mountPath: /etc/postgresql
        resources:
          requests:
            cpu: 2
            memory: 4Gi
          limits:
            cpu: 4
            memory: 8Gi
        livenessProbe:
          exec:
            command:
            - /bin/sh
            - -c
            - pg_isready -U postgres
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          exec:
            command:
            - /bin/sh
            - -c
            - pg_isready -U postgres && psql -U postgres -c "SELECT 1"
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
      volumes:
      - name: postgres-config
        configMap:
          name: postgres-primary-config
  volumeClaimTemplates:
  - metadata:
      name: postgres-data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 100Gi
---
# PostgreSQL Read Replicas (Multi-AZ)
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres-replica
  namespace: llm-research-lab-data
spec:
  replicas: 2
  serviceName: postgres-replica
  selector:
    matchLabels:
      app: postgres
      role: replica
  template:
    metadata:
      labels:
        app: postgres
        role: replica
    spec:
      topologySpreadConstraints:
      - maxSkew: 1
        topologyKey: topology.kubernetes.io/zone
        whenUnsatisfiable: DoNotSchedule
        labelSelector:
          matchLabels:
            app: postgres
            role: replica
      containers:
      - name: postgres
        image: postgres:15-alpine
        ports:
        - containerPort: 5432
          name: postgres
        env:
        - name: POSTGRES_PRIMARY_HOST
          value: postgres-primary.llm-research-lab-data.svc.cluster.local
        - name: POSTGRES_REPLICATION_USER
          value: replicator
        - name: POSTGRES_REPLICATION_PASSWORD
          valueFrom:
            secretKeyRef:
              name: postgres-replication
              key: password
        command:
        - /bin/bash
        - -c
        - |
          set -e
          # Wait for primary to be ready
          until pg_isready -h ${POSTGRES_PRIMARY_HOST}; do
            echo "Waiting for primary..."
            sleep 2
          done

          # Create base backup from primary
          rm -rf /var/lib/postgresql/data/*
          pg_basebackup -h ${POSTGRES_PRIMARY_HOST} \
            -U ${POSTGRES_REPLICATION_USER} \
            -D /var/lib/postgresql/data \
            -Fp -Xs -P -R

          # Start PostgreSQL in replica mode
          postgres
        volumeMounts:
        - name: postgres-data
          mountPath: /var/lib/postgresql/data
        resources:
          requests:
            cpu: 1
            memory: 2Gi
          limits:
            cpu: 2
            memory: 4Gi
  volumeClaimTemplates:
  - metadata:
      name: postgres-data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 100Gi
```

### 6.3.2 Failover Automation

```yaml
# k8s/postgres-failover-automation.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: postgres-failover-script
  namespace: llm-research-lab-data
data:
  failover.sh: |
    #!/bin/bash
    set -e

    PRIMARY_HOST="postgres-primary.llm-research-lab-data.svc.cluster.local"
    REPLICA_SELECTOR="app=postgres,role=replica"
    NAMESPACE="llm-research-lab-data"

    echo "=== PostgreSQL Failover Automation ==="
    echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""

    # Check primary health
    if pg_isready -h ${PRIMARY_HOST} -t 5; then
      echo "Primary is healthy. No failover needed."
      exit 0
    fi

    echo "⚠️  Primary is unhealthy. Initiating failover..."

    # Find healthiest replica
    BEST_REPLICA=""
    BEST_LSN=0

    for pod in $(kubectl get pods -n ${NAMESPACE} -l ${REPLICA_SELECTOR} -o name); do
      POD_NAME=${pod#pod/}
      echo "Checking replica: ${POD_NAME}"

      # Get LSN (log sequence number)
      LSN=$(kubectl exec -n ${NAMESPACE} ${POD_NAME} -- \
        psql -U postgres -t -c "SELECT pg_last_wal_receive_lsn();" | tr -d ' ')

      echo "  LSN: ${LSN}"

      if [ "${LSN}" \> "${BEST_LSN}" ]; then
        BEST_LSN=${LSN}
        BEST_REPLICA=${POD_NAME}
      fi
    done

    if [ -z "${BEST_REPLICA}" ]; then
      echo "❌ No healthy replica found. Manual intervention required."
      exit 1
    fi

    echo "Selected replica for promotion: ${BEST_REPLICA}"
    echo "Promoting replica to primary..."

    # Promote replica
    kubectl exec -n ${NAMESPACE} ${BEST_REPLICA} -- \
      pg_ctl promote -D /var/lib/postgresql/data

    # Wait for promotion
    sleep 5

    # Verify new primary
    NEW_PRIMARY_IP=$(kubectl get pod -n ${NAMESPACE} ${BEST_REPLICA} -o jsonpath='{.status.podIP}')

    if pg_isready -h ${NEW_PRIMARY_IP}; then
      echo "✓ Failover successful. New primary: ${BEST_REPLICA}"

      # Update service to point to new primary
      kubectl patch service postgres-primary -n ${NAMESPACE} -p \
        "{\"spec\":{\"selector\":{\"statefulset.kubernetes.io/pod-name\":\"${BEST_REPLICA}\"}}}"

      # Send alert
      curl -X POST http://alertmanager:9093/api/v1/alerts -d "[{
        \"labels\": {
          \"alertname\": \"PostgreSQLFailover\",
          \"severity\": \"critical\",
          \"component\": \"postgres\"
        },
        \"annotations\": {
          \"summary\": \"PostgreSQL failover completed\",
          \"description\": \"Promoted ${BEST_REPLICA} to primary\"
        }
      }]"

      exit 0
    else
      echo "❌ Failover failed. New primary is not responding."
      exit 1
    fi
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-failover-monitor
  namespace: llm-research-lab-data
spec:
  schedule: "*/1 * * * *"  # Every minute
  concurrencyPolicy: Forbid
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: postgres-operator
          restartPolicy: Never
          containers:
          - name: failover-monitor
            image: postgres:15-alpine
            command: ["/bin/bash", "/scripts/failover.sh"]
            volumeMounts:
            - name: failover-script
              mountPath: /scripts
          volumes:
          - name: failover-script
            configMap:
              name: postgres-failover-script
              defaultMode: 0755
```

### 6.3.3 Health Check Configuration

```yaml
# k8s/health-checks.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: health-check-config
  namespace: llm-research-lab-prod
data:
  # PostgreSQL Health Check
  postgres-health.sh: |
    #!/bin/bash
    set -e

    # Connection check
    pg_isready -h ${PGHOST} -p ${PGPORT} -U ${PGUSER} || exit 1

    # Replication lag check (for replicas)
    if [ "${ROLE}" == "replica" ]; then
      LAG=$(psql -U ${PGUSER} -t -c "SELECT EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp()));" | tr -d ' ')
      if (( $(echo "$LAG > 60" | bc -l) )); then
        echo "Replication lag too high: ${LAG}s"
        exit 1
      fi
    fi

    # Connection pool check
    CONNECTIONS=$(psql -U ${PGUSER} -t -c "SELECT count(*) FROM pg_stat_activity;" | tr -d ' ')
    MAX_CONNECTIONS=$(psql -U ${PGUSER} -t -c "SHOW max_connections;" | tr -d ' ')
    USAGE=$(echo "scale=2; ${CONNECTIONS}/${MAX_CONNECTIONS}*100" | bc)

    if (( $(echo "$USAGE > 90" | bc -l) )); then
      echo "Connection pool near capacity: ${USAGE}%"
      exit 1
    fi

    exit 0

  # ClickHouse Health Check
  clickhouse-health.sh: |
    #!/bin/bash
    set -e

    # Basic connectivity
    clickhouse-client --query "SELECT 1" || exit 1

    # Check replicas (if applicable)
    REPLICA_STATUS=$(clickhouse-client --query "SELECT count() FROM system.replicas WHERE is_session_expired = 1")
    if [ "${REPLICA_STATUS}" -gt 0 ]; then
      echo "Found ${REPLICA_STATUS} expired replica sessions"
      exit 1
    fi

    # Check merge queue
    MERGE_QUEUE=$(clickhouse-client --query "SELECT count() FROM system.merges")
    if [ "${MERGE_QUEUE}" -gt 100 ]; then
      echo "Merge queue too large: ${MERGE_QUEUE}"
      exit 1
    fi

    exit 0

  # Kafka Health Check
  kafka-health.sh: |
    #!/bin/bash
    set -e

    # Broker connectivity
    kafka-broker-api-versions.sh --bootstrap-server ${KAFKA_BOOTSTRAP} || exit 1

    # Check under-replicated partitions
    URP=$(kafka-topics.sh --bootstrap-server ${KAFKA_BOOTSTRAP} \
      --describe --under-replicated-partitions | wc -l)

    if [ "${URP}" -gt 0 ]; then
      echo "Found ${URP} under-replicated partitions"
      exit 1
    fi

    exit 0

  # Redis Health Check
  redis-health.sh: |
    #!/bin/bash
    set -e

    # Ping check
    redis-cli -h ${REDIS_HOST} PING | grep -q PONG || exit 1

    # Memory usage check
    MEMORY_USAGE=$(redis-cli -h ${REDIS_HOST} INFO memory | \
      grep used_memory_rss_human | cut -d: -f2 | tr -d '\r')

    echo "Memory usage: ${MEMORY_USAGE}"

    # Replication check (for replicas)
    if [ "${ROLE}" == "replica" ]; then
      MASTER_LINK=$(redis-cli -h ${REDIS_HOST} INFO replication | \
        grep master_link_status | cut -d: -f2 | tr -d '\r')

      if [ "${MASTER_LINK}" != "up" ]; then
        echo "Master link down"
        exit 1
      fi
    fi

    exit 0
---
# Liveness and Readiness Probe Examples
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-research-lab-api
  namespace: llm-research-lab-prod
spec:
  template:
    spec:
      containers:
      - name: api
        image: llm-research-lab-api:latest
        ports:
        - containerPort: 8000
          name: http
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8000
            httpHeaders:
            - name: X-Health-Check
              value: liveness
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8000
            httpHeaders:
            - name: X-Health-Check
              value: readiness
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        startupProbe:
          httpGet:
            path: /health/startup
            port: 8000
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 30  # 30 * 5s = 2.5 min startup time
```

---

## 6.4 Disaster Recovery Plan

### 6.4.1 DR Scenarios and Triggers

```yaml
# disaster-recovery-scenarios.yaml
scenarios:
  - id: DR-001
    name: Single AZ Failure
    trigger:
      - Multiple node failures in single AZ
      - AZ network partition
      - AWS AZ status page indicates outage
    severity: high
    rto: 1 hour
    rpo: 15 minutes
    response:
      - Automatic pod rescheduling to healthy AZs
      - Verify replica promotion for databases
      - Monitor cross-AZ traffic increase
      - Validate application functionality

  - id: DR-002
    name: Database Primary Failure
    trigger:
      - PostgreSQL primary unresponsive for 60s
      - Failed health checks (3 consecutive)
      - Replication lag exceeds 5 minutes
    severity: critical
    rto: 15 minutes
    rpo: 30 seconds
    response:
      - Execute automatic failover to replica
      - Update DNS/service endpoints
      - Verify data consistency
      - Rebuild failed primary as new replica

  - id: DR-003
    name: Data Corruption
    trigger:
      - Checksum validation failures
      - Application reports data inconsistencies
      - Failed integrity checks
    severity: critical
    rto: 4 hours
    rpo: 1 hour
    response:
      - Identify corruption scope
      - Restore from last known good backup
      - Run PITR to minimize data loss
      - Perform full integrity verification

  - id: DR-004
    name: Ransomware/Security Breach
    trigger:
      - Security alert from IDS/IPS
      - Unexplained file encryption
      - Unauthorized database access
    severity: critical
    rto: 8 hours
    rpo: 4 hours
    response:
      - Isolate affected systems immediately
      - Preserve forensic evidence
      - Restore from immutable backups
      - Rotate all credentials
      - Conduct security audit

  - id: DR-005
    name: Regional Outage
    trigger:
      - AWS region status page indicates major outage
      - Cannot reach services in multiple AZs
      - Loss of connectivity to control plane
    severity: critical
    rto: 4 hours
    rpo: 1 hour
    response:
      - Activate DR region
      - Restore latest backups to DR region
      - Update DNS to point to DR region
      - Notify stakeholders
      - Monitor DR region performance
```

### 6.4.2 Recovery Runbook Template

```markdown
# Disaster Recovery Runbook: [Scenario Name]

**Scenario ID**: [DR-XXX]
**RTO**: [X hours]
**RPO**: [X minutes/hours]
**Last Updated**: [Date]
**Owner**: [Team/Individual]

---

## 1. Detection & Alert

### Symptoms
- [List observable symptoms]
- [Include metric thresholds]
- [Log patterns]

### Alert Channels
- PagerDuty: [incident-type]
- Slack: #incidents
- Email: oncall@company.com

### Initial Assessment Checklist
- [ ] Confirm incident severity
- [ ] Identify affected components
- [ ] Estimate blast radius
- [ ] Page on-call engineer
- [ ] Create incident ticket

---

## 2. Immediate Response (First 15 Minutes)

### Stop the Bleeding
```bash
# Emergency commands to stabilize system
# Example: Stop write traffic, enable read-only mode
```

### Communication
- [ ] Post in #incidents channel
- [ ] Notify stakeholders
- [ ] Update status page

### Data Preservation
- [ ] Capture logs
- [ ] Take system snapshots
- [ ] Document timeline

---

## 3. Recovery Procedure

### Step 1: [Action Name]
**Duration**: [X minutes]
**Owner**: [Role]

```bash
# Exact commands to execute
```

**Validation**:
- [ ] [Check 1]
- [ ] [Check 2]

### Step 2: [Action Name]
**Duration**: [X minutes]
**Owner**: [Role]

```bash
# Exact commands to execute
```

**Validation**:
- [ ] [Check 1]
- [ ] [Check 2]

[Continue for all steps...]

---

## 4. Verification

### Functional Tests
```bash
# Smoke tests to verify service functionality
./scripts/smoke-tests.sh
```

### Data Integrity
```bash
# Data verification commands
./scripts/verify-data-integrity.sh
```

### Performance Baseline
- [ ] API latency < 100ms p99
- [ ] Database connections healthy
- [ ] Message queue no backlog
- [ ] Cache hit rate > 80%

---

## 5. Post-Recovery

### Monitoring
- [ ] Enable enhanced monitoring (30 minutes)
- [ ] Watch for anomalies
- [ ] Monitor error rates

### Cleanup
- [ ] Remove temporary fixes
- [ ] Clean up failed pods/containers
- [ ] Archive logs

### Communication
- [ ] Send all-clear notification
- [ ] Update status page
- [ ] Schedule post-mortem

---

## 6. Rollback Procedure

If recovery fails:

```bash
# Rollback commands
```

---

## 7. Contacts

| Role | Name | Contact |
|------|------|---------|
| Incident Commander | [Name] | [Phone/Slack] |
| Database DRI | [Name] | [Phone/Slack] |
| Infrastructure Lead | [Name] | [Phone/Slack] |
| Engineering Manager | [Name] | [Phone/Slack] |

---

## 8. Related Resources

- Architecture Diagram: [Link]
- Monitoring Dashboard: [Link]
- Previous Incidents: [Links]
```

### 6.4.3 DR Testing Schedule

```yaml
# k8s/dr-testing-schedule.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: dr-testing-schedule
  namespace: llm-research-lab-prod
data:
  schedule.yaml: |
    # Quarterly DR Testing Schedule

    # Q1 - January
    - month: January
      week: 3
      test_type: Database Failover
      scope: PostgreSQL Primary Failover
      duration: 2 hours
      participants:
        - Database Team
        - Platform Team
      objectives:
        - Verify automatic failover within RTO
        - Validate replica promotion
        - Test monitoring and alerting
        - Document actual recovery time

    # Q1 - February
    - month: February
      week: 3
      test_type: Backup Restore
      scope: Full PostgreSQL PITR
      duration: 4 hours
      participants:
        - Database Team
        - Application Team
      objectives:
        - Restore from 24-hour-old backup
        - Verify data integrity
        - Test recovery procedures
        - Validate RPO compliance

    # Q1 - March
    - month: March
      week: 3
      test_type: AZ Failure Simulation
      scope: Drain single availability zone
      duration: 3 hours
      participants:
        - Platform Team
        - SRE Team
      objectives:
        - Simulate AZ outage
        - Verify pod rescheduling
        - Test cross-AZ failover
        - Monitor performance impact

    # Q2 - April
    - month: April
      week: 3
      test_type: ClickHouse Recovery
      scope: Full ClickHouse restore from backup
      duration: 3 hours
      participants:
        - Data Platform Team
        - Analytics Team
      objectives:
        - Restore ClickHouse from S3 backup
        - Verify table schemas and data
        - Test query performance post-restore
        - Validate backup integrity

    # Q2 - May
    - month: May
      week: 3
      test_type: Redis Failover
      scope: Redis master failover
      duration: 1 hour
      participants:
        - Cache Team
        - Application Team
      objectives:
        - Trigger Redis Sentinel failover
        - Verify cache hit rates post-failover
        - Test application resilience
        - Document failover timing

    # Q2 - June
    - month: June
      week: 3
      test_type: Full System DR
      scope: Complete regional failover
      duration: 8 hours
      participants:
        - All Engineering Teams
        - Management
      objectives:
        - Simulate complete region failure
        - Activate DR region
        - Restore all services
        - Full end-to-end testing
        - Update DR playbooks

    # Q3 - July
    - month: July
      week: 3
      test_type: Kafka Recovery
      scope: Kafka cluster rebuild
      duration: 3 hours
      participants:
        - Streaming Team
        - Platform Team
      objectives:
        - Restore Kafka metadata
        - Verify topic configurations
        - Test consumer group offsets
        - Validate message delivery

    # Q3 - August
    - month: August
      week: 3
      test_type: Security Incident Response
      scope: Simulated ransomware attack
      duration: 4 hours
      participants:
        - Security Team
        - Incident Response Team
      objectives:
        - Test isolation procedures
        - Restore from immutable backups
        - Validate credential rotation
        - Review forensic processes

    # Q3 - September
    - month: September
      week: 3
      test_type: Data Corruption Recovery
      scope: Restore from corrupted state
      duration: 4 hours
      participants:
        - Database Team
        - Application Team
      objectives:
        - Simulate data corruption
        - Execute PITR to pre-corruption state
        - Run integrity verification
        - Test detection mechanisms

    # Q4 - October
    - month: October
      week: 3
      test_type: Network Partition
      scope: Simulate network split-brain
      duration: 2 hours
      participants:
        - Network Team
        - Platform Team
      objectives:
        - Simulate network partition
        - Test split-brain prevention
        - Verify quorum behavior
        - Validate fencing mechanisms

    # Q4 - November
    - month: November
      week: 3
      test_type: Capacity Failure
      scope: Storage exhaustion recovery
      duration: 2 hours
      participants:
        - Storage Team
        - Platform Team
      objectives:
        - Simulate disk full condition
        - Test automatic cleanup
        - Verify volume expansion
        - Test monitoring alerts

    # Q4 - December
    - month: December
      week: 2  # Earlier in month due to holidays
      test_type: Year-End Full DR Drill
      scope: Complete DR validation
      duration: 8 hours
      participants:
        - All Teams
        - Executive Stakeholders
      objectives:
        - Full regional failover
        - Restore all data stores
        - End-to-end application testing
        - Update all DR documentation
        - Year-end compliance review
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: dr-test-reminder
  namespace: llm-research-lab-prod
spec:
  schedule: "0 9 1 * *"  # First day of each month at 9 AM
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: Never
          containers:
          - name: reminder
            image: curlimages/curl:latest
            command:
            - /bin/sh
            - -c
            - |
              MONTH=$(date +%B)

              curl -X POST https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK \
                -H 'Content-Type: application/json' \
                -d "{
                  \"text\": \"📋 *DR Testing Reminder*\",
                  \"blocks\": [
                    {
                      \"type\": \"section\",
                      \"text\": {
                        \"type\": \"mrkdwn\",
                        \"text\": \"DR testing for *${MONTH}* is scheduled for week 3. Please review the DR testing schedule and prepare.\"
                      }
                    },
                    {
                      \"type\": \"section\",
                      \"text\": {
                        \"type\": \"mrkdwn\",
                        \"text\": \"📘 <https://docs.company.com/dr-testing|DR Testing Guide>\"
                      }
                    }
                  ]
                }"
```

---

## 6.5 Business Continuity

### 6.5.1 Service Degradation Procedures

```yaml
# service-degradation-procedures.yaml
degradation_modes:
  # Level 1: Minor Degradation
  - level: 1
    name: Read-Only Mode
    triggers:
      - Database primary failure (during failover)
      - High write latency (>500ms p99)
      - Storage approaching capacity (>90%)
    actions:
      enable_read_only: true
      disable_features:
        - experiment_creation
        - parameter_updates
        - user_registration
      keep_enabled:
        - experiment_viewing
        - result_querying
        - dashboard_access
    implementation: |
      # Enable read-only mode
      kubectl patch configmap app-config -n llm-research-lab-prod \
        -p '{"data":{"READ_ONLY_MODE":"true"}}'

      # Restart pods to pick up new config
      kubectl rollout restart deployment llm-research-lab-api \
        -n llm-research-lab-prod

      # Update status page
      curl -X POST https://status.company.com/api/incidents \
        -H "Authorization: Bearer $TOKEN" \
        -d '{"status": "investigating", "message": "Write operations temporarily disabled"}'

  # Level 2: Moderate Degradation
  - level: 2
    name: Essential Services Only
    triggers:
      - Multiple component failures
      - Severe performance degradation
      - Approaching resource limits
    actions:
      disable_features:
        - experiment_creation
        - batch_processing
        - report_generation
        - background_jobs
        - non-critical_apis
      keep_enabled:
        - authentication
        - experiment_viewing
        - critical_dashboards
    implementation: |
      # Scale down non-essential services
      kubectl scale deployment experiment-runner --replicas=0 -n llm-research-lab-prod
      kubectl scale deployment report-generator --replicas=0 -n llm-research-lab-prod
      kubectl scale deployment background-worker --replicas=0 -n llm-research-lab-prod

      # Enable circuit breakers
      kubectl apply -f - <<EOF
      apiVersion: networking.istio.io/v1beta1
      kind: DestinationRule
      metadata:
        name: circuit-breaker
        namespace: llm-research-lab-prod
      spec:
        host: llm-research-lab-api
        trafficPolicy:
          outlierDetection:
            consecutiveErrors: 5
            interval: 30s
            baseEjectionTime: 30s
      EOF

  # Level 3: Severe Degradation
  - level: 3
    name: Emergency Maintenance Mode
    triggers:
      - Critical security incident
      - Data corruption detected
      - Cascading failures
    actions:
      disable_all_writes: true
      enable_maintenance_page: true
      preserve_read_access: limited
    implementation: |
      # Enable maintenance mode
      kubectl apply -f - <<EOF
      apiVersion: v1
      kind: ConfigMap
      metadata:
        name: maintenance-mode
        namespace: llm-research-lab-prod
      data:
        enabled: "true"
        message: "System under emergency maintenance. Expected recovery: [TIME]"
      EOF

      # Deploy maintenance page
      kubectl apply -f k8s/maintenance-page.yaml

      # Notify all users
      ./scripts/send-maintenance-notification.sh
```

### 6.5.2 Recovery Priority Matrix

```yaml
# recovery-priority-matrix.yaml
recovery_priorities:
  # Priority 1: Critical (RTO < 30 minutes)
  - priority: P1
    description: Core authentication and data access
    components:
      - name: Authentication Service
        rto: 15 minutes
        rpo: 5 minutes
        dependencies:
          - PostgreSQL Primary
          - Redis Cache
        recovery_steps:
          - Verify database connectivity
          - Restore Redis cache
          - Validate OAuth flows
          - Test login functionality

      - name: PostgreSQL Primary
        rto: 15 minutes
        rpo: 30 seconds
        dependencies: []
        recovery_steps:
          - Execute automatic failover
          - Promote replica to primary
          - Update service endpoints
          - Verify replication

      - name: API Gateway
        rto: 10 minutes
        rpo: 0 (stateless)
        dependencies:
          - Load Balancer
          - Authentication Service
        recovery_steps:
          - Verify pod health
          - Scale up if needed
          - Test routing rules
          - Validate SSL certificates

  # Priority 2: High (RTO < 1 hour)
  - priority: P2
    description: Core experiment functionality
    components:
      - name: Experiment API
        rto: 30 minutes
        rpo: 5 minutes
        dependencies:
          - PostgreSQL
          - Kafka
          - Redis
        recovery_steps:
          - Restore database connection
          - Verify Kafka connectivity
          - Test experiment CRUD
          - Validate result storage

      - name: ClickHouse Analytics
        rto: 1 hour
        rpo: 15 minutes
        dependencies:
          - Storage volumes
        recovery_steps:
          - Restore from backup if needed
          - Verify table integrity
          - Test query performance
          - Rebuild materialized views

      - name: Message Queue (Kafka)
        rto: 45 minutes
        rpo: 10 minutes
        dependencies:
          - ZooKeeper
        recovery_steps:
          - Restore broker configuration
          - Verify topic health
          - Check consumer groups
          - Test message flow

  # Priority 3: Medium (RTO < 4 hours)
  - priority: P3
    description: Analytics and reporting
    components:
      - name: Report Generator
        rto: 2 hours
        rpo: 1 hour
        dependencies:
          - ClickHouse
          - PostgreSQL
        recovery_steps:
          - Restore service deployment
          - Verify data access
          - Test report generation
          - Validate export functionality

      - name: Batch Processing
        rto: 4 hours
        rpo: 2 hours
        dependencies:
          - Kafka
          - ClickHouse
        recovery_steps:
          - Restore worker pods
          - Verify queue connectivity
          - Resume processing
          - Monitor backlog

  # Priority 4: Low (RTO < 24 hours)
  - priority: P4
    description: Nice-to-have features
    components:
      - name: Email Notifications
        rto: 8 hours
        rpo: 24 hours
        dependencies:
          - SMTP Service
        recovery_steps:
          - Restore service
          - Test email delivery
          - Process queued emails

      - name: Background Jobs
        rto: 24 hours
        rpo: 24 hours
        dependencies:
          - Redis
          - PostgreSQL
        recovery_steps:
          - Restore job scheduler
          - Verify job definitions
          - Resume processing
```

### 6.5.3 Communication Templates

```markdown
# Communication Templates

## Initial Incident Notification

**Subject**: [SEVERITY] Incident: [Brief Description]

Team,

We are currently experiencing [brief description of the issue].

**Impact**: [User-facing impact]
**Affected Services**: [List of services]
**Current Status**: [Investigating/Identified/Monitoring/Resolved]
**Estimated Resolution**: [Time or "Unknown - will update in X minutes"]

We will provide updates every [frequency].

Status Page: https://status.company.com/incidents/[ID]
Incident Channel: #incident-[ID]

---

## Update Notification

**Subject**: UPDATE - [SEVERITY] Incident: [Brief Description]

Team,

**Status Update** - [Timestamp]

**Progress**: [What we've done]
**Current State**: [Where we are now]
**Next Steps**: [What we're doing next]
**Expected Resolution**: [Updated ETA]

Previous updates: [Link to status page]

---

## Resolution Notification

**Subject**: RESOLVED - [SEVERITY] Incident: [Brief Description]

Team,

The incident affecting [services] has been resolved as of [timestamp].

**Root Cause**: [Brief explanation]
**Resolution**: [What was done to fix it]
**Preventive Measures**: [What we're doing to prevent recurrence]

**Timeline**:
- [Timestamp]: Issue detected
- [Timestamp]: Response initiated
- [Timestamp]: Fix implemented
- [Timestamp]: Service restored
- [Timestamp]: Verification complete

**Post-Mortem**: We will conduct a blameless post-mortem and share findings by [date].

Thank you for your patience.

---

## External/Customer Communication

**Subject**: Service Disruption Notice - [Date]

Dear Valued Customers,

We experienced a service disruption on [date] from [start time] to [end time].

**What Happened**: [Non-technical explanation]
**Impact**: [What users experienced]
**Resolution**: [What we did to fix it]
**Prevention**: [Steps we're taking to prevent this]

We sincerely apologize for any inconvenience this may have caused. If you have any questions or concerns, please contact our support team.

Best regards,
[Company] Engineering Team
```

---

## Appendix A: Recovery Scripts

### A.1 Emergency Database Recovery Script

```bash
#!/bin/bash
# scripts/emergency-db-recovery.sh
# Emergency PostgreSQL Recovery - Run under supervision

set -e

usage() {
  cat <<EOF
Emergency PostgreSQL Recovery Script

Usage: $0 [OPTIONS]

Options:
  -m, --mode          Recovery mode: failover|restore|rebuild
  -t, --target-time   Target time for PITR (ISO 8601 format)
  -b, --backup-name   Specific backup name to restore
  -f, --force         Skip confirmations (USE WITH CAUTION)
  -h, --help          Show this help message

Examples:
  # Automatic failover to replica
  $0 --mode failover

  # Point-in-time recovery
  $0 --mode restore --target-time "2024-01-15 14:30:00+00"

  # Restore specific backup
  $0 --mode restore --backup-name "base_20240115_143000"

EOF
  exit 1
}

# Parse arguments
MODE=""
TARGET_TIME=""
BACKUP_NAME=""
FORCE=false

while [[ $# -gt 0 ]]; do
  case $1 in
    -m|--mode)
      MODE="$2"
      shift 2
      ;;
    -t|--target-time)
      TARGET_TIME="$2"
      shift 2
      ;;
    -b|--backup-name)
      BACKUP_NAME="$2"
      shift 2
      ;;
    -f|--force)
      FORCE=true
      shift
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

# Validate mode
if [[ ! "$MODE" =~ ^(failover|restore|rebuild)$ ]]; then
  echo "Error: Invalid mode. Must be failover, restore, or rebuild"
  usage
fi

# Confirmation
if [ "$FORCE" != true ]; then
  echo "⚠️  WARNING: This script will perform emergency database recovery."
  echo "Mode: $MODE"
  echo "Target Time: ${TARGET_TIME:-latest}"
  echo "Backup Name: ${BACKUP_NAME:-auto-select}"
  echo ""
  read -p "Are you sure you want to continue? (type 'YES' to confirm): " confirm

  if [ "$confirm" != "YES" ]; then
    echo "Aborted."
    exit 0
  fi
fi

# Logging
LOG_FILE="/var/log/postgres-recovery-$(date +%Y%m%d_%H%M%S).log"
exec 1> >(tee -a "$LOG_FILE")
exec 2>&1

echo "=== Emergency PostgreSQL Recovery ==="
echo "Start Time: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Mode: $MODE"
echo "Log File: $LOG_FILE"
echo ""

# Mode-specific recovery
case $MODE in
  failover)
    echo "Initiating automatic failover..."
    /scripts/postgres-failover.sh
    ;;

  restore)
    echo "Initiating point-in-time recovery..."
    if [ -n "$BACKUP_NAME" ]; then
      /scripts/postgres-pitr-recovery.sh "$BACKUP_NAME"
    elif [ -n "$TARGET_TIME" ]; then
      /scripts/postgres-pitr-recovery.sh "$TARGET_TIME"
    else
      /scripts/postgres-pitr-recovery.sh "latest"
    fi
    ;;

  rebuild)
    echo "Rebuilding database from latest backup..."
    /scripts/postgres-rebuild.sh
    ;;
esac

# Post-recovery verification
echo ""
echo "=== Post-Recovery Verification ==="
/scripts/verify-data-integrity.sh

# Notify team
echo ""
echo "=== Sending Notifications ==="
curl -X POST https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK \
  -H 'Content-Type: application/json' \
  -d "{
    \"text\": \"🚨 Emergency database recovery completed\",
    \"blocks\": [
      {
        \"type\": \"section\",
        \"text\": {
          \"type\": \"mrkdwn\",
          \"text\": \"*Mode*: $MODE\n*Status*: Success\n*Duration*: $SECONDS seconds\n*Log*: \`$LOG_FILE\`\"
        }
      }
    ]
  }"

echo ""
echo "=== Recovery Complete ==="
echo "End Time: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Duration: $SECONDS seconds"
echo "Log File: $LOG_FILE"
```

### A.2 Full System Health Check Script

```bash
#!/bin/bash
# scripts/full-system-health-check.sh
# Comprehensive system health verification

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

check_component() {
  local component=$1
  local check_cmd=$2

  echo -n "Checking ${component}... "

  if eval "$check_cmd" &>/dev/null; then
    echo -e "${GREEN}✓${NC}"
  else
    echo -e "${RED}✗${NC}"
    ((ERRORS++))
  fi
}

check_metric() {
  local component=$1
  local metric=$2
  local threshold=$3
  local operator=$4
  local current=$5

  echo -n "  ${metric}: ${current} "

  if (( $(echo "$current $operator $threshold" | bc -l) )); then
    echo -e "${GREEN}✓${NC}"
  else
    echo -e "${YELLOW}⚠${NC}"
    ((WARNINGS++))
  fi
}

echo "=== Full System Health Check ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Kubernetes Health
echo "[Kubernetes Cluster]"
check_component "API Server" "kubectl cluster-info"
check_component "Node Readiness" "kubectl get nodes | grep -v NotReady"

NODE_COUNT=$(kubectl get nodes --no-headers | wc -l)
READY_NODES=$(kubectl get nodes --no-headers | grep " Ready" | wc -l)
check_metric "Kubernetes" "Ready Nodes" "${NODE_COUNT}" "==" "${READY_NODES}"

# PostgreSQL Health
echo ""
echo "[PostgreSQL]"
check_component "Primary Connectivity" "pg_isready -h postgres-primary"
check_component "Replica Connectivity" "pg_isready -h postgres-replica"

REPLICATION_LAG=$(psql -h postgres-replica -U postgres -t -c "SELECT EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp()));" | tr -d ' ')
check_metric "PostgreSQL" "Replication Lag (sec)" "60" "<" "${REPLICATION_LAG:-0}"

CONNECTION_COUNT=$(psql -h postgres-primary -U postgres -t -c "SELECT count(*) FROM pg_stat_activity;" | tr -d ' ')
check_metric "PostgreSQL" "Active Connections" "100" "<" "${CONNECTION_COUNT}"

# ClickHouse Health
echo ""
echo "[ClickHouse]"
check_component "Connectivity" "clickhouse-client --query 'SELECT 1'"

MERGE_COUNT=$(clickhouse-client --query "SELECT count() FROM system.merges" 2>/dev/null || echo "0")
check_metric "ClickHouse" "Active Merges" "50" "<" "${MERGE_COUNT}"

# Kafka Health
echo ""
echo "[Kafka]"
check_component "Broker Connectivity" "kafka-broker-api-versions.sh --bootstrap-server kafka:9092"

URP_COUNT=$(kafka-topics.sh --bootstrap-server kafka:9092 --describe --under-replicated-partitions 2>/dev/null | wc -l)
check_metric "Kafka" "Under-Replicated Partitions" "0" "==" "${URP_COUNT}"

# Redis Health
echo ""
echo "[Redis]"
check_component "Master Connectivity" "redis-cli -h redis-master PING | grep PONG"
check_component "Replica Connectivity" "redis-cli -h redis-replica PING | grep PONG"

REDIS_MEMORY=$(redis-cli -h redis-master INFO memory | grep used_memory_human | cut -d: -f2 | tr -d '\r')
echo "  Memory Usage: ${REDIS_MEMORY}"

# Application Health
echo ""
echo "[Application Services]"
check_component "API Server" "curl -sf http://llm-research-lab-api:8000/health"
check_component "Experiment Runner" "curl -sf http://experiment-runner:8001/health"

# Backup Status
echo ""
echo "[Backup Status]"
LAST_PG_BACKUP=$(aws s3 ls s3://llm-research-lab-postgres-backups/wal-archive/ --recursive | sort | tail -1 | awk '{print $1" "$2}')
BACKUP_AGE=$(($(date +%s) - $(date -d "$LAST_PG_BACKUP" +%s)))
check_metric "Backups" "Last PostgreSQL Backup Age (hours)" "24" "<" "$((BACKUP_AGE / 3600))"

# Summary
echo ""
echo "=== Health Check Summary ==="
echo -e "Errors: ${RED}${ERRORS}${NC}"
echo -e "Warnings: ${YELLOW}${WARNINGS}${NC}"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
  echo -e "${GREEN}All systems operational${NC}"
  exit 0
elif [ $ERRORS -eq 0 ]; then
  echo -e "${YELLOW}Systems operational with warnings${NC}"
  exit 0
else
  echo -e "${RED}System health check failed${NC}"
  exit 1
fi
```

---

## Appendix B: Runbook Templates

### B.1 Database Failover Runbook

```markdown
# PostgreSQL Failover Runbook

**Runbook ID**: RB-DB-001
**Service**: PostgreSQL
**RTO**: 15 minutes
**RPO**: 30 seconds
**Last Updated**: 2024-01-15
**Owner**: Database Team

---

## Prerequisites

- [ ] Access to Kubernetes cluster
- [ ] Database credentials
- [ ] PagerDuty incident created
- [ ] #incidents Slack channel open

---

## Detection

### Symptoms
- Primary database health check failing
- Connection timeout errors in application logs
- Prometheus alert: `PostgreSQLDown`
- Replication lag increasing rapidly

### Verification Commands

```bash
# Check primary status
kubectl get pod postgres-primary-0 -n llm-research-lab-data

# Check primary logs
kubectl logs postgres-primary-0 -n llm-research-lab-data --tail=50

# Test connectivity
pg_isready -h postgres-primary.llm-research-lab-data.svc.cluster.local
```

---

## Response Procedure

### Step 1: Confirm Primary Failure (2 minutes)

```bash
# Attempt connection
psql -h postgres-primary -U postgres -c "SELECT 1"

# Check pod status
kubectl describe pod postgres-primary-0 -n llm-research-lab-data

# Review recent events
kubectl get events -n llm-research-lab-data --sort-by='.lastTimestamp' | head -20
```

**Decision Point**: If primary is truly down, proceed. If recoverable, attempt restart first.

### Step 2: Enable Read-Only Mode (1 minute)

```bash
# Prevent new writes
kubectl patch configmap app-config -n llm-research-lab-prod \
  -p '{"data":{"READ_ONLY_MODE":"true"}}'

# Restart API to pick up config
kubectl rollout restart deployment llm-research-lab-api -n llm-research-lab-prod
```

### Step 3: Select Promotion Candidate (2 minutes)

```bash
# List replicas
kubectl get pods -l app=postgres,role=replica -n llm-research-lab-data

# Check LSN for each replica
for pod in $(kubectl get pods -l role=replica -n llm-research-lab-data -o name); do
  echo "=== $pod ==="
  kubectl exec -n llm-research-lab-data $pod -- \
    psql -U postgres -t -c "SELECT pg_last_wal_receive_lsn();"
done
```

Select replica with highest LSN.

### Step 4: Promote Replica (5 minutes)

```bash
REPLICA_POD="postgres-replica-0"  # Replace with selected pod

# Promote replica
kubectl exec -n llm-research-lab-data ${REPLICA_POD} -- \
  pg_ctl promote -D /var/lib/postgresql/data

# Wait for promotion
sleep 10

# Verify promotion
kubectl exec -n llm-research-lab-data ${REPLICA_POD} -- \
  psql -U postgres -c "SELECT pg_is_in_recovery();"  # Should return 'f'
```

### Step 5: Update Service (2 minutes)

```bash
# Get new primary IP
NEW_PRIMARY_IP=$(kubectl get pod ${REPLICA_POD} -n llm-research-lab-data -o jsonpath='{.status.podIP}')

# Update service to point to new primary
kubectl patch service postgres-primary -n llm-research-lab-data -p \
  "{\"spec\":{\"selector\":{\"statefulset.kubernetes.io/pod-name\":\"${REPLICA_POD}\"}}}"

# Verify service
kubectl get service postgres-primary -n llm-research-lab-data -o yaml
```

### Step 6: Verify Functionality (2 minutes)

```bash
# Test write operation
psql -h postgres-primary.llm-research-lab-data.svc.cluster.local \
  -U postgres -c "CREATE TABLE failover_test (id SERIAL PRIMARY KEY, ts TIMESTAMP DEFAULT NOW());"

# Verify
psql -h postgres-primary.llm-research-lab-data.svc.cluster.local \
  -U postgres -c "SELECT * FROM failover_test;"

# Cleanup
psql -h postgres-primary.llm-research-lab-data.svc.cluster.local \
  -U postgres -c "DROP TABLE failover_test;"
```

### Step 7: Disable Read-Only Mode (1 minute)

```bash
# Re-enable writes
kubectl patch configmap app-config -n llm-research-lab-prod \
  -p '{"data":{"READ_ONLY_MODE":"false"}}'

# Restart API
kubectl rollout restart deployment llm-research-lab-api -n llm-research-lab-prod
```

---

## Post-Failover

### Monitoring (30 minutes)

- [ ] Watch error rates
- [ ] Monitor connection pool
- [ ] Verify replication (if other replicas exist)
- [ ] Check application logs

### Cleanup

```bash
# Remove failed primary pod
kubectl delete pod postgres-primary-0 -n llm-research-lab-data

# Rebuild as new replica (if needed)
# Follow replica rebuild procedure
```

### Communication

```bash
# Send all-clear
curl -X POST https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK \
  -d '{"text":"✅ PostgreSQL failover completed. Service restored."}'
```

---

## Rollback

If failover fails:

```bash
# Revert service to point to original primary (if it recovers)
kubectl patch service postgres-primary -n llm-research-lab-data -p \
  "{\"spec\":{\"selector\":{\"statefulset.kubernetes.io/pod-name\":\"postgres-primary-0\"}}}"
```

---

## Contacts

| Role | Contact |
|------|---------|
| Database DRI | @db-oncall |
| Platform Lead | @platform-lead |
| Engineering Manager | @eng-manager |

---

## Related Resources

- [PostgreSQL Architecture](https://docs.company.com/architecture/postgres)
- [Grafana Dashboard](https://grafana.company.com/d/postgres)
- [Previous Incidents](https://docs.company.com/incidents/postgres)
```

---

## Appendix C: DR Testing Checklist

```markdown
# DR Testing Checklist

**Test Date**: __________
**Test Type**: __________
**Test Lead**: __________
**Participants**: __________

---

## Pre-Test Preparation

### Documentation
- [ ] Review DR plan and procedures
- [ ] Identify test objectives
- [ ] Define success criteria
- [ ] Prepare rollback procedures
- [ ] Schedule test window

### Communication
- [ ] Notify stakeholders
- [ ] Schedule team meeting
- [ ] Create test channel (#dr-test-YYYY-MM-DD)
- [ ] Prepare status updates

### Environment
- [ ] Verify backup status
- [ ] Check monitoring systems
- [ ] Prepare test environment (if applicable)
- [ ] Document current state

---

## During Test

### Execution
- [ ] Start recording (screen capture if needed)
- [ ] Document start time
- [ ] Follow runbook precisely
- [ ] Record all commands executed
- [ ] Note any deviations from plan
- [ ] Capture screenshots of key steps

### Monitoring
- [ ] Monitor system metrics
- [ ] Watch for unexpected issues
- [ ] Track recovery time
- [ ] Verify data integrity
- [ ] Test functionality

### Communication
- [ ] Provide regular updates
- [ ] Document issues encountered
- [ ] Note team collaboration effectiveness

---

## Post-Test

### Verification
- [ ] All services operational
- [ ] Data integrity confirmed
- [ ] Performance within SLA
- [ ] No data loss detected
- [ ] All functionality working

### Cleanup
- [ ] Restore original state (if needed)
- [ ] Clean up test artifacts
- [ ] Archive logs and recordings

### Documentation
- [ ] Calculate actual RTO achieved
- [ ] Calculate actual RPO achieved
- [ ] Document lessons learned
- [ ] Identify procedure improvements
- [ ] Update runbooks if needed

### Reporting
- [ ] Complete test report
- [ ] Share findings with team
- [ ] Update DR documentation
- [ ] Schedule follow-up actions

---

## Test Results

### Metrics

| Metric | Target | Actual | Pass/Fail |
|--------|--------|--------|-----------|
| RTO | ________ | ________ | ________ |
| RPO | ________ | ________ | ________ |
| Data Integrity | 100% | ________ | ________ |

### Success Criteria

- [ ] Met RTO target
- [ ] Met RPO target
- [ ] No data loss
- [ ] All services restored
- [ ] Runbook accuracy confirmed

### Issues Encountered

1. __________________________________________
2. __________________________________________
3. __________________________________________

### Action Items

| Action | Owner | Due Date | Status |
|--------|-------|----------|--------|
| ________ | ________ | ________ | ________ |
| ________ | ________ | ________ | ________ |

---

## Sign-Off

**Test Lead**: __________________ Date: __________
**Manager**: __________________ Date: __________

**Overall Result**: [ ] Pass [ ] Pass with Issues [ ] Fail

**Next Test Date**: __________
```

---

## Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2024-01-15 | Platform Team | Initial release |

**Review Schedule**: Quarterly
**Next Review**: 2024-04-15
**Owner**: Platform Team / SRE
**Approved By**: Engineering Leadership

---

**End of Document**
