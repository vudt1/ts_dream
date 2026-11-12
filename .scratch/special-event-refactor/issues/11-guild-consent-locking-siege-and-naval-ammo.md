# 11: Guild Lifecycle Consent Flow, Two-Phase State Locking, City Siege Dispatcher Routing & Naval Ammo Consumption

**What to build:** From a guild member's perspective, guild invitations require the invited player's explicit acceptance before joining, both client sub-opcode sets (`0x04 | 0x07` Kick, `0x05 | 0x0A` Leadership Transfer, `0x06 | 0x12` Badge Upload) are supported seamlessly, all two-player guild mutations are protected by Two-Phase State Locking and atomic database checks, and invalid guild actions return clear system messages. City Siege (`0x29`) and Arena/Event notices (`0x25`) route through the live packet dispatcher, entering Water War (`0x3F 0x01`) provisions 10 naval ammunition units (`id = 46080` in slot 1) that Naval Cannon attacks (`0x40 0x01`) consume, and emergency event drains clear temporary boat inventories.

**Blocked by:** 09: Event Domain Prefactoring, Opcode Constants & Natural Multi-Session Test Flow

**Status:** done

- [x] Guild Invite (`C→S 0x27 0x02`) records `pending_guild_invite: Option<(u32, u32)>` (`(inviter_id, guild_id)`) on the target player's session and forwards `S→C 27 02 [inviter_id][guild_id]` without inserting into `guild_members` or mutating `target.guild_id`; membership is only committed to SQLite and synced to online state when the target sends Guild Accept (`C→S 0x27 0x03`, `choice == 1`), while `choice == 0` clears the pending invite and notifies refusal.
- [x] Guild handler supports both sub-opcode aliases: `0x04 | 0x07` for Kick, `0x05 | 0x0A` for Leadership Transfer, and `0x06 | 0x12` for 288-byte Badge Upload.
- [x] Guild Kick (`0x27 0x04 | 0x07`) and Guild Accept (`0x27 0x03`) acquire `lock_player_operations([min(id1, id2), max(id1, id2)])` before reading or mutating partner session state.
- [x] Database mutations in Guild Invite Accept, Kick, and Leadership Transfer check `Result` outcomes before updating in-memory session state (no ignored `let _ =` database errors) and abort on failure.
- [x] Invalid guild requests (self-invite, self-kick, self-transfer, uninvited accept, non-leader kick/transfer, or acting with `guild_id == 0`) emit a descriptive system message (`sys_msg_frame` on `020102`) instead of silently dropping the packet.
- [x] Level-1 packet dispatcher routes City Siege (`0x29`) and Arena/Event Notice (`0x25`) opcodes to their respective handlers.
- [x] Entering Water War (`C→S 0x3F 0x01`) initializes the 25-slot `boat_inventory` with 10 default naval ammo units (`id = 46080, count = 10` in slot 1). Naval Cannon fire (`C→S 0x40 0x01`) deducts 1 ammo unit from the first non-empty slot in `boat_inventory` and rejects the shot with `sys_msg_frame("Khong du dan duoc thuy chien!")` when all 25 slots are empty; `emergency_cascading_stop` clears `boat_inventory` and sends `S→C 17 65` to affected participants.
- [x] End-to-end dispatcher tests cover the two-step invite/accept flow, uninvited accept rejection, sub-opcode aliases, concurrent kick locking, City Siege dispatcher routing, and naval ammo consumption/depletion rejection.
