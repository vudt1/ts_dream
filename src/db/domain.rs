use crate::db::pool::DbPool;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct GuildRow {
    pub guild_id: i64,
    pub name: String,
    pub leader_id: i64,
    pub level: i64,
    pub experience: i64,
    pub treasury: i64,
    pub status: String,
    pub member_count: i64,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorldBossRow {
    pub boss_id: i64,
    pub npc_id: i64,
    pub scene_id: i64,
    pub state: String,
    pub hp: i64,
    pub hp_max: i64,
    pub starts_at: i64,
    pub ends_at: i64,
    pub participant_count: i64,
}

pub async fn list_guilds(pool: &DbPool) -> Result<Vec<GuildRow>, sqlx::Error> {
    sqlx::query_as::<_, GuildRow>(
        "SELECT g.guild_id, g.name, g.leader_id, g.level, g.experience, g.treasury, g.status,
                (SELECT COUNT(*) FROM guild_members gm WHERE gm.guild_id = g.guild_id) AS member_count
         FROM guilds g ORDER BY g.guild_id DESC",
    )
    .fetch_all(&pool.read)
    .await
}

pub async fn create_guild(pool: &DbPool, name: &str, leader_id: i64) -> Result<i64, sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    let res = sqlx::query("INSERT INTO guilds (name, leader_id) VALUES (?, ?)")
        .bind(name)
        .bind(leader_id)
        .execute(&mut *tx)
        .await?;
    let guild_id = res.last_insert_rowid();

    sqlx::query("INSERT INTO guild_members (guild_id, char_id, rank) VALUES (?, ?, 2)")
        .bind(guild_id)
        .bind(leader_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE characters SET guild_id = ? WHERE playerid = ?")
        .bind(guild_id)
        .bind(leader_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(guild_id)
}

pub async fn add_guild_member(pool: &DbPool, guild_id: i64, char_id: i64, rank: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("INSERT OR REPLACE INTO guild_members (guild_id, char_id, rank) VALUES (?, ?, ?)")
        .bind(guild_id)
        .bind(char_id)
        .bind(rank)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE characters SET guild_id = ? WHERE playerid = ?")
        .bind(guild_id)
        .bind(char_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn remove_guild_member(pool: &DbPool, char_id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("DELETE FROM guild_members WHERE char_id = ?")
        .bind(char_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE characters SET guild_id = 0 WHERE playerid = ?")
        .bind(char_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn transfer_guild_leadership(pool: &DbPool, guild_id: i64, new_leader_id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("UPDATE guild_members SET rank = 0 WHERE guild_id = ? AND rank = 2")
        .bind(guild_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE guilds SET leader_id = ? WHERE guild_id = ?")
        .bind(new_leader_id)
        .bind(guild_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE guild_members SET rank = 2 WHERE guild_id = ? AND char_id = ?")
        .bind(guild_id)
        .bind(new_leader_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn update_guild_badge(pool: &DbPool, guild_id: i64, badge: &[u8]) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("UPDATE guilds SET badge = ? WHERE guild_id = ?")
        .bind(badge)
        .bind(guild_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_city_owner(pool: &DbPool, city_id: i64, guild_id: Option<i64>) -> Result<(), sqlx::Error> {
    let mut tx = pool.write.begin().await?;
    sqlx::query("UPDATE guild_cities SET guild_id = ?, updated_at = strftime('%s', 'now') WHERE city_id = ?")
        .bind(guild_id)
        .bind(city_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}


pub async fn list_world_bosses(pool: &DbPool) -> Result<Vec<WorldBossRow>, sqlx::Error> {
    sqlx::query_as::<_, WorldBossRow>(
        "SELECT b.boss_id, b.npc_id, b.scene_id, b.state, b.hp, b.hp_max, b.starts_at, b.ends_at,
                (SELECT COUNT(*) FROM world_boss_participants p WHERE p.boss_id = b.boss_id) AS participant_count
         FROM world_bosses b ORDER BY b.boss_id DESC",
    )
    .fetch_all(&pool.read)
    .await
}

pub async fn write_audit(
    pool: &DbPool,
    admin_id: Option<i64>,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<i64>,
    details: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO admin_audit_log (admin_id, action, target_type, target_id, details, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(admin_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(details)
    .bind(now)
    .execute(&pool.write)
    .await
    .map(|_| ())
}
