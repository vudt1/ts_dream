# 12: Continuous Dungeon 40-NPC Wave Launch, 40-Wave Escalation & Milestone Completion

**What to build:** From the challenger's perspective, entering the 40-NPC Arena map (`10851`) and speaking with the arena challenger NPC (`14628`) launches wave 1 combat preloaded with all 40 escalating NPC waves (resolving enemy formations from `Data/eve.emg` / `GameData.npcs` in production runtime, with deterministic fallback templates when `GameData` is empty in unit tests). Defeating each wave (`1..=39`) seamlessly spawns the next wave inside the existing battle without reloading the scene, and clearing wave 40 concludes the battle, grants milestone rewards (`26051` Đậu Dũng Khí, `23086` Lông Vũ, and `10,000` EXP), resets `QuestStep(218 = 0)`, and returns the challenger to the lobby (`12001`).

**Blocked by:** 09: Event Domain Prefactoring, Opcode Constants & Natural Multi-Session Test Flow

**Status:** done

- [x] Interacting with the 40-NPC Arena challenger NPC (`14628`) on map `10851` when `QuestStep(218) >= 1` starts wave 1 combat and configures 40 escalating NPC waves on the battle instance via `set_waves` (sourcing wave enemy groups from `Data/eve.emg` `scene_eve_data` + `GameData.npcs` when loaded at runtime, and falling back to deterministic NPC templates when `scene_eve_data` is empty in unit tests).
- [x] Defeating all enemies in wave `N` (`1 <= N < 40`) advances `QuestStep(218)` to `N + 1` and chains directly into wave `N + 1` in-engine without emitting `battle_trailer` until wave 40 is cleared or the challenger is defeated.
- [x] Defeating wave 40 triggers combat victory completion, awards milestone rewards (`26051` Đậu Dũng Khí, `23086` Lông Vũ, and `10,000` EXP) with inventory slot integrity preserved, resets `QuestStep(218 = 0)`, and warps the challenger back to lobby map `12001`.
- [x] Challenging NPC `14628` without an active registration (`QuestStep(218) == 0`) is rejected with a system message (`020102`).
- [x] Integration tests verify NPC `14628` battle launch with 40 waves loaded, multi-wave progression through wave 40 victory, milestone reward delivery, and unregistered challenge rejection.
