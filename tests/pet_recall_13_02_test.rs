//! Ticket 07 — `13 02` pet recall branch.
//!
//! `LAB_00644A18` (HANDOFF §14/§15, G11; evidence `00644a18:60` + asm:70-72)
//! is the delayed callback the client runs a few hundred ms after leaving a
//! battle: when the followed pet is dead (`pet+0x3EA <= 1`) it clears the slot
//! and sends C→S `13 02` ("recall pet"). The Rust handler already exists
//! (`pet_actions.rs` sub 2); these tests pin its two branches — an active pet is
//! cleared and echoed, and the recall is silent when no pet is active.

use ts_dream::data::loader::GameData;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::pet_actions;
use ts_dream::server::session::{Conn, PetState};

fn ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a ts_dream::battle::service::BattleService,
    out: &'a mut HandleOutcome,
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        out,
        env: ServerEnv::none(),
        data,
        service,
        decoded: &[],
        opcode: 0x13,
        sub: 2,
        payload: &[],
    }
}

/// `13 02` with an active pet: `active_pet_stt` is cleared and the client gets
/// the exact echo `F44402001302`.
#[tokio::test]
async fn recall_13_02_with_active_pet_clears_and_echoes() {
    let mut conn = Conn::default();
    conn.session.id = 50;
    conn.session.active_pet_stt = 2;
    conn.session.pets.push(PetState {
        stt: 2,
        id: 1200,
        ..Default::default()
    });

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    let mut c = ctx(&mut conn, &data, &service, &mut out);
    pet_actions::handle_pet_summon(&mut c).await;

    assert_eq!(conn.session.active_pet_stt, 0, "recall clears the active pet");
    assert_eq!(out.outgoing.len(), 1, "recall echoes exactly one frame");
    assert_eq!(out.outgoing[0].frame, "F44402001302");
}

/// `13 02` when `active_pet_stt == 0` is a silent no-op.
#[tokio::test]
async fn recall_13_02_without_active_pet_is_silent() {
    let mut conn = Conn::default();
    conn.session.id = 51;
    conn.session.active_pet_stt = 0;

    let data = GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();

    let mut c = ctx(&mut conn, &data, &service, &mut out);
    pet_actions::handle_pet_summon(&mut c).await;

    assert_eq!(conn.session.active_pet_stt, 0);
    assert!(out.outgoing.is_empty(), "nothing to recall → no frame");
}
