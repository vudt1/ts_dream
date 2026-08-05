//! Character creation & name check handler (Opcode 0x09).
//!
//! Sub 1 (create) mirrors C# `Update_H9` case 1: parse the client layout, then
//! in **one atomic transaction** INSERT the `players` row (stats computed via
//! the TEXP/HP formula), seed `SkillSave` 1..10 / IdSkill=0, rebuild the
//! `Skill` table and update `accounts.pass1/pass2`. Any failure → `shutdown()`
//! (Ch5 §5.6). Sub 2 checks the candidate name against `players.Name`.
//! Without a pool (golden replay) it degrades to the in-memory stub.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::protocol::encoder;
use crate::server::handler::OpcodeCtx;
use crate::server::session::Session;

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
/// `[0] sex [2] hair [4..12] color(8B) [12] thuoctinh [13..19] int atk def hpx
/// spx agi [19] pass1_len [20..20+len] pass1 [20+len+1..] pass2`.
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
        hair: encoder::u16_le(payload[2], payload[3]),
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
                    let exists = sqlx::query_scalar::<_, String>(
                        "SELECT HEX(Name) FROM players WHERE HEX(Name) = HEX(?) LIMIT 1",
                    )
                    .bind(candidate)
                    .fetch_optional(pool)
                    .await;
                    match exists {
                        Ok(Some(_)) => out.send("F4440300090301"), // Name used
                        Ok(None) => {
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
                    if create_char_db(pool, &mut conn.session, &data).await.is_err() {
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

/// C# `Update_H9` case 1: one atomic transaction INSERTing `players`, seeding
/// `SkillSave` 1..10, rebuilding `Skill` and updating `accounts` — all-or-nothing.
async fn create_char_db(
    pool: &sqlx::MySqlPool,
    session: &mut Session,
    data: &CreateCharData,
) -> Result<(), sqlx::Error> {
    let id = i64::from(session.id);
    let name = if session.pending_new_char_name.is_empty() {
        &session.name
    } else {
        &session.pending_new_char_name
    };

    // New-character stats (C# num25/num26): reborn 0, job 0, lv 1.
    let hp = get_hp_max(0, 0, 1, i64::from(data.hpx)) as i64;
    let sp = get_sp_max(0, 0, 1, i64::from(data.spx)) as i64;

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
    .bind(id)
    .bind(name.as_slice())
    .bind(hp)
    .bind(hp)
    .bind(sp)
    .bind(sp)
    .bind(i64::from(data.int1))
    .bind(i64::from(data.atk))
    .bind(i64::from(data.def))
    .bind(i64::from(data.hpx))
    .bind(i64::from(data.spx))
    .bind(i64::from(data.agi))
    .bind(i64::from(data.sex))
    .bind(i64::from(data.hair))
    .bind(i64::from(data.thuoctinh))
    .bind(&data.color_hex)
    .execute(&mut *tx)
    .await?;

    // 2. SkillSave seed: rows 1..10 / IdSkill=0 (mandatory seed).
    for slot in 1..=10i64 {
        sqlx::query(
            "INSERT INTO skillsave (player_id, ID, IdSkill) VALUES (?, ?, 0)",
        )
        .bind(id)
        .bind(slot)
        .execute(&mut *tx)
        .await?;
    }

    // 3. Rebuild Skill: a fresh character owns no skills — clear stale rows.
    sqlx::query("DELETE FROM skill WHERE player_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // 4. accounts pass1/pass2 (C# `Data.MemberChangedPass`).
    sqlx::query("UPDATE accounts SET pass1 = ?, pass2 = ? WHERE player_id = ?")
        .bind(data.pass1.as_slice())
        .bind(data.pass2.as_slice())
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Reflect the new character into the live session for the upcoming login.
    session.name = name.to_vec();
    apply_to_session(session, data);
    Ok(())
}

fn apply_to_session(session: &mut Session, data: &CreateCharData) {
    session.sex = data.sex;
    session.hair = data.hair;
    session.thuoctinh = data.thuoctinh;
    session.color = data.color_hex.clone();
    session.int1 = u16::from(data.int1);
    session.atk = u16::from(data.atk);
    session.def = u16::from(data.def);
    session.hpx = u16::from(data.hpx);
    session.spx = u16::from(data.spx);
    session.agi = u16::from(data.agi);
}
