//! Standardized domain binary codecs.
//!
//! Provides byte-exact serializers and deserializers for core TS Online entities:
//! - `ThingData` (35 bytes item wire format)
//! - `PlayerCard` & `FriendExtra` (Player card and social extra data)
//! - `BattleRoleData` & `BattleRoleSerializer` (Battle entity appearances for opcodes `0x0B` / `0x32`)

pub mod battle_role;
pub mod npc_talk;
pub mod player_info;
pub mod quest_sync;
pub mod thing_data;

pub use battle_role::{ehuman, BattleRoleData, BattleRoleSerializer};
pub use npc_talk::{
    NpcTalkCodec, TalkLockMode, EVE_RESULT_SIZE, SUB_CLOSE_TALK_WINDOW, SUB_END_TALK,
    SUB_LOCK_ACTOR, SUB_SELECT_MENU, SUB_TALK_CONTINUE, SUB_TALK_STEP, TALK_STEP_BODY_SIZE,
};
pub use player_info::{FriendExtra, PlayerCard, PlayerInfoCodec, FRIEND_EXTRA_SIZE};
pub use quest_sync::{
    QuestDontEntry, QuestSyncCodec, QuestTaskEntry, OP_QUEST_SYNC, SUB_ACTOR_STATE_FLAG,
    SUB_QUEST_DONT_BULK, SUB_QUEST_DONT_SINGLE, SUB_QUEST_FULL, SUB_QUEST_ITEM_ADD,
    SUB_QUEST_ITEM_CLEAR, SUB_QUEST_ITEM_REMOVE, SUB_QUEST_TASK_LOG,
};
pub use thing_data::{ThingData, ThingDataCodec, THING_DATA_SIZE};
