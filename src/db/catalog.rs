use crate::data::loader::BinaryAssetMeta;
use crate::db::pool::DbPool;

/// Replace the current asset provenance rows atomically for one boot.
///
/// This makes the dashboard/audit layer able to answer exactly which binary
/// inputs were loaded, their byte sizes, and their content hashes.
pub async fn replace_asset_catalog(
    pool: &DbPool,
    assets: &[BinaryAssetMeta],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("DELETE FROM data_asset_catalog")
        .execute(&mut *tx)
        .await?;

    let now = chrono::Utc::now().timestamp();
    for asset in assets {
        sqlx::query(
            "INSERT INTO data_asset_catalog (source_file, source_sha256, byte_size, record_count, loaded_at, status) VALUES (?, ?, ?, ?, ?, 'loaded')",
        )
        .bind(&asset.source_file)
        .bind(&asset.sha256)
        .bind(asset.byte_size as i64)
        .bind(0i64)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}
