//! Binary loader and domain models for `eve.emg` container.
//!
//! The container mirrors the client Eve scripts (`Eve_NpcData.lua`,
//! `Eve_NpcEventData.lua`, `Eve_DoorData.lua`, `Eve_SceneInfoData.lua`,
//! `Eve_FightData.lua`, `Eve_SurfaceData.lua`, `Eve_GroupData.lua`).

use crate::data::reader::DatReader;
use crate::error::{Result, TsError};
use std::collections::HashMap;

/// Condition comparison operators in Eve scripts (`conditionOps`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EveConditionOps {
    Equal0 = 0,
    Equal1 = 1,
    LessThan = 2,
    LessThanOrEqual = 3,
    GreaterThan = 4,
    GreaterThanOrEqual = 5,
    NotEqual = 6,
}

impl EveConditionOps {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Equal0,
            1 => Self::Equal1,
            2 => Self::LessThan,
            3 => Self::LessThanOrEqual,
            4 => Self::GreaterThan,
            5 => Self::GreaterThanOrEqual,
            _ => Self::NotEqual,
        }
    }
}

/// Known condition classes in Eve scripts (`conditionClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EveConditionClass {
    Unconditional = 0,
    BagItem = 1,
    QuestStep = 2,
    PlayerAttribute = 7,
    BattleResult = 8,
    FollowPet = 9,
    DialogChoice = 10,
    SceneEventCount = 12,
    RoleCount = 14,
    Unknown(u8),
}

impl From<u8> for EveConditionClass {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Unconditional,
            1 => Self::BagItem,
            2 => Self::QuestStep,
            7 => Self::PlayerAttribute,
            8 => Self::BattleResult,
            9 => Self::FollowPet,
            10 => Self::DialogChoice,
            12 => Self::SceneEventCount,
            14 => Self::RoleCount,
            other => Self::Unknown(other),
        }
    }
}

/// Event result type (`resultType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EveResultType {
    Action = 0,
    Talk = 1,
    Door = 2,
    Battle = 3,
    Animation = 5,
    Surface = 6,
    NpcAction = 9,
    Unknown(u8),
}

impl From<u8> for EveResultType {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Action,
            1 => Self::Talk,
            2 => Self::Door,
            3 => Self::Battle,
            5 => Self::Animation,
            6 => Self::Surface,
            9 => Self::NpcAction,
            other => Self::Unknown(other),
        }
    }
}

/// Event result class (`resultClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EveResultClass {
    Item = 1,
    Quest = 2,
    NpcTeam = 3,
    Skill = 4,
    Player = 7,
    RewardPet = 8,
    Unknown(u8),
}

impl From<u8> for EveResultClass {
    fn from(val: u8) -> Self {
        match val {
            1 => Self::Item,
            2 => Self::Quest,
            3 => Self::NpcTeam,
            4 => Self::Skill,
            7 => Self::Player,
            8 => Self::RewardPet,
            other => Self::Unknown(other),
        }
    }
}

/// Event Result item (corresponds to S:020-001 packet payload, 15 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveResult {
    pub result_group_no: u16,
    pub result_no: u8,
    pub result_type: u8,
    pub result_class: u8,
    pub parameter: u16,
    pub parameter_style: u8,
    pub result_value: i32,
    pub result_mean_no: u16,
}

/// Event Condition with associated result array.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveCondition {
    pub condition_no: u8,
    pub condition_class: u8,
    pub condition_parameter: u16,
    pub condition_parameter_style: u8,
    pub condition_ops: u8,
    pub condition_value: i32,
    pub condition_sub_item: u16,
    pub to_result: u8,
    pub and_num: u8,
    pub results: Vec<EveResult>,
}

/// NPC Event definition (`Eve_NpcEventData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NpcEventData {
    pub eve_no: u16,
    /// Trigger flags: [0]=OnClickObject, [1]=OnStrokeObject, [2]=OnIntoObjectArea, [3]=OnSerStrokeObject
    pub when_happen: [bool; 4],
    pub conditions: Vec<EveCondition>,
}

/// NPC placement on scene map (`Eve_NpcData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveNpcPlacement {
    pub id: u16,
    pub npc_id: u16,
    pub events: Vec<u8>,
    pub x: i32,
    pub y: i32,
    pub close: bool,
    pub motion_type: u8,
    pub motion_back: u8,
    pub motion_cycle_num: u8,
    pub direction: u8,
    pub motion_suspend_ms: u16,
    pub motion_speed_lv: u8,
    pub motion_nodes: Vec<(i32, i32)>,
    pub trace_radius: u16,
    pub can_grow: u8,
}

/// Warp door placement on scene map (`Eve_DoorData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveDoorPlacement {
    pub id: u16,
    pub events: Vec<u8>,
    pub x: i32,
    pub y: i32,
}

/// Scene warp destination information (`Eve_SceneInfoData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveSceneInfo {
    pub eve_no: u16,
    pub background_no: u16,
    pub player_appear_x: i32,
    pub player_appear_y: i32,
    pub direction: u8,
}

/// Enemy configuration in scripted battle (`Eve_FightData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveFightEnemy {
    pub no: u16,
    pub npc_id: u16,
    pub location_pos: u8,
    pub ai: u8,
}

impl EveFightEnemy {
    /// Enemy column on board (0: front row, 1: back row).
    pub fn col(&self) -> u8 {
        self.location_pos / 5
    }

    /// Enemy row on board (0..4).
    pub fn row(&self) -> u8 {
        self.location_pos % 5
    }
}

/// Scripted PvE Fight data (`Eve_FightData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveFightData {
    pub eve_no: u16,
    pub fight_type: u8,
    pub link_to_result: u8,
    pub left_enemies: Vec<EveFightEnemy>,
    pub right_enemies: Vec<EveFightEnemy>,
    pub fight_limit: u32,
}

/// Sentence in Surface dialogue UI (`Eve_SurfaceData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveSentence {
    pub data_id: u16,
    pub data_kind: u8,
    pub style: u8,
    pub can_cut: bool,
}

/// Surface dialogue/choice interactive UI (`Eve_SurfaceData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveSurfaceData {
    pub id: u16,
    pub sentence_count: u8,
    pub option_index: u8,
    pub option_count: u8,
    pub option_mode: u8,
    pub sentences: HashMap<u8, EveSentence>,
}

/// Group data for weighted result selection (`Eve_GroupData.lua`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EveGroupData {
    pub eve_no: u16,
    pub event_no: u16,
    pub condition_no: u8,
    pub use_mode: u8,
    pub member_no_ay: Vec<u8>,
    pub probability_rate_ay: Vec<u8>,
    pub pick_member: u8,
}

/// Complete scene event data container for a single map/scene.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneEveData {
    pub npcs: HashMap<u16, EveNpcPlacement>,
    pub doors: HashMap<u16, EveDoorPlacement>,
    pub npc_events: HashMap<u16, NpcEventData>,
    pub scene_infos: HashMap<u16, EveSceneInfo>,
    pub fight_datas: HashMap<u16, EveFightData>,
    pub surface_datas: HashMap<u16, EveSurfaceData>,
    pub group_datas: HashMap<u16, EveGroupData>,
}

/// Directory entry header inside `eve.emg` container.
#[derive(Debug, Clone)]
struct SceneDirectoryEntry {
    scene_id: u32,
    position: usize,
    #[allow(dead_code)]
    size: usize,
}

/// Binary loader for `Data/eve.emg` container.
pub struct EveDataLoader;

impl EveDataLoader {
    /// Load `eve.emg` binary buffer into `scene_id -> SceneEveData` map.
    pub fn load(data: &[u8]) -> Result<HashMap<u32, SceneEveData>> {
        let mut reader = DatReader::new(data.to_vec());
        let scene_count = reader.read_u16() as usize;

        let mut entries = Vec::with_capacity(scene_count);
        for _ in 0..scene_count {
            let name_len = reader.read_u8() as usize;
            let name_bytes = reader.read_bytes(23);
            let position = reader.read_i32();
            let size = reader.read_i32();

            let valid_len = name_len.min(23);
            let name_slice = &name_bytes[..valid_len];
            let name_str = String::from_utf8_lossy(name_slice);
            let stem = name_str.trim_end_matches('\0');
            let clean_name = stem
                .strip_suffix(".eve")
                .or_else(|| stem.strip_suffix(".EVE"))
                .unwrap_or(stem);

            if let Ok(scene_id) = clean_name.parse::<u32>() {
                if position > 0 && size > 0 {
                    entries.push(SceneDirectoryEntry {
                        scene_id,
                        position: position as usize,
                        size: size as usize,
                    });
                }
            }
        }

        let mut result = HashMap::with_capacity(entries.len());
        for entry in entries {
            if entry.position + 103 >= data.len() {
                continue;
            }
            // DataManager.lua line 773: position = ReadInt32() + 103
            reader.seek(entry.position + 103);
            if let Ok(scene_data) = Self::parse_scene_data(&mut reader) {
                if !scene_data.npcs.is_empty()
                    || !scene_data.doors.is_empty()
                    || !scene_data.npc_events.is_empty()
                    || !scene_data.scene_infos.is_empty()
                    || !scene_data.fight_datas.is_empty()
                    || !scene_data.surface_datas.is_empty()
                    || !scene_data.group_datas.is_empty()
                {
                    result.insert(entry.scene_id, scene_data);
                }
            }
        }

        Ok(result)
    }

    /// Parse a single scene's 9 sections.
    fn parse_scene_data(reader: &mut DatReader) -> Result<SceneEveData> {
        let npcs = Self::parse_npc_data_section(reader)?;
        Self::skip_goods_section(reader)?;
        let doors = Self::parse_door_section(reader)?;
        Self::skip_mine_section(reader)?;
        let surface_datas = Self::parse_surface_section(reader)?;
        let scene_infos = Self::parse_scene_info_section(reader)?;
        let group_datas = Self::parse_group_section(reader)?;
        let npc_events = Self::parse_npc_event_section(reader)?;
        let fight_datas = Self::parse_fight_data_section(reader)?;

        Ok(SceneEveData {
            npcs,
            doors,
            npc_events,
            scene_infos,
            fight_datas,
            surface_datas,
            group_datas,
        })
    }

    /// Section 1: NpcData (Eve_NpcData.lua)
    fn parse_npc_data_section(reader: &mut DatReader) -> Result<HashMap<u16, EveNpcPlacement>> {
        let count = reader.read_i32();
        if count <= 0 {
            return Ok(HashMap::new());
        }
        if count > 5000 || (count as usize) * 20 > reader.remaining() {
            return Err(TsError::Data("invalid npc count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count as usize);
        for _ in 0..count {
            let id = reader.read_u16();
            let npc_id = reader.read_u16();

            let event_count = reader.read_u16() as usize;
            if event_count > reader.remaining() {
                return Err(TsError::Data("invalid event_count in npc".into()));
            }
            let mut events = Vec::with_capacity(event_count);
            for _ in 0..event_count {
                events.push(reader.read_u8());
            }

            let sale_kind_count = reader.read_u8() as usize;
            if sale_kind_count > reader.remaining() {
                return Err(TsError::Data("invalid sale_kind_count in npc".into()));
            }
            reader.skip(sale_kind_count);

            let motion_node_count = reader.read_u8() as usize;
            if (motion_node_count + 1) * 8 > reader.remaining() {
                return Err(TsError::Data("invalid motion_node_count in npc".into()));
            }
            let mut motion_nodes = Vec::with_capacity(motion_node_count + 1);
            for _ in 0..=motion_node_count {
                let nx = reader.read_i32();
                let ny = reader.read_i32();
                motion_nodes.push((nx, ny));
            }

            let motion_type = reader.read_u8();
            let motion_back = reader.read_u8();
            let motion_cycle_num = reader.read_u8();
            let direction = reader.read_u8();
            let motion_suspend_ms = reader.read_u16();
            let motion_speed_lv = reader.read_u8();

            // roleGrid (16B) + moveOffsetGrid (8B)
            reader.skip(24);

            let pos_x = reader.read_i32();
            let pos_y = reader.read_i32();

            reader.skip(2); // isHide + isVisible
            reader.skip(2); // rideNpcId
            let can_grow = reader.read_u8();
            reader.skip(1); // isLie

            // innerNode (16B) + outerNode (16B)
            reader.skip(32);

            reader.skip(1); // traceSpeedLv
            let trace_radius = reader.read_u16();
            let close = reader.read_u8() != 0;

            if id > 0 {
                result.insert(
                    id,
                    EveNpcPlacement {
                        id,
                        npc_id,
                        events,
                        x: pos_x,
                        y: pos_y,
                        close,
                        motion_type,
                        motion_back,
                        motion_cycle_num,
                        direction,
                        motion_suspend_ms,
                        motion_speed_lv,
                        motion_nodes,
                        trace_radius,
                        can_grow,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 2: GoodsData (Skip 13B each)
    fn skip_goods_section(reader: &mut DatReader) -> Result<()> {
        let count = reader.read_u16() as usize;
        if count > 0 {
            if count * 13 > reader.remaining() {
                return Err(TsError::Data("invalid goods count in scene".into()));
            }
            reader.skip(count * 13);
        }
        Ok(())
    }

    /// Section 3: DoorData (Eve_DoorData.lua)
    fn parse_door_section(reader: &mut DatReader) -> Result<HashMap<u16, EveDoorPlacement>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 10 > reader.remaining() {
            return Err(TsError::Data("invalid door count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let id = reader.read_u16();
            let event_count = reader.read_u16() as usize;
            if event_count > reader.remaining() {
                return Err(TsError::Data("invalid event_count in door".into()));
            }
            let mut events = Vec::with_capacity(event_count);
            for _ in 0..event_count {
                events.push(reader.read_u8());
            }

            let grid_x = reader.read_i32();
            let grid_y = reader.read_i32();
            let grid_w = reader.read_i32();
            let grid_h = reader.read_i32();
            reader.skip(1); // imageKind
            reader.skip(2); // imgInfo.position.x
            reader.skip(2); // imgInfo.position.y

            let door_x = grid_x.wrapping_add(grid_w / 2).wrapping_mul(20);
            let door_y = grid_y.wrapping_add(grid_h / 2).wrapping_mul(20);
            reader.skip(1); // close

            if id > 0 {
                result.insert(
                    id,
                    EveDoorPlacement {
                        id,
                        events,
                        x: door_x,
                        y: door_y,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 4: MineData (Skip)
    fn skip_mine_section(reader: &mut DatReader) -> Result<()> {
        let count = reader.read_u16() as usize;
        if count * 10 > reader.remaining() {
            return Err(TsError::Data("invalid mine count in scene".into()));
        }
        for _ in 0..count {
            reader.skip(2); // id
            let event_count = reader.read_u16() as usize;
            if event_count > reader.remaining() {
                return Err(TsError::Data("invalid event_count in mine".into()));
            }
            reader.skip(event_count); // events[n]
            reader.skip(16); // grid: 4 * i32
            reader.skip(1); // sizeKind
        }
        Ok(())
    }

    /// Section 5: SurfaceData (Eve_SurfaceData.lua)
    fn parse_surface_section(reader: &mut DatReader) -> Result<HashMap<u16, EveSurfaceData>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 6 > reader.remaining() {
            return Err(TsError::Data("invalid surface count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let id = reader.read_u16();
            reader.skip(2); // unused
            let sentence_count = reader.read_u8();
            let option_index = reader.read_u8();
            let option_count = reader.read_u8();
            let option_mode = reader.read_u8();

            if (sentence_count as usize) * 5 > reader.remaining() {
                return Err(TsError::Data("invalid sentence_count in surface".into()));
            }

            let mut sentences = HashMap::new();
            let mut ignore_count = 0;
            for i in 0..sentence_count {
                let data_id = reader.read_u16();
                let data_kind = reader.read_u8();
                let style = reader.read_u8();
                let can_cut = reader.read_bool();

                // Skip placeholder sentence style=3 && dataKind=1 && dataId=20393 (Eve_SurfaceData.lua lines 27-31)
                if style == 3 && data_kind == 1 && data_id == 20393 {
                    ignore_count += 1;
                } else {
                    sentences.insert(
                        i + 1,
                        EveSentence {
                            data_id,
                            data_kind,
                            style,
                            can_cut,
                        },
                    );
                }
            }

            let adjusted_count = sentence_count.saturating_sub(ignore_count);
            if id > 0 {
                result.insert(
                    id,
                    EveSurfaceData {
                        id,
                        sentence_count: adjusted_count,
                        option_index,
                        option_count,
                        option_mode,
                        sentences,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 6: SceneInfoData (Eve_SceneInfoData.lua, 39 bytes fixed)
    fn parse_scene_info_section(reader: &mut DatReader) -> Result<HashMap<u16, EveSceneInfo>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 39 > reader.remaining() {
            return Err(TsError::Data("invalid scene_info count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let eve_no = reader.read_u16();
            let background_no = reader.read_u16();
            reader.skip(2); // eveFileName
            let player_appear_x = reader.read_i32();
            let player_appear_y = reader.read_i32();
            let direction = reader.read_u8();
            reader.skip(21); // areaName
            reader.skip(2); // linkCount
            reader.skip(1); // sceneEffect

            if eve_no > 0 {
                result.insert(
                    eve_no,
                    EveSceneInfo {
                        eve_no,
                        background_no,
                        player_appear_x,
                        player_appear_y,
                        direction,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 7: GroupData (Eve_GroupData.lua)
    fn parse_group_section(reader: &mut DatReader) -> Result<HashMap<u16, EveGroupData>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 6 > reader.remaining() {
            return Err(TsError::Data("invalid group count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let eve_no = reader.read_u16();
            let event_no = reader.read_u16();
            let condition_no = reader.read_u8();
            let use_mode = reader.read_u8();
            let member_num = reader.read_u8() as usize;

            if member_num * 2 > reader.remaining() {
                return Err(TsError::Data("invalid member_num in group".into()));
            }

            let mut member_no_ay = Vec::with_capacity(member_num);
            for _ in 0..member_num {
                member_no_ay.push(reader.read_u8());
            }

            let mut probability_rate_ay = Vec::with_capacity(member_num);
            for _ in 0..member_num {
                probability_rate_ay.push(reader.read_u8());
            }

            let pick_member = reader.read_u8();

            if eve_no > 0 {
                result.insert(
                    eve_no,
                    EveGroupData {
                        eve_no,
                        event_no,
                        condition_no,
                        use_mode,
                        member_no_ay,
                        probability_rate_ay,
                        pick_member,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 8: NpcEventData (Eve_NpcEventData.lua)
    fn parse_npc_event_section(reader: &mut DatReader) -> Result<HashMap<u16, NpcEventData>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 14 > reader.remaining() {
            return Err(TsError::Data("invalid npc_event count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let eve_no = reader.read_u16();
            reader.skip(9); // EventName[9]

            let on_click_obj = reader.read_u8() != 0;
            let on_stroke_obj = reader.read_u8() != 0;
            let on_into_area = reader.read_u8() != 0;
            let on_ser_stroke = reader.read_u8() != 0;
            let when_happen = [on_click_obj, on_stroke_obj, on_into_area, on_ser_stroke];

            let condition_count = reader.read_u8() as usize;
            if condition_count * 20 > reader.remaining() {
                return Err(TsError::Data("invalid condition_count in npc_event".into()));
            }
            let mut conditions = Vec::with_capacity(condition_count);

            for _ in 0..condition_count {
                let condition_no = reader.read_u8();
                let condition_class = reader.read_u8();
                let condition_parameter = reader.read_u16();
                let condition_parameter_style = reader.read_u8();
                let condition_ops = reader.read_u8();
                let condition_value = reader.read_i32();
                reader.skip(8); // CTimeValue (8B Double)
                let to_result = reader.read_u8();
                let and_num = reader.read_u8();
                let condition_sub_item = reader.read_u16();

                let result_count = reader.read_u8() as usize;
                if result_count * 15 > reader.remaining() {
                    return Err(TsError::Data("invalid result_count in condition".into()));
                }
                let mut results = Vec::with_capacity(result_count);

                for _ in 0..result_count {
                    let result_group_no = reader.read_u16();
                    let result_no = reader.read_u8();
                    let result_type = reader.read_u8();
                    let result_class = reader.read_u8();
                    let parameter = reader.read_u16();
                    let parameter_style = reader.read_u8();
                    let result_value = reader.read_i32();
                    let result_mean_no = reader.read_u16();

                    results.push(EveResult {
                        result_group_no,
                        result_no,
                        result_type,
                        result_class,
                        parameter,
                        parameter_style,
                        result_value,
                        result_mean_no,
                    });
                }

                conditions.push(EveCondition {
                    condition_no,
                    condition_class,
                    condition_parameter,
                    condition_parameter_style,
                    condition_ops,
                    condition_value,
                    condition_sub_item,
                    to_result,
                    and_num,
                    results,
                });
            }

            if eve_no > 0 {
                result.insert(
                    eve_no,
                    NpcEventData {
                        eve_no,
                        when_happen,
                        conditions,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Section 9: FightData (Eve_FightData.lua)
    fn parse_fight_data_section(reader: &mut DatReader) -> Result<HashMap<u16, EveFightData>> {
        let count = reader.read_u16() as usize;
        if count == 0 {
            return Ok(HashMap::new());
        }
        if count * 8 > reader.remaining() {
            return Err(TsError::Data("invalid fight count in scene".into()));
        }

        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let eve_no = reader.read_u16();
            let fight_type = reader.read_u8();
            reader.skip(1); // BKMusic
            let link_to_result = reader.read_u8();

            let left_count = reader.read_u16() as usize;
            if left_count * 6 > reader.remaining() {
                return Err(TsError::Data("invalid left_count in fight".into()));
            }
            let mut left_enemies = Vec::with_capacity(left_count);
            for _ in 0..left_count {
                let no = reader.read_u16();
                let npc_id = reader.read_u16();
                let location_pos = reader.read_u8();
                let ai = reader.read_u8();
                left_enemies.push(EveFightEnemy {
                    no,
                    npc_id,
                    location_pos,
                    ai,
                });
            }

            let right_count = reader.read_u16() as usize;
            if right_count * 6 > reader.remaining() {
                return Err(TsError::Data("invalid right_count in fight".into()));
            }
            let mut right_enemies = Vec::with_capacity(right_count);
            for _ in 0..right_count {
                let no = reader.read_u16();
                let npc_id = reader.read_u16();
                let location_pos = reader.read_u8();
                let ai = reader.read_u8();
                right_enemies.push(EveFightEnemy {
                    no,
                    npc_id,
                    location_pos,
                    ai,
                });
            }

            let fight_event_count = reader.read_u16() as usize;
            if fight_event_count * 11 > reader.remaining() {
                return Err(TsError::Data("invalid fight_event_count in fight".into()));
            }
            for _ in 0..fight_event_count {
                reader.skip(2); // EventNo
                reader.skip(9); // EventName[9]
                let win_cond_count = reader.read_u8() as usize;
                if win_cond_count * 12 > reader.remaining() {
                    return Err(TsError::Data("invalid win_cond_count in fight".into()));
                }
                reader.skip(win_cond_count * 12);
            }

            let fight_limit = reader.read_u32();

            if eve_no > 0 && (!left_enemies.is_empty() || !right_enemies.is_empty()) {
                result.insert(
                    eve_no,
                    EveFightData {
                        eve_no,
                        fight_type,
                        link_to_result,
                        left_enemies,
                        right_enemies,
                        fight_limit,
                    },
                );
            }
        }
        Ok(result)
    }
}
