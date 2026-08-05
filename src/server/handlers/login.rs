//! Login & session handlers (Opcode 0x00, 0x01, 0x03).
//!
//! Live-server path (env.pool present) mirrors C# `Update_H1`/`Update_H3`:
//! version gate → account exists → pass1 check → double-login guard → load the
//! player row + skills/hotkeys/inventory/pets → `Logined1`. Without a pool
//! (golden replay) the handlers run in-memory over the seeded session.

use crate::protocol::encoder;
use crate::protocol::{ID_PREFIX, MIN_VERSION};
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::session::{Conn, InventoryItem, PetState, Session};
use crate::server::spawn;
use crate::web::server_control::{ClientSender, ServerControl};
use sqlx::{FromRow, MySqlPool};

/// Op 0x00 — Hello: exact opcode 0x00 with length 1 and no sub byte.
pub fn handle_hello(ctx: &mut OpcodeCtx) {
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        out.send(spawn::HELLO_REPLY);
    }
}

/// Op 0x01 — Login (version check >= 186, auth & session initialization).
pub async fn handle_login(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.len() < 8 {
        return;
    }
    let acc_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
    let prefix = &payload[4..6];
    if !prefix.eq_ignore_ascii_case(ID_PREFIX.as_bytes()) {
        return; // Prefix mismatch -> silent return
    }
    let version = encoder::u16_le(payload[6], payload[7]);
    if version < MIN_VERSION {
        out.shutdown = true; // Version gate < 186 -> disconnect
        return;
    }

    let password = &payload[8..];
    conn.session.id = acc_id;
    conn.session.pending_pass = password.to_vec();
    conn.session.authed = true;

    match ctx.env.pool {
        Some(pool) => {
            if login_db(conn, out, pool, ctx.env.hub, ctx.env.sender, password)
                .await
                .is_err()
            {
                out.shutdown = true; // C# exception -> disconnect
            }
        }
        None => {
            // In-memory fallback (golden replay): seeded session drives Logined1.
            if password == b"WRONG" {
                out.send(spawn::LOGIN_WRONG_PASS);
            } else if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty() {
                out.send(spawn::LOGIN_CREATE_CHAR);
            } else {
                conn.session.logined = true;
                if conn.session.name.is_empty() {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                }
                let seq = spawn::build_logined_sequence_session(&conn.session);
                out.outgoing.extend(seq);
            }
        }
    }
}

/// Op 0x03 — Enter game confirmation.
pub async fn handle_enter_game(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let sub = ctx.sub;
    if sub != 1 {
        return;
    }
    if !conn.session.authed {
        out.send(spawn::ENTER_GAME_CREATE); // Not authed -> create char screen
        return;
    }
    if conn.session.logined {
        return;
    }
    match ctx.env.pool {
        Some(pool) => {
            let pass = conn.session.pending_pass.clone();
            if login_db(conn, out, pool, ctx.env.hub, ctx.env.sender, &pass)
                .await
                .is_err()
            {
                out.shutdown = true;
            }
        }
        None => {
            if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty() {
                out.send(spawn::ENTER_GAME_CREATE);
            } else {
                conn.session.logined = true;
                if conn.session.name.is_empty() {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                }
                let seq = spawn::build_logined_sequence_session(&conn.session);
                out.outgoing.extend(seq);
            }
        }
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

/// Load the player's skills, hotbar, inventory tables and pets (C# `Logined1`
/// data load). Runs as separate SELECTs; the whole flow is still best-effort.
async fn load_player_data(pool: &MySqlPool, s: &mut Session) -> Result<(), sqlx::Error> {
    let id = i64::from(s.id);

    s.skills = sqlx::query_as::<_, (i64, i64)>(
        "SELECT Id AS id, Lv AS lv FROM skill WHERE player_id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(sk, lv)| (sk as u16, lv as u8))
    .collect();

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

    s.homdo = load_items(pool, id, "homdo").await?;
    s.trangbi = load_items(pool, id, "trangbi").await?;
    s.tientrang = load_items(pool, id, "tientrang").await?;
    s.tuideo = load_items(pool, id, "tuideo").await?;
    s.luulang = load_items(pool, id, "luulang").await?;

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

async fn load_items(pool: &MySqlPool, player_id: i64, table: &str) -> Result<Vec<InventoryItem>, sqlx::Error> {
    // Tables share the same item columns (verified against 0001_init.sql).
    let sql = format!(
        "SELECT Slot AS slot, Id AS id, `Count` AS cnt, Lv AS lv, DoBen AS doben, \
         `Long` AS longv, GiatriLong AS glong, Khang AS khang, Texp AS texp FROM {table} WHERE player_id = ?"
    );
    Ok(sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64, i64, i64)>(&sql)
        .bind(player_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(slot, id, count, lv, doben, longv, glong, khang, texp)| InventoryItem {
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
        })
        .collect())
}

/// C# `Update_H1` success path: account exists → pass1 matches → double-login
/// guard → load the player → `Logined1` (or the create-char screen).
async fn login_db(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    pool: &MySqlPool,
    hub: Option<&ServerControl>,
    sender: Option<&ClientSender>,
    password: &[u8],
) -> Result<(), sqlx::Error> {
    let id = i64::from(conn.session.id);

    // Account existence (C# `Data.MemberGetIdExits`).
    let row: Option<(String,)> =
        sqlx::query_as("SELECT pass1 FROM accounts WHERE player_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    let Some((db_pass,)) = row else {
        out.shutdown = true; // Unknown account -> disconnect
        return Ok(());
    };
    if db_pass.as_bytes() != password {
        out.send(spawn::LOGIN_WRONG_PASS);
        return Ok(());
    }

    // Player existence: an account with no character goes to the create-char
    // screen (and is NOT registered as online — C# `Logined()` only adds the
    // client to `Server.Clients` once a character exists).
    let id_u32 = conn.session.id;
    let Some(player) = PlayerRow::fetch(pool, id).await? else {
        out.send(spawn::LOGIN_CREATE_CHAR);
        return Ok(());
    };

    // Double-login guard (C# `Server.Clients.ContainsKey` + Add): the
    // check+register is one atomic lock so concurrent logins cannot race.
    if let (Some(hub), Some(sender)) = (hub, sender) {
        if !hub.login_register(conn.session.id, sender).await {
            out.shutdown = true; // Already online elsewhere -> disconnect
            return Ok(());
        }
    }

    player.into_session(&mut conn.session);
    load_player_data(pool, &mut conn.session).await?;
    conn.session.id = id_u32;
    conn.session.logined = true;
    conn.session.authed = true;
    let seq = spawn::build_logined_sequence_session(&conn.session);
    out.outgoing.extend(seq);
    Ok(())
}
