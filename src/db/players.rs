//! `players` + per-player gameplay-tables repository.
//!
//! All login-time reads (character row, skills, hotbar, the five item tables,
//! pets) and the atomic character-creation transaction (Ch5 §5.6) live here so
//! handlers stay SQL-free. Every query carries the `player_id` predicate — the
//! schema is shared, so any unscoped `WHERE Id = n` would cross players.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::protocol::encoder;
use crate::server::session::{InventoryItem, PetState, Session};
use sqlx::{FromRow, MySqlPool};

/// Load a character's row + all per-player data into `s`.
///
/// Returns `false` when the account has no character (the login handler then
/// shows the create-character screen). Mirrors the C# `Logined1` data load.
pub async fn load(pool: &MySqlPool, s: &mut Session) -> Result<bool, sqlx::Error> {
    let id = i64::from(s.id);
    let Some(row) = PlayerRow::fetch(pool, id).await? else {
        return Ok(false);
    };
    row.into_session(s);
    load_skills(pool, s).await?;
    load_hotkeys(pool, s).await?;
    for table in ["homdo", "trangbi", "tientrang", "tuideo", "luulang"] {
        assign_items(s, table, load_items(pool, id, table).await?);
    }
    load_pets(pool, s).await?;
    Ok(true)
}

/// Route a loaded item table into the matching `Session` slot field.
fn assign_items(s: &mut Session, table: &str, items: Vec<InventoryItem>) {
    match table {
        "homdo" => s.homdo = items,
        "trangbi" => s.trangbi = items,
        "tientrang" => s.tientrang = items,
        "tuideo" => s.tuideo = items,
        "luulang" => s.luulang = items,
        _ => {}
    }
}

/// Is `name` (VISCII bytes) already taken? Compared byte-for-byte via HEX so
/// the latin1 stored bytes match the wire bytes exactly.
pub async fn name_exists(pool: &MySqlPool, name: &[u8]) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar::<_, String>(
        "SELECT HEX(Name) FROM players WHERE HEX(Name) = HEX(?) LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

/// Parameters for [`create`] (one atomic transaction, Ch5 §5.6).
pub struct CreateCharacter {
    pub player_id: i64,
    pub name: Vec<u8>,
    pub hp: i64,
    pub sp: i64,
    pub sex: u8,
    pub hair: u16,
    pub thuoctinh: u8,
    pub color_hex: String,
    pub int1: u8,
    pub atk: u8,
    pub def: u8,
    pub hpx: u8,
    pub spx: u8,
    pub agi: u8,
    pub pass1: Vec<u8>,
    pub pass2: Vec<u8>,
}

/// Create a character in one atomic transaction (Ch5 §5.6):
/// INSERT `players` (computed stats, remaining columns rely on DEFAULT),
/// seed `SkillSave` rows 1..10 / IdSkill=0, rebuild `Skill`, then update
/// `accounts.pass1/pass2`. Any failure rolls everything back.
pub async fn create(pool: &MySqlPool, c: &CreateCharacter) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. players row — every column explicit (Ch5 §5.4), same layout as C#.
    sqlx::query(
        "INSERT INTO players (\
         player_id, Name, Lv, Hp, HpMax, Sp, SpMax, Point, SkillPoint, `Int`, Atk, Def, \
         Hpx, Spx, Agi, Int2, Atk2, Def2, Hpx2, Spx2, Agi2, Texp, MapId, MapX, MapY, \
         Reborn, Job, Sex, Hair, Thuoctinh, Ghost, God, Color, Gold, Tiengtam, Gocnhin, \
         SttPetXuatchien, Pk, ThamChien) \
         VALUES (?, ?, 1, ?, ?, ?, ?, 0, 0, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 1, 6, \
                 10817, 442, 758, 0, 0, ?, ?, ?, 0, 0, ?, 0, 1, 0, 0, 0, 1)",
    )
    .bind(c.player_id)
    .bind(c.name.as_slice())
    .bind(c.hp)
    .bind(c.hp)
    .bind(c.sp)
    .bind(c.sp)
    .bind(i64::from(c.int1))
    .bind(i64::from(c.atk))
    .bind(i64::from(c.def))
    .bind(i64::from(c.hpx))
    .bind(i64::from(c.spx))
    .bind(i64::from(c.agi))
    .bind(i64::from(c.sex))
    .bind(i64::from(c.hair))
    .bind(i64::from(c.thuoctinh))
    .bind(&c.color_hex)
    .execute(&mut *tx)
    .await?;

    // 2. SkillSave seed: rows 1..10 / IdSkill=0 (mandatory seed).
    for slot in 1..=10i64 {
        sqlx::query("INSERT INTO skillsave (player_id, ID, IdSkill) VALUES (?, ?, 0)")
            .bind(c.player_id)
            .bind(slot)
            .execute(&mut *tx)
            .await?;
    }

    // 3. Rebuild Skill: a fresh character owns no skills — clear stale rows.
    sqlx::query("DELETE FROM skill WHERE player_id = ?")
        .bind(c.player_id)
        .execute(&mut *tx)
        .await?;

    // 4. accounts pass1/pass2 (C# `Data.MemberChangedPass`).
    sqlx::query("UPDATE accounts SET pass1 = ?, pass2 = ? WHERE player_id = ?")
        .bind(c.pass1.as_slice())
        .bind(c.pass2.as_slice())
        .bind(c.player_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}

/// New-character HP/SP (C# num25/num26): reborn 0, job 0, lv 1.
pub fn starting_hp_sp(hpx: u8, spx: u8) -> (i64, i64) {
    let hp = get_hp_max(0, 0, 1, i64::from(hpx));
    let sp = get_sp_max(0, 0, 1, i64::from(spx));
    (hp, sp)
}

/// One `players` row loaded from MySQL (columns aliased to snake_case).
#[derive(FromRow)]
struct PlayerRow {
    name_hex: Option<String>,
    lv: i64,
    hp: i64,
    hp_max: i64,
    sp: i64,
    sp_max: i64,
    point: i64,
    skill_point: i64,
    int1: i64,
    atk: i64,
    def: i64,
    hpx: i64,
    spx: i64,
    agi: i64,
    int2: i64,
    atk2: i64,
    def2: i64,
    hpx2: i64,
    spx2: i64,
    agi2: i64,
    texp: i64,
    map_id: i64,
    map_x: i64,
    map_y: i64,
    reborn: i64,
    job: i64,
    sex: i64,
    hair: i64,
    thuoctinh: i64,
    god: i64,
    color_hex: Option<String>,
    gold: i64,
    tiengtam: i64,
    gocnhin: i64,
    stt_pet: i64,
    pk: i64,
    tham_chien: i64,
    sp_store: i64,
    hp_store: i64,
}

impl PlayerRow {
    async fn fetch(pool: &MySqlPool, player_id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, PlayerRow>(
            "SELECT HEX(Name) AS name_hex, Lv AS lv, Hp AS hp, HpMax AS hp_max, Sp AS sp, \
             SpMax AS sp_max, Point AS point, SkillPoint AS skill_point, `Int` AS int1, \
             Atk AS atk, Def AS def, Hpx AS hpx, Spx AS spx, Agi AS agi, Int2 AS int2, \
             Atk2 AS atk2, Def2 AS def2, Hpx2 AS hpx2, Spx2 AS spx2, Agi2 AS agi2, \
             Texp AS texp, MapId AS map_id, MapX AS map_x, MapY AS map_y, Reborn AS reborn, \
             Job AS job, Sex AS sex, Hair AS hair, Thuoctinh AS thuoctinh, \
             God AS god, HEX(Color) AS color_hex, Gold AS gold, Tiengtam AS tiengtam, \
             Gocnhin AS gocnhin, SttPetXuatchien AS stt_pet, Pk AS pk, ThamChien AS tham_chien, \
             SP_Store AS sp_store, HP_Store AS hp_store \
             FROM players WHERE player_id = ?",
        )
        .bind(player_id)
        .fetch_optional(pool)
        .await
    }

    fn into_session(self, s: &mut Session) {
        if let Some(hex) = self.name_hex {
            if let Some(bytes) = encoder::bytes(&hex) {
                s.name = bytes;
            }
        }
        s.level = self.lv as u8;
        s.hp = self.hp as u16;
        s.hp_max = self.hp_max as u16;
        s.sp = self.sp as u16;
        s.sp_max = self.sp_max as u16;
        s.point = self.point as u16;
        s.skill_point = self.skill_point as u16;
        s.int1 = self.int1 as u16;
        s.atk = self.atk as u16;
        s.def = self.def as u16;
        s.hpx = self.hpx as u16;
        s.spx = self.spx as u16;
        s.agi = self.agi as u16;
        s.int2 = self.int2 as u32;
        s.atk2 = self.atk2 as u32;
        s.def2 = self.def2 as u32;
        s.hpx2 = self.hpx2 as u32;
        s.spx2 = self.spx2 as u32;
        s.agi2 = self.agi2 as u32;
        s.texp = self.texp as u32;
        s.map_id = self.map_id as u16;
        s.map_x = self.map_x as u16;
        s.map_y = self.map_y as u16;
        s.reborn = self.reborn as u8;
        s.job = self.job as u8;
        s.sex = self.sex as u8;
        s.hair = self.hair as u16;
        s.thuoctinh = self.thuoctinh as u8;
        s.god = self.god as u32;
        if let Some(hex) = self.color_hex {
            s.color = hex;
        }
        s.gold = self.gold as u32;
        s.tiengtam = self.tiengtam as u16;
        s.gocnhin = self.gocnhin as u8;
        s.active_pet_stt = self.stt_pet as u8;
        s.pk = self.pk as u8;
        s.tham_chien = self.tham_chien as u8;
        s.sp_store = self.sp_store as u32;
        s.hp_store = self.hp_store as u32;
    }
}

async fn load_skills(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);
    s.skills =
        sqlx::query_as::<_, (i64, i64)>("SELECT Id AS id, Lv AS lv FROM skill WHERE player_id = ?")
            .bind(id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|(sk, lv)| (sk as u16, lv as u8))
            .collect();
    Ok(())
}

async fn load_hotkeys(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);
    let saves = sqlx::query_as::<_, (i64, i64)>(
        "SELECT ID AS slot, IdSkill AS skill FROM skillsave WHERE player_id = ? AND IdSkill > 0",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    for (slot, skill) in saves {
        if (1..=10).contains(&slot) {
            s.hotkeys[slot as usize] = skill as u16;
        }
    }
    Ok(())
}

async fn load_pets(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);
    s.pets = sqlx::query_as::<_, (i64, i64, Option<String>)>(
        "SELECT Stt AS stt, Id AS id, HEX(Name) AS name_hex FROM pet WHERE player_id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(stt, pet_id, name_hex)| {
        let mut p = PetState {
            stt: stt as u8,
            id: pet_id as u16,
            ..Default::default()
        };
        if let Some(h) = name_hex {
            if let Some(bytes) = encoder::bytes(&h) {
                p.name = bytes;
            }
        }
        p
    })
    .collect();
    Ok(())
}

async fn load_items(
    pool: &MySqlPool,
    player_id: i64,
    table: &str,
) -> Result<Vec<InventoryItem>, sqlx::Error> {
    // The five item tables share the same columns (verified against 0001_init.sql).
    let sql = format!(
        "SELECT Slot AS slot, Id AS id, `Count` AS cnt, Lv AS lv, DoBen AS doben, \
         `Long` AS longv, GiatriLong AS glong, Khang AS khang, Texp AS texp FROM {table} WHERE player_id = ?"
    );
    Ok(
        sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64)>(&sql)
            .bind(player_id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(
                |(slot, id, count, lv, doben, longv, glong, khang, texp)| InventoryItem {
                    slot: slot as u8,
                    id: id as u16,
                    count: count as u8,
                    lv: lv as u8,
                    doben: doben as u8,
                    long_val: longv as u8,
                    giatri_long: glong as u8,
                    khang: khang as u8,
                    texp: texp as u32,
                    ..Default::default()
                },
            )
            .collect(),
    )
}
