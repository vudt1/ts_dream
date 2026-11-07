//! Special Events & Tournament Subsystem Lifecycle Manager (Tickets 02 & 03).
//!
//! Provides the central [`EventLifecycleManager`] that governs:
//! - 5-phase event state machine: `Idle` -> `Announcing` -> `Active` -> `Ending` -> `Cooldown`
//! - North Star Quiz question bank (UTF-8 to VISCII 1.1) and modal Q&A protocol (`Opcode 0x10`)
//! - Strict Mutex policy (no duplicate active instances)
//! - Starvation watchdog (120-minute maximum runtime ceiling)
//! - Cascading emergency drain on TCP shutdown (aborts battle via 00 2F / 00 36, warps to savemap)
//! - Graceful stop with 3-minute drain period

use axum::http::StatusCode;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub const EVENT_NORTH_STAR: &str = "north_star";
pub const EVENT_40_NPC: &str = "continuous_40_npc";
pub const EVENT_RESCUE_2K: &str = "rescue_2k";
pub const EVENT_NINE_HEAVENS: &str = "nine_heavens";
pub const EVENT_ARENA: &str = "arena";
pub const EVENT_GUILD_SIEGE: &str = "guild_siege";
pub const EVENT_WATER_WAR: &str = "water_war";
pub const EVENT_DRAGON_BOAT: &str = "dragon_boat";
pub const EVENT_SEASONAL_FESTIVAL: &str = "seasonal_festival";

/// The 5 formal lifecycle phases of a special event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventPhase {
    Idle,
    Announcing,
    Active,
    Ending,
    Cooldown,
}

impl std::fmt::Display for EventPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventPhase::Idle => write!(f, "Idle"),
            EventPhase::Announcing => write!(f, "Announcing"),
            EventPhase::Active => write!(f, "Active"),
            EventPhase::Ending => write!(f, "Ending"),
            EventPhase::Cooldown => write!(f, "Cooldown"),
        }
    }
}

/// A registered special event definition and live state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpecialEvent {
    pub id: String,
    pub name: String,
    pub phase: EventPhase,
    #[serde(skip)]
    pub started_at: Option<Instant>,
    #[serde(skip)]
    pub ending_at: Option<Instant>,
    pub participants: usize,
    pub encounter_rate: u8,
}

impl SpecialEvent {
    pub fn badge_class(&self) -> &'static str {
        match self.phase {
            EventPhase::Active => "status-online",
            EventPhase::Ending => "status-connecting",
            _ => "status-offline",
        }
    }
}

/// Canonical question bank entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NorthStarQuestion {
    pub id: u32,
    pub question: String,
    pub options: [String; 4],
    pub correct: u32, // 1..=4
}

/// Active in-battle quiz session.
#[derive(Debug, Clone)]
pub struct ActiveQuiz {
    pub battle_id: i32,
    pub leader_id: u32,
    pub question: NorthStarQuestion,
    pub started_at: Instant,
    pub timeout: Duration,
}

/// Result of evaluating a quiz answer submission.
#[derive(Debug, Clone)]
pub struct QuizAnswerResult {
    pub battle_id: i32,
    pub leader_id: u32,
    pub is_correct: bool,
    pub result_frame: String,
}

/// Frame builder for S->C 10 01 modal question dialog.
pub fn build_quiz_modal_frame(question: &str, options: &[String; 4]) -> String {
    let q_bytes = crate::encoding::viscii_encode(question);
    let opt_bytes: Vec<Vec<u8>> = options
        .iter()
        .map(|o| crate::encoding::viscii_encode(o))
        .collect();

    let mut payload = Vec::new();
    payload.push(0x10); // opcode
    payload.push(0x01); // sub-opcode
    payload.push(q_bytes.len().min(255) as u8);
    payload.extend_from_slice(&q_bytes);

    for (i, opt) in opt_bytes.iter().enumerate() {
        if i < 3 {
            payload.push(opt.len().min(255) as u8);
            payload.extend_from_slice(opt);
        } else {
            // 4th option occupies remaining payload bytes without prefix
            payload.extend_from_slice(opt);
        }
    }

    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    hex::encode(frame).to_uppercase()
}

/// Anti-spectate rejection frame: S->C 21 01 03.
pub fn anti_spectate_frame() -> &'static str {
    "F4440300210103"
}

/// Central Special Events Lifecycle Manager.
#[derive(Clone)]
pub struct EventLifecycleManager {
    events: Arc<RwLock<HashMap<String, SpecialEvent>>>,
    active_quizzes: Arc<RwLock<HashMap<i32, ActiveQuiz>>>,
    player_to_battle: Arc<RwLock<HashMap<u32, i32>>>,
    question_bank: Arc<Vec<NorthStarQuestion>>,
    default_quiz_timeout: Duration,
    max_active_duration: Duration,
}

impl Default for EventLifecycleManager {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl EventLifecycleManager {
    pub fn new(questions: Vec<NorthStarQuestion>) -> Self {
        let mut map = HashMap::new();
        let default_events = [
            (EVENT_NORTH_STAR, "Vấn Đáp Bắc Tinh Quân", 5),
            (EVENT_40_NPC, "Khiêu Chiến 40 NPC", 0),
            (EVENT_RESCUE_2K, "Giải Cứu Nhị Kiều 2K", 0),
            (EVENT_NINE_HEAVENS, "Chinh Phục Cửu Thiên", 0),
            (EVENT_ARENA, "Đấu Trường Lôi Đài", 0),
            (EVENT_GUILD_SIEGE, "Công Thành Chiến", 0),
            (EVENT_WATER_WAR, "Thủy Chiến Hải Tặc", 0),
            (EVENT_DRAGON_BOAT, "Đua Thuyền Rồng", 0),
            (EVENT_SEASONAL_FESTIVAL, "Lễ Hội Theo Mùa", 0),
        ];

        for (id, name, rate) in default_events {
            map.insert(
                id.to_string(),
                SpecialEvent {
                    id: id.to_string(),
                    name: name.to_string(),
                    phase: EventPhase::Idle,
                    started_at: None,
                    ending_at: None,
                    participants: 0,
                    encounter_rate: rate,
                },
            );
        }

        Self {
            events: Arc::new(RwLock::new(map)),
            active_quizzes: Arc::new(RwLock::new(HashMap::new())),
            player_to_battle: Arc::new(RwLock::new(HashMap::new())),
            question_bank: Arc::new(if questions.is_empty() {
                Self::fallback_questions()
            } else {
                questions
            }),
            default_quiz_timeout: Duration::from_secs(45),
            max_active_duration: Duration::from_secs(120 * 60), // 120 minutes ceiling
        }
    }

    /// Load questions from data directory or fall back to defaults.
    pub fn from_data_dir(data_dir: &Path) -> Self {
        let json_path = data_dir.join("north_star_questions.json");
        let questions = if json_path.exists() {
            match std::fs::read_to_string(&json_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse north_star_questions.json: {e}");
                    Vec::new()
                }),
                Err(e) => {
                    tracing::warn!("Failed to read north_star_questions.json: {e}");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        Self::new(questions)
    }

    pub fn fallback_questions() -> Vec<NorthStarQuestion> {
        vec![
            NorthStarQuestion {
                id: 1,
                question: "Ai là người kết nghĩa đào viên cùng Lưu Bị và Trương Phi?".to_string(),
                options: [
                    "Quan Vũ".to_string(),
                    "Triệu Vân".to_string(),
                    "Mã Siêu".to_string(),
                    "Hoàng Trung".to_string(),
                ],
                correct: 1,
            },
            NorthStarQuestion {
                id: 2,
                question: "Vũ khí nổi tiếng của Quan Vũ có tên là gì?".to_string(),
                options: [
                    "Thanh Long Yển Nguyệt Đao".to_string(),
                    "Phương Thiên Họa Kích".to_string(),
                    "Bát Xà Mâu".to_string(),
                    "Ỷ Thiên Kiếm".to_string(),
                ],
                correct: 1,
            },
            NorthStarQuestion {
                id: 3,
                question: "Ai được mệnh danh là Ngọa Long tiên sinh trong Tam Quốc?".to_string(),
                options: [
                    "Bàng Thống".to_string(),
                    "Gia Cát Lượng".to_string(),
                    "Tư Mã Ý".to_string(),
                    "Chu Du".to_string(),
                ],
                correct: 2,
            },
            NorthStarQuestion {
                id: 4,
                question: "Thời gian tối đa để trả lời Vấn Đáp Bắc Tinh Quân là bao nhiêu?".to_string(),
                options: [
                    "30 giây".to_string(),
                    "45 giây".to_string(),
                    "60 giây".to_string(),
                    "90 giây".to_string(),
                ],
                correct: 2,
            },
            NorthStarQuestion {
                id: 5,
                question: "Khi có Thần Tài đi theo, kinh nghiệm nhận được tăng lên bao nhiêu?".to_string(),
                options: [
                    "1.2 lần".to_string(),
                    "1.5 lần".to_string(),
                    "2.0 lần".to_string(),
                    "3.0 lần".to_string(),
                ],
                correct: 2,
            },
        ]
    }

    /// Set a custom quiz timeout for testing.
    pub fn with_quiz_timeout(mut self, timeout: Duration) -> Self {
        self.default_quiz_timeout = timeout;
        self
    }

    /// Get default quiz timeout duration.
    pub fn get_quiz_timeout(&self) -> Duration {
        self.default_quiz_timeout
    }

    /// Check if a specific event is in Active phase.
    pub fn is_event_active(&self, id: &str) -> bool {
        self.events
            .read()
            .map(|e| e.get(id).map(|ev| ev.phase == EventPhase::Active).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Get current phase of an event.
    pub fn get_event_phase(&self, id: &str) -> EventPhase {
        self.events
            .read()
            .map(|e| e.get(id).map(|ev| ev.phase).unwrap_or(EventPhase::Idle))
            .unwrap_or(EventPhase::Idle)
    }

    /// Get all events snapshot for dashboard / API.
    pub fn list_events(&self) -> Vec<SpecialEvent> {
        let events = self.events.read().unwrap();
        let mut list: Vec<SpecialEvent> = events.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    /// Get North Star encounter rate (1..=30).
    pub fn get_north_star_rate(&self) -> u8 {
        self.events
            .read()
            .map(|e| e.get(EVENT_NORTH_STAR).map(|ev| ev.encounter_rate).unwrap_or(5))
            .unwrap_or(5)
    }

    /// Set North Star encounter rate (clamped 1..=30).
    pub fn set_north_star_rate(&self, rate: u8) {
        let clamped = rate.clamp(1, 30);
        if let Ok(mut events) = self.events.write() {
            if let Some(event) = events.get_mut(EVENT_NORTH_STAR) {
                event.encounter_rate = clamped;
            }
        }
    }

    /// Start an event with server-state gating and strict mutex policy.
    pub fn start_event(
        &self,
        id: &str,
        is_server_running: bool,
    ) -> Result<(), (StatusCode, String)> {
        if !is_server_running {
            return Err((
                StatusCode::CONFLICT,
                "Game server is stopped. Please start the server first".to_string(),
            ));
        }

        let mut events = self.events.write().unwrap();
        let event = events.get_mut(id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Event '{id}' does not exist"),
            )
        })?;

        // Strict Mutex: cannot trigger if already Announcing or Active
        if matches!(event.phase, EventPhase::Announcing | EventPhase::Active) {
            return Err((
                StatusCode::CONFLICT,
                format!("Event '{id}' is already running"),
            ));
        }

        event.phase = EventPhase::Active;
        event.started_at = Some(Instant::now());
        event.ending_at = None;
        tracing::info!(event_id = id, "Special event started");
        Ok(())
    }

    /// Initiate a graceful stop for an active event (3-minute drain period).
    pub fn stop_event(&self, id: &str) -> Result<(), (StatusCode, String)> {
        let mut events = self.events.write().unwrap();
        let event = events.get_mut(id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Event '{id}' does not exist"),
            )
        })?;

        if !matches!(event.phase, EventPhase::Active | EventPhase::Announcing) {
            return Err((
                StatusCode::CONFLICT,
                format!("Event '{id}' is not currently active"),
            ));
        }

        event.phase = EventPhase::Ending;
        event.ending_at = Some(Instant::now());
        tracing::info!(event_id = id, "Special event entered graceful Ending drain");
        Ok(())
    }

    /// Immediate transition to Idle.
    pub fn force_idle(&self, id: &str) {
        if let Ok(mut events) = self.events.write() {
            if let Some(event) = events.get_mut(id) {
                event.phase = EventPhase::Idle;
                event.started_at = None;
                event.ending_at = None;
                event.participants = 0;
            }
        }
    }

    /// Starvation watchdog: enforce 120-minute maximum runtime and graceful drain expiration.
    pub fn check_watchdog(&self) {
        let mut events = self.events.write().unwrap();
        let now = Instant::now();

        for event in events.values_mut() {
            if event.phase == EventPhase::Active {
                if let Some(started) = event.started_at {
                    if now.duration_since(started) >= self.max_active_duration {
                        tracing::warn!(
                            event_id = %event.id,
                            "Starvation watchdog: active event exceeded 120m, forcing Idle"
                        );
                        event.phase = EventPhase::Idle;
                        event.started_at = None;
                        event.ending_at = None;
                        event.participants = 0;
                    }
                }
            } else if event.phase == EventPhase::Ending {
                if let Some(ending) = event.ending_at {
                    if now.duration_since(ending) >= Duration::from_secs(180) {
                        tracing::info!(
                            event_id = %event.id,
                            "Graceful drain period expired (3m), event is now Idle"
                        );
                        event.phase = EventPhase::Idle;
                        event.started_at = None;
                        event.ending_at = None;
                        event.participants = 0;
                    }
                }
            }
        }
    }

    /// Cascading emergency drain across all active events upon TCP shutdown.
    /// Aborts active battles via 00 2F / 00 36, warps players to savemap, and resets to Idle.
    pub async fn emergency_cascading_stop(&self, control: &crate::web::server_control::ServerControl) {
        let active_ids: Vec<String> = {
            let events = self.events.read().unwrap();
            events
                .iter()
                .filter(|(_, e)| matches!(e.phase, EventPhase::Active | EventPhase::Announcing | EventPhase::Ending))
                .map(|(id, _)| id.clone())
                .collect()
        };

        if active_ids.is_empty() {
            return;
        }

        tracing::warn!(
            count = active_ids.len(),
            "Initiating cascading emergency drain across active special events"
        );

        // Abort active North Star quiz battles with 00 2F
        let quiz_battles: Vec<(i32, u32)> = {
            let quizzes = self.active_quizzes.read().unwrap();
            quizzes.values().map(|q| (q.battle_id, q.leader_id)).collect()
        };

        for (battle_id, leader_id) in quiz_battles {
            // S->C 00 2F: EventEndedBeforeBattle
            control
                .send_to(leader_id, &crate::protocol::SystemAlertReason::EventEndedBeforeBattle.to_hex())
                .await;
            control.battle_service.abort_battle(battle_id);
            let mut map = crate::server::session::lock_online_sessions();
            let mem_ids = if let Some(session) = map.get_mut(&leader_id) {
                session.battle_id = 0;
                session.id_mem
            } else {
                [0; 4]
            };
            for mem_id in mem_ids {
                if mem_id > 0 {
                    if let Some(mem) = map.get_mut(&mem_id) {
                        mem.battle_id = 0;
                    }
                }
            }
        }

        self.active_quizzes.write().unwrap().clear();
        self.player_to_battle.write().unwrap().clear();

        // Evacuate players in dungeon/arena maps to savemap
        let online_players: Vec<u32> = {
            let sessions = crate::server::session::lock_online_sessions();
            sessions
                .iter()
                .filter(|(_, s)| matches!(s.map_id, 10851 | 10852 | 12911..=12928 | 49960..=49969))
                .map(|(id, _)| *id)
                .collect()
        };

        for pid in online_players {
            let mut sessions = crate::server::session::lock_online_sessions();
            if let Some(session) = sessions.get_mut(&pid) {
                let target_map = if session.savemap > 0 {
                    session.savemap
                } else {
                    12001
                };
                session.map_id = target_map;
                session.map_x = 400;
                session.map_y = 400;
                if session.battle_id > 0 {
                    control.battle_service.leave_battle(session);
                }
            }
        }

        // Reset all events to Idle
        let mut events = self.events.write().unwrap();
        for id in active_ids {
            if let Some(event) = events.get_mut(&id) {
                event.phase = EventPhase::Idle;
                event.started_at = None;
                event.ending_at = None;
                event.participants = 0;
            }
        }
    }

    // --- Quiz Management ---

    /// Check if a battle is an active North Star quiz battle (for anti-spectate lock).
    pub fn is_north_star_battle(&self, battle_id: i32) -> bool {
        self.active_quizzes
            .read()
            .map(|q| q.contains_key(&battle_id))
            .unwrap_or(false)
    }

    /// Register and start a North Star quiz battle.
    pub fn register_quiz(
        &self,
        battle_id: i32,
        leader_id: u32,
        custom_question: Option<NorthStarQuestion>,
    ) -> (NorthStarQuestion, String) {
        let question = match custom_question {
            Some(q) => q,
            None => {
                let count = self.question_bank.len();
                if count == 0 {
                    Self::fallback_questions().remove(0)
                } else {
                    let idx = (battle_id.unsigned_abs() as usize) % count;
                    self.question_bank[idx].clone()
                }
            }
        };

        let frame = build_quiz_modal_frame(&question.question, &question.options);

        let active = ActiveQuiz {
            battle_id,
            leader_id,
            question: question.clone(),
            started_at: Instant::now(),
            timeout: self.default_quiz_timeout,
        };

        self.active_quizzes.write().unwrap().insert(battle_id, active);
        self.player_to_battle.write().unwrap().insert(leader_id, battle_id);

        (question, frame)
    }

    /// Submit an answer by leader ID (1-based index 1..=4).
    pub fn submit_answer(&self, leader_id: u32, answer_index: u32) -> Option<QuizAnswerResult> {
        let battle_id = self.player_to_battle.write().unwrap().remove(&leader_id)?;
        let quiz = self.active_quizzes.write().unwrap().remove(&battle_id)?;

        let is_correct = answer_index == quiz.question.correct;
        let result_frame = if is_correct {
            "F4440300100201".to_string()
        } else {
            "F4440300100202".to_string()
        };

        Some(QuizAnswerResult {
            battle_id,
            leader_id,
            is_correct,
            result_frame,
        })
    }

    /// Check if a battle quiz has timed out (> 45s).
    pub fn check_quiz_timeout(&self, battle_id: i32) -> Option<QuizAnswerResult> {
        let mut quizzes = self.active_quizzes.write().unwrap();
        let quiz = quizzes.get(&battle_id)?;

        if quiz.started_at.elapsed() >= quiz.timeout {
            let quiz = quizzes.remove(&battle_id)?;
            drop(quizzes);
            self.player_to_battle.write().unwrap().remove(&quiz.leader_id);

            Some(QuizAnswerResult {
                battle_id,
                leader_id: quiz.leader_id,
                is_correct: false,
                result_frame: "F4440300100202".to_string(),
            })
        } else {
            None
        }
    }

    /// Check and resolve all quizzes that have expired (> 45s).
    pub fn get_timed_out_quizzes(&self) -> Vec<QuizAnswerResult> {
        let battle_ids: Vec<i32> = {
            let quizzes = self.active_quizzes.read().unwrap();
            quizzes.keys().copied().collect()
        };
        let mut expired = Vec::new();
        for id in battle_ids {
            if let Some(res) = self.check_quiz_timeout(id) {
                expired.push(res);
            }
        }
        expired
    }

    /// Remove a quiz when battle finishes or player disconnects.
    pub fn remove_quiz(&self, battle_id: i32) {
        if let Some(q) = self.active_quizzes.write().unwrap().remove(&battle_id) {
            self.player_to_battle.write().unwrap().remove(&q.leader_id);
        }
    }
}
