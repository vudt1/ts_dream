//! Compatibility boundary for documented but not-yet-semantic opcode families.
//!
//! This handler is deliberately conservative: it validates the dispatcher
//! contract, emits structured diagnostics, and never mutates player state or
//! fabricates a success response. It prevents valid documented packets from
//! disappearing in an unobservable wildcard branch while deeper Kotlin/client
//! parity is implemented.

use crate::protocol::is_documented_client_opcode;
use crate::server::dispatcher::OpcodeCtx;

/// Handle a documented opcode whose full business semantics are not yet safe
/// to infer from the available payload corpus. Returning without a response is
/// intentional for unsupported subcodes; the client can retry or disconnect,
/// while the bounded trace gives operators a reproducible audit trail.
pub fn handle(ctx: &mut OpcodeCtx<'_>) {
    if !is_documented_client_opcode(ctx.opcode) {
        tracing::warn!(
            opcode = ctx.opcode,
            sub = ctx.sub,
            payload_len = ctx.payload.len(),
            "received undocumented client opcode"
        );
        return;
    }

    tracing::debug!(
        opcode = ctx.opcode,
        sub = ctx.sub,
        payload_len = ctx.payload.len(),
        "documented opcode reached compatibility boundary"
    );
}
