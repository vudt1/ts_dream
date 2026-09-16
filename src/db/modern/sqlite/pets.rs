//! `character_pets` repository — the four general storages.

use crate::db::modern::model::{PetRecord, PetSkill, PetStorageType};
use crate::db::modern::traits::{PetRepository, RepoResult};
use crate::db::pool::DbPool;
use sqlx::Row;

pub struct SqlitePetRepository<'a> {
    pub pool: &'a DbPool,
}

const SELECT_COLUMNS: &str = "slot, petid AS pet_id, name, level, element, rebornstage AS reborn, curhp AS hp, maxhp AS hp_max, cursp AS sp, \
     maxsp AS sp_max, baseint AS int_attr, baseatk AS atk, basedef AS def, basehpx AS hpx, basespx AS spx, baseagi AS agi, fai, texp, skillpoint AS skill_point, thd, \
     skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, \
     skill4_id, skill4_level, quest, isactive AS is_active";

impl PetRepository for SqlitePetRepository<'_> {
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
    ) -> RepoResult<Vec<PetRecord>> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM character_pets \
             WHERE playerid = ? AND storagetype = ? AND petid > 0 ORDER BY slot"
        );
        let rows = sqlx::query(&sql)
            .bind(character_id)
            .bind(storage_type.value())
            .fetch_all(&self.pool.read)
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
             (playerid, storagetype, slot, petid, name, level, element, rebornstage, curhp, maxhp, \
              cursp, maxsp, baseint, baseatk, basedef, basehpx, basespx, baseagi, fai, texp, skillpoint, thd, \
              skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, \
              skill4_id, skill4_level, quest, isactive) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                     ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(playerid, storagetype, slot) DO UPDATE SET petid = excluded.petid, \
             name = excluded.name, level = excluded.level, element = excluded.element, \
             rebornstage = excluded.rebornstage, curhp = excluded.curhp, maxhp = excluded.maxhp, \
             cursp = excluded.cursp, maxsp = excluded.maxsp, baseint = excluded.baseint, \
             baseatk = excluded.baseatk, basedef = excluded.basedef, basehpx = excluded.basehpx, basespx = excluded.basespx, \
             baseagi = excluded.baseagi, fai = excluded.fai, texp = excluded.texp, \
             skillpoint = excluded.skillpoint, thd = excluded.thd, \
             skill1_id = excluded.skill1_id, skill1_level = excluded.skill1_level, \
             skill2_id = excluded.skill2_id, skill2_level = excluded.skill2_level, \
             skill3_id = excluded.skill3_id, skill3_level = excluded.skill3_level, \
             skill4_id = excluded.skill4_id, skill4_level = excluded.skill4_level, \
             quest = excluded.quest, isactive = excluded.isactive",
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
        .bind(pet.is_active as i64)
        .execute(&self.pool.write)
        .await?;
        Ok(())
    }

    async fn delete_pet(
        &self,
        character_id: i64,
        storage_type: PetStorageType,
        slot: u16,
    ) -> RepoResult<()> {
        sqlx::query(
            "DELETE FROM character_pets WHERE playerid = ? AND storagetype = ? AND slot = ?",
        )
        .bind(character_id)
        .bind(storage_type.value())
        .bind(slot)
        .execute(&self.pool.write)
        .await?;
        Ok(())
    }
}

fn row_to_pet(r: &sqlx::sqlite::SqliteRow) -> Option<PetRecord> {
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
        is_active: r.try_get::<i64, _>("is_active").map(|v| v != 0).unwrap_or(false),
    })
}
