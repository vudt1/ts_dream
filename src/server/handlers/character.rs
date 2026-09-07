//! Character creation & name check handler (Opcode 0x09).
//!
//! Sub 1 (create): parse the client layout, then in **one atomic transaction**
//! INSERT the `characters` row (stats computed via the TEXP/HP formula) and
//! seed the starter Homdo/Trangbi through the modern repository. It does **not**
//! write `accounts.pass1/pass2` — the PC create-char packet carries an empty
//! `pass1` (`golden/06`), and the password is set only via the web dashboard
//! (create) or op `0x23` sub 1 (change). Any failure → `shutdown()`. Sub 2
//! checks the candidate name against `characters.name`. Without a pool (golden
//! replay) it degrades to the in-memory stub.

use crate::db::modern::traits::CharacterSeed;
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
                Some(_) => {
                    if let Some(repos) = ctx.env.repos {
                        match repos.characters().find_id_by_name(candidate).await {
                            Ok(Some(_)) => out.send("F4440300090301"),
                            Ok(None) => {
                                conn.session.pending_new_char_name = candidate.to_vec();
                                out.send("F4440300090300");
                            }
                            Err(_) => out.shutdown = true,
                        }
                    } else {
                        out.shutdown = true;
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
                Some(_) => {
                    if let Some(repos) = ctx.env.repos {
                        if create_char_db(repos, &mut conn.session, &data)
                            .await
                            .is_err()
                        {
                            out.shutdown = true; // Exception -> shutdown (modern cutover)
                        } else {
                            out.send("F44402000901"); // Character created success
                        }
                    } else {
                        out.shutdown = true;
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

/// Build the modern character seed for the pending name, then run the one
/// atomic transaction through the modern repository. On success the session is
/// updated in-memory to match what the DB now holds.
async fn create_char_db(
    repos: &crate::db::modern::mysql::MySqlRepositories,
    session: &mut Session,
    data: &CreateCharData,
) -> Result<(), sqlx::Error> {
    let name = if session.pending_new_char_name.is_empty() {
        session.name.clone()
    } else {
        session.pending_new_char_name.clone()
    };

    // Reflect the new character into the live session before persisting the
    // complete modern row set.
    session.name = name;
    apply_to_session(session, data);
    let seed = CharacterSeed {
        level: 1,
        sex: i64::from(data.sex),
        hair: i64::from(data.hair),
        element: i64::from(data.thuoctinh),
        map_id: 10817,
        map_x: 442,
        map_y: 758,
    };
    let account_id = i64::from(session.id);
    let character_name = session.name.clone();
    repos
        .sessions()
        .create_and_seed(account_id, &character_name, &seed, session)
        .await?;
    Ok(())
}

pub fn apply_to_session(session: &mut Session, data: &CreateCharData) {
    // Mirror every column the modern character creation transaction writes (reborn 0 /
    // job 0 / lv 1, computed HP/SP via `starting_hp_sp`, map 10817/442/758,
    // Tiengtam/ThamChien = 1) plus the seeded starter Homdo/Trangbi rows, so a
    // create → login in the golden stub yields the same Logined1 sequence as
    // the live path. On the live path the next login also reloads everything
    // from MySQL.
    let hp = crate::battle::engine::get_hp_max(0, 0, 1, i64::from(data.hpx));
    let sp = crate::battle::engine::get_sp_max(0, 0, 1, i64::from(data.spx));
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
    session.homdo.push(InventoryItem {
        slot: 1,
        id: 32012,
        count: 4,
        ..Default::default()
    });
    session.trangbi.push(InventoryItem {
        slot: 2,
        id: 19737,
        count: 1,
        agi1: 1,
        loai: 2,
        ..Default::default()
    });
}
