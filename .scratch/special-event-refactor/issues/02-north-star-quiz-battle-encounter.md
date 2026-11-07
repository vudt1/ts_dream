# 02: North Star Quiz In-Battle Encounter, Modal Q&A Protocol, and Anti-Spectate Lock

**What to build:** From the player's perspective, walking on adventure maps during an active North Star event can trigger a random PvE battle against North Star (`NPC 1643`) or South Star (`NPC 1644`). In this battle, external spectating is strictly blocked, combat action menus are replaced by a 45-second modal quiz dialog with 4 choices, and answering awards the Party Leader with God of Fortune or penalizes them with 1 HP and a Ghost without affecting trailing party members.

**Blocked by:** 01: Character God & Ghost State, Map Appearance, and Real-Time Synchronization

**Status:** completed

- [x] Question bank `Data/north_star_questions.json` provides 30–50 canonical questions and answers in standard UTF-8 Vietnamese, loaded into memory upon server boot.
- [x] Movement step checker (`try_trigger_random_encounter`) injects special encounters with NPC 1643/1644 when the North Star event is active based on configured encounter rate and map filter.
- [x] Combat spectate requests against a North Star battle instance are rejected with `S→C 21 01 03` and client alert *"Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến"*.
- [x] Combat initialization emits `S→C 10 01 [qLen][question][opt1Len][opt1]...[opt4]` converted from UTF-8 to VISCII 1.1 via `viscii_encode`, displaying the modal question window with combat action buttons locked.
- [x] Client answer packet `C→S 10 01 [answer_index: u32 LE]` is evaluated against the correct choice.
- [x] A 45-second server-side countdown enforces decision pacing; expiring the timer without an answer automatically resolves as an incorrect answer.
- [x] In a Party, the question modal is presented strictly to the Party Leader; answer submission applies reward or penalty exclusively to the Leader while member HP, God, and Ghost states remain untouched.
- [x] Correct answer emits `S→C 10 02 01`, awards +10 battles of God of Fortune buff (`FGod`, cap 240) via `S→C 18 08 [playerId][02 00][01]`, and exits combat with victory.
- [x] Incorrect answer or timeout emits `S→C 10 02 02`, sets Leader HP to 1, afflicts the Leader with Ghost debuff (`FGhost`) via `S→C 18 08 [playerId][01 00][01]`, and exits combat.
- [x] Integration tests in `tests/festival_quiz_test.rs` verify encounter trigger, anti-spectate rejection frame, modal question emission, 45-second timeout expiration, party member isolation, and reward/penalty application.
