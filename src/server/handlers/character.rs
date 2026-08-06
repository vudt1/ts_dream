//! Character creation & name check handler (Opcode 0x09).
//!
//! Sub 1 (create) mirrors C# `Update_H9` case 1: parse the client layout, then
//! in **one atomic transaction** INSERT the `players` row (stats computed via
//! the TEXP/HP formula), seed `SkillSave` 1..10 / IdSkill=0, rebuild the
//! `Skill` table and update `accounts.pass1/pass2` (Ch5 §5.6 — the transaction
//! lives in the `db::players` repository). Any failure → `shutdown()`. Sub 2
//! checks the candidate name against `players.Name`. Without a pool (golden
//! replay) it degrades to the in-memory stub.

use crate::db;
use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::session::{InventoryItem, Session};

/// Parsed create-character payload (C# `Update_H9` layout).
struct CreateCharData {
    sex: u8,
    hair: u16,
    color_hex: String,
    thuoctinh: u8,
    int1: u8,
    atk: u8,
    def: u8,
    hpx: u8,
    spx: u8,
    agi: u8,
    pass1: Vec<u8>,
    pass2: Vec<u8>,
}

/// Parse the create payload (`decoded[6..]`). Layout (C# packet indices −6):
/// `[0] sex [1] unused [2] hair(1B) [3] unused [4..12] color(8B) [12] thuoctinh
/// [13..19] int atk def hpx spx agi [19] pass1_len [20..20+len] pass1
/// [20+len+1..] pass2`. `hair` is a single byte at `[2]`; the byte at `[3]`
/// (C# packet[9]) is an unused gap — never merged into hair.
fn parse_create(payload: &[u8]) -> Option<CreateCharData> {
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

    // New-character stats (C# num25/num26): reborn 0, job 0, lv 1.
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

fn apply_to_session(session: &mut Session, data: &CreateCharData) {
    // Mirror every column the `db::players::create` INSERT writes (reborn 0 /
    // job 0 / lv 1, computed HP/SP via `starting_hp_sp`, map 10817/442/758,
    // Tiengtam/ThamChien = 1) plus the seeded starter Homdo/Trangbi rows, so a
    // create → login in the golden stub yields the same Logined1 a C# server
    // would. On the live path the next login also reloads everything from MySQL.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uses_single_byte_hair() {
        // hair lives at payload[2] (C# packet[8]); payload[3] (packet[9]) is an
        // unused gap that must never raise hair to a 16-bit value.
        let mut payload = vec![0u8; 26];
        payload[0] = 1; // sex
        payload[2] = 5; // hair
        payload[3] = 0xFF; // unused gap
        payload[19] = 4; // pass1_len
        let data = parse_create(&payload).expect("valid payload");
        assert_eq!(data.hair, 5);
        assert_eq!(data.sex, 1);
    }

    #[test]
    fn parse_maps_color_and_stats_by_offset() {
        // 25-byte payload: sex[0], hair[2], color[4..12], thuoctinh[12],
        // stats[13..19], pass1_len[19], pass1[20..22], gap[22], pass2[23..25].
        let mut p = vec![0u8; 25];
        p[10] = 0xAB; // color byte (index 10 of the [4..12] window)
        p[12] = 3; // thuoctinh
        p[13] = 7; // Int
        p[14] = 8; // Atk
        p[15] = 9; // Def
        p[16] = 10; // Hpx
        p[17] = 11; // Spx
        p[18] = 12; // Agi
        p[19] = 2; // pass1_len
        p[20] = 0x41; // pass1[0]
        p[21] = 0x42; // pass1[1]
        p[23] = 0x43; // pass2[0]
        p[24] = 0x44; // pass2[1]
        let d = parse_create(&p).expect("valid payload");
        assert_eq!(d.thuoctinh, 3, "thuoctinh sits after the 8 color bytes");
        assert_eq!(d.color_hex, "000000000000AB00");
        assert_eq!((d.int1, d.atk, d.def, d.hpx, d.spx, d.agi), (7, 8, 9, 10, 11, 12));
        assert_eq!(d.pass1, vec![0x41, 0x42]);
        assert_eq!(d.pass2, vec![0x43, 0x44]);
    }

    #[test]
    fn apply_to_session_reflects_players_row_and_starter_items() {
        let mut session = Session::new();
        let d = CreateCharData {
            sex: 1,
            hair: 2,
            color_hex: "0000000000000000".to_string(),
            thuoctinh: 3,
            int1: 1,
            atk: 2,
            def: 3,
            hpx: 6,
            spx: 6,
            agi: 4,
            pass1: vec![],
            pass2: vec![],
        };
        apply_to_session(&mut session, &d);

        // HP/SP computed from the formula (engine get_hp_max, lv 1).
        assert_eq!(session.hp, 105);
        assert_eq!(session.hp_max, 105);
        assert_eq!(session.sp, 73);
        assert_eq!(session.sp_max, 73);
        assert_eq!(session.level, 1);
        assert_eq!(session.job, 0);
        assert_eq!(session.reborn, 0);
        assert_eq!(session.map_id, 10817);
        assert_eq!(session.map_x, 442);
        assert_eq!(session.map_y, 758);
        assert_eq!(session.texp, 6);
        assert_eq!(session.tiengtam, 1);
        assert_eq!(session.tham_chien, 1);

        // Starter inventory seeded into Homdo + Trangbi.
        assert!(session.homdo.iter().any(|i| i.id == 32012 && i.count == 4));
        let armor = session.trangbi.iter().find(|i| i.id == 19737);
        assert_eq!(armor.map(|i| (i.count, i.agi1, i.loai)), Some((1, 1, 2)));
    }
}
