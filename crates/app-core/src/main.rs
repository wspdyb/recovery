use clap::Parser;
use core_io::{BlockReader, FileBlockReader};
use disk_layout::{detect_file_system, detect_layout, FileSystemType, PartitionLayout};
use fs_ntfs::{parse_mft_record, NtfsBootSector, DataInfo};
use result_store::{DbArtifact, ResultStore};
use carver::SignatureEngine;
use serde::Serialize;
use anyhow::{Result, Context};
use std::time::Instant;

const DEFAULT_DB_PATH: &str = "phoenixrecover_scan.db";
const DEFAULT_SIG_PATH: &str = "crates/carver/signatures.json";
const DEFAULT_MAX_RECORDS: u64 = 10000;
const DEFAULT_BATCH_SIZE: usize = 500;
const SECTOR_SIZE: u64 = 512;
const MFT_RECORD_SIZE: usize = 1024;

#[derive(Parser, Debug)]
#[command(author, version, about = "PhoenixRecover CLI Core Scanner")]
struct Args {
    /// Path to an image file or raw device
    #[arg(short, long)]
    source: String,

    /// SQLite database output path
    #[arg(long, default_value = DEFAULT_DB_PATH)]
    db: String,

    /// Path to signatures.json
    #[arg(long, default_value = DEFAULT_SIG_PATH)]
    signatures: String,

    /// Physical block size used by the reader
    #[arg(long, default_value_t = SECTOR_SIZE)]
    block_size: u64,

    /// Maximum MFT records to inspect
    #[arg(long, default_value_t = DEFAULT_MAX_RECORDS)]
    max_records: u64,

    /// Bulk insert batch size
    #[arg(long, default_value_t = DEFAULT_BATCH_SIZE)]
    batch_size: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug mode
    #[arg(long)]
    debug: bool,
}

#[derive(Debug, Serialize)]
struct ScanSummary {
    source: String,
    database: String,
    bytes_total: u64,
    partition_layout: String,
    ntfs_detected: bool,
    ntfs_partition_offset: u64,
    mft_offset: u64,
    scanned_records: u64,
    stored_artifacts: u64,
    deleted_artifacts: u64,
    duration_ms: u128,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let start_time = Instant::now();

    match run_scan(args, start_time).await {
        Ok(summary) => {
            println!("\n[SUCCESS] Scan Completed!");
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\n[ERROR] Scan failed: {}", e);
            // הקצאת קודי שגיאה לפי סוג הכישלון
            let error_msg = format!("{:?}", e);
            if error_msg.contains("No NTFS partition found") {
                std::process::exit(2);
            } else if error_msg.contains("Failed to read signatures") {
                std::process::exit(1);
            } else if error_msg.contains("DB error") {
                std::process::exit(4);
            } else {
                std::process::exit(3);
            }
        }
    }
}

async fn run_scan(args: Args, start_time: Instant) -> Result<ScanSummary> {
    // 1. Load signatures
    let sig_json = std::fs::read_to_string(&args.signatures)
        .context(format!("Failed to read signatures from {}", args.signatures))?;
    let sig_engine = SignatureEngine::load_from_json(&sig_json)
        .map_err(|e| anyhow::anyhow!(e))?;

    if args.verbose {
        println!("[SYSTEM] Loaded {} signatures", sig_engine.signatures.len());
    }

    // 2. Open source
    let mut reader = FileBlockReader::open(&args.source, args.block_size).await
        .context(format!("Failed to open source: {}", args.source))?;
    let total_size = reader.source_info().total_size;

    if args.verbose {
        println!("[SYSTEM] Target: {} ({} bytes)", args.source, total_size);
    }

    // 3. Detect Layout & NTFS
    let layout = detect_layout(&mut reader).await
        .context("Failed to detect partition layout")?;
    let ntfs_offset = find_ntfs_partition_offset(&mut reader, &layout).await?;

    if args.verbose {
        println!("[INFO] Detected Layout: {:?}, NTFS Offset: {}", layout_name(&layout), ntfs_offset);
    }

    // 4. Parse Boot Sector
    let boot_data = reader.read_at(ntfs_offset, SECTOR_SIZE as usize).await?;
    let boot = NtfsBootSector::parse(&boot_data)
        .map_err(|e| anyhow::anyhow!("Boot sector parse error: {:?}", e))?;
    let mft_base = ntfs_offset + boot.mft_offset();
    let bytes_per_cluster = boot.bytes_per_sector as u64 * boot.sectors_per_cluster as u64;

    if args.verbose {
        println!("[INFO] MFT Start: {}, Bytes/Cluster: {}", mft_base, bytes_per_cluster);
    }

    // 5. Initialize Store
    let mut store = ResultStore::open(&args.db)
        .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
    let session_id = store.create_session(&args.source)
        .map_err(|e| anyhow::anyhow!("Session creation error: {}", e))?;

    // 6. Scan MFT
    let mut scanned = 0;
    let mut stored = 0;
    let mut deleted_count = 0;
    let mut batch = Vec::with_capacity(args.batch_size);

    if args.verbose {
        println!("[ACTION] Starting MFT scan...");
    }

    for i in 0..args.max_records {
        let offset = mft_base + (i * MFT_RECORD_SIZE as u64);
        let record_bytes = match reader.read_at(offset, MFT_RECORD_SIZE).await {
            Ok(b) if b.len() == MFT_RECORD_SIZE => b,
            _ => break,
        };

        scanned += 1;
        if &record_bytes[0..4] != b"FILE" {
            continue;
        }

        let parsed = parse_mft_record(&record_bytes, i);
        let Some(filename) = parsed.filename else { continue; };

        // Identify MIME & Disk Offset
        let mut mime_type = "application/octet-stream".to_string();
        let mut real_size = 0;
        let mut is_resident = false;
        let mut disk_offset = 0;

        if let Some(info) = parsed.data_info {
            match info {
                DataInfo::Resident(data) => {
                    is_resident = true;
                    real_size = data.len() as u64;
                    let (_, _, mime, _) = sig_engine.identify_smart(&data);
                    mime_type = mime;
                    disk_offset = offset + 56; // Simplified: attribute usually starts around here
                }
                DataInfo::NonResident { real_size: size, data_runs, .. } => {
                    real_size = size;
                    if !data_runs.is_empty() {
                        let first_cluster_offset = (data_runs[0].cluster_offset as u64) * bytes_per_cluster + ntfs_offset;
                        disk_offset = first_cluster_offset;
                        if let Ok(sample) = reader.read_at(first_cluster_offset, 4096).await {
                            let (_, _, mime, _) = sig_engine.identify_smart(&sample);
                            mime_type = mime;
                        }
                    }
                }
            }
        }

        if parsed.is_deleted {
            deleted_count += 1;
        }

        batch.push(DbArtifact {
            session_id,
            mft_id: parsed.mft_id,
            parent_mft_id: parsed.parent_mft_id,
            filename,
            is_dir: parsed.is_directory,
            is_resident,
            real_size,
            mime_type,
            is_deleted: parsed.is_deleted,
            disk_offset,
        });

        if batch.len() >= args.batch_size {
            store.insert_artifacts_batch(&batch).map_err(|e| anyhow::anyhow!(e))?;
            stored += batch.len() as u64;
            batch.clear();
            if args.verbose && i % 1000 == 0 {
                println!("[PROGRESS] Scanned {} records, found {} artifacts", scanned, stored);
            }
        }
    }

    if !batch.is_empty() {
        store.insert_artifacts_batch(&batch).map_err(|e| anyhow::anyhow!(e))?;
        stored += batch.len() as u64;
    }

    Ok(ScanSummary {
        source: args.source,
        database: args.db,
        bytes_total: total_size,
        partition_layout: layout_name(&layout).to_string(),
        ntfs_detected: true,
        ntfs_partition_offset: ntfs_offset,
        mft_offset: mft_base,
        scanned_records: scanned,
        stored_artifacts: stored,
        deleted_artifacts: deleted_count,
        duration_ms: start_time.elapsed().as_millis(),
    })
}

async fn find_ntfs_partition_offset<R: BlockReader + Send>(
    reader: &mut R,
    layout: &PartitionLayout,
) -> Result<u64> {
    // Check if raw source is NTFS
    let lba0 = reader.read_at(0, SECTOR_SIZE as usize).await?;
    if matches!(detect_file_system(&lba0), FileSystemType::NTFS) {
        return Ok(0);
    }

    let entries = match layout {
        PartitionLayout::MBR(e) | PartitionLayout::GPT(e) => e,
        PartitionLayout::Unknown => return anyhow::bail!("No NTFS partition found (Unknown Layout)"),
    };

    for entry in entries {
        let offset = entry.start_lba * SECTOR_SIZE;
        let sector = reader.read_at(offset, SECTOR_SIZE as usize).await?;
        if matches!(detect_file_system(&sector), FileSystemType::NTFS) {
            return Ok(offset);
        }
    }

    anyhow::bail!("No NTFS partition found")
}

fn layout_name(layout: &PartitionLayout) -> &'static str {
    match layout {
        PartitionLayout::MBR(_) => "MBR",
        PartitionLayout::GPT(_) => "GPT",
        PartitionLayout::Unknown => "Unknown",
    }
}
