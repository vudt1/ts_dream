//! `character_pets` repository — the four general storages.

use crate::db::modern::model::{PetRecord, PetSkill, PetStorageType};
use crate::db::modern::traits::{PetRepository, RepoResult};
use sqlx::{MySqlPool, Row};

pub struct MySqlPetRepository<'a> {
    pub pool: &'a MySqlPool,
}

const SELECT_COLUMNS: &str = "slot, pet_id, name, level, element, reborn, hp, hp_max, sp, \
     sp_max, int_attr, atk, def, hpx, spx, agi, fai, texp, skill_point, thd, \
     skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, \
     skill4_id, skill4_level, quest";

impl PetRepository for MySqlPetRepository<'_> {
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
    ) -> RepoResult<Vec<PetRecord>> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM character_pets \
             WHERE character_id = ? AND storage_type = ? AND pet_id > 0 ORDER BY slot"
        );
        let rows = sqlx::query(&sql)
            .bind(character_id)
            .bind(storage_type.value())
            .fetch_all(self.pool)
            .await?;
        Ok(rows.iter().filter_map(row_to_pet).collect())
    }

    async fn save_pet(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
        pet: &PetRecord,
    ) -> RepoResult<()> {
        let [s1, s2, s3, s4] = pet.skills;
        sqlx::query(
            "INSERT INTO character_pets \
             (character_id, storage_type, slot, pet_id, name, level, element, reborn, hp, hp_max, \
              sp, sp_max, int_attr, atk, def, hpx, spx, agi, fai, texp, skill_point, thd, \
              skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, \
              skill4_id, skill4_level, quest) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                     ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE pet_id = VALUES(pet_id), name = VALUES(name), \
             level = VALUES(level), element = VALUES(element), reborn = VALUES(reborn), \
             hp = VALUES(hp), hp_max = VALUES(hp_max), sp = VALUES(sp), sp_max = VALUES(sp_max), \
             int_attr = VALUES(int_attr), atk = VALUES(atk), def = VALUES(def), \
             hpx = VALUES(hpx), spx = VALUES(spx), agi = VALUES(agi), fai = VALUES(fai), \
             texp = VALUES(texp), skill_point = VALUES(skill_point), thd = VALUES(thd), \
             skill1_id = VALUES(skill1_id), skill1_level = VALUES(skill1_level), \
             skill2_id = VALUES(skill2_id), skill2_level = VALUES(skill2_level), \
             skill3_id = VALUES(skill3_id), skill3_level = VALUES(skill3_level), \
             skill4_id = VALUES(skill4_id), skill4_level = VALUES(skill4_level), \
             quest = VALUES(quest)",
        )
        .bind(character_id)
        .bind(storage_type.value())
        .bind(pet.slot)
        .bind(pet.pet_id)
        .bind(&pet.name[..])
        .bind(pet.level)
        .bind(pet.element)
        .bind(pet.reborn)
        .bind(pet.hp)
        .bind(pet.hp_max)
        .bind(pet.sp)
        .bind(pet.sp_max)
        .bind(pet.int_attr)
        .bind(pet.atk)
        .bind(pet.def)
        .bind(pet.hpx)
        .bind(pet.spx)
        .bind(pet.agi)
        .bind(pet.fai)
        .bind(pet.texp)
        .bind(pet.skill_point)
        .bind(pet.thd)
        .bind(s1.id)
        .bind(s1.level)
        .bind(s2.id)
        .bind(s2.level)
        .bind(s3.id)
        .bind(s3.level)
        .bind(s4.id)
        .bind(s4.level)
        .bind(pet.quest)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    async fn delete_pet(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
        slot: u16,
    ) -> RepoResult<()> {
        sqlx::query("DELETE FROM character_pets WHERE character_id = ? AND storage_type = ? AND slot = ?")
            .bind(character_id)
            .bind(storage_type.value())
            .bind(slot)
            .execute(self.pool)
            .await?;
        Ok(())
    }
}

fn row_to_pet(r: &sqlx::mysql::MySqlRow) -> Option<PetRecord> {
    let skill = |id_col: &str, lv_col: &str| -> Option<PetSkill> {
        Some(PetSkill {
            id: r.try_get(id_col).ok()?,
            level: r.try_get(lv_col).ok()?,
        })
    };
    Some(PetRecord {
        slot: r.try_get("slot").ok()?,
        pet_id: r.try_get("pet_id").ok()?,
        name: r.try_get::<Vec<u8>, _>("name").ok()?,
        level: r.try_get("level").ok()?,
        element: r.try_get("element").ok()?,
        reborn: r.try_get("reborn").ok()?,
        hp: r.try_get("hp").ok()?,
        hp_max: r.try_get("hp_max").ok()?,
        sp: r.try_get("sp").ok()?,
        sp_max: r.try_get("sp_max").ok()?,
        int_attr: r.try_get("int_attr").ok()?,
        atk: r.try_get("atk").ok()?,
        def: r.try_get("def").ok()?,
        hpx: r.try_get("hpx").ok()?,
        spx: r.try_get("spx").ok()?,
        agi: r.try_get("agi").ok()?,
        fai: r.try_get("fai").ok()?,
        texp: r.try_get("texp").ok()?,
        skill_point: r.try_get("skill_point").ok()?,
        thd: r.try_get("thd").ok()?,
        skills: [
            skill("skill1_id", "skill1_level")?,
            skill("skill2_id", "skill2_level")?,
            skill("skill3_id", "skill3_level")?,
            skill("skill4_id", "skill4_level")?,
        ],
        quest: r.try_get("quest").ok()?,
    })
}
