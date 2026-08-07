//! Use item handler (Op 0x17 sub 15) — C# `Update_H17` case 15 (Client.cs:3801-5361).
//!
//! Full case-15 parity port, split into focused modules for maintainability:
//! - `rewards` — lucky-box random rewards + fixed multi-item packs (99999…46953 family).
//! - `books`   — skill books, Texp/god books, stat books, pet-stat books, HP/SP store items.
//! - `misc`    — doll summon, dice items, special frames, no-op ids, full-heal.
//! - `reborn`  — reborn-by-item (46170, 46247-46250) which hard-reset + close the socket.
//!
//! Dispatch order mirrors the C# branch order (warp → add-pet → sleep → the big else
//! chain → point books → party buffs → generic potion). Random branches source their
//! RNG from an injected `.NET`-compatible `DotNetRandom` (`battle/rng.rs`) so unit
//! tests can seed it deterministically and prod uses `time_seeded()`.

use crate::battle::rng::DotNetRandom;
use crate::data::loader::GameData;
use crate::db::persist;
use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::Conn;

mod books;
mod misc;
mod reborn;
mod rewards;

/// Warp items: item id → (map_id, x, y). Source: C# case-15 warp table
/// (Client.cs:3821-3971). Item 46016 warps to the save map (here: current map).
fn warp_target(id: u16, current_map: u16) -> Option<(u16, u16, u16)> {
    Some(match id {
        46016 => (current_map, 410, 510),
        46022 => (12403, 442, 375),
        46027 => (12003, 530, 510),
        54002 => (12901, 202, 1175),
        46084 => (21011, 222, 455),
        46055 => (18990, 602, 235),
        46105 => (26011, 502, 775),
        46085 => (23241, 402, 515),
        46054 => (14241, 402, 495),
        46086 => (25241, 662, 655),
        46103 => (20001, 762, 615),
        46102 => (19241, 462, 435),
        46052 => (15025, 462, 375),
        45005 => (54811, 500, 500),
        46104 => (24262, 362, 395),
        54001 => (54812, 500, 500),
        46051 => (15002, 442, 335),
        46019 => (15000, 522, 535),
        46025 => (15001, 562, 515),
        46087 => (15012, 222, 295),
        46023 => (54901, 1722, 835),
        45822 => (54004, 426, 635),
        45003 => (54826, 402, 375),
        46070 => (59401, 402, 775),
        _ => return None,
    })
}

/// A shared per-use context bundling the mutated session, frame sink, optional
/// DB pool, static data, the parsed packet fields, and the injectable RNG.
pub(crate) struct UseCtx<'a> {
    pub conn: &'a mut Conn,
    pub out: &'a mut HandleOutcome,
    pub pool: Option<&'a sqlx::MySqlPool>,
    pub data: &'a GameData,
    /// Homdo slot (C# `packet[6]`).
    pub slot: u8,
    /// Used count (C# `packet[7]`).
    pub count: u16,
    /// Use-type / pet slot (C# `packet[8]`): 0 = player, 1..4 = pet.
    pub use_type: u8,
    /// Item id (C# `_ID`).
    pub id: u16,
    pub rng: &'a mut DotNetRandom,
}

impl UseCtx<'_> {
    /// Player stat update frame `F4440C000801` + type + sign + le32 + `00000000`
    /// (C# `PlayerUpdateDataId` stat-emitting branches; Type_Status codes).
    pub fn stat(&mut self, ty: u8, val: i32) {
        self.out.send(crate::server::handlers::stats::build_stat_update(ty, val));
    }

    /// Pet stat update frame `F4440F00080204` + le16(stt) + type + sign + le32
    /// + `00000000` (C# `Data.PetUpdateData`, Data.cs:2689).
    pub fn pet_stat(&mut self, stt: u8, ty: u8, val: i32) {
        let (sign, abs) = if val >= 0 {
            ("01", val as u32)
        } else {
            ("02", (-val) as u32)
        };
        let body = format!(
            "04{}{:02X}{}{}00000000",
            encoder::le16(stt as u16),
            ty,
            sign,
            encoder::le32(abs)
        );
        self.out.send(crate::protocol::frame("0802", &body));
    }

    /// Add `count` of `item_id` to Homdo (C# `HomdoAddItem`): emits the
    /// `F4440E001706`+id+count+`000000000000000000` reward frame, or
    /// `F44403001B0102` (inventory full). Returns whether the item was added.
    pub async fn add_reward(&mut self, item_id: u16, count: u8) -> bool {
        let item = crate::server::inventory::from_template(self.data, item_id, count);
        let affected = self.conn.session.add_homdo_item(item.clone());
        if affected.is_empty() {
            self.out.send("F44403001B0102".to_string());
            return false;
        }
        for s in &affected {
            if let Some(a) = self.conn.session.homdo.iter().find(|i| i.slot == *s) {
                persist::upsert_item(self.pool, self.conn.session.id, "homdo", a).await;
            }
        }
        self.out.send(format!(
            "F4440E001706{}{:02X}000000000000000000",
            encoder::le16(item_id),
            count
        ));
        true
    }

    /// Consume `count` of the item at `slot` and emit the standard end feedback
    /// `F44404001709` + slot + used-count + `F4440200170F` (C# `HomdoUseHPSPFAI`,
    /// Data.cs:3638). Returns true when the item was consumed.
    pub async fn consume(&mut self) -> bool {
        let Some(pos) = self.conn.session.homdo.iter().position(|i| i.slot == self.slot) else {
            return false;
        };
        if self.conn.session.homdo[pos].id == 0 {
            return false;
        }
        let used = self.count.min(u16::from(self.conn.session.homdo[pos].count));
        if used == 0 {
            return false;
        }
        let rem = self.conn.session.homdo[pos].count - used as u8;
        if rem > 0 {
            self.conn.session.homdo[pos].count = rem;
        } else {
            self.conn.session.homdo.remove(pos);
        }
        let slot = self.slot;
        let pid = self.conn.session.id;
        match self.conn.session.homdo.iter().find(|i| i.slot == slot) {
            Some(kept) => persist::upsert_item(self.pool, pid, "homdo", kept).await,
            None => {
                let empty = crate::server::session::InventoryItem {
                    slot,
                    ..Default::default()
                };
                persist::upsert_item(self.pool, pid, "homdo", &empty).await;
            }
        }
        self.out.send(format!("F44404001709{:02X}{:02X}", slot, used));
        self.out.send("F4440200170F".to_string());
        true
    }

    /// Emit only the standard end feedback without consuming the item
    /// (C# no-op ids 46013/46014/46015/46042/46091 and point books 50010/50011).
    pub fn end_feedback(&mut self) {
        self.out
            .send(format!("F44404001709{:02X}{:02X}", self.slot, self.count));
        self.out.send("F4440200170F".to_string());
    }

    /// VISCII-encoded red message frame (`F444 + len + 020B + 00000000 + msg`).
    pub fn red(&mut self, msg: &str) {
        let visc = crate::encoding::viscii_encode(msg);
        let body = format!("00000000{}", encoder::strhex(&visc));
        self.out.send(crate::protocol::frame("020B", &body));
    }

    /// Sleep-equivalent for items 46036 and 46167: send the sleep frames, full-heal
    /// the player (and the active pet via stat frames when the player is the leader).
    pub async fn sleep(&mut self) {
        if self.conn.session.battle_id > 0 {
            return;
        }
        self.out.send("F44402001F0A".to_string());
        if self.conn.session.hp < self.conn.session.hp_max {
            self.stat(0x19, i32::from(self.conn.session.hp_max));
            self.conn.session.hp = self.conn.session.hp_max;
            persist::update_player(
                self.pool,
                self.conn.session.id,
                "Hp",
                i64::from(self.conn.session.hp_max),
            )
            .await;
        }
        if self.conn.session.sp < self.conn.session.sp_max {
            self.stat(0x1A, i32::from(self.conn.session.sp_max));
            self.conn.session.sp = self.conn.session.sp_max;
            persist::update_player(
                self.pool,
                self.conn.session.id,
                "Sp",
                i64::from(self.conn.session.sp_max),
            )
            .await;
        }
        self.out.send("F44403001F0100".to_string());
    }
}

/// Op 0x17 sub 15 — use item at `slot`, `count` times (C# case 15).
pub async fn use_item(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    data: &GameData,
) {
    let mut rng = DotNetRandom::time_seeded();
    use_item_rng(conn, payload, out, pool, data, &mut rng).await;
}

/// Test-facing entry with an injected seeded RNG.
pub async fn use_item_rng(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    data: &GameData,
    rng: &mut DotNetRandom,
) {
    if payload.is_empty() {
        return;
    }
    let slot = payload[0];
    let count = if payload.len() >= 2 && payload[1] > 0 {
        payload[1] as u16
    } else {
        1
    };
    let use_type = payload.get(2).copied().unwrap_or(0);
    if count == 0 {
        return;
    }

    let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) else {
        return;
    };
    let item = conn.session.homdo[pos].clone();
    let id = item.id;
    // C# case-15 gate: `iD12 > 0 && num61 > 0` and the slot must hold enough.
    if id == 0 || item.count == 0 || u16::from(item.count) < count {
        return;
    }
    let mut ctx = UseCtx {
        conn,
        out,
        pool,
        data,
        slot,
        count,
        use_type,
        id,
        rng,
    };
    dispatch(&mut ctx).await;
}

/// The C# case-15 dispatch chain, in the exact C# branch order.
async fn dispatch(ctx: &mut UseCtx<'_>) {
    // --- 1. Warp items (C# flag2 table): consume + warp, no 170F tail. ---
    if let Some((map_id, x, y)) = warp_target(ctx.id, ctx.conn.session.map_id) {
        ctx.conn.session.map_id = map_id;
        ctx.conn.session.map_x = x;
        ctx.conn.session.map_y = y;
        ctx.out
            .send(format!("F44404001709{:02X}{:02X}", ctx.slot, ctx.count));
        let frame = format!(
            "F4440D000C{}{}{}{}00",
            encoder::le32(ctx.conn.session.id),
            encoder::le16(map_id),
            encoder::le16(x),
            encoder::le16(y)
        );
        ctx.out.send(frame);
        ctx.consume().await;
        return;
    }

    // --- 2. Add-pet items (C# `_AddPet > 10000`). ---
    let add_pet = ctx
        .data
        .items
        .get(&i64::from(ctx.id))
        .map(|i| i.add_pet)
        .unwrap_or(0);
    if add_pet > 10000 {
        let pet_id = add_pet as u16;
        if ctx.conn.session.pets.iter().any(|p| p.id == pet_id) {
            ctx.red("Ban da co pet nay roi");
            return;
        }
        if ctx.conn.session.pets.len() < 4 {
            let stt = (1..=4)
                .find(|s| !ctx.conn.session.pets.iter().any(|p| p.stt == *s))
                .unwrap_or(1);
            ctx.conn.session.pets.push(crate::server::session::PetState {
                stt,
                id: pet_id,
                level: 1,
                hp: 100,
                hp_max: 100,
                sp: 100,
                sp_max: 100,
                ..Default::default()
            });
            ctx.consume().await;
            return;
        }
        ctx.red("Pet box full");
        return;
    }

    // --- 3. Leader-only sleep item (C# 46167). ---
    if ctx.id == 46167 {
        let leader_ok = ctx.conn.session.id == ctx.conn.session.id_leader
            || ctx.conn.session.id_leader == 0;
        if leader_ok {
            ctx.sleep().await;
            ctx.consume().await;
        }
        return;
    }

    // --- 4. The big else chain (C# 4039-5353), in order. ---
    if reborn::handle(ctx).await {
        return;
    }
    if books::handle(ctx).await {
        return;
    }
    if rewards::handle(ctx).await {
        return;
    }
    if misc::handle(ctx).await {
        return;
    }

    // --- 5. Point / SkillPoint books (C# case 50010/50011). No consume. ---
    match ctx.id {
        50010 => {
            ctx.conn.session.point += 1;
            persist::update_player(
                ctx.pool,
                ctx.conn.session.id,
                "Point",
                i64::from(ctx.conn.session.point),
            )
            .await;
            ctx.stat(0x26, i32::from(ctx.conn.session.point));
            ctx.end_feedback();
            return;
        }
        50011 => {
            ctx.conn.session.skill_point += 1;
            persist::update_player(
                ctx.pool,
                ctx.conn.session.id,
                "SkillPoint",
                i64::from(ctx.conn.session.skill_point),
            )
            .await;
            ctx.stat(0x25, i32::from(ctx.conn.session.skill_point));
            ctx.end_feedback();
            return;
        }
        _ => {}
    }

    // --- 6. Party-buff / special frames (C# 46092 / 46041 / 46093). ---
    match ctx.id {
        46092 => {
            ctx.out.send("F44404000B0702FF".to_string());
            ctx.consume().await;
            return;
        }
        46041 | 46093 => {
            ctx.out.send("F44404000B09FF01".to_string());
            ctx.consume().await;
            return;
        }
        _ => {}
    }

    // --- 7. Generic potion path (C# default: `Hp*Sp*Fai1` × count). ---
    potion(ctx).await;
}

/// Generic potion restore (C# `Client.cs:5187-5323`). In battle: silent return.
/// Use-type 0 = player (Hp/Sp only; Fai ignored), 1..4 = pet slot (Hp/Sp/Fai,
/// Fai capped at 100). A potion with Hp/Sp>0 is always consumed (C# trailing
/// `HomdoUseHPSPFAI`), sending stat packets even when already at max. A truly
/// statless unknown item ends silently (no frames, no consume).
async fn potion(ctx: &mut UseCtx<'_>) {
    if ctx.conn.session.battle_id > 0 {
        return;
    }
    let info = ctx.data.items.get(&i64::from(ctx.id));
    let hp_amt = info
        .map(|i| i.hp.saturating_mul(i64::from(ctx.count)))
        .unwrap_or(0);
    let sp_amt = info
        .map(|i| i.sp.saturating_mul(i64::from(ctx.count)))
        .unwrap_or(0);
    let fai_amt = info
        .map(|i| i.fai1.saturating_mul(i64::from(ctx.count)))
        .unwrap_or(0);

    match ctx.use_type {
        0 => {
            // Player: C# handles Hp and Sp only (Fai ignored for players).
            let mut touched = false;
            if hp_amt > 0 {
                if ctx.conn.session.hp < ctx.conn.session.hp_max {
                    let new = (i64::from(ctx.conn.session.hp) + hp_amt)
                        .min(i64::from(ctx.conn.session.hp_max)) as u16;
                    ctx.conn.session.hp = new;
                    persist::update_player(
                        ctx.pool,
                        ctx.conn.session.id,
                        "Hp",
                        i64::from(new),
                    )
                    .await;
                }
                // C# always re-broadcasts the Hp stat (current or new value).
                ctx.stat(0x19, i32::from(ctx.conn.session.hp));
                touched = true;
            }
            if sp_amt > 0 {
                if ctx.conn.session.sp < ctx.conn.session.sp_max {
                    let new = (i64::from(ctx.conn.session.sp) + sp_amt)
                        .min(i64::from(ctx.conn.session.sp_max)) as u16;
                    ctx.conn.session.sp = new;
                    persist::update_player(
                        ctx.pool,
                        ctx.conn.session.id,
                        "Sp",
                        i64::from(new),
                    )
                    .await;
                }
                ctx.stat(0x1A, i32::from(ctx.conn.session.sp));
                touched = true;
            }
            if touched {
                ctx.consume().await;
            }
        }
        1..=4 => {
            // Pet: require the pet slot to exist (C# `PetGetData(_ID) <= 0 → break`).
            let stt = ctx.use_type;
            let Some(pet_idx) = ctx.conn.session.pets.iter().position(|p| p.stt == stt) else {
                return;
            };
            if ctx.conn.session.pets[pet_idx].id == 0 {
                return;
            }
            let mut touched = false;
            if hp_amt > 0 {
                if ctx.conn.session.pets[pet_idx].hp < ctx.conn.session.pets[pet_idx].hp_max {
                    let new = (i64::from(ctx.conn.session.pets[pet_idx].hp) + hp_amt)
                        .min(i64::from(ctx.conn.session.pets[pet_idx].hp_max)) as u16;
                    ctx.conn.session.pets[pet_idx].hp = new;
                    persist::upsert_pet(
                        ctx.pool,
                        ctx.conn.session.id,
                        &ctx.conn.session.pets[pet_idx],
                    )
                    .await;
                }
                ctx.pet_stat(stt, 0x19, i32::from(ctx.conn.session.pets[pet_idx].hp));
                touched = true;
            }
            if sp_amt > 0 {
                if ctx.conn.session.pets[pet_idx].sp < ctx.conn.session.pets[pet_idx].sp_max {
                    let new = (i64::from(ctx.conn.session.pets[pet_idx].sp) + sp_amt)
                        .min(i64::from(ctx.conn.session.pets[pet_idx].sp_max)) as u16;
                    ctx.conn.session.pets[pet_idx].sp = new;
                    persist::upsert_pet(
                        ctx.pool,
                        ctx.conn.session.id,
                        &ctx.conn.session.pets[pet_idx],
                    )
                    .await;
                }
                ctx.pet_stat(stt, 0x1A, i32::from(ctx.conn.session.pets[pet_idx].sp));
                touched = true;
            }
            if fai_amt > 0 {
                if ctx.conn.session.pets[pet_idx].fai < 100 {
                    let new = (i64::from(ctx.conn.session.pets[pet_idx].fai) + fai_amt).min(100)
                        as u16;
                    ctx.conn.session.pets[pet_idx].fai = new;
                    persist::upsert_pet(
                        ctx.pool,
                        ctx.conn.session.id,
                        &ctx.conn.session.pets[pet_idx],
                    )
                    .await;
                }
                ctx.pet_stat(stt, 0x40, i32::from(ctx.conn.session.pets[pet_idx].fai));
                touched = true;
            }
            if touched {
                ctx.consume().await;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::session::InventoryItem;

    fn item(id: u16, count: u8) -> InventoryItem {
        InventoryItem {
            slot: 1,
            id,
            count,
            ..Default::default()
        }
    }

    fn seeded() -> DotNetRandom {
        DotNetRandom::new(42)
    }

    #[tokio::test]
    async fn potion_restores_hp_and_ends_standard() {
        let mut conn = Conn::new();
        conn.session.hp = 50;
        conn.session.hp_max = 200;
        conn.session.sp = 30;
        conn.session.sp_max = 200;
        conn.session.homdo.push(item(30001, 5));
        let mut data = GameData::default();
        data.items.insert(
            30001,
            crate::data::tables::Item {
                id: 30001,
                hp: 100,
                sp: 50,
                ..Default::default()
            },
        );
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        // slot 1, count 2, use_type 0
        use_item_rng(&mut conn, &[1, 2, 0], &mut out, None, &data, &mut rng).await;

        assert_eq!(conn.session.hp, 200); // 50 + 200 capped
        assert_eq!(conn.session.sp, 130); // 30 + 100
        assert_eq!(conn.session.homdo[0].count, 3); // 5 - 2
        assert_eq!(
            out.outgoing,
            vec![
                "F4440C0008011901C800000000000000".to_string(), // Hp -> 200
                "F4440C0008011A018200000000000000".to_string(), // Sp -> 130
                "F444040017090102".to_string(),                 // 1709 slot 1, used 2
                "F4440200170F".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn warp_item_moves_map_and_consumes() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        conn.session.homdo.push(item(46022, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

        assert_eq!(conn.session.map_id, 12403);
        assert_eq!(conn.session.map_x, 442);
        assert_eq!(conn.session.map_y, 375);
        assert!(conn.session.homdo.is_empty(), "warp item consumed");
        assert!(out.outgoing.iter().any(|f| f.contains("17090101")));
        assert!(out.outgoing.iter().any(|f| f.starts_with("F4440D000C")));
    }

    #[tokio::test]
    async fn add_pet_item_gives_pet() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(46001, 1));
        let mut data = GameData::default();
        data.items.insert(
            46001,
            crate::data::tables::Item {
                id: 46001,
                add_pet: 10001,
                ..Default::default()
            },
        );
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

        assert_eq!(conn.session.pets.len(), 1);
        assert_eq!(conn.session.pets[0].id, 10001);
        assert!(conn.session.homdo.is_empty(), "pet item consumed");
    }

    #[tokio::test]
    async fn point_book_adds_point_and_keeps_item() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(50010, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;

        assert_eq!(conn.session.point, 1);
        assert_eq!(
            conn.session.homdo.len(),
            1,
            "point book is not consumed (C# quirk)"
        );
        assert!(out
            .outgoing
            .iter()
            .any(|f| f.starts_with("F4440C0008012601")));
    }

    #[tokio::test]
    async fn unknown_zero_effect_item_is_silent() {
        // Truly statless unknown item: no frames, no consume (C# edge).
        let mut conn = Conn::new();
        conn.session.homdo.push(item(40001, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
        assert_eq!(conn.session.homdo.len(), 1, "not consumed");
        assert!(out.outgoing.is_empty(), "silent: no frames");
    }

    #[tokio::test]
    async fn noop_ids_send_end_frame_without_consume() {
        for id in [46013u16, 46014, 46015, 46042, 46091] {
            let mut conn = Conn::new();
            conn.session.homdo.push(item(id, 1));
            let data = GameData::default();
            let mut out = HandleOutcome::default();
            let mut rng = seeded();
            use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
            assert_eq!(conn.session.homdo.len(), 1, "id {id}: not consumed");
            assert!(
                out.outgoing.iter().any(|f| f == "F444040017090101"),
                "id {id}: end feedback present"
            );
            assert!(out.outgoing.iter().any(|f| f == "F4440200170F"));
        }
    }

    #[tokio::test]
    async fn skill_book_learns_at_level_ten() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(46230, 1));
        let mut data = GameData::default();
        data.skills.insert(
            10016,
            crate::data::tables::Skill {
                id: 10016,
                name: "Ky Nang".into(),
                sp: 10,
                ..Default::default()
            },
        );
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skills[0], (10016, 10));
        assert!(conn.session.homdo.is_empty(), "skill book consumed");
        assert!(out
            .outgoing
            .iter()
            .any(|f| f.starts_with("F4440C0008016E01")));
    }

    #[tokio::test]
    async fn texp_book_adds_exp() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(46211, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
        assert_eq!(conn.session.texp, 106); // session starts at texp 6 + 100
        assert!(conn.session.homdo.is_empty());
    }

    #[tokio::test]
    async fn potion_at_full_still_consumes() {
        let mut conn = Conn::new();
        conn.session.hp = 200;
        conn.session.hp_max = 200;
        conn.session.sp = 200;
        conn.session.sp_max = 200;
        conn.session.homdo.push(item(30001, 5));
        let mut data = GameData::default();
        data.items.insert(
            30001,
            crate::data::tables::Item {
                id: 30001,
                hp: 100,
                sp: 50,
                ..Default::default()
            },
        );
        let mut out = HandleOutcome::default();
        let mut rng = seeded();
        use_item_rng(&mut conn, &[1, 1], &mut out, None, &data, &mut rng).await;
        assert_eq!(conn.session.hp, 200);
        assert_eq!(conn.session.homdo[0].count, 4, "consumed even at full");
        assert!(out.outgoing.iter().any(|f| f == "F444040017090101"));
    }
}
