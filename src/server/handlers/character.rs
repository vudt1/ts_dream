//! Character creation & name check handler (Opcode 0x09).
//!
//! Sub 1 (create): parse the client layout, then in **one atomic transaction**
//! INSERT the `players` row (stats computed via the TEXP/HP formula), seed
//! `SkillSave` 1..10 / IdSkill=0, rebuild the `Skill` table and update
//! `accounts.pass1/pass2` (Ch5 §5.6 — the transaction lives in the
//! `db::players` repository). Any failure → `shutdown()`. Sub 2 checks the
//! candidate name against `players.Name`. Without a pool (golden replay) it
//! degrades to the in-memory stub.

use crate::db;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::{InventoryItem, Session};

/// Parsed create-character payload.
pub struct CreateCharData {
    pub sex: u8,
    pub hair: u16,
    pub color_hex: String,
    pub thuoctinh: u8,
    pub int1: u8,
    pub atk: u8,
    pub def: u8,
    pub hpx: u8,
    pub spx: u8,
    pub agi: u8,
    pub pass1: Vec<u8>,
    pub pass2: Vec<u8>,
}

/// Parse the create payload (`decoded[6..]`). Raw packet layout (indices −6):
/// `[0] sex [1] unused [2] hair(1B) [3] unused [4..12] color(8B) [12] thuoctinh
/// [13..19] int atk def hpx spx agi [19] pass1_len [20..20+len] pass1
/// [20+len+1..] pass2`. `hair` is a single byte at `[2]`; the byte at `[3]`
/// is an unused gap — never merged into hair.
pub fn parse_create(payload: &[u8]) -> Option<CreateCharData> {
    if payload.len() < 20 {
        return None;
    }
    let pass1_len = payload[19] as usize;
    if payload.len() < 20 + pass1_len + 1 {
        return None;
    }
    let pass1 = payload[20..20 + pass1_len].to_vec();
    let pass2 = payload[20 + pass1_len + 1..].to_vec();
    Some(CreateCharData {
        sex: payload[0],
        hair: u16::from(payload[2]),
        color_hex: encoder::hex(&payload[4..12]),
        thuoctinh: payload[12],
        int1: payload[13],
        atk: payload[14],
        def: payload[15],
        hpx: payload[16],
        spx: payload[17],
        agi: payload[18],
        pass1,
        pass2,
    })
}

/// Op 0x09 — Create character / name check.
pub async fn handle_character(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 2: Name check
        2 => {
            let candidate = payload;
            match ctx.env.pool {
                Some(pool) => {
                    match db::players::name_exists(pool, candidate).await {
                        Ok(true) => out.send("F4440300090301"), // Name used
                        Ok(false) => {
                            conn.session.pending_new_char_name = candidate.to_vec();
                            out.send("F4440300090300"); // Name available
                        }
                        Err(_) => out.shutdown = true, // DB error -> disconnect
                    }
                }
                None => {
                    if candidate == b"EXISTS" {
                        out.send("F4440300090301"); // Name used
                    } else {
                        conn.session.pending_new_char_name = candidate.to_vec();
                        out.send("F4440300090300"); // Name available
                    }
                }
            }
        }
        // Sub 1: Create character
        1 => {
            let Some(data) = parse_create(payload) else {
                out.shutdown = true;
                return;
            };
            match ctx.env.pool {
                Some(pool) => {
                    if create_char_db(pool, &mut conn.session, &data)
                        .await
                        .is_err()
                    {
                        out.shutdown = true; // Exception -> shutdown (Ch5 §5.6)
                    } else {
                        out.send("F44402000901"); // Character created success
                    }
                }
                None => {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                    apply_to_session(&mut conn.session, &data);
                    out.send("F44402000901");
                }
            }
        }
        _ => {}
    }
}

/// Build the `db::players::CreateCharacter` parameters for the pending name,
/// then run the one atomic transaction (repository). On success the session is
/// updated in-memory to match what the DB now holds.
async fn create_char_db(
    pool: &sqlx::MySqlPool,
    session: &mut Session,
    data: &CreateCharData,
) -> Result<(), sqlx::Error> {
    let name = if session.pending_new_char_name.is_empty() {
        session.name.clone()
    } else {
        session.pending_new_char_name.clone()
    };

    // New-character stats: reborn 0, job 0, lv 1 (formula-computed HP/SP).
    let (hp, sp) = db::players::starting_hp_sp(data.hpx, data.spx);

    let params = db::players::CreateCharacter {
        player_id: i64::from(session.id),
        name,
        hp,
        sp,
        sex: data.sex,
        hair: data.hair,
        thuoctinh: data.thuoctinh,
        color_hex: data.color_hex.clone(),
        int1: data.int1,
        atk: data.atk,
        def: data.def,
        hpx: data.hpx,
        spx: data.spx,
        agi: data.agi,
        pass1: data.pass1.clone(),
        pass2: data.pass2.clone(),
    };
    db::players::create(pool, &params).await?;

    // Reflect the new character into the live session for the upcoming login.
    session.name = params.name;
    apply_to_session(session, data);
    Ok(())
}

pub fn apply_to_session(session: &mut Session, data: &CreateCharData) {
    // Mirror every column the `db::players::create` INSERT writes (reborn 0 /
    // job 0 / lv 1, computed HP/SP via `starting_hp_sp`, map 10817/442/758,
    // Tiengtam/ThamChien = 1) plus the seeded starter Homdo/Trangbi rows, so a
    // create → login in the golden stub yields the same Logined1 sequence as
    // the live path. On the live path the next login also reloads everything
    // from MySQL.
    let (hp, sp) = db::players::starting_hp_sp(data.hpx, data.spx);
    session.level = 1;
    session.job = 0;
    session.reborn = 0;
    session.sex = data.sex;
    session.hair = data.hair;
    session.thuoctinh = data.thuoctinh;
    session.color = data.color_hex.clone();
    session.hp = hp as u16;
    session.hp_max = hp as u16;
    session.sp = sp as u16;
    session.sp_max = sp as u16;
    session.point = 0;
    session.skill_point = 0;
    session.int1 = u16::from(data.int1);
    session.atk = u16::from(data.atk);
    session.def = u16::from(data.def);
    session.hpx = u16::from(data.hpx);
    session.spx = u16::from(data.spx);
    session.agi = u16::from(data.agi);
    session.texp = 6;
    session.map_id = 10817;
    session.map_x = 442;
    session.map_y = 758;
    session.gold = 0;
    session.tiengtam = 1;
    session.god = 0;
    session.gocnhin = 0;
    session.pk = 0;
    session.tham_chien = 1;
    for row in db::players::starter_rows() {
        let item = InventoryItem {
            slot: row.slot as u8,
            id: row.id as u16,
            count: row.count as u8,
            agi1: row.agi1 as i16,
            loai: row.loai as u8,
            ..Default::default()
        };
        match row.table {
            "homdo" => session.homdo.push(item),
            "trangbi" => session.trangbi.push(item),
            _ => {}
        }
    }
}
