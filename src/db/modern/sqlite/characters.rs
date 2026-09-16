//! `characters` + `character_money` repository.

use crate::db::modern::model::Money;
use crate::db::modern::traits::{CharacterRepository, CharacterSeed, CharacterSummary, RepoResult};
use crate::db::pool::DbPool;
use sqlx::Row;

pub struct SqliteCharacterRepository<'a> {
    pub pool: &'a DbPool,
}

impl SqliteCharacterRepository<'_> {
    pub async fn create(
        &self,
        account_id: i64,
        name: &[u8],
        seed: &CharacterSeed,
    ) -> RepoResult<i64> {
        // Shared PK: characters.playerid = accounts.playerid (1:1, no account_id column)
        let mut tx = self.pool.write.begin().await?;
        sqlx::query(
            "INSERT INTO characters (playerid, name, level, gender, hair, element, mapid, mapx, mapy) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account_id).bind(name).bind(seed.level).bind(seed.sex).bind(seed.hair)
        .bind(seed.element).bind(seed.map_id).bind(seed.map_x).bind(seed.map_y)
        .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_money (playerid) VALUES (?)")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(account_id)
    }

    pub async fn find_id_by_name(&self, name: &[u8]) -> RepoResult<Option<i64>> {
        sqlx::query_scalar::<_, i64>("SELECT playerid FROM characters WHERE HEX(name) = HEX(?) LIMIT 1")
            .bind(name)
            .fetch_optional(&self.pool.read)
            .await
    }

    pub async fn delete(&self, character_id: i64) -> RepoResult<()> {
        let mut tx = self.pool.write.begin().await?;
        for table in [
            "character_money",
            "inventories",
            "character_pets",
            "character_skills",
            "character_hotkeys",
            "character_missions",
            "character_mission_flags",
            "character_bit_flags",
            "character_completed_events",
            "friends",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE playerid = ?"))
                .bind(character_id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM friends WHERE friendid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM mails WHERE receiverid = ? OR senderid = ?")
            .bind(character_id)
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM characters WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    }
}

/// The canonical money-row read; shared with the transaction layer so the
/// query shape lives in exactly one place.
pub(crate) async fn money_row<'e, E>(executor: E, character_id: i64) -> RepoResult<Money>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(
        "SELECT gold, bankgold, shoppoint FROM character_money WHERE playerid = ?",
    )
    .bind(character_id)
    .fetch_one(executor)
    .await?;
    Ok(Money {
        gold: row.get("gold"),
        bank_gold: row.get("bankgold"),
        shop_point: row.get("shoppoint"),
    })
}

impl CharacterRepository for SqliteCharacterRepository<'_> {
    async fn create(&self, account_id: i64, name: &[u8], seed: &CharacterSeed) -> RepoResult<i64> {
        let mut tx = self.pool.write.begin().await?;

        sqlx::query(
            "INSERT INTO characters \
             (playerid, name, level, gender, hair, element, mapid, mapx, mapy) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(name)
        .bind(seed.level)
        .bind(seed.sex)
        .bind(seed.hair)
        .bind(seed.element)
        .bind(seed.map_id)
        .bind(seed.map_x)
        .bind(seed.map_y)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO character_money (playerid) VALUES (?)")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(account_id)
    }

    async fn list_by_account(&self, account_id: i64) -> RepoResult<Vec<CharacterSummary>> {
        let rows = sqlx::query(
            "SELECT playerid AS id, name, level, gender AS sex, hair, element FROM characters \
             WHERE playerid = ? ORDER BY playerid",
        )
        .bind(account_id)
        .fetch_all(&self.pool.read)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| CharacterSummary {
                id: r.get::<i64, _>("id"),
                name: r.get::<Vec<u8>, _>("name"),
                level: r.get::<i64, _>("level"),
                sex: r.get::<i64, _>("sex"),
                hair: r.get::<i64, _>("hair"),
                element: r.get::<i64, _>("element"),
            })
            .collect())
    }

    async fn find_id_by_name(&self, name: &[u8]) -> RepoResult<Option<i64>> {
        let found = sqlx::query_scalar::<_, i64>(
            "SELECT playerid FROM characters WHERE HEX(name) = HEX(?) LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool.read)
        .await?;
        Ok(found)
    }

    async fn load_money(&self, character_id: i64) -> RepoResult<Money> {
        money_row(&self.pool.read, character_id).await
    }

    async fn delete(&self, character_id: i64) -> RepoResult<()> {
        let mut tx = self.pool.write.begin().await?;

        // Order does not matter without FKs, but every dependent table must be
        // covered — this list is the authoritative "what belongs to a
        // character" set from the 0001 schema.
        for table in [
            "character_money",
            "inventories",
            "character_pets",
            "character_skills",
            "character_hotkeys",
            "character_missions",
            "character_mission_flags",
            "character_bit_flags",
            "character_completed_events",
            "friends",
        ] {
            let sql = format!("DELETE FROM {table} WHERE playerid = ?");
            sqlx::query(&sql)
                .bind(character_id)
                .execute(&mut *tx)
                .await?;
        }
        // mails/friends key the counterpart side too.
        sqlx::query("DELETE FROM friends WHERE friendid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM mails WHERE receiverid = ? OR senderid = ?")
            .bind(character_id)
            .bind(character_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM characters WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}
