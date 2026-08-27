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
/// shows the create-character screen). Loads the full login data set: the row,
/// skills, hotkeys, item tables, pets and quest steps.
pub async fn load(pool: &MySqlPool, s: &mut Session) -> Result<bool, sqlx::Error> {
    let id = i64::from(s.id);
    let Some(row) = PlayerRow::fetch(pool, id).await? else {
        return Ok(false);
    };
    row.into_session(s);
    load_skills(pool, s).await?;
    load_hotkeys(pool, s).await?;
    for table in ["homdo", "trangbi"] {
        assign_items(s, table, load_items(pool, id, table).await?);
    }
    load_pets(pool, s).await?;
    load_quests(pool, s).await?;
    Ok(true)
}

/// Route a loaded item table into the matching `Session` slot field.
/// TienTrang / Tuideo / LuuLang have no backing table anymore (usage audit,
/// 0001_init.sql) — their Session pouches stay in-memory only.
fn assign_items(s: &mut Session, table: &str, items: Vec<InventoryItem>) {
    match table {
        "homdo" => s.homdo = items,
        "trangbi" => s.trangbi = items,
        _ => {}
    }
}

/// Delete a character and all of its per-player gameplay rows in one
/// transaction (the per-table deletes plus the `players` row, op 0x23 sub 2).
///
/// Every statement is scoped by `player_id`. The `players` row and the six
/// gameplay tables (`homdo`, `trangbi`, `pet`, `quest`, `skill`, `skillsave`)
/// are removed; the `accounts` row is
/// **not** touched (account identity outlives the character). Returns `false`
/// when the character row did not exist.
pub async fn delete_character(pool: &MySqlPool, player_id: i64) -> Result<bool, sqlx::Error> {
    const TABLES: [&str; 6] = [
        "homdo",
        "trangbi",
        "pet",
        "quest",
        "skill",
        "skillsave",
    ];
    let mut tx = pool.begin().await?;
    for table in TABLES {
        let q = format!("DELETE FROM {table} WHERE player_id = ?");
        sqlx::query(&q)
            .bind(player_id)
            .execute(&mut *tx)
            .await?;
    }
    let res = sqlx::query("DELETE FROM players WHERE player_id = ?")
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
    let existed = res.rows_affected() == 1;
    tx.commit().await?;
    Ok(existed)
}

/// Load quest steps into `s.quest_steps` scoped by `player_id` (`(npc_id, step)`
/// pairs keyed by the quest NPC — the runtime quest bookkeeping snapshot).
async fn load_quests(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);
    let rows = crate::db::quest::list(pool, id).await?;
    s.quest_steps = rows
        .into_iter()
        .map(|r| (r.npc_id, r.step))
        .collect();
    Ok(())
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

    // 1. players row — every column explicit (Ch5 §5.4).
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

    // 4. Seed starter Homdo/Trangbi rows (the new-character inventory
    // template). Every numeric column the item loader reads
    // (Lv/DoBen/Long/GiatriLong/Khang/Texp) is written explicitly so the row
    // round-trips with non-NULL values.
    for row in starter_rows() {
        let Some(table) = item_table(row.table) else {
            continue;
        };
        let q = format!(
            "INSERT INTO {table} (player_id, Slot, Id, `Count`, Lv, DoBen, Agi1, \
             `Long`, GiatriLong, Khang, Loai, Texp) VALUES (?, ?, ?, ?, 0, 0, ?, 0, 0, 0, ?, 0)"
        );
        sqlx::query(&q)
            .bind(c.player_id)
            .bind(row.slot)
            .bind(row.id)
            .bind(row.count)
            .bind(row.agi1)
            .bind(row.loai)
            .execute(&mut *tx)
            .await?;
    }

    // 5. accounts pass1/pass2.
    sqlx::query("UPDATE accounts SET pass1 = ?, pass2 = ? WHERE player_id = ?")
        .bind(c.pass1.as_slice())
        .bind(c.pass2.as_slice())
        .bind(c.player_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}

/// New-character HP/SP: reborn 0, job 0, level 1.
pub fn starting_hp_sp(hpx: u8, spx: u8) -> (i64, i64) {
    let hp = get_hp_max(0, 0, 1, i64::from(hpx));
    let sp = get_sp_max(0, 0, 1, i64::from(spx));
    (hp, sp)
}

/// One starter inventory row a freshly created character owns (the
/// new-character inventory template).
///
/// The shared MySQL schema (Ch5 §5.4) carries the template rows as data:
/// they are INSERTed (scoped by `player_id`) inside the same atomic create
/// transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarterRow {
    pub table: &'static str,
    pub slot: i64,
    pub id: i64,
    pub count: i64,
    pub agi1: i64,
    pub loai: i64,
}

/// The starter inventory for a new character:
/// - `Homdo` slot 1: item 32012 × 4 (the starter potion "Đăng các");
/// - `Trangbi` slot 2: item 19737 × 1, Agi1=1, Loai=2 (the starter armor).
///
/// The template's empty `Trangbi` slot-1 row (Id=0) is intentionally omitted:
/// it carries no stats and loads as a filtered no-op in the session.
pub fn starter_rows() -> Vec<StarterRow> {
    vec![
        StarterRow {
            table: "homdo",
            slot: 1,
            id: 32012,
            count: 4,
            agi1: 0,
            loai: 0,
        },
        StarterRow {
            table: "trangbi",
            slot: 2,
            id: 19737,
            count: 1,
            agi1: 1,
            loai: 2,
        },
    ]
}

/// Whitelist the item tables the starter seed may write (never interpolate
/// client input into `{table}`). Public so the moved integration test can
/// verify every starter row targets a whitelisted table.
pub fn item_table(table: &'static str) -> Option<&'static str> {
    match table {
        "homdo" => Some("homdo"),
        "trangbi" => Some("trangbi"),
        _ => None,
    }
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
    bank_gold: i64,
    tiengtam: i64,
    gocnhin: i64,
    stt_pet: i64,
    pk: i64,
    tham_chien: i64,
    sp_store: i64,
    hp_store: i64,
    newbie: i64,
    shop_point: i64,
    savemap: i64,
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
              God AS god, HEX(Color) AS color_hex, Gold AS gold, BankGold AS bank_gold, Tiengtam AS tiengtam, \
             Gocnhin AS gocnhin, SttPetXuatchien AS stt_pet, Pk AS pk, ThamChien AS tham_chien, \
             SP_Store AS sp_store, HP_Store AS hp_store, newbie AS newbie, \
             ShopPoint AS shop_point, savemap AS savemap \
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
        s.bank_gold = self.bank_gold as u32;
        s.tiengtam = self.tiengtam as u16;
        s.gocnhin = self.gocnhin as u8;
        s.active_pet_stt = self.stt_pet as u8;
        s.pk = self.pk as u8;
        s.tham_chien = self.tham_chien as u8;
        s.sp_store = self.sp_store as u32;
        s.hp_store = self.hp_store as u32;
        s.newbie = self.newbie as u32;
        s.shop_point = self.shop_point as u32;
        s.savemap = self.savemap as u16;
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

#[derive(FromRow)]
struct PetRow {
    stt: i64,
    id: i64,
    name_hex: Option<String>,
    lv: i64,
    thuoctinh: i64,
    reborn: i64,
    hp: i64,
    hp_max: i64,
    sp: i64,
    sp_max: i64,
    int1: i64,
    atk: i64,
    def: i64,
    hpx: i64,
    spx: i64,
    agi: i64,
    fai: i64,
    texp: i64,
    int2: i64,
    atk2: i64,
    def2: i64,
    hpx2: i64,
    spx2: i64,
    agi2: i64,
    thd: i64,
    skill_point: i64,
    quest: i64,
    skill1: i64,
    skill1_lv: i64,
    skill2: i64,
    skill2_lv: i64,
    skill3: i64,
    skill3_lv: i64,
    skill4: i64,
    skill4_lv: i64,
}

async fn load_pets(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);
    s.pets = sqlx::query_as::<_, PetRow>(
        "SELECT Stt AS stt, Id AS id, HEX(Name) AS name_hex, Lv AS lv, Thuoctinh AS thuoctinh, \
         Reborn AS reborn, Hp AS hp, HpMax AS hp_max, Sp AS sp, SpMax AS sp_max, `Int` AS int1, \
         Atk AS atk, Def AS def, Hpx AS hpx, Spx AS spx, Agi AS agi, Fai AS fai, Texp AS texp, \
         Int2 AS int2, Atk2 AS atk2, Def2 AS def2, Hpx2 AS hpx2, Spx2 AS spx2, Agi2 AS agi2, \
         Thd AS thd, SkillPoint AS skill_point, Quest AS quest, Idskill1 AS skill1, \
         LvSkill1 AS skill1_lv, IdSkill2 AS skill2, LvSkill2 AS skill2_lv, IdSkill3 AS skill3, \
         LvSkill3 AS skill3_lv, IdSkill4 AS skill4, LvSkill4 AS skill4_lv \
         FROM pet WHERE player_id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        let mut p = PetState {
            stt: r.stt as u8,
            id: r.id as u16,
            level: r.lv as u8,
            thuoctinh: r.thuoctinh as u8,
            reborn: r.reborn as u8,
            hp: r.hp as u16,
            hp_max: r.hp_max as u16,
            sp: r.sp as u16,
            sp_max: r.sp_max as u16,
            int1: r.int1 as u16,
            atk: r.atk as u16,
            def: r.def as u16,
            hpx: r.hpx as u16,
            spx: r.spx as u16,
            agi: r.agi as u16,
            fai: r.fai as u16,
            texp: r.texp as u32,
            int2: r.int2 as u16,
            atk2: r.atk2 as u16,
            def2: r.def2 as u16,
            hpx2: r.hpx2 as u16,
            spx2: r.spx2 as u16,
            agi2: r.agi2 as u16,
            thd: r.thd as u16,
            skill_point: r.skill_point as u16,
            quest: r.quest as u8,
            skills: [
                (r.skill1 as u16, r.skill1_lv as u8),
                (r.skill2 as u16, r.skill2_lv as u8),
                (r.skill3 as u16, r.skill3_lv as u8),
                (r.skill4 as u16, r.skill4_lv as u8),
            ],
            ..Default::default()
        };
        if let Some(h) = r.name_hex {
            if let Some(bytes) = encoder::bytes(&h) {
                p.name = bytes;
            }
        }
        p
    })
    .collect();
    Ok(())
}

/// One item-table row (all five tables share this column layout, Ch5 §5.4).
#[derive(Debug, Clone, FromRow)]
struct ItemRow {
    slot: i64,
    id: i64,
    cnt: i64,
    lv: i64,
    doben: i64,
    int1: i64,
    atk1: i64,
    def1: i64,
    hpx1: i64,
    spx1: i64,
    agi1: i64,
    fai1: i64,
    int2: i64,
    atk2: i64,
    def2: i64,
    hpx2: i64,
    spx2: i64,
    agi2: i64,
    fai2: i64,
    item_hp: i64,
    item_sp: i64,
    longv: i64,
    glong: i64,
    khang: i64,
    thuoctinh: i64,
    gth: i64,
    loai: i64,
    texp: i64,
}

impl From<ItemRow> for InventoryItem {
    fn from(r: ItemRow) -> Self {
        InventoryItem {
            slot: r.slot as u8,
            id: r.id as u16,
            count: r.cnt as u8,
            lv: r.lv as u8,
            doben: r.doben as u8,
            int1: r.int1 as i16,
            atk1: r.atk1 as i16,
            def1: r.def1 as i16,
            hpx1: r.hpx1 as i16,
            spx1: r.spx1 as i16,
            agi1: r.agi1 as i16,
            fai1: r.fai1 as i16,
            int2: r.int2 as i16,
            atk2: r.atk2 as i16,
            def2: r.def2 as i16,
            hpx2: r.hpx2 as i16,
            spx2: r.spx2 as i16,
            agi2: r.agi2 as i16,
            fai2: r.fai2 as i16,
            item_hp: r.item_hp as i16,
            item_sp: r.item_sp as i16,
            long_val: r.longv as u8,
            giatri_long: r.glong as u8,
            khang: r.khang as u8,
            thuoctinh: r.thuoctinh as u8,
            giatri_thuoctinh: r.gth as u8,
            loai: r.loai as u8,
            texp: r.texp as u32,
        }
    }
}

async fn load_items(
    pool: &MySqlPool,
    player_id: i64,
    table: &str,
) -> Result<Vec<InventoryItem>, sqlx::Error> {
    // The five item tables share the same columns (verified against 0001_init.sql).
    let sql = format!(
        "SELECT Slot AS slot, Id AS id, `Count` AS cnt, Lv AS lv, DoBen AS doben, \
         Int1 AS int1, Atk1 AS atk1, Def1 AS def1, Hpx1 AS hpx1, Spx1 AS spx1, Agi1 AS agi1, \
         Fai1 AS fai1, Int2 AS int2, Atk2 AS atk2, Def2 AS def2, Hpx2 AS hpx2, Spx2 AS spx2, \
         Agi2 AS agi2, Fai2 AS fai2, Hp AS item_hp, Sp AS item_sp, `Long` AS longv, GiatriLong AS glong, Khang AS khang, \
         Thuoctinh AS thuoctinh, GiatriThuoctinh AS gth, Loai AS loai, Texp AS texp \
         FROM {table} WHERE player_id = ?"
    );
    Ok(sqlx::query_as::<_, ItemRow>(&sql)
        .bind(player_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(InventoryItem::from)
        .collect())
}
