use ts_dream::protocol::{SystemAlert, SystemAlertReason, OP_SYSTEM_ALERT};

#[test]
fn test_system_alert_opcode_and_codes() {
    assert_eq!(OP_SYSTEM_ALERT, 0x00);
    assert_eq!(SystemAlertReason::Disconnected.sub_code(), 0);
    assert_eq!(SystemAlertReason::VersionMismatch.sub_code(), 10);
    assert_eq!(SystemAlertReason::ServerBusy.sub_code(), 56);
    assert_eq!(SystemAlertReason::UnknownReason.sub_code(), 57);
}

#[test]
fn test_system_alert_from_u8() {
    assert_eq!(SystemAlertReason::from_u8(0), SystemAlertReason::Disconnected);
    assert_eq!(SystemAlertReason::from_u8(10), SystemAlertReason::VersionMismatch);
    assert_eq!(SystemAlertReason::from_u8(30), SystemAlertReason::AccountLocked);
    assert_eq!(SystemAlertReason::from_u8(56), SystemAlertReason::ServerBusy);
    assert_eq!(SystemAlertReason::from_u8(57), SystemAlertReason::UnknownReason);
    assert_eq!(SystemAlertReason::from_u8(200), SystemAlertReason::UnknownReason);
}

#[test]
fn test_system_alert_frame_hex_encoding() {
    assert_eq!(SystemAlertReason::Disconnected.to_hex(), "F44402000000");
    assert_eq!(SystemAlertReason::VersionMismatch.to_hex(), "F4440200000A");
    assert_eq!(SystemAlertReason::ServerBusy.to_hex(), "F44402000038");
    assert_eq!(SystemAlert::disconnected(), "F44402000000");
    assert_eq!(SystemAlert::version_mismatch(), "F4440200000A");
    assert_eq!(SystemAlert::account_locked(), "F4440200001E");
    assert_eq!(SystemAlert::server_busy(), "F44402000038");
    assert_eq!(SystemAlert::unknown_reason(), "F44402000039");
}

#[test]
fn test_system_alert_descriptions_not_empty() {
    for code in 0..=57 {
        let reason = SystemAlertReason::from_u8(code);
        assert!(!reason.description().is_empty());
        let hex = reason.to_hex();
        assert!(hex.starts_with("F444020000"));
        assert_eq!(hex.len(), 12);
    }
}
