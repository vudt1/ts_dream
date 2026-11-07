//! Learn / upgrade skills (Opcode 0x1C) & Pet Reborn (Opcode 0x2C) handlers.

use crate::battle::rng::DotNetRandom;
use crate::data::tables::Npc;
use crate::db::persist;
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::session::Conn;

pub fn can_learn_element(player_element: u8, skill_element: i64) -> bool {
    match player_element {
        1 => skill_element != 3,
        2 => skill_element != 4,
        3 => skill_element != 1,
        4 => skill_element != 2,
        _ => true,
    }
}

pub fn get_point_skill_add(player_element: u8, skill_element: i64, base_point: i64) -> u16 {
    let double_cost = match player_element {
        1 => skill_element == 2 || skill_element == 4,
        2 => skill_element == 1 || skill_element == 3,
        3 => skill_element == 2 || skill_element == 4,
        4 => skill_element == 1 || skill_element == 3,
        _ => false,
    };
    if double_cost {
        (base_point * 2) as u16
    } else {
        base_point as u16
    }
}

/// A pet base stat the reborn bonus points can land on.
#[derive(Clone, Copy, PartialEq)]
enum PetStatKind {
    Int,
    Atk,
    Def,
    Hpx,
    Spx,
    Agi,
}

/// Weighted random stat pick.
///
/// Consumes 7 draws per call in a fixed order: 6 tie-break
/// `next_range(1, 999)` calls (one per stat, Int→Agi) + 1 selection
/// `next_range(1, 1000)`.
fn random_point_stat(rng: &mut DotNetRandom, npc: &Npc) -> PetStatKind {
    let raw = [
        (npc.int1, PetStatKind::Int),
        (npc.atk, PetStatKind::Atk),
        (npc.def, PetStatKind::Def),
        (npc.hpx, PetStatKind::Hpx),
        (npc.spx, PetStatKind::Spx),
        (npc.agi, PetStatKind::Agi),
    ];
    let total: i64 = raw.iter().map(|(v, _)| *v).sum();
    if total <= 0 {
        return PetStatKind::Atk; // default fallback
    }
    let mut rows: Vec<(i64, i64, PetStatKind)> = raw
        .into_iter()
        .map(|(v, kind)| {
            let w = ((v as f64 / total as f64) * 1000.0).round() as i64;
            let r = i64::from(rng.next_range(1, 999));
            (w, r, kind)
        })
        .collect();
    // Sort `Point ASC, Random DESC`.
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let num7 = rng.next_range(1, 1000);
    let mut acc = 0i64;
    for (w, _, kind) in &rows {
        if i64::from(num7) <= acc + *w {
            return *kind;
        }
        acc += *w;
    }
    rows.last().map(|(_, _, k)| *k).unwrap_or(PetStatKind::Atk)
}

pub fn is_job_skill(skill_id: u16) -> bool {
    (14039..=14054).contains(&skill_id)
}

pub fn validate_job_gate(job: u8, skill_id: u16) -> bool {
    match skill_id {
        14039..=14042 => job == 2, // Bá Vương
        14043..=14046 => job == 1, // Hiệp Khách
        14047..=14050 => job == 3, // Hiền Nhân
        14051..=14054 => job == 4, // Tiên Nhân
        _ => true,
    }
}

/// Dispatch Opcode 0x1C — Learn / upgrade skills.
pub async fn handle_skills(ctx: &mut OpcodeCtx<'_>) {
    let pool = ctx.env.pool;
    let data = ctx.data;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Player skill learn / upgrade
        1 => handle_player_skill_learn(conn, payload, out, data, pool).await,
        // Sub 2: Pet skill upgrade
        2 => handle_pet_skill_upgrade(conn, payload, out, data, pool).await,
        // Sub 4: Bulk skill tree commit
        4 => handle_player_skill_bulk(conn, payload, out, data, pool).await,
        // Sub 5: Skill Reborn 2
        5 => handle_skill_reborn_2(conn, payload, out, data, pool).await,
        _ => {}
    }
}

pub async fn handle_player_skill_learn(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&crate::db::pool::DbPool>,
) {
    if payload.is_empty() {
        return;
    }

    let mut idx = 0;
    let mut count = 0u32;

    while idx + 2 < payload.len() {
        let skill_id = encoder::u16_le(payload[idx], payload[idx + 1]);
        let target_lv = payload[idx + 2];
        idx += 3;

        if skill_id == 0 {
            continue;
        }

        let Some(skill_def) = data.skills.get(&i64::from(skill_id)) else {
            break;
        };

        if target_lv > skill_def.lv_max as u8 || (skill_def.reborn as u8) > conn.session.reborn {
            continue;
        }

        // Job gates & Great Heavenly Cycle (14038)
        if !validate_job_gate(conn.session.job, skill_id) {
            break;
        }
        if is_job_skill(skill_id) && !conn.session.skills.iter().any(|(id, lv)| *id == 14038 && *lv >= 1) {
            break;
        }

        let existing_pos = conn
            .session
            .skills
            .iter()
            .position(|(id, _)| *id == skill_id);

        if let Some(pos) = existing_pos {
            let current_lv = conn.session.skills[pos].1;
            if target_lv <= current_lv {
                continue;
            }
            let needed = u16::from(target_lv - current_lv);
            if conn.session.skill_point < needed {
                break;
            }
            conn.session.skill_point -= needed;
            conn.session.skills[pos].1 = target_lv;

            out.send(format!(
                "F4440C0008016E01{}{}",
                encoder::le32(target_lv as u32),
                encoder::le32(skill_id as u32)
            ));
            let _ = persist::upsert_skill(
                pool,
                conn.session.id,
                skill_id,
                target_lv,
                skill_def.sp as u8,
                0,
            )
            .await;
            count += 1;
        } else {
            // Learning a new skill
            if !can_learn_element(conn.session.thuoctinh, skill_def.thuoctinh) {
                break;
            }
            // Check prerequisites id_dk (must be all 0, or at least 1 learned)
            let has_prereq = skill_def.id_dk.iter().all(|&id| id == 0)
                || skill_def.id_dk.iter().any(|&id| {
                    id > 0
                        && conn
                            .session
                            .skills
                            .iter()
                            .any(|(s_id, _)| *s_id == id as u16)
                });
            if !has_prereq {
                break;
            }

            let base_cost =
                get_point_skill_add(conn.session.thuoctinh, skill_def.thuoctinh, skill_def.point);
            let extra_cost = u16::from(target_lv.saturating_sub(1));
            let total_needed = base_cost + extra_cost;

            if skill_def.point <= 0 || conn.session.skill_point < total_needed {
                break;
            }

            conn.session.skill_point -= total_needed;
            conn.session.skills.push((skill_id, target_lv));

            out.send(format!(
                "F4440C0008016E01{}{}",
                encoder::le32(target_lv as u32),
                encoder::le32(skill_id as u32)
            ));
            let _ = persist::upsert_skill(
                pool,
                conn.session.id,
                skill_id,
                target_lv,
                skill_def.sp as u8,
                0,
            )
            .await;
            count += 1;
        }
    }

    if count > 0 {
        persist::update_player(
            pool,
            conn.session.id,
            "SkillPoint",
            i64::from(conn.session.skill_point),
        )
        .await;
        out.send(format!(
            "F4440C0008012501{}00000000",
            encoder::le32(conn.session.skill_point as u32)
        ));
        crate::server::session::registry_update_if_online(&conn.session);
    }
}

/// Result of skill upgrade cost validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillUpgradeValidation {
    pub cost: u16,
    pub sp: u8,
    pub is_new: bool,
}

/// Tính toán chi phí và xác thực nâng cấp kỹ năng dùng chung cho bulk commit (0x1C 04) và Reborn 2 (0x1C 05).
pub fn calculate_skill_upgrade_cost(
    skill_id: u16,
    target_lv: u8,
    session_reborn: u8,
    session_job: u8,
    session_element: u8,
    current_skills: &[(u16, u8)],
    data: &crate::data::loader::GameData,
) -> Option<SkillUpgradeValidation> {
    if skill_id == 0 || target_lv == 0 {
        return None;
    }
    let skill_def = data.skills.get(&i64::from(skill_id))?;
    if target_lv > skill_def.lv_max as u8 || (skill_def.reborn as u8) > session_reborn {
        return None;
    }
    if !validate_job_gate(session_job, skill_id) {
        return None;
    }
    if is_job_skill(skill_id) && !current_skills.iter().any(|(id, lv)| *id == 14038 && *lv >= 1) {
        return None;
    }

    if let Some(&(_, current_lv)) = current_skills.iter().find(|(id, _)| *id == skill_id) {
        if target_lv <= current_lv {
            return None;
        }
        let needed = u16::from(target_lv - current_lv);
        Some(SkillUpgradeValidation {
            cost: needed,
            sp: skill_def.sp as u8,
            is_new: false,
        })
    } else {
        if !can_learn_element(session_element, skill_def.thuoctinh) {
            return None;
        }
        let has_prereq = skill_def.id_dk.iter().all(|&id| id == 0)
            || skill_def.id_dk.iter().any(|&id| {
                id > 0 && current_skills.iter().any(|(s_id, _)| *s_id == id as u16)
            });
        if !has_prereq || skill_def.point <= 0 {
            return None;
        }

        let base_cost = get_point_skill_add(session_element, skill_def.thuoctinh, skill_def.point);
        let extra_cost = u16::from(target_lv.saturating_sub(1));
        let total_needed = base_cost + extra_cost;
        Some(SkillUpgradeValidation {
            cost: total_needed,
            sp: skill_def.sp as u8,
            is_new: true,
        })
    }
}

pub async fn handle_player_skill_bulk(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&crate::db::pool::DbPool>,
) {
    if payload.is_empty() || payload.len() % 3 != 0 {
        return;
    }

    let mut temp_skills = conn.session.skills.clone();
    let mut total_cost = 0u16;
    let mut mutations: Vec<(u16, u8, u8)> = Vec::new();

    let mut cursor = 0;
    while cursor + 2 < payload.len() {
        let skill_id = encoder::u16_le(payload[cursor], payload[cursor + 1]);
        let target_lv = payload[cursor + 2];
        cursor += 3;

        if skill_id == 0 {
            continue;
        }

        let Some(validation) = calculate_skill_upgrade_cost(
            skill_id,
            target_lv,
            conn.session.reborn,
            conn.session.job,
            conn.session.thuoctinh,
            &temp_skills,
            data,
        ) else {
            return;
        };

        let Some(new_cost) = total_cost.checked_add(validation.cost) else {
            return;
        };
        if new_cost > conn.session.skill_point {
            return;
        }
        total_cost = new_cost;

        if validation.is_new {
            temp_skills.push((skill_id, target_lv));
        } else if let Some(entry) = temp_skills.iter_mut().find(|(id, _)| *id == skill_id) {
            entry.1 = target_lv;
        }
        mutations.push((skill_id, target_lv, validation.sp));
    }

    if mutations.is_empty() {
        return;
    }

    let new_sp = conn.session.skill_point - total_cost;

    if let Some(pool) = pool {
        let mut tx = match pool.write.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Failed to begin transaction for bulk skills: {e}");
                return;
            }
        };

        for (skill_id, target_lv, sp) in &mutations {
            let res = sqlx::query(
                "INSERT INTO character_skills (playerid, skillid, level, sp, saveflag) VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT(playerid, skillid) DO UPDATE SET level=excluded.level, sp=excluded.sp, saveflag=excluded.saveflag"
            )
            .bind(i64::from(conn.session.id))
            .bind(i64::from(*skill_id))
            .bind(i64::from(*target_lv))
            .bind(i64::from(*sp))
            .bind(0i64)
            .execute(&mut *tx)
            .await;

            if let Err(e) = res {
                tracing::error!("Failed to persist bulk skill: {e}");
                let _ = tx.rollback().await;
                return;
            }
        }

        let res_sp = sqlx::query("UPDATE characters SET skillpoint = ? WHERE playerid = ?")
            .bind(i64::from(new_sp))
            .bind(i64::from(conn.session.id))
            .execute(&mut *tx)
            .await;

        if let Err(e) = res_sp {
            tracing::error!("Failed to update skillpoint in bulk commit: {e}");
            let _ = tx.rollback().await;
            return;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("Failed to commit bulk skills transaction: {e}");
            return;
        }
    }

    conn.session.skills = temp_skills;
    conn.session.skill_point = new_sp;

    for (skill_id, target_lv, _) in &mutations {
        out.send(format!(
            "F4440C0008016E01{}{}",
            encoder::le32(u32::from(*target_lv)),
            encoder::le32(u32::from(*skill_id))
        ));
    }

    out.send(format!(
        "F4440C0008012501{}00000000",
        encoder::le32(u32::from(conn.session.skill_point))
    ));

    crate::server::session::registry_update_if_online(&conn.session);
}

async fn handle_pet_skill_upgrade(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&crate::db::pool::DbPool>,
) {
    if payload.len() < 4 {
        return;
    }
    let stt = payload[0];
    let Some(pet_pos) = conn.session.pets.iter().position(|p| p.stt == stt) else {
        return;
    };

    let mut cursor = 1;
    let mut updated = false;
    while cursor + 3 <= payload.len() {
        let skill_id = encoder::u16_le(payload[cursor], payload[cursor + 1]);
        let target_lv = payload[cursor + 2];
        cursor += 3;

        let Some(skill_def) = data.skills.get(&i64::from(skill_id)) else {
            continue;
        };

        if target_lv > skill_def.lv_max as u8 {
            continue;
        }

        let pet = &mut conn.session.pets[pet_pos];
        // Upgrade ONLY an existing skill slot
        if let Some(sk) = pet.skills.iter_mut().find(|s| s.0 == skill_id && s.0 != 0) {
            if target_lv > sk.1 {
                let cost = u16::from(target_lv - sk.1);
                if pet.skill_point >= cost {
                    pet.skill_point -= cost;
                    sk.1 = target_lv;
                    updated = true;

                    // Reply with LE16 hex for stt (4 hex chars)
                    out.send(format!(
                        "F4440F00080204{}6E01{}{}",
                        encoder::le16(stt as u16),
                        encoder::le32(target_lv as u32),
                        encoder::le32(skill_id as u32)
                    ));
                }
            }
        }
    }
    if updated {
        let pet = &conn.session.pets[pet_pos];
        persist::upsert_pet(pool, conn.session.id, pet).await;
        crate::server::session::registry_update_if_online(&conn.session);
    }
}

pub async fn handle_skill_reborn_2(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&crate::db::pool::DbPool>,
) {
    if payload.len() < 4 {
        return;
    }
    let mut cursor = 0;
    // Section 1: L1 dragon balls
    let ball_count = encoder::u32_le(
        payload[cursor],
        payload[cursor + 1],
        payload[cursor + 2],
        payload[cursor + 3],
    ) as usize;
    cursor += 4;
    if cursor + ball_count > payload.len() {
        return;
    }
    let mut temp_dragon_balls = conn.session.dragon_balls;
    for &ball_idx in &payload[cursor..cursor + ball_count] {
        if (1..=12).contains(&ball_idx) {
            temp_dragon_balls |= 1 << (ball_idx - 1);
        }
    }
    cursor += ball_count;

    // Section 2: L2 hero skills
    if cursor + 4 > payload.len() {
        return;
    }
    let skill_count = encoder::u32_le(
        payload[cursor],
        payload[cursor + 1],
        payload[cursor + 2],
        payload[cursor + 3],
    ) as usize;
    cursor += 4;

    let mut temp_skills = conn.session.skills.clone();
    let mut total_cost = 0u16;
    let mut hero_mutations: Vec<(u16, u8, u8)> = Vec::new();

    for _ in 0..skill_count {
        if cursor + 3 > payload.len() {
            return;
        }
        let skill_id = encoder::u16_le(payload[cursor], payload[cursor + 1]);
        let target_lv = payload[cursor + 2];
        cursor += 3;

        if skill_id == 0 {
            continue;
        }

        let Some(validation) = calculate_skill_upgrade_cost(
            skill_id,
            target_lv,
            conn.session.reborn,
            conn.session.job,
            conn.session.thuoctinh,
            &temp_skills,
            data,
        ) else {
            return;
        };

        let Some(new_cost) = total_cost.checked_add(validation.cost) else {
            return;
        };
        if new_cost > conn.session.skill_point {
            return;
        }
        total_cost = new_cost;

        if validation.is_new {
            temp_skills.push((skill_id, target_lv));
        } else if let Some(entry) = temp_skills.iter_mut().find(|(id, _)| *id == skill_id) {
            entry.1 = target_lv;
        }
        hero_mutations.push((skill_id, target_lv, validation.sp));
    }

    // Section 3: L3 hero ball sockets (8 sockets DAT_0094B3A4[6..13])
    let mut temp_skill_rb2 = conn.session.skill_rb2;
    if cursor + 4 <= payload.len() {
        let l3_bytes = encoder::u32_le(
            payload[cursor],
            payload[cursor + 1],
            payload[cursor + 2],
            payload[cursor + 3],
        ) as usize;
        cursor += 4;
        let socket_entries = l3_bytes / 3;
        for _ in 0..socket_entries {
            if cursor + 3 > payload.len() {
                break;
            }
            let slot = payload[cursor];
            let skill_id = encoder::u16_le(payload[cursor + 1], payload[cursor + 2]);
            cursor += 3;

            if (6..=13).contains(&slot) {
                temp_skill_rb2[(slot - 6) as usize] = skill_id;
            } else if (1..=8).contains(&slot) {
                temp_skill_rb2[(slot - 1) as usize] = skill_id;
            }
        }
    }

    let new_sp = conn.session.skill_point - total_cost;

    // Bundled atomic write transaction for Sections 1, 2, and 3
    if let Some(pool) = pool {
        let mut tx = match pool.write.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Failed to begin transaction for reborn 2: {e}");
                return;
            }
        };

        for (skill_id, target_lv, sp) in &hero_mutations {
            let res = sqlx::query(
                "INSERT INTO character_skills (playerid, skillid, level, sp, saveflag) VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT(playerid, skillid) DO UPDATE SET level=excluded.level, sp=excluded.sp, saveflag=excluded.saveflag"
            )
            .bind(i64::from(conn.session.id))
            .bind(i64::from(*skill_id))
            .bind(i64::from(*target_lv))
            .bind(i64::from(*sp))
            .bind(0i64)
            .execute(&mut *tx)
            .await;

            if let Err(e) = res {
                tracing::error!("Failed to persist reborn 2 skill: {e}");
                let _ = tx.rollback().await;
                return;
            }
        }

        let composite_dragon_balls: i64 = (i64::from(temp_dragon_balls) & 0x0FFF)
            | ((i64::from(conn.session.ball_point) & 0xFF) << 12);
        let rb2_slots_str = temp_skill_rb2.map(|s| s.to_string()).join(",");

        let res_char = sqlx::query(
            "UPDATE characters SET dragon_balls = ?, rb2_slots = ?, skillpoint = ? WHERE playerid = ?"
        )
        .bind(composite_dragon_balls)
        .bind(&rb2_slots_str)
        .bind(i64::from(new_sp))
        .bind(i64::from(conn.session.id))
        .execute(&mut *tx)
        .await;

        if let Err(e) = res_char {
            tracing::error!("Failed to update characters in reborn 2 commit: {e}");
            let _ = tx.rollback().await;
            return;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("Failed to commit reborn 2 transaction: {e}");
            return;
        }
    }

    conn.session.dragon_balls = temp_dragon_balls;
    conn.session.skills = temp_skills;
    conn.session.skill_point = new_sp;
    conn.session.skill_rb2 = temp_skill_rb2;

    for (skill_id, target_lv, _) in &hero_mutations {
        out.send(format!(
            "F4440C0008016E01{}{}",
            encoder::le32(u32::from(*target_lv)),
            encoder::le32(u32::from(*skill_id))
        ));
    }
    if !hero_mutations.is_empty() {
        out.send(format!(
            "F4440C0008012501{}00000000",
            encoder::le32(u32::from(conn.session.skill_point))
        ));
    }

    out.send(crate::server::spawn::dragon_balls_frame(
        conn.session.ball_point,
        conn.session.dragon_balls,
    ));
    out.send(crate::server::spawn::hero_ball_slots_frame(
        &conn.session.skill_rb2,
    ));

    crate::server::session::registry_update_if_online(&conn.session);
}

/// Handle Opcode 0x2C — Pet Reborn.
pub async fn handle_pet_reborn(ctx: &mut OpcodeCtx<'_>) {
    match ctx.sub {
        2 => handle_pet_add_skill_4(ctx).await,
        1 | 3..=7 => handle_pet_reborn_flow(ctx).await,
        _ => {}
    }
}

async fn handle_pet_add_skill_4(ctx: &mut OpcodeCtx<'_>) {
    let pool = ctx.env.pool;
    let data = ctx.data;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        return;
    }
    let stt = payload[0];
    let Some(pet_pos) = conn.session.pets.iter().position(|p| p.stt == stt) else {
        return;
    };
    let pet_id = conn.session.pets[pet_pos].id;
    let Some(npc_def) = data.npcs.get(&i64::from(pet_id)) else {
        return;
    };
    let skill_4 = npc_def.skill[3];
    if skill_4 > 0 {
        let skill_4_id = skill_4 as u16;
        let pet = &mut conn.session.pets[pet_pos];
        pet.skills[3] = (skill_4_id, 1);
        persist::upsert_pet(pool, conn.session.id, pet).await;
        out.send("F44402002C01");
        for f in crate::server::spawn::pet_status_single(&conn.session, stt) {
            out.send(f);
        }
    }
}

async fn handle_pet_reborn_flow(ctx: &mut OpcodeCtx<'_>) {
    let pool = ctx.env.pool;
    let hub = ctx.env.hub;
    let data = ctx.data;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        return;
    }
    let stt = payload[0];

    let Some(pet_pos) = conn.session.pets.iter().position(|p| p.stt == stt) else {
        return;
    };

    let pet_id = conn.session.pets[pet_pos].id;
    let pet_lv = conn.session.pets[pet_pos].level;
    let pet_reborn = conn.session.pets[pet_pos].reborn;

    let rank = if ctx.sub == 0 || ctx.sub == 1 { 1 } else { ctx.sub.saturating_sub(1) };
    let threshold = if rank == 1 && pet_reborn == 0 { 30u8 } else { 60u8 };

    // Scan homdo slots 1..25 for reborn item
    let mut match_slot = None;
    let mut target_npc_id = 0i64;

    for item in &conn.session.homdo {
        if item.slot >= 1 && item.slot <= 25 && item.id > 0 && item.count > 0 {
            if let Some(item_def) = data.items.get(&i64::from(item.id)) {
                if item_def.rb_pet_from == i64::from(pet_id)
                    && item_def.rb_pet_to > 0
                    && data.npcs.contains_key(&item_def.rb_pet_to)
                {
                    match_slot = Some(item.slot);
                    target_npc_id = item_def.rb_pet_to;
                    break;
                }
            }
        }
    }

    let Some(hd_slot) = match_slot else {
        return; // Silent failure when no valid Rb item found
    };

    // Consume 1 unit of reborn item from homdo
    let hd_pos = conn
        .session
        .homdo
        .iter()
        .position(|i| i.slot == hd_slot)
        .unwrap();
    if conn.session.homdo[hd_pos].count > 1 {
        conn.session.homdo[hd_pos].count -= 1;
    } else {
        conn.session.homdo.remove(hd_pos);
    }
    let updated_item = conn
        .session
        .homdo
        .iter()
        .find(|i| i.slot == hd_slot)
        .cloned()
        .unwrap_or(crate::server::session::InventoryItem {
            slot: hd_slot,
            ..Default::default()
        });
    persist::upsert_item(pool, conn.session.id, "homdo", &updated_item).await;

    let new_npc = &data.npcs[&target_npc_id];

    let bonus_points = (u16::from(pet_lv).saturating_sub(u16::from(threshold))) / 5;

    // Snapshot the NPC base stats BEFORE the bonus distribution and derive
    // HpMax/SpMax from them; the boosted values are stored to the stat
    // columns after.
    let base_hpx = new_npc.hpx as u16;
    let base_spx = new_npc.spx as u16;
    let mut int_val = new_npc.int1 as u16;
    let mut atk_val = new_npc.atk as u16;
    let mut def_val = new_npc.def as u16;
    let mut hpx_val = base_hpx;
    let mut spx_val = base_spx;
    let mut agi_val = new_npc.agi as u16;

    // Distribute bonus points via the weighted random draw, NOT a fixed
    // rotation. Fresh time-seeded RNG (non-deterministic, so not
    // golden-covered).
    let mut rng = DotNetRandom::time_seeded();
    for _ in 0..bonus_points {
        match random_point_stat(&mut rng, new_npc) {
            PetStatKind::Int => int_val += 1,
            PetStatKind::Atk => atk_val += 1,
            PetStatKind::Def => def_val += 1,
            PetStatKind::Hpx => hpx_val += 1,
            PetStatKind::Spx => spx_val += 1,
            PetStatKind::Agi => agi_val += 1,
        }
    }

    // Pet HpMax/SpMax map the reborn level onto the player formula:
    // rb 0/1 → get_x_max(0), rb 2 → get_x_max(1), all at level 1 and from
    // the BASE stats.
    let rb_map = if new_npc.reborn == 2 { 1 } else { 0 };
    let hp_max = crate::battle::engine::get_hp_max(rb_map, 0, 1, i64::from(base_hpx)) as u16;
    let sp_max = crate::battle::engine::get_sp_max(rb_map, 0, 1, i64::from(base_spx)) as u16;

    let pet = &mut conn.session.pets[pet_pos];
    pet.id = new_npc.id as u16;
    pet.name = new_npc.name.clone();
    pet.level = 1;
    pet.thuoctinh = new_npc.thuoctinh as u8;
    pet.reborn = new_npc.reborn as u8;
    pet.hp = hp_max;
    pet.hp_max = hp_max;
    pet.sp = sp_max;
    pet.sp_max = sp_max;
    pet.int1 = int_val;
    pet.atk = atk_val;
    pet.def = def_val;
    pet.hpx = hpx_val;
    pet.spx = spx_val;
    pet.agi = agi_val;
    pet.fai = 60;
    pet.texp = 6;

    for i in 0..4 {
        let sk_id = new_npc.skill[i] as u16;
        let lv = match sk_id {
            10016 | 11016 | 12016 | 13015 => 10,
            _ => 1,
        };
        pet.skills[i] = if sk_id > 0 { (sk_id, lv) } else { (0, 0) };
    }

    persist::upsert_pet(pool, conn.session.id, pet).await;

    let player_id_le = encoder::le32(conn.session.id);
    let pet_id_le = encoder::le32(pet.id as u32);

    // Map broadcasts first: `07000F02` / `0C000F01` go to the player AND the
    // map — the latter via `broadcast_except`.
    let f_0f02 = format!("F44407000F02{}{:02X}", player_id_le, stt);
    let f_0f01 = format!("F4440C000F01{}{:02X}{}01", player_id_le, stt, pet_id_le);
    out.send(f_0f02.clone());
    out.send(f_0f01.clone());
    if let Some(hub) = hub {
        hub.broadcast_except(conn.session.id, &f_0f02).await;
        hub.broadcast_except(conn.session.id, &f_0f01).await;
    }

    // Pet status + trailer frames.
    for f in crate::server::spawn::pet_status_single(&conn.session, stt) {
        out.send(f);
    }

    out.send(format!("F44406001301{}", pet_id_le));
    out.send("F44402002C01");
}
