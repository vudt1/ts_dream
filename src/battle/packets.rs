//! Battle packet builders (Chapter 6 §6.8).
//!
//! All packets are hex strings, byte-for-byte stable.
//! Key frames use the `F444` magic + LE16 length encoding.

use crate::protocol::encoder;

/// Battle board open: `F444` + LE16(payload len) + `0BFA` + LE16(diahinh) + `04`
/// + `_Packet` + trailer.
///
/// The self record marker is `04`, **not** `03`/`05`: the client writes the
/// record's `kind` byte straight into `player+0x376` (`006438ac:262`) and only
/// `0x376 == 4` unlocks the "leave battle" button (`00590ba8:159`) and the
/// `0x376 = 0` reset inside `0B 00` (`00641fa4:69-76`). Sending `03`/`05` traps
/// the client in the battle — do not change back.
///
/// The frame length is computed from the payload (as [`battle_open_text`] and
/// [`battle_open_pk_member`] already do) instead of the former hardcoded `1C`.
/// With the current 23-byte leader snapshot the payload is 28 bytes (`0x1C`),
/// so the emitted bytes are unchanged.
pub fn battle_open_leader(diahinh: u16, leader_packet: &str) -> String {
    let payload = format!("0BFA{}04{}", encoder::le16(diahinh), leader_packet);
    let payload_len = payload.len() / 2;
    format!(
        "F444{}{}F44403000B0A01",
        encoder::le16(payload_len as u16),
        payload
    )
}

/// Battle open frame for member/NPC battles with arbitrary text.
pub fn battle_open_text(diahinh: u16, text: &str) -> String {
    let payload = format!("0BFA{}{}", encoder::le16(diahinh), text);
    let payload_len = payload.len() / 2;
    format!(
        "F444{}{}F44403000B0A01",
        encoder::le16(payload_len as u16),
        payload
    )
}

/// PK member frame: uses the intentional fixed `7000` magic instead of DiaHinh.
///
/// The `7000` word is deliberate — it is the PK sentinel written where a normal
/// open frame carries `le16(diahinh)` (`0B FA` `RP[1..2]` mode/sub). Do not
/// "fix" it to a terrain id.
pub fn battle_open_pk_member(text: &str) -> String {
    let payload = format!("0BFA7000{}", text);
    let payload_len = payload.len() / 2;
    format!(
        "F444{}{}F44403000B0A01",
        encoder::le16(payload_len as u16),
        payload
    )
}

/// NPC entity in grid: `F4441A000B0503` + `_Packet`.
pub fn entity_npc(packet: &str) -> String {
    format!("F4441A000B0503{}", packet)
}

/// Player/pet entity in grid: `F4441A000B0505` + `_Packet`.
pub fn entity_player(packet: &str) -> String {
    format!("F4441A000B0505{}", packet)
}

/// Show entity on map (in battle): `F4440A000B0402` + LE32(id) + tail.
pub fn show_on_map(id: u32, tail: &str) -> String {
    format!("F4440A000B0402{}{}", encoder::le32(id), tail)
}

/// Show on map — standard player tail `000003`.
pub fn show_player_on_map(id: u32) -> String {
    show_on_map(id, "000003")
}

/// Show on map — member tail `000005`.
pub fn show_member_on_map(id: u32) -> String {
    show_on_map(id, "000005")
}

/// Show on map — PK leader tail `000002`.
pub fn show_pk_leader_on_map(id: u32) -> String {
    show_on_map(id, "000002")
}

/// Hide entity from map: `F44408000B00` + LE32(id) + `0000`.
pub fn hide_from_map(id: u32) -> String {
    format!("F44408000B00{}0000", encoder::le32(id))
}

/// Property ids for the in-battle `33 01` unit-stat record.
///
/// Source: client `aLogin.exe` `FUN_0064cae0` (`02_battle_positions.md §2.5`) —
/// `flag 0x19` writes HP (`unit+0x3EA`) and `flag 0x1A` writes SP (`unit+0xFB`).
/// With `0x19` and `value > 0` the client also clears the unit state and plays
/// the revive effect (`0064cae0.c:124-180`).
pub mod unit_prop {
    pub const HP: u8 = 0x19;
    pub const SP: u8 = 0x1A;
}

/// One `33 01` record (7 bytes): `[grp][slot][propId][value u32 LE]`.
///
/// Client layout (`FUN_0064cae0.c:86-121`, index convention per
/// `HANDOFF-client-reverse.md §0.2`): record `i` starts at `content[2 + 7i]`
/// with grp at `+0`, slot at `+1`, prop id at `+2`, value u32 LE at `+3..+6`.
pub fn set_unit_prop(row: u8, col: u8, prop_id: u8, value: i64) -> String {
    let value = value.clamp(0, u32::MAX as i64) as u32;
    format!("{:02X}{:02X}{:02X}{}", row, col, prop_id, encoder::le32(value))
}

/// `33 01` frame packing one or more 7-byte records:
/// `F444` + LE16(2 + 7n) + `3301` + records.
///
/// This is the only frame that refreshes an in-battle unit's HP/SP bar.
pub fn set_unit_prop_frame(records: &[(u8, u8, u8, i64)]) -> String {
    let mut payload = String::from("3301");
    for &(row, col, prop_id, value) in records {
        payload.push_str(&set_unit_prop(row, col, prop_id, value));
    }
    format!("F444{}{}", encoder::le16(payload.len() as u16 / 2), payload)
}

/// Single-record HP update (`33 01` propId `0x19`). A `value > 0` on a dead
/// unit triggers the client revive effect.
pub fn set_hp(row: u8, col: u8, value: i64) -> String {
    set_unit_prop_frame(&[(row, col, unit_prop::HP, value)])
}

/// Single-record SP update (`33 01` propId `0x1A`).
pub fn set_sp(row: u8, col: u8, value: i64) -> String {
    set_unit_prop_frame(&[(row, col, unit_prop::SP, value)])
}

/// Reset ("dọn") one battle-grid cell: `F44404000B01` + row + col.
///
/// Payload is exactly 4 bytes `[0B][01][grp][slot]`. The client handler
/// `FUN_0064bb24` reads only `content[2]` (`row`, bound `<= 3`) and
/// `content[3]` (`col`, bound `<= 4`) and ignores everything after them, so a
/// trailing `00` pad byte is optional (evidence `0064bb24:50-62`,
/// `HANDOFF-client-reverse.md §3.2`).
///
/// This frame only resets the cell / replays the leave FX — it does **not**
/// move a unit on the grid (that is [`move_cell`], `35 07`).
pub fn reset_cell(row: u8, col: u8) -> String {
    format!("F44404000B01{:02X}{:02X}", row, col)
}

/// Move a unit between grid cells: `F44406003507` + srcRow + srcCol + dstRow + dstCol.
///
/// Content is `[35][07][srcGrp][srcSlot][dstGrp][dstSlot]` (6 bytes); the
/// coordinate payload after `35 07` is 4 bytes. `srcGrp`/`dstGrp <= 3` and
/// `srcSlot`/`dstSlot <= 4`.
///
/// Source: client `aLogin.exe` `FUN_0064e3e0`
/// (`.scratch/client-pseudo-op-code/battle/02_battle_positions.md §2.10`,
/// `§1.7`). This is the **only** frame that updates a grid cell after the
/// battle opens — `0B 01` does not.
///
/// NOTE: the engine has no unit-movement call-site yet, so this builder is
/// shipped (and tested) unused; wiring it into the movement flow is deferred
/// to a later ticket.
pub fn move_cell(src_row: u8, src_col: u8, dst_row: u8, dst_col: u8) -> String {
    format!(
        "F44406003507{:02X}{:02X}{:02X}{:02X}",
        src_row, src_col, dst_row, dst_col
    )
}

/// Force the unit standing in a grid cell into state 7 (attack animation):
/// `F44404003503` + row + col.
///
/// Content is `[35][03][grp][slot]` — 4 bytes, with only two coordinate bytes
/// after `35 03` (`grp <= 3`, `slot <= 4`), so the `LEN` field is 4.
///
/// Source: client `aLogin.exe` `FUN_0064cfec`
/// (`.scratch/client-pseudo-op-code/battle/02_battle_positions.md §2.7`): the
/// handler reads `content[2]` (`grp`, `BoundErr` when `> 3`) and `content[3]`
/// (`slot`, `BoundErr` when `> 4`) and, only when the target cell is occupied
/// (`cell+0x48 != 0`), flushes the 21 FX slots and calls
/// `FUN_00647708(unit, 7)` on the unit in that cell. Unlike
/// [`set_unit_prop_frame`] it does not change HP/SP and unlike [`move_cell`]
/// it does not move the unit — it only **forces** the animation state.
///
/// NOTE: shipped (and tested) **unused**. The engine has no "attack animation"
/// call-site, and **when** the client expects this frame must come from a real
/// session capture (ticket 14); do not call it from `runner.rs` yet.
///
// ============================================================================
// Opcode 0x35: In-Battle Action & Status Synchronization
// ============================================================================
//
// Client Architecture & Reverse-Engineering Evidence:
// ----------------------------------------------------
// 1. Two-level Dispatcher (`FUN_0078a89c`):
//    - `byte[opcode + 0x78A8EE]` translates Opcode `0x35` -> Case Index 46 (`0x2E`),
//      which jumps to `FUN_007952ab`.
//    - (Note: Case Index 53 (`0x35`) belongs to Opcode `0x3C`, which was previously confused with 0x35).
// 2. Sub-Opcode Handlers in `FUN_007952ab`:
//    - Sub 0x01 (`FUN_0064cd74`): Purge / clear status effects (also used by Bear C#).
//    - Sub 0x03 (`FUN_0064cfec`): Force animation state 7 (`force_state7`, attack preparation).
//    - Sub 0x04 (`FUN_0064d128`): Battle drop animation (`giveDrop` in Bear C#).
//    - Sub 0x05 (`FUN_0064dfa4`): Unit acting indicator (`acting`).
//    - Sub 0x07 (`FUN_0064e3e0`): Grid cell movement (`move_cell`).
//    - Sub 0x0E (`FUN_0064fbd8`): Unit action: invokes unit vtable `(**(code **)(*unit + 0x1c))(unit, val, 1)`.
//    - Sub 0x0F (`FUN_0064fd34`): Unit status buff/debuff: writes unit status slots and invokes
//      `FUN_0058f5e0` to display visual icons and tooltip overlays on the in-battle HUD.
//
// Specific Buff / Debuff Categories for 35 0F (`status_char` 'B'..'G'):
// ---------------------------------------------------------------------
// In client function `FUN_0064fd34` and formatting helper `FUN_0058faf0`, characters 'B'..'G'
// correspond directly to the unit's HUD status slots and VISCII string literals in `aLogin.exe`:
//
//   - 'B' (0x42) `STATUS_DEF_MODIFIER`: "Sức phòng ngự" (Defense modifier).
//     * Client struct slot: `unit + 0x5be` (char flag/turns), `unit + 0x5bf` (ushort value LE).
//     * Value encoding (base 1000):
//       - `val >= 1000`: buff displays "+{val - 1000}" (e.g. 1050 displays "+50" DEF).
//       - `val < 1000`: debuff displays "-{1000 - val}" (e.g. 950 displays "-50" DEF).
//
//   - 'C' (0x43) `STATUS_ATK_MODIFIER`: "Sức tấn công" (Attack modifier).
//     * Client struct slot: `unit + 0x5c1` (char flag/turns), `unit + 0x5c2` (ushort value LE).
//     * Value encoding (base 1000):
//       - `val >= 1000`: buff displays "+{val - 1000}" (e.g. 1080 displays "+80" ATK).
//       - `val < 1000`: debuff displays negative reduction.
//
//   - 'D' (0x44) `STATUS_INT_MODIFIER`: "Trí lực" (Intelligence modifier).
//     * Client struct slot: `unit + 0x5c4` (char flag/turns), `unit + 0x5c5` (ushort value LE).
//     * Value encoding (base 1000):
//       - `val >= 1000`: buff displays "+{val - 1000}" (e.g. 1040 displays "+40" INT).
//       - `val < 1000`: debuff displays negative reduction.
//
//   - 'E' (0x45) `STATUS_DMG_BONUS_PCT`: "Sức tổn thương+%d%%" (Damage bonus percentage).
//     * Client struct slot: `unit + 0x5c7` (char flag/turns), `unit + 0x5c8` (ushort value LE).
//     * Formatted string literal at VA 0x58FD38: "Sức tổn thương+%d%%" (e.g. 20 displays "+20%").
//
//   - 'F' (0x46) `STATUS_SEAL_RESIST_PCT`: "Đề kháng kỹ phong ấn+%d%%" (Seal/disable resistance %).
//     * Client struct slot: `unit + 0x5ca` (char flag/turns), `unit + 0x5cb` (ushort value LE).
//     * Formatted string literal at VA 0x58FD54: "Đề kháng kỹ phong ấn+%d%%" (e.g. 50 displays "+50%").
//
//   - 'G' (0x47) `STATUS_SP_REDUCE_PCT`: "Mỗi hiệp SP giảm+%d%%" (SP consumption reduction %).
//     * Client struct slot: `unit + 0x5cd` (char flag/turns), `unit + 0x5ce` (ushort value LE).
//     * Formatted string literal at VA 0x58FD78: "Mỗi hiệp SP giảm+%d%%" (e.g. 30 displays "+30%").
//
// Flag / Turns parameter in 35 0F:
// --------------------------------
// - `flag_or_turns == 0`: Clears/removes the status slot from HUD and shifts remaining icons.
// - `flag_or_turns > 0`: Active duration in turns or stack counter displayed in client HUD.
//
// Comparison with Bear C# Server:
// --------------------------------
// - Bear (`TS_Server_Bear/TS_Server/Server/BattleClasses/BattleParticipant.cs`) tracks durations internally
//   (`buff`, `debuff`, `disable`, `aura`) and clears them when reaching 0 via Opcode 53 sub 1 (`35 01`):
//   `new byte[7] { 53, 1, row, col, purge_type, 0, 0 }` (where 1=disable, 2=buff, 3=debuff, 5=aura).
// - Bear does NOT implement `35 0E` or `35 0F`. Consequently, in Bear the client never displays
//   the visual HUD buff icons ('B'..'G') or their tooltips.
// - Rust `ts_dream` supports both: `35 0F` for visual buff/debuff HUD parity and `35 01` for status purging!

/// Status category: Defense stat modifier ('B', 0x42). Text: "Sức phòng ngự".
pub const STATUS_DEF_MODIFIER: u8 = b'B';

/// Status category: Attack stat modifier ('C', 0x43). Text: "Sức tấn công".
pub const STATUS_ATK_MODIFIER: u8 = b'C';

/// Status category: Intelligence stat modifier ('D', 0x44). Text: "Trí lực".
pub const STATUS_INT_MODIFIER: u8 = b'D';

/// Status category: Damage bonus percentage ('E', 0x45). Text: "Sức tổn thương+%d%%".
pub const STATUS_DMG_BONUS_PCT: u8 = b'E';

/// Status category: Seal/disable resistance percentage ('F', 0x46). Text: "Đề kháng kỹ phong ấn+%d%%".
pub const STATUS_SEAL_RESIST_PCT: u8 = b'F';

/// Status category: Per-turn SP reduction percentage ('G', 0x47). Text: "Mỗi hiệp SP giảm+%d%%".
pub const STATUS_SP_REDUCE_PCT: u8 = b'G';

/// Purge type: Disable / crowd control (sleep, freeze, stun, confuse).
pub const PURGE_TYPE_DISABLE: u8 = 1;

/// Purge type: Positive buff.
pub const PURGE_TYPE_BUFF: u8 = 2;

/// Purge type: Negative debuff.
pub const PURGE_TYPE_DEBUFF: u8 = 3;

/// Purge type: Aura effect.
pub const PURGE_TYPE_AURA: u8 = 5;

/// Helper to encode a relative stat modifier for status types `'B'`, `'C'`, `'D'`.
///
/// Base value is 1000 in client `aLogin.exe` `FUN_0058faf0`:
/// - `delta = +50` -> `1050` (displays "+50")
/// - `delta = -50` -> `950` (displays "-50")
pub fn stat_modifier_val(delta: i16) -> u16 {
    (1000i32 + delta as i32).clamp(0, 65535) as u16
}

/// Force the unit standing in a grid cell into state 7 (attack animation):
/// `F44404003503` + row + col.
///
/// Handled in client by `FUN_0064cfec` (`case 46 / sub 0x03`). Flushes FX slots and
/// calls `FUN_00647708(unit, 7)` to set attack stance animation.
pub fn force_state7(row: u8, col: u8) -> String {
    format!("F44404003503{:02X}{:02X}", row, col)
}

/// Unit action frame: `F4440600350E` + row + col + LE16(val).
///
/// Handled in client by `FUN_0064fbd8` (`case 46 / sub 0x0E`), which resolves the
/// unit at `(row, col)` on grid and executes `(**(code **)(*unit + 0x1c))(unit, val, 1)`.
/// Length is 6 bytes payload (`06 00`).
pub fn unit_action(row: u8, col: u8, val: u16) -> String {
    format!(
        "F4440600350E{:02X}{:02X}{}",
        row,
        col,
        encoder::le16(val)
    )
}

/// Alias for [`unit_action`].
pub fn battle_unit_action(row: u8, col: u8, val: u16) -> String {
    unit_action(row, col, val)
}

/// Unit status buff/debuff frame: `F4440800350F` + row + col + status_char + flag_or_turns + LE16(val).
///
/// Handled in client by `FUN_0064fd34` (`case 46 / sub 0x0F`), which updates the
/// unit status slots (`unit + 0x5be`..'G') and calls `FUN_0058f5e0` to refresh the
/// in-battle status / buff icon display and tooltips on the client HUD.
/// Length is 8 bytes payload (`08 00`).
pub fn battle_unit_status(
    row: u8,
    col: u8,
    status_char: u8,
    flag_or_turns: u8,
    val: u16,
) -> String {
    format!(
        "F4440800350F{:02X}{:02X}{:02X}{:02X}{}",
        row,
        col,
        status_char,
        flag_or_turns,
        encoder::le16(val)
    )
}

/// Alias for [`battle_unit_status`].
pub fn unit_status(
    row: u8,
    col: u8,
    status_char: u8,
    flag_or_turns: u8,
    val: u16,
) -> String {
    battle_unit_status(row, col, status_char, flag_or_turns, val)
}

/// Clear / remove a specific status category ('B'..'G') from unit HUD:
/// `F4440800350F` + row + col + status_char + `00` + `0000`.
///
/// Calling `battle_unit_status` with `flag_or_turns == 0` causes `FUN_0058f5e0`
/// to purge the status slot and shift remaining slots.
pub fn clear_unit_status(row: u8, col: u8, status_char: u8) -> String {
    battle_unit_status(row, col, status_char, 0, 0)
}

/// Purge status frame: `F44407003501` + row + col + purge_type + `0000`.
///
/// Handled in client by `FUN_0064cd74` (`case 46 / sub 0x01`).
/// Matches Bear C# server implementation (`[53, 1, row, col, purge_type, 0, 0]`):
/// - `purge_type = 1`: disable / control expired or cured
/// - `purge_type = 2`: buff expired or dispelled
/// - `purge_type = 3`: debuff expired or cleansed
/// - `purge_type = 5`: aura expired
pub fn purge_status(row: u8, col: u8, purge_type: u8) -> String {
    format!("F44407003501{:02X}{:02X}{:02X}0000", row, col, purge_type)
}

/// Your turn prompt: `F44402003401`.
pub fn your_turn() -> String {
    "F44402003401".to_string()
}

/// Turn action finish frame: `F44402003202` (ticket 21).
///
/// Sent after all 0x32 01 action animations in a turn have finished (`FUN_0064dd74`).
/// Tells the client to set `[TFightManage+0x1C] = 1`, release animation queues and
/// prepare for the next turn.
pub fn turn_action_finish() -> String {
    "F44402003202".to_string()
}

/// Acting indicator: `F44404003505` + row + col.
pub fn acting(row: u8, col: u8) -> String {
    format!("F44404003505{:02X}{:02X}", row, col)
}

/// Combo packet: `F444130032010F00` + row + col + `264E0101` + row + col + `010301E0000000`.
pub fn combo(row: u8, col: u8) -> String {
    format!(
        "F444130032010F00{:02X}{:02X}264E0101{:02X}{:02X}010301E0000000",
        row, col, row, col
    )
}

/// Buff end: `F44407003501` + row + col + troi_end + `0000`.
pub fn troi_end(row: u8, col: u8, troi_end_byte: u8) -> String {
    format!(
        "F44407003501{:02X}{:02X}{:02X}0000",
        row, col, troi_end_byte
    )
}

/// Buff start on caster: `F44407003501` + row + col + `01` + LE16(skillId).
pub fn troi_start(row: u8, col: u8, skill_id: u16) -> String {
    format!(
        "F44407003501{:02X}{:02X}01{}",
        row,
        col,
        encoder::le16(skill_id)
    )
}

/// Drop reward: `F44408003504` + LE16(itemId) + npcRow + npcCol + row + col.
pub fn drop_item(item_id: u16, npc_row: u8, npc_col: u8, row: u8, col: u8) -> String {
    format!(
        "F44408003504{}{:02X}{:02X}{:02X}{:02X}",
        encoder::le16(item_id),
        npc_row,
        npc_col,
        row,
        col
    )
}

/// In-battle NPC talk / dialogue banner: `F4440B003506` + k + LE32(actor_id) + LE32(tpl_id).
///
/// Handled in client by `FUN_0064e064` (`case 46 / sub 0x06` in `FUN_007952ab`):
/// - `k`: mode / display flag (1 = dialogue toast banner 2000ms, etc.)
/// - `actor_id`: character / actor ID speaking
/// - `tpl_id`: NPC template ID or dialogue string ID
///
/// Triggers `THuman.Create`, sets template id, puts into battle HUD (`FUN_0063c1d0`)
/// and displays speech banner toast (`FUN_0064e064:83-129`).
pub fn battle_npc_talk(k: u8, actor_id: u32, tpl_id: u32) -> String {
    format!(
        "F4440B003506{:02X}{}{}",
        k,
        encoder::le32(actor_id),
        encoder::le32(tpl_id)
    )
}

/// Helper wrapper for in-battle NPC dialogue toast following Bear C# convention.
///
/// Equivalent to Bear `PacketCreator(53, 6)` (`k = 1`, `actor_id = 0`, `tpl_id = dialog_id as u32`).
pub fn battle_npc_dialog(dialog_id: u16) -> String {
    battle_npc_talk(1, 0, dialog_id as u32)
}

/// Battle notice prefix types supported natively by client `FUN_0075c518`.
///
/// - `SYSTEM` (1): Channel/Type 1 (client prepends "Hàng 1" / system style prefix `0x75C6A8`)
/// - `NOTICE` (2): Channel/Type 2 (client prepends "Hàng 2" / notice style prefix `0x75C6B8`)
/// - `COMBAT` (3): Channel/Type 3 (client prepends "Hàng 3" / combat log prefix `0x75C6C8`)
pub mod battle_notice_type {
    pub const SYSTEM: u8 = 1;
    pub const NOTICE: u8 = 2;
    pub const COMBAT: u8 = 3;
}

pub mod notice_type {
    pub use super::battle_notice_type::*;
}

/// Battle chat / notice frame: `F444` + LE16(payload_len) + `350B` + `(type:1B + len:1B + text:len B)*n`.
///
/// Handled in client by `FUN_0075c518` (`case 46 / sub 0x0B` in `FUN_007952ab`).
/// Pushes formatted combat log / announcement lines directly into `TTalkMsgForm`.
/// Supports up to 21 records per frame (`local_20 < 0x15`). Text is VISCII 1.1 encoded.
pub fn battle_chat_notice(entries: &[(u8, &str)]) -> String {
    let mut payload = String::from("350B");
    for &(msg_type, text) in entries.iter().take(21) {
        let bytes = crate::encoding::to_viscii(text);
        let len = bytes.len().min(255) as u8;
        let slice = &bytes[..len as usize];
        payload.push_str(&format!("{:02X}{:02X}{}", msg_type, len, encoder::hex(slice)));
    }
    let payload_len = payload.len() / 2;
    format!("F444{}{}", encoder::le16(payload_len as u16), payload)
}

/// Helper wrapper to send a single system notice (`type = 1`) in battle.
pub fn battle_system_message(text: &str) -> String {
    battle_chat_notice(&[(battle_notice_type::SYSTEM, text)])
}

/// Helper wrapper to send a single notice with explicit type in battle.
pub fn battle_notice(msg_type: u8, text: &str) -> String {
    battle_chat_notice(&[(msg_type, text)])
}


/// Status update: `F4440C000801` + status_byte + sign + LE32(abs_value) + `00000000`.
pub fn status_update(status_byte: u8, value: i32) -> String {
    let sign: u8 = if value >= 0 { 0x01 } else { 0x02 };
    format!(
        "F4440C000801{:02X}{:02X}{}00000000",
        status_byte,
        sign,
        encoder::le32(value.unsigned_abs())
    )
}

/// Gold awarded to a player: `F44406001A01` + LE32(amount).
///
/// Client handler `FUN_0072b084` adds `amount` to `player+0x12F8` (capped at
/// 9,999,999), refreshes the money form and shows the "Nhận được X Tiền"
/// banner (`04_battle_end_rewards.md §2.7`, evidence class A). This is the
/// post-battle gold channel; the byte layout matches the shop sell path
/// (`shops.rs`, `F44406001A01{le32(revenue)}`).
pub fn gold_gain(amount: u32) -> String {
    format!("F44406001A01{}", encoder::le32(amount))
}

/// Battle level-up event: `F4440600050A` + LE32(charId).
///
/// Client handler `FUN_0072f970` reads the character id at `content[2..5]`
/// (`HANDOFF-client-reverse.md §0.2`): when it is the local player it attaches
/// the `"LevelUP"` nameplate label and plays `sound\WA0013.wav`, and for any
/// other known actor it attaches the label only
/// (`.scratch/client-pseudo-op-code/battle/04_battle_end_rewards.md §2.8`,
/// evidence class A). This is the only channel that shows the level-up label —
/// `08 01 23` only changes the level number.
pub fn level_up(char_id: u32) -> String {
    format!("F4440600050A{}", encoder::le32(char_id))
}

/// Battle start trailer: `F44403000B0A01`.
pub fn battle_trailer() -> String {
    "F44403000B0A01".to_string()
}

/// Battle exit UI: `F44402000504`.
pub fn battle_exit_move() -> String {
    "F44402000504".to_string()
}

/// Battle exit talk: `F44402001408`.
pub fn battle_exit_talk() -> String {
    "F44402001408".to_string()
}

/// Skilling effect (10 bytes):
/// row + col + miss_attack + atk_def_lantranh + count_hieuung + troi_hp_sp + LE16(damage) + buff_or_attack.
#[allow(clippy::too_many_arguments)]
pub fn skilling_int(
    row: u8,
    col: u8,
    miss_attack: u8,
    atk_def_lantranh: u8,
    count: u8,
    troi_hp_sp: u8,
    damage: u16,
    buff_or_attack: u8,
) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{}{:02X}",
        row,
        col,
        miss_attack,
        atk_def_lantranh,
        count,
        troi_hp_sp,
        encoder::le16(damage),
        buff_or_attack
    )
}

/// Full `Skilling` effect (17 bytes):
/// `0F00` + row + col + LE16(skillId) + SLdanh + skillType + rowAttack + colAttack +
/// miss + adl + count + troi + LE16(damage) + buff.
///
/// Used for the flee (type-12) and berserk full-effect frames broadcast as
/// `F44413003201` + this payload.
#[allow(clippy::too_many_arguments)]
pub fn skilling_full(
    row: u8,
    col: u8,
    skill_id: u16,
    sl_danh: u8,
    skill_type: u8,
    row_attack: u8,
    col_attack: u8,
    miss_attack: u8,
    adl: u8,
    count: u8,
    troi: u8,
    damage: u16,
    buff: u8,
) -> String {
    format!(
        "0F00{:02X}{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{}{:02X}",
        row,
        col,
        encoder::le16(skill_id),
        sl_danh,
        skill_type,
        row_attack,
        col_attack,
        miss_attack,
        adl,
        count,
        troi,
        encoder::le16(damage),
        buff
    )
}

/// Short `Skilling` (4 bytes) — row + col + miss + adl.
pub fn skilling_short(row: u8, col: u8, miss_attack: u8, adl: u8) -> String {
    format!("{:02X}{:02X}{:02X}{:02X}", row, col, miss_attack, adl)
}

/// `SkillingHieuUng` (3 bytes) — troi + LE16(damage) + buff.
pub fn skilling_effect(troi: u8, damage: u16, buff: u8) -> String {
    format!("{:02X}{}{:02X}", troi, encoder::le16(damage), buff)
}

/// Skill-20007 combo footer sent after a combo turn flush:
/// `F444130032010F00` + row + col + LE16(20007) + `0101` + row + col + `01030119000000`.
pub fn combo_footer_20007(row: u8, col: u8) -> String {
    format!(
        "F444130032010F00{:02X}{:02X}{}0101{:02X}{:02X}01030119000000",
        row,
        col,
        encoder::le16(20007),
        row,
        col
    )
}

/// Berserk / flee miss frame — `F44413003201` + a full Skilling payload.
pub fn skilling_full_frame(payload: &str) -> String {
    format!("F44413003201{payload}")
}

/// TroiBuffHpSp byte constants.
pub mod troi_byte {
    pub const MISS: u8 = 0x00;
    pub const TYPE3: u8 = 0xDD; // 221
    pub const TYPE4: u8 = 0xDE; // 222
    pub const TYPE15: u8 = 0xDF; // 223
    pub const TYPE19: u8 = 0xE1; // 225
    pub const HP: u8 = 0x19; // 25
    pub const SP: u8 = 0x1A; // 26
    pub const HOCHU: u8 = 0x0E; // 14
}

/// TroiBuffEnd byte constants.
pub mod troi_end_byte {
    pub const TYPE3: u8 = 1;
    pub const TYPE4: u8 = 2;
    pub const TYPE15: u8 = 3;
    pub const TYPE19: u8 = 5;
}

/// Attack/Def/Lantranh byte constants.
pub mod attack_status {
    pub const ATTACK: u8 = 0;
    pub const DEF: u8 = 1;
    pub const LANTRANH: u8 = 2;
}

/// Miss/Attack byte constants.
pub mod miss_status {
    pub const ATTACK: u8 = 1;
    pub const MISS: u8 = 0;
}

/// Status bytes for stat push packets.
pub mod stat_byte {
    pub const HP: u8 = 0x19;
    pub const SP: u8 = 0x1A;
    pub const INT: u8 = 0x1B;
    pub const ATK: u8 = 0x1C;
    pub const DEF: u8 = 0x1D;
    pub const AGI: u8 = 0x1E;
    pub const HPX: u8 = 0x1F;
    pub const SPX: u8 = 0x20;
    pub const LV: u8 = 0x23;
    pub const TEXP: u8 = 0x24;
    pub const SKILL_POINT: u8 = 0x25;
    pub const POINT: u8 = 0x26;
}
