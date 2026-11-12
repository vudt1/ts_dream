# 09: Event Domain Prefactoring, Opcode Constants & Natural Multi-Session Test Flow

**What to build:** Special event NPC interactions, dungeon map fallback resolution, party quiz evacuation, and Web Admin status badges all delegate through a single cohesive event domain boundary without duplicated routing or state logic. Protocol packets across all special events use canonical opcode constants instead of raw hex literals, and multi-session tests register online players through natural login/dispatcher flows rather than manually inserting into the session registry.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] Event-specific NPC workflows (Exorcist `1752`, 40-NPC Arena Registrar `15162`, 2K Registrar `15312`, Cửu Thiên Registrar `61209`, and Floor Reward Chest `20001`) are extracted into dedicated event domain handlers rather than inlined inside the general NPC dialogue start handler.
- [x] Dungeon map fallback resolution (formaps `10851..=10852`, `12911..=12928`, and `49960..=49969`) and party quiz evacuation share one unified implementation across login trap recovery, player disconnect evacuation, Web Admin emergency drain, and in-battle quiz completion.
- [x] Web Admin special event status rendering uses the domain `badge_class` helper rather than duplicating phase-to-CSS-class matching.
- [x] All special event packet builders and handlers (North Star Quiz, Festival UI/Announcements, Arena Scoreboards/Notices, Guild Lifecycle, City Siege, Water War, Naval Combat, and Dragon Boat Racing) reference named `OP_*` protocol constants instead of raw hex opcode literals.
- [x] Multi-session integration tests for Guild Warfare, North Star Quiz, and Web Admin Event Control register sessions via natural dispatcher/session lifecycle APIs with zero manual `lock_online_sessions().insert(...)` calls.
- [x] Architecture guardrails and existing special event test suites pass without regression.
