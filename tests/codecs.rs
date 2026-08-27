use ts_dream::protocol::codecs::{
    ehuman, BattleRoleData, BattleRoleSerializer, FriendExtra, PlayerCard, PlayerInfoCodec,
    ThingData, ThingDataCodec, FRIEND_EXTRA_SIZE, THING_DATA_SIZE,
};
use ts_dream::protocol::{PacketReader, PacketWriter, XOR_KEY};

#[test]
fn test_thing_data_codec_35_bytes_exact() {
    let item = ThingData {
        item_id: 11025,
        quantity: 99,
        damage: 5,
        element: 2, // Water
        element_value: 30,
        proof_kind: 1,
        grow_level: 10,
        grow_exp: 50000,
        special_kind: 1,
        stone_attr: 3,
        stone_level: 8,
        enhance_level: 12,
        delete_time: 45000.125,
        damaged_item_id: 11000,
        is_locked: true,
        reinforced: 4,
        affix1: 7,
        affix2: 14,
        affix3: 21,
        style_level: 5,
    };

    let encoded_bytes = ThingDataCodec::encode(&item);
    assert_eq!(encoded_bytes.len(), THING_DATA_SIZE);
    assert_eq!(encoded_bytes.len(), 35);

    let decoded = ThingDataCodec::decode(&encoded_bytes).expect("Failed to decode ThingData");
    assert_eq!(item, decoded);

    // Verify through PacketWriter / PacketReader integration
    let mut writer = PacketWriter::empty();
    writer.write_thing_data(&item);
    assert_eq!(writer.len(), 35);

    let mut reader = PacketReader::new(writer.as_slice());
    let reader_decoded = reader
        .read_thing_data()
        .expect("PacketReader read_thing_data failed");
    assert_eq!(item, reader_decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_thing_data_batch_encoding() {
    let items = vec![
        ThingData {
            item_id: 20001,
            quantity: 1,
            ..Default::default()
        },
        ThingData {
            item_id: 20002,
            quantity: 5,
            damage: 20,
            ..Default::default()
        },
        ThingData::empty(),
    ];

    let mut writer = PacketWriter::empty();
    for item in &items {
        writer.write_thing_data(item);
    }
    assert_eq!(writer.len(), 35 * 3);

    let mut reader = PacketReader::new(writer.as_slice());
    for expected in &items {
        let actual = reader.read_thing_data().unwrap();
        assert_eq!(actual, *expected);
    }
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_player_card_and_friend_extra() {
    let card = PlayerCard {
        name: "GiaCátLượng".to_string(),
        level: 150,
        element: 4, // Wind
        turn3_element: 3,
        turn: 2,
        career: 3, // Adviser / Mưu sĩ
        sex: 1,
        hair: 2,
        color1: 0x00FF00AA,
        color2: 0x0000FF55,
    };

    let extra = FriendExtra {
        online: true,
        friendly: 9999,
        function_flag: 1,
        add_time: 44000.5,
        offline_time: 44010.5,
    };

    let mut writer = PacketWriter::empty();
    PlayerInfoCodec::write_player_card(&mut writer, &card);
    PlayerInfoCodec::write_friend_extra(&mut writer, &extra);

    let mut reader = PacketReader::new(writer.as_slice());
    let decoded_card = reader.read_player_card().unwrap();
    let decoded_extra = reader.read_friend_extra().unwrap();

    assert_eq!(card, decoded_card);
    assert_eq!(extra, decoded_extra);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_friend_extra_fixed_20_bytes() {
    let extra = FriendExtra {
        online: false,
        friendly: 1234,
        function_flag: 0,
        add_time: 12345.678,
        offline_time: 87654.321,
    };

    let bytes = extra.to_bytes();
    assert_eq!(bytes.len(), FRIEND_EXTRA_SIZE);
    assert_eq!(bytes.len(), 20);

    let decoded = FriendExtra::from_bytes(&bytes).unwrap();
    assert_eq!(extra, decoded);
}

#[test]
fn test_battle_role_player_appearance_roundtrip() {
    let player_role = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::PLAYER,
        role_id: 300005,
        npc_id: 0,
        master_id: 0,
        col: 2,
        row: 1,
        max_hp: 5000,
        max_sp: 2500,
        hp: 4800,
        sp: 2400,
        level: 160,
        upgrade_lv: 2,
        element: 1, // Earth
        name: "ChiếnThần".to_string(),
        sex: 1,
        face: 2,
        hair: 7,
        color1: 0x12345678,
        color2: 0x87654321,
        turn: 2,
        career: 1,
        equip_item_ids: vec![12001, 12002, 12003, 12004, 12005, 12006],
        outfit_item_ids: vec![19001, 19002],
    };

    let bytes = BattleRoleSerializer::serialize(&player_role);
    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader)
        .expect("Failed to deserialize player BattleRoleData");

    assert_eq!(player_role, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_battle_role_follow_npc_roundtrip() {
    let pet_role = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::FOLLOW_NPC,
        role_id: 40010,
        npc_id: 11005,
        master_id: 300005,
        col: 1,
        row: 1,
        max_hp: 4200,
        max_sp: 1800,
        hp: 4200,
        sp: 1800,
        level: 120,
        upgrade_lv: 1,
        element: 3,
        name: "Quan Vũ".to_string(),
        ..Default::default()
    };

    let bytes = BattleRoleSerializer::serialize(&pet_role);
    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader)
        .expect("Failed to deserialize pet BattleRoleData");

    assert_eq!(pet_role, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_battle_role_mine_npc_roundtrip() {
    let mob = BattleRoleData {
        war_type: 0,
        human_kind: ehuman::MINE_NPC,
        role_id: 50020,
        npc_id: 13001,
        master_id: 0,
        col: 3,
        row: 2,
        max_hp: 850,
        max_sp: 300,
        hp: 850,
        sp: 300,
        level: 45,
        upgrade_lv: 0,
        element: 2,
        ..Default::default()
    };

    let bytes = BattleRoleSerializer::serialize(&mob);
    // Base fields without appearance = 42 bytes (1+1+8+2+8+1+1+4+4+4+4+2+1+1)
    assert_eq!(bytes.len(), 42);

    let mut reader = PacketReader::new(&bytes);
    let decoded = BattleRoleSerializer::deserialize(&mut reader)
        .expect("Failed to deserialize mob BattleRoleData");

    assert_eq!(mob, decoded);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_battle_role_full_packet_frame_and_xor() {
    let player = BattleRoleData {
        war_type: 1,
        human_kind: ehuman::PLAYER,
        role_id: 300001,
        npc_id: 0,
        master_id: 0,
        col: 1,
        row: 1,
        max_hp: 1000,
        max_sp: 500,
        hp: 1000,
        sp: 500,
        level: 50,
        upgrade_lv: 0,
        element: 1,
        name: "Hero".to_string(),
        sex: 1,
        face: 1,
        hair: 1,
        color1: 0,
        color2: 0,
        turn: 0,
        career: 0,
        equip_item_ids: vec![10001],
        outfit_item_ids: vec![],
    };

    let mut writer = PacketWriter::new(0x0B, 0x05); // Opcode 0x0B sub 0x05 (Role appear)
    writer.write_battle_role(&player);

    let frame = writer.build_frame();
    assert_eq!(&frame[0..2], &[0xF4, 0x44]); // Header magic
    let body_len = u16::from_le_bytes([frame[2], frame[3]]) as usize;
    assert_eq!(body_len, writer.len());
    assert_eq!(&frame[4..6], &[0x0B, 0x05]); // Opcode and sub

    let wire = writer.build_wire();
    assert_eq!(wire.len(), frame.len());

    // Decrypt wire XOR 0xAD
    let decrypted: Vec<u8> = wire.iter().map(|b| b ^ XOR_KEY).collect();
    assert_eq!(decrypted, frame);

    // Read payload from frame body after opcode/sub (bytes 6..)
    let mut reader = PacketReader::new(&frame[6..]);
    let decoded_player = reader.read_battle_role().unwrap();
    assert_eq!(player, decoded_player);
    assert_eq!(reader.remaining(), 0);
}
