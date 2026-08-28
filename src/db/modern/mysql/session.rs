//! Session-level adapter for the modern 0001 schema.
//! Shared PK: `accounts.player_id = characters.character_id`.
//!
//! Network/account ids remain in `Session.id`; the normalized database primary
//! key is carried by `Session.db_character_id`.

use crate::db::modern::model::PetStorageType;
use crate::server::session::{InventoryItem, PetState, Session};
use sqlx::{MySqlPool, Row};

pub struct MySqlSessionRepository<'a> {
    pub pool: &'a MySqlPool,
}

impl MySqlSessionRepository<'_> {
    /// Load the character selected by its account id and hydrate the complete
    /// wire-visible session from 0001 tables. `account_id` is the shared PK (`character_id`).
    pub async fn load(&self, account_id: i64, session: &mut Session) -> Result<bool, sqlx::Error> {
        let Some(row) = sqlx::query(
            "SELECT character_id AS id, name, level, job, sex, hair, element, reborn, hp, hp_max, sp, sp_max,
                    stat_point, skill_point, int_attr, atk, def, hpx, spx, agi, map_id, map_x, map_y,
                    COALESCE(m.gold, 0) AS gold, COALESCE(m.bank_gold, 0) AS bank_gold,
                    COALESCE(m.shop_point, 0) AS shop_point,
                    newbie
             FROM characters c
             LEFT JOIN character_money m ON m.character_id = c.character_id
             WHERE c.character_id = ?
             LIMIT 1",
        )
        .bind(account_id)
        .fetch_optional(self.pool)
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
        session.gold = row.get::<i64, _>("gold") as u32;
        session.bank_gold = row.get::<i64, _>("bank_gold") as u32;
        session.shop_point = row.get::<i64, _>("shop_point") as u32;
        session.newbie = row.get::<i64, _>("newbie") as u32;

        session.homdo = self.load_items(session.db_character_id, 1).await?;
        session.tientrang = self.load_items(session.db_character_id, 2).await?;
        session.tuideo = self.load_items(session.db_character_id, 4).await?;
        session.trangbi = self.load_items(session.db_character_id, 8).await?;
        session.luulang = self.load_items(session.db_character_id, 16).await?;
        session.pets = self.load_pets(session.db_character_id).await?;
        session.skills = sqlx::query(
            "SELECT skill_id, level FROM character_skills WHERE character_id = ? ORDER BY skill_id",
        )
        .bind(session.db_character_id)
        .fetch_all(self.pool)
        .await?
        .into_iter()
        .map(|r| (clamp_u16(r.get("skill_id")), clamp_u8(r.get("level"))))
        .collect();
        session.hotkeys = [0; 11];
        for row in
            sqlx::query("SELECT slot, skill_id FROM character_hotkeys WHERE character_id = ?")
                .bind(session.db_character_id)
                .fetch_all(self.pool)
                .await?
        {
            let slot = row.get::<i64, _>("slot");
            if (1..=10).contains(&slot) {
                session.hotkeys[slot as usize] = clamp_u16(row.get("skill_id"));
            }
        }
        Ok(true)
    }

    async fn load_items(
        &self,
        character_id: i64,
        storage_type: u8,
    ) -> Result<Vec<InventoryItem>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT slot, item_id, quantity, damage
             FROM inventories WHERE character_id = ? AND storage_type = ? AND item_id > 0 ORDER BY slot",
        )
        .bind(character_id)
        .bind(storage_type)
        .fetch_all(self.pool)
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

    async fn load_pets(&self, character_id: i64) -> Result<Vec<PetState>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT storage_type, slot, pet_id, name, level, element, reborn, hp, hp_max, sp, sp_max,
                    int_attr, atk, def, hpx, spx, agi, fai,
                    thd, texp, skill_point, quest, skill1_id, skill1_level, skill2_id, skill2_level,
                    skill3_id, skill3_level, skill4_id, skill4_level
             FROM character_pets WHERE character_id = ? AND pet_id > 0 ORDER BY storage_type, slot",
        )
        .bind(character_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let storage: u8 = r.get("storage_type");
                let slot = clamp_u8(r.get("slot"));
                PetState {
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
                }
            })
            .collect())
    }

    /// Save all mutable Session state in one InnoDB transaction.
    pub async fn save(&self, session: &Session) -> Result<(), sqlx::Error> {
        let character_id = session.db_character_id;
        if character_id <= 0 {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE characters SET level=?, job=?, sex=?, hair=?, element=?, reborn=?, hp=?, hp_max=?, sp=?, sp_max=?,
             stat_point=?, skill_point=?, int_attr=?, atk=?, def=?, hpx=?, spx=?, agi=?, map_id=?, map_x=?, map_y=?,
             newbie=? WHERE character_id=?",
        )
        .bind(i64::from(session.level)).bind(i64::from(session.job)).bind(i64::from(session.sex)).bind(i64::from(session.hair))
        .bind(i64::from(session.thuoctinh)).bind(i64::from(session.reborn)).bind(i64::from(session.hp)).bind(i64::from(session.hp_max))
        .bind(i64::from(session.sp)).bind(i64::from(session.sp_max)).bind(i64::from(session.point)).bind(i64::from(session.skill_point))
        .bind(i64::from(session.int1)).bind(i64::from(session.atk)).bind(i64::from(session.def)).bind(i64::from(session.hpx))
        .bind(i64::from(session.spx)).bind(i64::from(session.agi)).bind(i64::from(session.map_id)).bind(i64::from(session.map_x))
        .bind(i64::from(session.map_y)).bind(i64::from(session.newbie))
        .bind(character_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO character_money (character_id, gold, bank_gold, shop_point) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE gold=VALUES(gold), bank_gold=VALUES(bank_gold), shop_point=VALUES(shop_point)")
            .bind(character_id).bind(i64::from(session.gold)).bind(i64::from(session.bank_gold)).bind(i64::from(session.shop_point)).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM inventories WHERE character_id = ?")
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
                sqlx::query("INSERT INTO inventories (character_id, storage_type, slot, item_id, quantity, damage) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(character_id).bind(storage).bind(i64::from(item.slot)).bind(i64::from(item.id)).bind(i64::from(item.count)).bind(i64::from(item.doben)).execute(&mut *tx).await?;
            }
        }
        sqlx::query("DELETE FROM character_pets WHERE character_id = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for pet in session.pets.iter().filter(|p| p.id > 0) {
            let (storage, slot) = if pet.stt <= 4 {
                (1u8, pet.stt)
            } else {
                (3u8, pet.stt.saturating_sub(4))
            };
            let [s1, s2, s3, s4] = pet.skills;
            sqlx::query("INSERT INTO character_pets (character_id, storage_type, slot, pet_id, name, level, element, reborn, hp, hp_max, sp, sp_max, int_attr, atk, def, hpx, spx, agi, fai, thd, texp, skill_point, quest, skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, skill4_id, skill4_level) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(character_id).bind(storage).bind(i64::from(slot)).bind(i64::from(pet.id)).bind(&pet.name).bind(i64::from(pet.level)).bind(i64::from(pet.thuoctinh)).bind(i64::from(pet.reborn)).bind(i64::from(pet.hp)).bind(i64::from(pet.hp_max)).bind(i64::from(pet.sp)).bind(i64::from(pet.sp_max)).bind(i64::from(pet.int1)).bind(i64::from(pet.atk)).bind(i64::from(pet.def)).bind(i64::from(pet.hpx)).bind(i64::from(pet.spx)).bind(i64::from(pet.agi)).bind(i64::from(pet.fai)).bind(i64::from(pet.thd)).bind(i64::from(pet.texp)).bind(i64::from(pet.skill_point)).bind(i64::from(pet.quest)).bind(i64::from(s1.0)).bind(i64::from(s1.1)).bind(i64::from(s2.0)).bind(i64::from(s2.1)).bind(i64::from(s3.0)).bind(i64::from(s3.1)).bind(i64::from(s4.0)).bind(i64::from(s4.1)).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM character_skills WHERE character_id = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for (id, level) in &session.skills {
            sqlx::query("INSERT INTO character_skills (character_id, skill_id, level, sp, save_flag) VALUES (?, ?, ?, 0, 0)")
                .bind(character_id).bind(i64::from(*id)).bind(i64::from(*level)).execute(&mut *tx).await?;
        }
        sqlx::query("DELETE FROM character_hotkeys WHERE character_id = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        for (slot, skill) in session.hotkeys.iter().enumerate().skip(1) {
            sqlx::query(
                "INSERT INTO character_hotkeys (character_id, slot, skill_id) VALUES (?, ?, ?)",
            )
            .bind(character_id)
            .bind(slot as i64)
            .bind(i64::from(*skill))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    pub async fn create_and_seed(
        &self,
        account_id: i64,
        name: &[u8],
        seed: &crate::db::modern::traits::CharacterSeed,
        session: &mut Session,
    ) -> Result<(), sqlx::Error> {
        let repos = crate::db::modern::mysql::MySqlRepositories::new(self.pool.clone());
        let id = repos.characters().create(account_id, name, seed).await?;
        session.db_character_id = id;
        self.save(session).await
    }
}

fn clamp_u8(v: i64) -> u8 {
    v.clamp(0, i64::from(u8::MAX)) as u8
}
fn clamp_u16(v: i64) -> u16 {
    v.clamp(0, i64::from(u16::MAX)) as u16
}
