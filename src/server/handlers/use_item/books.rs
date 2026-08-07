//! Skill / Texp / god / stat / pet-stat books and HP/SP store items —
//! C# case 15 (`Client.cs:4987-5151`, `46240`/`46238`/`26456` family, pet-stat
//! books `46185-46190`/`46239`/`46241`). Each consumes (or keeps, per quirk)
//! via the shared helpers in `mod.rs`. Runs after `reborn`, before `rewards`.

use super::UseCtx;
use crate::db::persist;
use crate::protocol::encoder;

/// C# skill-book family (Client.cs:5127-5151): item id → learned skill id.
/// Skill books learn at level 10 (`int num138 = 10`) and REMOVE ONE ITEM via
/// `HomdoRemoveItem` (no `1709/170F` end feedback). Already-known → red msg.
fn skill_book_target(id: u16) -> Option<u16> {
    Some(match id {
        46230 => 10016,
        46231 => 11016,
        46232 => 12016,
        46233 => 13015,
        46246 => 14038,
        _ => return None,
    })
}

/// Multi-skill books (C# 46136/46132/46133/46134): `SkillAdd` a fixed list of 8
/// skills at level 1, then standard `HomdoUseHPSPFAI` consume + end feedback.
fn multi_skill_set(id: u16) -> Option<&'static [u16]> {
    Some(match id {
        46136 => &[10027, 10028, 10029, 10030, 10031, 10032, 10033, 10034],
        46132 => &[11027, 11028, 11029, 11030, 11031, 11032, 11033, 11034],
        46133 => &[12027, 12028, 12029, 12030, 12031, 12032, 12033, 12034],
        46134 => &[13027, 13028, 13029, 13030, 13031, 13032, 13033, 13034],
        _ => return None,
    })
}

/// Texp books (C# 46214-46219): `_My_Lv <= 200` gate, add fixed exp, stat `24`,
/// then consume.
fn texp_book_value(id: u16) -> Option<u32> {
    Some(match id {
        46211 => 100,
        46212 => 200,
        46213 => 500,
        46214 => 750,
        46215 => 1000,
        46216 => 2000,
        46217 => 5000,
        46218 => 15000,
        46219 => 50000,
        _ => return None,
    })
}

/// Pet-stat books (C# 46185-46190/46239/46241): add +1 to a pet stat via
/// `PetUpdateData` (Type_Status code), then consume. Item id → (Type_Status).
fn pet_stat_book(id: u16) -> Option<u8> {
    Some(match id {
        46185 => 0x1B, // Int atk  1/31-199 nên dùng stat: PetUpdateData(_Int)
        46186 => 0x1C, // Atk
        46187 => 0x1D, // Def
        46188 => 0x1F, // Hpx
        46189 => 0x20, // Spx
        46190 => 0x1E, // Agi
        46239 => 0x20, // Spx
        46241 => 0x1F, // Hpx
        _ => return None,
    })
}

/// Handle the book family. Returns true when the item id was handled.
pub async fn handle(ctx: &mut UseCtx<'_>) -> bool {
    let id = ctx.id;
    let pid = ctx.conn.session.id;

    // --- Skill books: learn at level 10, consume 1 via HomdRemoveItem. ---
    if let Some(skill_id) = skill_book_target(id) {
        let known = ctx.conn.session.skills.iter().any(|(s, _)| *s == skill_id);
        if !known {
            let lv = 10u8;
            ctx.conn.session.skills.push((skill_id, lv));
            let sp = ctx
                .data
                .skills
                .get(&i64::from(skill_id))
                .map(|s| s.sp.min(255) as u8)
                .unwrap_or(0);
            persist::upsert_skill(ctx.pool, pid, skill_id, lv, sp, 0).await;
            // C# `HomdoRemoveItem(_My_Id, num63, 1)` — remove 1 of the item id.
            ctx.conn.session.remove_homdo_item(id, 1);
            // C# learn packet: F4440C0008016E01 + le32(lv) + le32(skillid).
            let body = format!(
                "6E01{}{}",
                encoder::le32(lv as u32),
                encoder::le32(skill_id as u32)
            );
            ctx.out.send(crate::protocol::frame("0801", &body));
        } else {
            ctx.red("Ban da co ky nang nay roi");
        }
        return true;
    }

    // --- Multi-skill books. ---
    if let Some(list) = multi_skill_set(id) {
        for &sk in list {
            let lv = 1u8;
            if !ctx.conn.session.skills.iter().any(|(s, _)| *s == sk) {
                ctx.conn.session.skills.push((sk, lv));
                let sp = ctx
                    .data
                    .skills
                    .get(&i64::from(sk))
                    .map(|s| s.sp.min(255) as u8)
                    .unwrap_or(0);
                persist::upsert_skill(ctx.pool, pid, sk, lv, sp, 0).await;
            }
        }
        ctx.consume().await;
        return true;
    }

    // --- God book 46169 (C# `_My_God <= 240`, silent write, then consume). ---
    if id == 46169 {
        if ctx.conn.session.god <= 240 {
            ctx.conn.session.god += 10;
            persist::update_player(ctx.pool, pid, "God", i64::from(ctx.conn.session.god)).await;
            // C# `PlayerUpdateDataId(_God)` is NOT stat-emitting (Data.cs:401).
        }
        ctx.consume().await;
        return true;
    }

    // --- Texp books. ---
    if let Some(amt) = texp_book_value(id) {
        if u16::from(ctx.conn.session.level) <= 200 {
            ctx.conn.session.texp = ctx.conn.session.texp.saturating_add(amt);
            persist::update_player(ctx.pool, pid, "Texp", i64::from(ctx.conn.session.texp)).await;
            ctx.stat(0x24, ctx.conn.session.texp as i32);
        }
        ctx.consume().await;
        return true;
    }

    // --- Hpx +1 book 46240 (C#: player Hpx+1 via PlayerUpdateDataId). ---
    if id == 46240 {
        if ctx.conn.session.hpx < 400 {
            ctx.conn.session.hpx += 1;
            persist::update_player(ctx.pool, pid, "Hpx", i64::from(ctx.conn.session.hpx)).await;
            ctx.stat(0x1F, i32::from(ctx.conn.session.hpx));
        }
        ctx.consume().await;
        return true;
    }

    // --- Spx2/SpMax/tanthu book 46238 (C#: Spx2+50, SpMax+50, tanthu+1). ---
    if id == 46238 {
        ctx.conn.session.spx2 = ctx.conn.session.spx2.saturating_add(50);
        ctx.conn.session.sp_max = ctx.conn.session.sp_max.saturating_add(50);
        ctx.conn.session.tanthu += 1;
        persist::update_player(ctx.pool, pid, "Spx2", i64::from(ctx.conn.session.spx2)).await;
        persist::update_player(ctx.pool, pid, "SpMax", i64::from(ctx.conn.session.sp_max)).await;
        persist::update_player(ctx.pool, pid, "tanthu", i64::from(ctx.conn.session.tanthu)).await;
        ctx.stat(0xD0, ctx.conn.session.spx2 as i32);
        ctx.consume().await;
        return true;
    }

    // --- Pet-stat books (target the pet in slot `use_type`, C# `num62`). ---
    if let Some(ty) = pet_stat_book(id) {
        let stt = ctx.use_type;
        if let Some(pet) = ctx.conn.session.pets.iter_mut().find(|p| p.stt == stt) {
            let inc = |v: &mut u16| {
                *v = v.saturating_add(1).min(400);
                i32::from(*v)
            };
            let val = match ty {
                0x1B => inc(&mut pet.int1), // map Pet Int -> session.int1
                0x1C => inc(&mut pet.atk),
                0x1D => inc(&mut pet.def),
                0x1F => inc(&mut pet.hpx),
                0x20 => inc(&mut pet.spx),
                _ => inc(&mut pet.agi),
            };
            // C# `PetUpdateData` sends le16 based frame (pet_stat_frame) + persists.
            let pet_snap = pet.clone();
            ctx.pet_stat(stt, ty, val);
            persist::upsert_pet(ctx.pool, pid, &pet_snap).await;
        }
        ctx.consume().await;
        return true;
    }

    // --- HP/SP store items (C# 26456/26457/46145/46146): +10000 + red msg. ---
    let store = match id {
        26456 | 46145 => Some((true, ctx.conn.session.hp_store)),
        26457 | 46146 => Some((false, ctx.conn.session.sp_store)),
        _ => None,
    };
    if let Some((is_hp, _)) = store {
        if is_hp {
            ctx.conn.session.hp_store += 10000;
            persist::update_player(
                ctx.pool,
                pid,
                "HP_Store",
                i64::from(ctx.conn.session.hp_store),
            )
            .await;
            ctx.red("Hp Luu Thanh Cong: 10000");
        } else {
            ctx.conn.session.sp_store += 10000;
            persist::update_player(
                ctx.pool,
                pid,
                "SP_Store",
                i64::from(ctx.conn.session.sp_store),
            )
            .await;
            ctx.red("Sp Luu Thanh Cong: 10000");
        }
        ctx.consume().await;
        return true;
    }

    false
}