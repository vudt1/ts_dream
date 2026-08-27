//! Server core state tests — migrated from inline #[cfg(test)] blocks (ticket 08).
//!
//! Sections mirror the source modules they exercise:
//! `state`, `web::server_control`, `server::session`, `server::spawn`,
//! `server::inventory`, `server::pet_box`, `server::character_sheet`,
//! `server::map_drops`, and `server::dispatcher`.
//!
//! Global-state hygiene: all tests in this file share one process. Tests that
//! mutate or depend on the process-global online-session registry serialize on
//! [`REGISTRY_LOCK`]. The map-drop registry is keyed per `(map_id, slot)`, so
//! its test uses a map id band (≥ 61001) disjoint from every other suite and
//! cleans up with the scoped `map_drops::take`/`clear_map` helpers instead of a
//! global wipe.

/// Serializes registry-mutating tests (online sessions). Tokio mutex so the
/// guard can be held across `.await` without clippy complaints.
static REGISTRY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ---------------------------------------------------------------------------
// src/state.rs
// ---------------------------------------------------------------------------
mod state {
    use ts_dream::state::{AppState, DbStatus};

    #[test]
    fn db_status_as_str_mapping() {
        assert_eq!(DbStatus::Connecting.as_str(), "light");
        assert_eq!(DbStatus::Connected.as_str(), "green");
        assert_eq!(DbStatus::Disconnected.as_str(), "dark");
    }

    #[test]
    fn app_state_new_starts_connecting() {
        assert_eq!(AppState::new(0).db_status, DbStatus::Connecting);
    }

    #[tokio::test]
    async fn db_status_broadcast_delivers_to_subscriber() {
        let state = AppState::new(0);
        let mut rx = state.db_status_tx.subscribe();
        let tx = state.db_status_tx.clone();
        tx.send(DbStatus::Connected).unwrap();
        let got = rx.recv().await.expect("subscriber should receive status");
        assert_eq!(got, DbStatus::Connected);
    }
}

// ---------------------------------------------------------------------------
// src/web/server_control.rs
// ---------------------------------------------------------------------------
mod web_server_control {
    use crate::REGISTRY_LOCK;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};
    use ts_dream::server::dispatcher::MapBroadcast;
    use ts_dream::server::session::{online_sessions, Session};
    use ts_dream::state::AppState;
    use ts_dream::web::server_control::ServerControl;

    fn control() -> ServerControl {
        let app = Arc::new(RwLock::new(AppState::new(100)));
        ServerControl::new(6414, app, None, None)
    }

    #[tokio::test]
    async fn start_refuses_when_data_not_loaded() {
        let c = control(); // AppState::new() leaves data_loaded = false
        let r = c.start().await;
        assert!(
            r.is_err(),
            "must refuse to start when static data not loaded"
        );
        assert!(
            !c.app.read().await.running,
            "must not flip running on a refused start"
        );
    }

    #[tokio::test]
    async fn start_accepts_when_data_loaded() {
        let app = Arc::new(RwLock::new(AppState::new(100)));
        app.write().await.data_loaded = true;
        // Port 0 binds an ephemeral port, avoiding collisions with other tests.
        let c = ServerControl::new(0, app.clone(), None, None);

        let r = c.start().await;
        assert!(r.unwrap_or(false), "must start once static data is loaded");
        assert!(
            c.app.read().await.running,
            "running flag flips once started"
        );

        // Stop the accept loop cleanly (no 5s countdown in tests).
        if let Some(tx) = c.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        c.app.write().await.running = false;
    }

    #[tokio::test]
    async fn login_register_is_atomic_double_login_guard() {
        let c = control();
        let (tx1, _rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, _rx2) = mpsc::unbounded_channel::<String>();

        assert!(
            c.login_register(300001, &tx1).await,
            "first login registers"
        );
        assert!(
            !c.login_register(300001, &tx2).await,
            "second concurrent login is rejected (double-login guard)"
        );
        // The failed registration must not clobber the original sender.
        assert_eq!(c.clients.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn broadcast_except_skips_origin() {
        let c = control();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        c.login_register(300001, &tx1).await;
        c.login_register(300002, &tx2).await;

        c.broadcast_except(300001, "F4440B000601FFFFFFFF026400C800")
            .await;

        assert_eq!(
            rx1.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "origin excluded"
        );
        assert_eq!(
            rx2.try_recv().unwrap(),
            "F4440B000601FFFFFFFF026400C800",
            "peer receives the broadcast"
        );
    }

    #[tokio::test]
    async fn broadcast_map_scopes_to_same_map_and_skips_subjects() {
        // P2 + P3: each `(subject, frame)` goes only to clients on the source's
        // map whose id != subject — nobody receives their own move/expression.
        let _registry = REGISTRY_LOCK.lock().await;
        let c = control();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>(); // 300001 map 12001
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>(); // 300002 map 12001
        let (tx3, mut rx3) = mpsc::unbounded_channel::<String>(); // 300003 map 13001
        c.login_register(300001, &tx1).await;
        c.login_register(300002, &tx2).await;
        c.login_register(300003, &tx3).await;
        {
            let mut sessions = online_sessions().lock().unwrap();
            sessions.insert(
                300001,
                Session {
                    map_id: 12001,
                    ..Default::default()
                },
            );
            sessions.insert(
                300002,
                Session {
                    map_id: 12001,
                    ..Default::default()
                },
            );
            sessions.insert(
                300003,
                Session {
                    map_id: 13001,
                    ..Default::default()
                },
            );
        }

        // Leader 300001 walks self + member 300002.
        c.broadcast_map(
            300001,
            &[
                MapBroadcast {
                    subject: 300001,
                    frame: "F4440B000601E1930400026400C800".into(),
                },
                MapBroadcast {
                    subject: 300002,
                    frame: "F4440B000601E2930400026400C800".into(),
                },
            ],
        )
        .await;

        // Same-map peer 300002: gets leader's walk, NOT its own walk.
        assert_eq!(rx2.try_recv().unwrap(), "F4440B000601E1930400026400C800");
        assert_eq!(
            rx2.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "member must not receive its own walk"
        );
        // Different map 300003: nothing.
        assert_eq!(
            rx3.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "other map receives nothing"
        );
        // The origin 300001 receives nothing at all (no self-echo).
        assert_eq!(
            rx1.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "origin excluded"
        );

        let mut sessions = online_sessions().lock().unwrap();
        sessions.remove(&300001);
        sessions.remove(&300002);
        sessions.remove(&300003);
    }

    #[tokio::test]
    async fn broadcast_map_is_noop_without_online_source() {
        // An unregistered source has no map scope → no fan-out.
        let _registry = REGISTRY_LOCK.lock().await;
        let c = control();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        c.login_register(300002, &tx2).await;
        {
            let mut sessions = online_sessions().lock().unwrap();
            sessions.insert(
                300002,
                Session {
                    map_id: 12001,
                    ..Default::default()
                },
            );
        }

        c.broadcast_map(
            300001,
            &[MapBroadcast {
                subject: 300001,
                frame: "F4440B000601E1930400026400C800".into(),
            }],
        )
        .await;

        assert_eq!(
            rx2.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "no broadcast when the source has no online session"
        );

        let mut sessions = online_sessions().lock().unwrap();
        sessions.remove(&300002);
    }

    #[tokio::test]
    async fn disconnect_player_broadcasts_offline_and_unregisters() {
        let _registry = REGISTRY_LOCK.lock().await;
        let c = control();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        c.login_register(300001, &tx1).await;
        c.login_register(300002, &tx2).await;
        online_sessions()
            .lock()
            .unwrap()
            .insert(300001, Default::default());

        c.disconnect_player(300001).await;

        // Peers receive the leave/offline hide frame (Ch2 §2.1).
        assert_eq!(rx2.try_recv().unwrap(), "F44408000B00E19304000000");
        assert_eq!(
            rx1.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "origin gets nothing"
        );
        // Registration and online snapshot are dropped.
        assert_eq!(c.clients.lock().await.len(), 1);
        assert!(!online_sessions().lock().unwrap().contains_key(&300001));
    }
}

// ---------------------------------------------------------------------------
// src/server/session.rs — cross-player operation locks
// ---------------------------------------------------------------------------
mod session_locks {
    use std::time::Duration;
    use ts_dream::server::session::lock_player_operations;

    #[tokio::test]
    async fn player_locks_block_same_player_but_not_unrelated_player() {
        // Generous windows: only relative ordering matters, never wall-clock speed.
        let held = lock_player_operations([388_881]).await;
        assert!(tokio::time::timeout(
            Duration::from_millis(2000),
            lock_player_operations([388_882])
        )
        .await
        .is_ok());
        assert!(tokio::time::timeout(
            Duration::from_millis(250),
            lock_player_operations([388_881])
        )
        .await
        .is_err());
        drop(held);
        assert!(tokio::time::timeout(
            Duration::from_millis(2000),
            lock_player_operations([388_881])
        )
        .await
        .is_ok());
    }
}

// ---------------------------------------------------------------------------
// src/server/spawn.rs — login / world packet builders
// ---------------------------------------------------------------------------
mod spawn_frames {
    use ts_dream::server::session::{InventoryItem, PetState, Session};
    use ts_dream::server::spawn::{
        pet_summary, server_name_frame, session_offline_frame, sys_msg_frame,
    };

    #[test]
    fn pet_summary_empty_for_petless_session() {
        let s = Session::new();
        assert!(pet_summary(&s).is_empty(), "no pet frames with no pets");
    }

    #[test]
    fn pet_summary_builds_0f08_0f14_for_active_pet() {
        let mut s = Session::new();
        s.pets.push(PetState {
            stt: 1,
            id: 15001,
            name: b"PET1".to_vec(),
            level: 2,
            hp: 100,
            sp: 50,
            int1: 10,
            atk: 20,
            def: 30,
            agi: 40,
            hpx: 50,
            spx: 60,
            fai: 70,
            texp: 1234,
            skill_point: 5,
            quest: 0,
            skills: [(10001, 1), (0, 0), (0, 0), (0, 0)],
            ..Default::default()
        });
        // Pet equipment lives in the Trangbi table at slot `stt*10+1` (11).
        s.trangbi.push(InventoryItem {
            slot: 11,
            id: 20001,
            ..Default::default()
        });

        let frames = pet_summary(&s);
        assert_eq!(frames.len(), 8);
        // 0F14 slot summary: stt(01) + 0000 → 3 bytes → length 0x05.
        assert_eq!(frames[1], "F44405000F14010000");
        assert_eq!(frames[2], "F44402000F0A");
        assert_eq!(frames[3], "F44405000F12010000");
        assert_eq!(frames[4], "F44405000F12020000");
        assert_eq!(frames[5], "F44405000F12030000");
        assert_eq!(frames[6], "F44405000F12040000");
        assert_eq!(frames[7], "F44404000F130100");

        // 0F08: fixed prefix `00` + stt, id le32, texp le32.
        assert!(frames[0].starts_with("F444"));
        assert!(
            frames[0].contains("01993A0000D2040000"),
            "got {}",
            frames[0]
        );
        assert!(
            frames[0].contains("50455431"),
            "pet name PET1 embedded: {}",
            frames[0]
        );
        // Pet equipment slot 1 id (20001 = 0x4E21) + 6 zero bytes.
        assert!(
            frames[0].contains("214E0000"),
            "pet equip id: {}",
            frames[0]
        );
    }

    #[test]
    fn pet_summary_skips_stable_slots_and_id_zero() {
        let mut s = Session::new();
        // Stable slot (stt 7) and a zero-id pet must not appear.
        s.pets.push(PetState {
            stt: 7,
            id: 15002,
            ..Default::default()
        });
        s.pets.push(PetState {
            stt: 2,
            id: 0,
            ..Default::default()
        });
        assert!(pet_summary(&s).is_empty());
    }

    #[test]
    fn session_offline_frame_is_leave_hide_packet() {
        // Same literal as golden/13-battle-leave: F444 0800 0B00 + id + 0000.
        assert_eq!(session_offline_frame(300001), "F44408000B00E19304000000");
    }

    #[test]
    fn sys_msg_frame_never_emits_utf8() {
        // ASCII text is unchanged on the wire.
        assert_eq!(sys_msg_frame("TSVN"), "F4440A00020B000000005453564E");
        // Latin-1 accented é (U+00E9) travels as the single byte 0xE9 — never
        // as the two-byte UTF-8 pair C3 A9. The reverse map covers ≤0xFF.
        let f = sys_msg_frame("café");
        assert!(f.ends_with("636166E9"), "got {f}"); // 63 61 66 E9
        assert!(!f.contains("C3A9"), "got {f}");
        // Proper-Unicode Đ (U+0110) maps through the VISCII encoder to 0xD0;
        // ậ (U+1EAD) is 0xA7 in the positional table.
        let cfg = sys_msg_frame("Đậu2");
        assert!(cfg.ends_with("D0A77532"), "got {cfg}"); // Đ ậ u 2
        assert!(!cfg.contains("C490"), "got {cfg}");
    }

    #[test]
    fn server_name_frame_counts_viscii_bytes_not_utf8() {
        // "câu" = c(63) â(0xE2) u(75) → name_len = 3 VISCII bytes, hex 63 E2 75.
        let n = server_name_frame(1, "câu");
        assert!(n.ends_with("0363E275"), "got {n}");
        assert!(!n.contains("C3A2"), "got {n}");
    }
}

// ---------------------------------------------------------------------------
// src/server/inventory.rs — bag stacking rules
// ---------------------------------------------------------------------------
mod inventory_rules {
    use ts_dream::server::inventory::{add_item, can_add_item, remove_item};
    use ts_dream::server::session::InventoryItem;

    fn item(id: u16, count: u8) -> InventoryItem {
        InventoryItem {
            id,
            count,
            ..Default::default()
        }
    }

    #[test]
    fn add_fills_free_slots_in_order() {
        let mut bag = vec![item(1001, 1), item(1002, 1)];
        bag[0].slot = 1;
        bag[1].slot = 2;
        assert_eq!(add_item(&mut bag, item(1003, 1)), vec![3]);
        assert_eq!(bag[2].slot, 3);
        assert_eq!(bag[2].id, 1003);
    }

    #[test]
    fn add_stacks_onto_nonfull_existing_slot() {
        let mut bag = vec![item(1001, 40)];
        bag[0].slot = 1;
        // Cap-50 merge: fills slot 1 to 50, remainder (40) goes to a new slot.
        // Both slots are returned so the caller persists the straddle.
        assert_eq!(add_item(&mut bag, item(1001, 50)), vec![1, 2]);
        assert_eq!(bag[0].count, 50);
        assert_eq!(bag.len(), 2);
        assert_eq!(bag[1].id, 1001);
        assert_eq!(bag[1].count, 40);
    }

    #[test]
    fn add_rejects_when_full() {
        let mut bag: Vec<InventoryItem> = (1..=25)
            .map(|slot| InventoryItem {
                slot,
                id: 9000 + u16::from(slot),
                count: 1,
                ..Default::default()
            })
            .collect();
        assert!(add_item(&mut bag, item(999, 1)).is_empty());
        assert!(!can_add_item(&bag, &item(999, 1)));
    }

    #[test]
    fn can_add_item_accepts_existing_stack() {
        let bag = vec![InventoryItem {
            slot: 1,
            id: 999,
            count: 1,
            ..Default::default()
        }];
        assert!(can_add_item(&bag, &item(999, 1)));
    }

    #[test]
    fn remove_partial_and_full() {
        let mut bag = vec![item(1001, 3), item(1001, 2)];
        assert_eq!(remove_item(&mut bag, 1001, 4), 4);
        assert_eq!(bag[0].count, 1);
        assert_eq!(remove_item(&mut bag, 1001, 1), 1);
        assert!(bag.is_empty());
    }
}

// ---------------------------------------------------------------------------
// src/server/pet_box.rs — pet roster slots
// ---------------------------------------------------------------------------
mod pet_roster {
    use ts_dream::server::pet_box::{add_caught, next_active_slot, next_stable_slot};
    use ts_dream::server::session::PetState;

    fn pet(stt: u8, id: u16) -> PetState {
        PetState {
            stt,
            id,
            ..Default::default()
        }
    }

    #[test]
    fn add_caught_assigns_next_active_slot() {
        let mut pets = vec![pet(1, 1001)];
        assert_eq!(add_caught(&mut pets, 1002, 30), Some(2));
        assert_eq!(pets[1].stt, 2);
        assert_eq!(pets[1].level, 1);
    }

    #[test]
    fn add_caught_rejects_duplicate_id() {
        let mut pets = vec![pet(1, 1001)];
        assert_eq!(add_caught(&mut pets, 1001, 30), None);
        assert_eq!(pets.len(), 1);
    }

    #[test]
    fn slots_respect_ranges() {
        let pets: Vec<PetState> = (1..=4).map(|s| pet(s, 2000 + u16::from(s))).collect();
        assert_eq!(next_active_slot(&pets), None);
        assert_eq!(next_stable_slot(&pets), Some(5));
        // Stable bound scans 5..=10.
        let full: Vec<PetState> = (5..=10).map(|s| pet(s, 3000 + u16::from(s))).collect();
        assert_eq!(next_stable_slot(&full), None);
    }
}

// ---------------------------------------------------------------------------
// src/server/character_sheet.rs — stat derivation
// ---------------------------------------------------------------------------
mod character_sheet_math {
    use ts_dream::server::character_sheet::{CharacterSheet, GearBonuses};
    use ts_dream::server::session::InventoryItem;

    #[test]
    fn no_gear_yields_base_max_hp_sp() {
        let sheet = CharacterSheet::recompute(0, 0, 1, 0, 0, 1, &[]);
        assert_eq!(sheet.gear, GearBonuses::default());
        assert!(sheet.hp_max > 0);
        assert!(sheet.sp_max > 0);
    }

    #[test]
    fn gear_bonuses_aggregate_nonzero_items() {
        let trangbi = vec![
            InventoryItem {
                id: 1000,
                atk1: 15,
                hpx1: 20,
                ..Default::default()
            },
            InventoryItem {
                id: 0,
                atk1: 999,
                ..Default::default()
            },
        ];
        let gear = GearBonuses::from_gear(&trangbi, 1);
        assert_eq!(gear.atk2, 15);
        assert_eq!(gear.hpx2, 20);
        assert_eq!(gear.int2, 0);
    }

    #[test]
    fn gear_bonuses_include_elemental_2_stats() {
        // Item carries a base _1 and an elemental _2 field; both are summed.
        let trangbi = vec![InventoryItem {
            id: 1000,
            int1: 5,
            int2: 7,
            thuoctinh: 1,
            giatri_thuoctinh: 10,
            ..Default::default()
        }];
        // Player element 1 matches the item → the element bonus (10) is added
        // to each nonzero int field (`_X1` and `_X2` both nonzero → +20).
        let gear = GearBonuses::from_gear(&trangbi, 1);
        assert_eq!(gear.int2, 5 + 7 + 20); //
                                           // Player element 2 does not match (nor == 5): no bonus.
        let gear = GearBonuses::from_gear(&trangbi, 2);
        assert_eq!(gear.int2, 5 + 7);
    }

    #[test]
    fn gear_element_bonus_applies_per_nonzero_field_and_element_5() {
        // Both _1 and _2 nonzero → the bonus is added twice (per-field rule).
        let trangbi = vec![InventoryItem {
            id: 1000,
            int1: 1,
            int2: 2,
            giatri_thuoctinh: 10,
            giatri_long: 4,
            long_val: 1,
            thuoctinh: 5, // all-elements also matches
            ..Default::default()
        }];
        let gear = GearBonuses::from_gear(&trangbi, 1);
        // thuoctinh==5 matches; long_val==1 matches → 3 + 10*2 + 4*2
        assert_eq!(gear.int2, 1 + 2 + 20 + 8);
    }
}

// ---------------------------------------------------------------------------
// src/server/map_drops.rs — per-map drop registry
//
// The registry is keyed `(map_id, slot)`: every suite owns a disjoint map-id
// band, so parallel tests cannot collide. This file reserves ids ≥ 61001.
// Never call `map_drops::clear_all()` from a test — it wipes every map and
// races with concurrently running suites; clean up with `clear_map(map_id)`
// (or targeted `take`) instead.
// ---------------------------------------------------------------------------
mod map_drop_registry {
    use ts_dream::server::map_drops;
    use ts_dream::server::session::InventoryItem;

    /// Map id reserved for this suite — far from game maps (12xxx) and from
    /// handler-test bands (12xxx), so registry entries never overlap.
    const TEST_MAP: u16 = 61_001;

    #[test]
    fn drop_take_and_get() {
        let item = InventoryItem {
            id: 1001,
            count: 2,
            ..Default::default()
        };
        map_drops::drop(TEST_MAP, 3, item, 400, 500);
        let got = map_drops::get(TEST_MAP, 3).unwrap();
        assert_eq!(got.item.id, 1001);
        assert_eq!(got.item.count, 2);
        assert_eq!(got.map_x, 400);
        let taken = map_drops::take(TEST_MAP, 3).unwrap();
        assert_eq!(taken.item.id, 1001);
        assert!(map_drops::get(TEST_MAP, 3).is_none());
        // Scoped hygiene: only this suite's map id was ever touched, so no
        // global reset is needed (and none is performed).
        map_drops::clear_map(TEST_MAP);
    }
}

// ---------------------------------------------------------------------------
// src/server/dispatcher.rs — opcode routing
// ---------------------------------------------------------------------------
mod dispatcher_routing {
    use std::sync::Arc;
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::data::tables::{NpcOnMap, Skill};
    use ts_dream::protocol::encoder;
    use ts_dream::server::dispatcher::{dispatch, ServerEnv};
    use ts_dream::server::session::{online_sessions, Conn, Session};

    fn dummy_data() -> GameData {
        let mut data = GameData::default();
        data.skills.insert(
            10001,
            Skill {
                id: 10001,
                point: 1,
                lv_max: 10,
                ..Default::default()
            },
        );
        data
    }

    fn dummy_service() -> BattleService {
        BattleService::new(Arc::new(GameData::default()))
    }

    #[tokio::test]
    async fn hello_replies() {
        let mut conn = Conn::new();
        // frame: F4 44 01 00 00 (opcode 0x00, length 1, no sub byte).
        let decoded = encoder::bytes("F444010000").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(out.outgoing, vec!["F4440300010901"]);
    }

    #[tokio::test]
    async fn hello_with_sub_byte_is_silent() {
        let mut conn = Conn::new();
        // op 0x00 with a sub byte and empty payload is NOT the exact
        // `F444010000` frame — anything else must be silently ignored (§2.3.1).
        let decoded = encoder::bytes("F44402000000").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(out.outgoing.is_empty());
    }

    #[tokio::test]
    async fn login_version_too_low_causes_shutdown() {
        let mut conn = Conn::new();
        // Login payload with version 100 (< 186): opcode 0x01 sub 0x01 id=1 prefix="vn" ver=100 pass="123"
        let decoded = encoder::bytes("F4440B00010101000000766E6400313233").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(out.shutdown);
    }

    #[tokio::test]
    async fn login_wrong_password() {
        let mut conn = Conn::new();
        // ver=186 (0xBA), pass="WRONG"
        let decoded = encoder::bytes("F4440D00010101000000766EBA0057524F4E47").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(out.outgoing, vec!["F44402000106"]);
        assert!(!out.shutdown);
        assert!(
            !conn.session.authed,
            "auth flag must only be set once login succeeds (wrong pass)"
        );
    }

    #[tokio::test]
    async fn create_character_name_check_and_creation() {
        let mut conn = Conn::new();

        // 1. Name check free: opcode 0x09 sub 2 name "TESTNAME"
        let name_check_decoded = encoder::bytes("F4440A000902544553544E414D45").unwrap();
        let out1 = dispatch(
            &mut conn,
            &name_check_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(out1.outgoing, vec!["F4440300090300"]);
        assert_eq!(conn.session.pending_new_char_name, b"TESTNAME");

        // 2. Create character: opcode 0x09 sub 1 with valid payload
        let mut payload = vec![0u8; 26];
        payload[0] = 1; // sex
        payload[2] = 2; // hair
        payload[12] = 3; // element
        payload[19] = 4; // pass1 len
        let mut frame_bytes = vec![0xF4, 0x44, 28, 0x00, 0x09, 0x01];
        frame_bytes.extend(payload);

        let out2 = dispatch(
            &mut conn,
            &frame_bytes,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(out2.outgoing, vec!["F44402000901"]);
    }

    #[tokio::test]
    async fn move_broadcasts_to_map() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Move opcode 0x06 sub 1: dir=2, x=100 (0x0064), y=200 (0x00C8)
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(
            &mut conn,
            &move_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.map_x, 100);
        assert_eq!(conn.session.map_y, 200);
        assert_eq!(conn.session.gocnhin, 2);
        // The walk is a map broadcast — the mover never receives its own echo.
        assert!(
            out.outgoing.is_empty(),
            "mover must not receive its own walk"
        );
        assert_eq!(out.map_broadcast.len(), 1);
        assert_eq!(out.map_broadcast[0].subject, 300001, "subject is the mover");
        assert!(out.map_broadcast[0].frame.starts_with("F4440B000601"));
    }

    #[tokio::test]
    async fn move_ignored_while_in_battle() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.battle_id = 7;
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(
            &mut conn,
            &move_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(out.outgoing.is_empty(), "move must be ignored in battle");
        assert!(out.map_broadcast.is_empty());
    }

    #[tokio::test]
    async fn move_leader_moves_party_members() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.id_leader = 300001;
        conn.session.id_mem = [300002, 300003, 0, 0];
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(
            &mut conn,
            &move_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        // Leader + 2 members → 3 map-broadcast walks; none echoed to the mover.
        assert!(
            out.outgoing.is_empty(),
            "mover must not receive its own walk"
        );
        assert_eq!(out.map_broadcast.len(), 3);
        assert_eq!(out.map_broadcast[0].subject, 300001);
        assert_eq!(out.map_broadcast[1].subject, 300002);
        assert_eq!(out.map_broadcast[2].subject, 300003);
        assert!(out.map_broadcast[0]
            .frame
            .starts_with("F4440B000601E1930400"));
        assert!(out.map_broadcast[1]
            .frame
            .starts_with("F4440B000601E2930400")); // member 300002
        assert!(out.map_broadcast[2]
            .frame
            .starts_with("F4440B000601E3930400")); // member 300003
    }

    #[tokio::test]
    async fn move_member_following_leader_stays_still() {
        let mut conn = Conn::new();
        conn.session.id = 300002;
        conn.session.id_leader = 300001; // not self
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(
            &mut conn,
            &move_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(out.outgoing.is_empty(), "member does not self-broadcast");
        assert!(out.map_broadcast.is_empty(), "member sends no walks at all");
    }

    #[tokio::test]
    async fn leader_move_persists_member_positions() {
        // P4: a leading walker persists each member's new position into the
        // shared online registry. Uses the 93xxxx player-id band so the seeded
        // entries never collide with other suites.
        let _registry = super::REGISTRY_LOCK.lock().await;
        const LEADER: u32 = 930_001;
        const MEM1: u32 = 930_002;
        const MEM2: u32 = 930_003;
        {
            let mut sessions = online_sessions().lock().unwrap();
            sessions.insert(MEM1, Session::new());
            sessions.insert(MEM2, Session::new());
        }
        let mut conn = Conn::new();
        conn.session.id = LEADER;
        conn.session.id_leader = LEADER;
        conn.session.id_mem = [MEM1, MEM2, 0, 0];
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let _out = dispatch(
            &mut conn,
            &move_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;

        let sessions = online_sessions().lock().unwrap();
        let mem1 = sessions.get(&MEM1).expect("member 930002 online");
        assert_eq!(mem1.map_x, 100);
        assert_eq!(mem1.map_y, 200);
        assert_eq!(mem1.gocnhin, 2);
        let mem2 = sessions.get(&MEM2).expect("member 930003 online");
        assert_eq!(mem2.map_x, 100);
        assert_eq!(mem2.map_y, 200);
        assert_eq!(mem2.gocnhin, 2);
        drop(sessions);

        // Cleanup so parallel tests never see the seeded members.
        let mut sessions = online_sessions().lock().unwrap();
        sessions.remove(&MEM1);
        sessions.remove(&MEM2);
    }

    #[tokio::test]
    async fn op_005_is_not_routed_to_move() {
        // P5: the spec lists only op 0x06 as Move; 0x05 (stats/appearance) must
        // not be fed into handle_move (a sub-1 0x05 frame would otherwise walk).
        let mut conn = Conn::new();
        conn.session.id = 300001;
        let decoded = encoder::bytes("F44407000501026400C800").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(out.outgoing.is_empty());
        assert!(out.map_broadcast.is_empty());
        assert_eq!(
            conn.session.map_x,
            Session::new().map_x,
            "position untouched"
        );
    }

    #[tokio::test]
    async fn expression_handling() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Expression sub 2 action=5
        let expr_decoded = encoder::bytes("F4440300200205").unwrap();
        let out = dispatch(
            &mut conn,
            &expr_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.dongtac, 5);
        // Map broadcast only — never echoed to the actor.
        assert!(out.outgoing.is_empty());
        assert_eq!(out.map_broadcast.len(), 1);
        assert_eq!(out.map_broadcast[0].subject, 300001);
        assert_eq!(out.map_broadcast[0].frame, "F44407002002E193040005");
    }

    #[tokio::test]
    async fn chat_slash_command_where() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        // Chat "/where": op 0x02 sub 2 msg="/where"
        let chat_decoded = encoder::bytes("F4440C0002022F7768657265").unwrap();
        let out = dispatch(
            &mut conn,
            &chat_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].contains("020B")); // sys msg frame
    }

    #[tokio::test]
    async fn dispatch_stat_allocation_and_hotkey() {
        let mut conn = Conn::new();
        conn.session.point = 10;
        // Op 0x08 sub 1: stat_id 27 (Int), points 3 -> hex: F444 0600 0801 0000 1B03
        let decoded = encoder::bytes("F4440600080100001B03").unwrap();
        let out = dispatch(
            &mut conn,
            &decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.point, 7);
        assert_eq!(conn.session.int1, 3);
        assert_eq!(out.outgoing.len(), 2);

        // Op 0x28 sub 1: skill 10001 (0x2711), slot 5 -> hex: F444 0400 2801 1127 05
        let decoded_hotkey = encoder::bytes("F4440600280100112705").unwrap();
        let out_hk = dispatch(
            &mut conn,
            &decoded_hotkey,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.hotkeys[5], 10001);
        assert!(out_hk.outgoing.is_empty());
    }

    #[tokio::test]
    async fn dispatch_npc_shop_player_shop_and_skill_learn() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.idtalking = 16;
        conn.session.map_id = 12002;
        conn.session.gold = 100000;
        conn.session.skill_point = 5;

        // NPC Shop buy (op 0x1B): menu 0 at map 12002 → item 20023 @ 58800 → gold 41200
        let shop_decoded = encoder::bytes("F44404001B010000").unwrap();
        let out_shop = dispatch(
            &mut conn,
            &shop_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.gold, 41200);
        assert!(conn.session.homdo.iter().any(|i| i.id == 20023));
        assert!(out_shop.outgoing.iter().any(|f| f.contains("1A04")));

        // Player shop open (op 0x17 sub 30): name "TEST" + one listing.
        let open_hex = ts_dream::protocol::frame("171E", "045445535400");
        let open_decoded = encoder::bytes(&open_hex).unwrap();
        let out_open = dispatch(
            &mut conn,
            &open_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(conn.session.shop.active);
        assert!(out_open.outgoing.iter().any(|f| f.contains("171E")));

        // Player shop close (op 0x17 sub 31 / wire 171F): reply 1720 + player id.
        let close_hex = ts_dream::protocol::frame("171F", "");
        let close_decoded = encoder::bytes(&close_hex).unwrap();
        let out_close = dispatch(
            &mut conn,
            &close_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert!(!conn.session.shop.active);
        assert!(out_close.outgoing.iter().any(|f| f.contains("1720")));

        // Skill learn skill 10001 (0x2711) lv 1 -> F444 0500 1C01 1127 01
        let skill_decoded = encoder::bytes("F44405001C01112701").unwrap();
        let out_skill = dispatch(
            &mut conn,
            &skill_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skill_point, 4);
        assert_eq!(out_skill.outgoing.len(), 2);
    }

    #[tokio::test]
    async fn dispatch_trade_bank_pk_and_pets() {
        let mut conn = Conn::new();
        conn.session.gold = 5000;
        conn.session.bank_gold = 2000;

        // Op 0x1D sub 1: withdraw 1000 gold (LE32 request).
        let bank_decoded = encoder::bytes("F44406001D01E8030000").unwrap();
        let out_bank = dispatch(
            &mut conn,
            &bank_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.gold, 6000);
        assert_eq!(conn.session.bank_gold, 1000);
        assert_eq!(
            out_bank.outgoing,
            vec!["F44406001D02E8030000", "F44406001A01E8030000"]
        );

        // Op 0x21 sub 1: set PK = 1 -> F444 0300 2101 01
        let pk_decoded = encoder::bytes("F4440300210101").unwrap();
        let out_pk = dispatch(
            &mut conn,
            &pk_decoded,
            &dummy_data(),
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.pk, 1);
        assert_eq!(out_pk.outgoing[0], "F444040021020100");

        // Op 0x14 sub 1: talk start — map object 10 (the map's banker entry)
        // resolves to template 16080 via NpcOnMap -> F444 0400 1401 0A00.
        let mut data = dummy_data();
        data.npc_on_map.push(NpcOnMap {
            map_id: i64::from(conn.session.map_id),
            id: 10,
            npc_id: 16080,
            x: 401,
            y: 501,
            ..Default::default()
        });
        let talk_decoded = encoder::bytes("F444040014010A00").unwrap();
        let out_talk = dispatch(
            &mut conn,
            &talk_decoded,
            &data,
            &dummy_service(),
            &ServerEnv::none(),
        )
        .await;
        assert_eq!(conn.session.idtalking, 10);
        assert_eq!(conn.session.idnpctalking, 16080);
        assert_eq!(out_talk.outgoing.len(), 2);
    }
}
