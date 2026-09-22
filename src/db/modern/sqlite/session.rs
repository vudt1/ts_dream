//! Session-level adapter for the modern 0001 schema.
//! Shared PK: `accounts.player_id = characters.character_id`.
//!
//! Network/account ids remain in `Session.id`; the normalized database primary
//! key is carried by `Session.db_character_id`.

use crate::db::modern::model::PetStorageType;
use crate::db::pool::DbPool;
use crate::server::session::{InventoryItem, PetState, Session};
use sqlx::Row;

pub struct SqliteSessionRepository<'a> {
    pub pool: &'a DbPool,
}

impl SqliteSessionRepository<'_> {
    /// Load the character selected by its account id and hydrate the complete
    /// wire-visible session from 0001 tables. `account_id` is the shared PK (`character_id`).
    pub async fn load(&self, account_id: i64, session: &mut Session) -> Result<bool, sqlx::Error> {
        let Some(row) = sqlx::query(
            "SELECT c.playerid AS id, c.name, c.level, c.jobtype AS job, c.gender AS sex, c.hair, c.element,
                    c.rebornstage AS reborn, c.curhp AS hp, c.maxhp AS hp_max, c.cursp AS sp, c.maxsp AS sp_max,
                    c.freepoints AS stat_point, c.skillpoint AS skill_point, c.baseint AS int_attr,
                    c.baseatk AS atk, c.basedef AS def, c.basehpx AS hpx, basespx AS spx, c.baseagi AS agi,
                    c.mapid AS map_id, c.mapx AS map_x, c.mapy AS map_y,
                    c.pk, c.curexp AS texp,
                    COALESCE(m.gold, 0) AS gold, COALESCE(m.bankgold, 0) AS bank_gold,
                    COALESCE(m.shoppoint, 0) AS shop_point,
                    c.newbie
             FROM characters c
             LEFT JOIN character_money m ON m.playerid = c.playerid
             WHERE c.playerid = ?
             LIMIT 1",
        )
        .bind(account_id)
        .fetch_optional(&self.pool.read)
        .await?
        else {
            return Ok(false);
        };

        session.db_character_id = row.get("id");
        session.name = row.get("name");
        session.level = clamp_u8(row.get("level"));
        session.job = clamp_u8(row.get("job"));
        session.sex = clamp_u8(row.get("sex"));
        session.hair = clamp_u16(row.get("hair"));
        session.thuoctinh = clamp_u8(row.get("element"));
        session.reborn = clamp_u8(row.get("reborn"));
        session.hp = clamp_u16(row.get("hp"));
        session.hp_max = clamp_u16(row.get("hp_max"));
        session.sp = clamp_u16(row.get("sp"));
        session.sp_max = clamp_u16(row.get("sp_max"));
        session.point = clamp_u16(row.get("stat_point"));
        session.skill_point = clamp_u16(row.get("skill_point"));
        session.int1 = clamp_u16(row.get("int_attr"));
        session.atk = clamp_u16(row.get("atk"));
        session.def = clamp_u16(row.get("def"));
        session.hpx = clamp_u16(row.get("hpx"));
        session.spx = clamp_u16(row.get("spx"));
        session.agi = clamp_u16(row.get("agi"));
        session.map_id = clamp_u16(row.get("map_id"));
        session.map_x = clamp_u16(row.get("map_x"));
        session.map_y = clamp_u16(row.get("map_y"));
        session.pk = clamp_u8(row.get("pk"));
        session.texp = row.get::<i64, _>("texp") as u32;
        session.gold = row.get::<i64, _>("gold") as u32;
        session.bank_gold = row.get::<i64, _>("bank_gold") as u32;
        session.shop_point = row.get::<i64, _>("shop_point") as u32;
        session.newbie = row.get::<i64, _>("newbie") as u32;

        session.homdo = self.load_items(session.db_character_id, 1).await?;
        session.tientrang = self.load_items(session.db_character_id, 2).await?;
        session.tuideo = self.load_items(session.db_character_id, 4).await?;
        session.trangbi = self.load_items(session.db_character_id, 8).await?;
        session.luulang = self.load_items(session.db_character_id, 16).await?;
        session.active_pet_stt = 0;
        session.pets = self
            .load_pets(session.db_character_id, &mut session.active_pet_stt)
            .await?;
        session.skills = sqlx::query(
            "SELECT skillid AS skill_id, level FROM character_skills WHERE playerid = ? ORDER BY skillid",
        )
        .bind(session.db_character_id)
        .fetch_all(&self.pool.read)
        .await?
        .into_iter()
        .map(|r| (clamp_u16(r.get("skill_id")), clamp_u8(r.get("level"))))
        .collect();
        session.hotkeys = [0; 11];
        for row in
            sqlx::query("SELECT slot, skillid AS skill_id FROM character_hotkeys WHERE playerid = ?")
                .bind(session.db_character_id)
                .fetch_all(&self.pool.read)
                .await?
        {
            let slot = row.get::<i64, _>("slot");
            if (1..=10).contains(&slot) {
                session.hotkeys[slot as usize] = clamp_u16(row.get("skill_id"));
            }
        }
        // CP6: quest-log / Eve state is best-effort — a database whose
        // `character_quest_*` tables are missing (e.g. a pre-CP6 baseline)
        // logs at debug and signs in with empty quest state instead of
        // failing. `0001_init.sql` is the single baseline containing them.
        if let Err(err) = self.load_eve_state(session).await {
            tracing::debug!(
                player = session.db_character_id,
                %err,
                "Eve/quest state load skipped (character_quest_* tables missing?)"
            );
        }
        Ok(true)
    }

    /// CP6: hydrate the Eve/quest-log collections (`quest_tasks`, `quest_dont`,
    /// `quest_items`, `completed_eve_counts`) from the quest tables.
    async fn load_eve_state(&self, session: &mut Session) -> Result<(), sqlx::Error> {
        let id = session.db_character_id;
        if id <= 0 {
            return Ok(());
        }
        for row in sqlx::query(
            "SELECT questid, slot, markstep FROM character_quest_tasks WHERE playerid = ?",
        )
        .bind(id)
        .fetch_all(&self.pool.read)
        .await?
        {
            if let Ok(quest_id) = u16::try_from(row.get::<i64, _>("questid")) {
                session.quest_tasks.insert(
                    quest_id,
                    (clamp_u8(row.get("slot")), clamp_u8(row.get("markstep"))),
                );
            }
        }
        for row in sqlx::query("SELECT mark FROM character_quest_dont WHERE playerid = ?")
            .bind(id)
            .fetch_all(&self.pool.read)
            .await?
        {
            if let Ok(mark) = u16::try_from(row.get::<i64, _>("mark")) {
                session.quest_dont.insert(mark);
            }
        }
        session.quest_items = sqlx::query(
            "SELECT slot, itemid AS item_id, count FROM character_quest_items \
             WHERE playerid = ? ORDER BY slot",
        )
        .bind(id)
        .fetch_all(&self.pool.read)
        .await?
        .into_iter()
        .map(|r| InventoryItem {
            slot: clamp_u8(r.get("slot")),
            id: clamp_u16(r.get("item_id")),
            count: clamp_u8(r.get("count")),
            ..Default::default()
        })
        .collect();
        for row in sqlx::query(
            "SELECT eventid, completioncount FROM character_completed_events WHERE playerid = ?",
        )
        .bind(id)
        .fetch_all(&self.pool.read)
        .await?
        {
            session.completed_eve_counts.insert(
                row.get::<i64, _>("eventid") as i32,
                row.get::<i64, _>("completioncount") as i32,
            );
        }
        Ok(())
    }

    async fn load_items(
        &self,
        character_id: i64,
        storage_type: u8,
    ) -> Result<Vec<InventoryItem>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT slot, itemid AS item_id, quantity, damage
             FROM inventories WHERE playerid = ? AND storagetype = ? AND itemid > 0 ORDER BY slot",
        )
        .bind(character_id)
        .bind(storage_type)
        .fetch_all(&self.pool.read)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| InventoryItem {
                slot: clamp_u8(r.get("slot")),
                id: clamp_u16(r.get("item_id")),
                count: clamp_u8(r.get("quantity")),
                doben: clamp_u8(r.get("damage")),
                ..Default::default()
            })
            .collect())
    }

    async fn load_pets(
        &self,
        character_id: i64,
        active_pet_stt: &mut u8,
    ) -> Result<Vec<PetState>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT storagetype AS storage_type, slot, petid AS pet_id, name, level, element,
                    rebornstage AS reborn, curhp AS hp, maxhp AS hp_max, cursp AS sp, maxsp AS sp_max,
                    baseint AS int_attr, baseatk AS atk, basedef AS def, basehpx AS hpx, basespx AS spx,
                    baseagi AS agi, fai,
                    thd, texp, skillpoint AS skill_point, quest, isactive,
                    skill1_id, skill1_level, skill2_id, skill2_level,
                    skill3_id, skill3_level, skill4_id, skill4_level
              FROM character_pets WHERE playerid = ? AND petid > 0 ORDER BY storagetype, slot",
        )
        .bind(character_id)
        .fetch_all(&self.pool.read)
        .await?;
        let mut pets = Vec::with_capacity(rows.len());
        for r in rows {
            let storage: u8 = r.get("storage_type");
            let slot = clamp_u8(r.get("slot"));
            let is_active: i64 = r.try_get("isactive").unwrap_or(0);
            if storage == 1 && is_active == 1 {
                *active_pet_stt = slot;
            }
            pets.push(PetState {
                stt: if storage == PetStorageType::Carried as u8 {
                    slot
                } else {
                    slot.saturating_add(4)
                },
                id: clamp_u16(r.get("pet_id")),
                name: r.get("name"),
                level: clamp_u8(r.get("level")),
                thuoctinh: clamp_u8(r.get("element")),
                reborn: clamp_u8(r.get("reborn")),
                hp: clamp_u16(r.get("hp")),
                hp_max: clamp_u16(r.get("hp_max")),
                sp: clamp_u16(r.get("sp")),
                sp_max: clamp_u16(r.get("sp_max")),
                int1: clamp_u16(r.get("int_attr")),
                atk: clamp_u16(r.get("atk")),
                def: clamp_u16(r.get("def")),
                hpx: clamp_u16(r.get("hpx")),
                spx: clamp_u16(r.get("spx")),
                agi: clamp_u16(r.get("agi")),
                fai: clamp_u16(r.get("fai")),
                thd: clamp_u16(r.get("thd")),
                texp: r.get::<i64, _>("texp") as u32,
                skill_point: clamp_u16(r.get("skill_point")),
                quest: clamp_u8(r.get("quest")),
                skills: [
                    (
                        clamp_u16(r.get("skill1_id")),
                        clamp_u8(r.get("skill1_level")),
                    ),
                    (
                        clamp_u16(r.get("skill2_id")),
                        clamp_u8(r.get("skill2_level")),
                    ),
                    (
                        clamp_u16(r.get("skill3_id")),
                        clamp_u8(r.get("skill3_level")),
                    ),
                    (
                        clamp_u16(r.get("skill4_id")),
                        clamp_u8(r.get("skill4_level")),
                    ),
                ],
                ..Default::default()
            });
        }
        Ok(pets)
    }

    /// Save all mutable Session state in one InnoDB transaction.
    pub async fn save(&self, session: &Session) -> Result<(), sqlx::Error> {
        let character_id = session.db_character_id;
        if character_id <= 0 {
            return Ok(());
        }
        let mut tx = self.pool.write.begin().await?;
        sqlx::query(
            "UPDATE characters SET level=?, jobtype=?, gender=?, hair=?, element=?, rebornstage=?, curhp=?, maxhp=?, cursp=?, maxsp=?,
             freepoints=?, skillpoint=?, baseint=?, baseatk=?, basedef=?, basehpx=?, basespx=?, baseagi=?, mapid=?, mapx=?, mapy=?,
             pk=?, curexp=?, newbie=? WHERE playerid=?",
        )
        .bind(i64::from(session.level)).bind(i64::from(session.job)).bind(i64::from(session.sex)).bind(i64::from(session.hair))
        .bind(i64::from(session.thuoctinh)).bind(i64::from(session.reborn)).bind(i64::from(session.hp)).bind(i64::from(session.hp_max))
        .bind(i64::from(session.sp)).bind(i64::from(session.sp_max)).bind(i64::from(session.point)).bind(i64::from(session.skill_point))
        .bind(i64::from(session.int1)).bind(i64::from(session.atk)).bind(i64::from(session.def)).bind(i64::from(session.hpx))
        .bind(i64::from(session.spx)).bind(i64::from(session.agi)).bind(i64::from(session.map_id)).bind(i64::from(session.map_x))
        .bind(i64::from(session.map_y)).bind(i64::from(session.pk)).bind(i64::from(session.texp)).bind(i64::from(session.newbie))
        .bind(character_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_money (playerid, gold, bankgold, shoppoint) VALUES (?, ?, ?, ?) ON CONFLICT(playerid) DO UPDATE SET gold=excluded.gold, bankgold=excluded.bankgold, shoppoint=excluded.shoppoint")
            .bind(character_id).bind(i64::from(session.gold)).bind(i64::from(session.bank_gold)).bind(i64::from(session.shop_point)).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM inventories WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for (storage, items) in [
            (1u8, &session.homdo),
            (2, &session.tientrang),
            (4, &session.tuideo),
            (8, &session.trangbi),
            (16, &session.luulang),
        ] {
            for item in items.iter().filter(|i| i.id > 0) {
                sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(character_id).bind(storage).bind(i64::from(item.slot)).bind(i64::from(item.id)).bind(i64::from(item.count)).bind(i64::from(item.doben)).execute(&mut *tx).await?;
            }
        }
        sqlx::query("DELETE FROM character_pets WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for pet in session.pets.iter().filter(|p| p.id > 0) {
            let (storage, slot) = if pet.stt <= 4 {
                (1u8, pet.stt)
            } else {
                (3u8, pet.stt.saturating_sub(4))
            };
            let is_active: i64 = if storage == 1 && pet.stt == session.active_pet_stt {
                1
            } else {
                0
            };
            let [s1, s2, s3, s4] = pet.skills;
            sqlx::query("INSERT INTO character_pets (playerid, storagetype, slot, petid, name, level, element, rebornstage, curhp, maxhp, cursp, maxsp, baseint, baseatk, basedef, basehpx, basespx, baseagi, fai, thd, texp, skillpoint, quest, skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, skill4_id, skill4_level, isactive) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(character_id).bind(storage).bind(i64::from(slot)).bind(i64::from(pet.id)).bind(&pet.name).bind(i64::from(pet.level)).bind(i64::from(pet.thuoctinh)).bind(i64::from(pet.reborn)).bind(i64::from(pet.hp)).bind(i64::from(pet.hp_max)).bind(i64::from(pet.sp)).bind(i64::from(pet.sp_max)).bind(i64::from(pet.int1)).bind(i64::from(pet.atk)).bind(i64::from(pet.def)).bind(i64::from(pet.hpx)).bind(i64::from(pet.spx)).bind(i64::from(pet.agi)).bind(i64::from(pet.fai)).bind(i64::from(pet.thd)).bind(i64::from(pet.texp)).bind(i64::from(pet.skill_point)).bind(i64::from(pet.quest)).bind(i64::from(s1.0)).bind(i64::from(s1.1)).bind(i64::from(s2.0)).bind(i64::from(s2.1)).bind(i64::from(s3.0)).bind(i64::from(s3.1)).bind(i64::from(s4.0)).bind(i64::from(s4.1)).bind(is_active).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM character_skills WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for (id, level) in &session.skills {
            sqlx::query("INSERT INTO character_skills (playerid, skillid, level, sp, saveflag) VALUES (?, ?, ?, 0, 0)")
                .bind(character_id).bind(i64::from(*id)).bind(i64::from(*level)).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM character_hotkeys WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for (slot, skill) in session.hotkeys.iter().enumerate().skip(1) {
            sqlx::query(
                "INSERT INTO character_hotkeys (playerid, slot, skillid) VALUES (?, ?, ?)",
            )
            .bind(character_id)
            .bind(slot as i64)
            .bind(i64::from(*skill))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        // CP6: quest/Eve state commits in its **own** transaction after the
        // main one — a database whose `character_quest_*` tables are missing
        // (e.g. a pre-CP6 baseline) then logs and skips instead of failing a
        // save that already succeeded.
        if let Err(err) = self.save_eve_state(session).await {
            tracing::debug!(
                player = session.id,
                %err,
                "Eve/quest state save skipped (character_quest_* tables missing?)"
            );
        }
        Ok(())
    }

    /// CP6: persist the Eve/quest-log collections (`quest_tasks`, `quest_dont`,
    /// `quest_items`, `completed_eve_counts`) in one transaction. Called by
    /// [`Self::save`] after the main transaction committed.
    ///
    /// Rows are written in sorted order so repeated saves of unchanged state
    /// produce identical table contents (the fingerprint ledger keys off the
    /// session, but stable rows keep diffs and tests quiet).
    pub async fn save_eve_state(&self, session: &Session) -> Result<(), sqlx::Error> {
        let character_id = session.db_character_id;
        if character_id <= 0 {
            return Ok(());
        }
        let mut tx = self.pool.write.begin().await?;

        sqlx::query("DELETE FROM character_quest_tasks WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        let mut tasks: Vec<(u16, (u8, u8))> = session
            .quest_tasks
            .iter()
            .map(|(&quest_id, &row)| (quest_id, row))
            .collect();
        tasks.sort_unstable();
        for (quest_id, (slot, step)) in tasks {
            sqlx::query(
                "INSERT INTO character_quest_tasks (playerid, questid, slot, markstep) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(character_id)
            .bind(i64::from(quest_id))
            .bind(i64::from(slot))
            .bind(i64::from(step))
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("DELETE FROM character_quest_dont WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        let mut marks: Vec<u16> = session.quest_dont.iter().copied().collect();
        marks.sort_unstable();
        for mark in marks {
            sqlx::query("INSERT INTO character_quest_dont (playerid, mark) VALUES (?, ?)")
                .bind(character_id)
                .bind(i64::from(mark))
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("DELETE FROM character_quest_items WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        let mut quest_items = session.quest_items.clone();
        quest_items.sort_by_key(|item| item.slot);
        for item in quest_items.iter().filter(|i| i.id > 0 && i.count > 0) {
            sqlx::query(
                "INSERT INTO character_quest_items (playerid, itemid, slot, count) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(character_id)
            .bind(i64::from(item.id))
            .bind(i64::from(item.slot))
            .bind(i64::from(item.count))
            .execute(&mut *tx)
            .await?;
        }

        let now = chrono::Utc::now().timestamp();
        let mut counts: Vec<(i32, i32)> = session
            .completed_eve_counts
            .iter()
            .map(|(&eve_no, &count)| (eve_no, count))
            .collect();
        counts.sort_unstable();
        for (event_id, count) in counts {
            // First completion stamps `completedat`; later ones only bump the
            // counter, keeping the original timestamp meaningful.
            sqlx::query(
                "INSERT INTO character_completed_events \
                 (playerid, eventid, completedat, completioncount) VALUES (?, ?, ?, ?) \
                 ON CONFLICT(playerid, eventid) DO UPDATE SET \
                 completioncount = excluded.completioncount",
            )
            .bind(character_id)
            .bind(i64::from(event_id))
            .bind(now)
            .bind(i64::from(count))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await
    }

    /// Create a character and seed its starter state in **one atomic
    /// transaction** (E3): the full `characters` row (stats from the aLogin
    /// packet; `hair` covers style/hair/face/color per the 0001 schema), the
    /// `character_money` ledger, the `accounts` password overwrite (Bear
    /// `initChar` parity: non-empty `pass1`/`pass2` from the packet replace
    /// the dashboard passwords), and the starter `inventories` rows — all
    /// commit or roll back together. A crash between the old two-transaction
    /// steps can no longer leave a half-created row. UNIQUE(name) violation
    /// rolls back the whole batch so the handler can toast `09 03 01`.
    ///
    /// `session` must already reflect the new character (via
    /// `apply_to_session`); on success `session.db_character_id` is set.
    /// The 8-byte appearance color (`session.color`, Bear `color1`/`color2`)
    /// travels in `session` RAM for `player_appear` and is covered by the
    /// `hair` column — no separate DB columns.
    pub async fn create_and_seed(
        &self,
        account_id: i64,
        name: &[u8],
        seed: &crate::db::modern::traits::CharacterSeed,
        session: &mut Session,
        pass1: &[u8],
        pass2: &[u8],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.write.begin().await?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        // 1. Full character row (mirrors `save()` columns so the row is never
        //    half-initialised, even if the process dies right after commit).
        sqlx::query(
            "INSERT INTO characters (playerid, name, level, gender, hair, element,
             rebornstage, curhp, maxhp, cursp, maxsp, curexp, nextexp, freepoints, skillpoint,
             baseint, baseatk, basedef, basehpx, basespx, baseagi,
             fai, pk, mapid, mapx, mapy, jobtype, newbie)
             VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, 0, 0, 0, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?, ?, 0, 0)",
        )
        .bind(account_id).bind(name)
        .bind(seed.level).bind(seed.sex).bind(seed.hair)
        .bind(seed.element)
        .bind(i64::from(session.hp)).bind(i64::from(session.hp_max))
        .bind(i64::from(session.sp)).bind(i64::from(session.sp_max))
        .bind(i64::from(session.texp))
        .bind(i64::from(session.int1)).bind(i64::from(session.atk)).bind(i64::from(session.def))
        .bind(i64::from(session.hpx)).bind(i64::from(session.spx)).bind(i64::from(session.agi))
        .bind(i64::from(session.map_id)).bind(i64::from(session.map_x)).bind(i64::from(session.map_y))
        .execute(&mut *tx).await?;
        // 2. Money ledger.
        sqlx::query("INSERT INTO character_money (playerid, gold, bankgold, shoppoint) VALUES (?, ?, ?, ?)")
            .bind(account_id)
            .bind(i64::from(session.gold))
            .bind(i64::from(session.bank_gold))
            .bind(i64::from(session.shop_point))
            .execute(&mut *tx).await?;
        // 3. Password overwrite (Bear parity). Empty passwords are skipped so a
        //    password-less packet never wipes the dashboard credentials.
        //    BLOB-bound: VISCII-safe, no UTF-8 transcode.
        if !pass1.is_empty() {
            sqlx::query("UPDATE accounts SET pass1 = ?, updatedat = ? WHERE playerid = ?")
                .bind(pass1)
                .bind(now_ms)
                .bind(account_id)
                .execute(&mut *tx)
                .await?;
        }
        if !pass2.is_empty() {
            sqlx::query("UPDATE accounts SET pass2 = ?, updatedat = ? WHERE playerid = ?")
                .bind(pass2)
                .bind(now_ms)
                .bind(account_id)
                .execute(&mut *tx)
                .await?;
        }
        // 4. Starter inventories (type 1 Homdo/bag: 32012×4; type 8 Trangbi:
        //    19737) — taken from `session`, committed in the same Tx.
        for (storage, items) in [
            (1u8, &session.homdo),
            (2, &session.tientrang),
            (4, &session.tuideo),
            (8, &session.trangbi),
            (16, &session.luulang),
        ] {
            for item in items.iter().filter(|i| i.id > 0) {
                sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(account_id).bind(storage).bind(i64::from(item.slot)).bind(i64::from(item.id)).bind(i64::from(item.count)).bind(i64::from(item.doben)).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        session.db_character_id = account_id;
        Ok(())
    }
}

fn clamp_u8(v: i64) -> u8 {
    v.clamp(0, i64::from(u8::MAX)) as u8
}
fn clamp_u16(v: i64) -> u16 {
    v.clamp(0, i64::from(u16::MAX)) as u16
}
