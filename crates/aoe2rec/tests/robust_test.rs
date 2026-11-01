use aoe2rec::{actions::ActionData, actions::Game, parse_operations, Operation, Savegame};
use binrw::BinReaderExt;
use std::io::Cursor;
use std::path::Path;

#[test]
fn test_parse_ai_operation() {
    let mut data = Cursor::new(vec![
        0x07, 0x00, 0x00, 0x00, // Magic 7 (AI)
        0x04, 0x00, 0x00, 0x00, // Length 4
        0xDE, 0xAD, 0xBE, 0xEF, // AI Data
    ]);
    let op: Operation = data.read_le_args((1u16,)).unwrap();
    if let Operation::Ai { length, data } = op {
        assert_eq!(length, 4);
        assert_eq!(data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    } else {
        panic!("Expected AI operation");
    }
}

#[test]
fn test_parse_operations_resync() {
    // parse_operations stops at magic 6.
    let mut data = Cursor::new(vec![
        0x99, 0x99, 0x99, // 3 bytes of garbage
        0x08, 0x00, 0x00, 0x00, // Magic 8 (MapNote)
        0x02, 0x00, 0x00, 0x00, // Length 2
        0x01, 0x02, // Data
        0x06, 0x00, 0x00, 0x00, // Magic 6 (PostGame)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // Padding for PostGame seeks
        0xCE, 0xA4, 0x59, 0xB1, 0x05, 0xDB, 0x7B, 0x43, // End bit
    ]);

    let ops = parse_operations(&mut data, binrw::Endian::Little, (1u16,)).unwrap();
    assert_eq!(ops.len(), 2);
    if let Operation::MapNote { length, .. } = &ops[0] {
        assert_eq!(*length, 2);
    } else {
        panic!("Expected MapNote operation");
    }
}

#[test]
fn test_parse_action_with_failed_data() {
    // Action magic = 1
    // Action header: length (u32), data (Vec<u8>), world_time (u32)
    let mut data = Cursor::new(vec![
        0x01, 0x00, 0x00, 0x00, // Magic 1 (Action)
        0x04, 0x00, 0x00, 0x00, // Length 4
        0x00, 0x00, 0x00, 0x00, // Garbage action data
        0x00, 0x00, 0x00, 0x00, // World time
    ]);
    let op: Operation = data.read_le_args((1u16,)).unwrap();
    if let Operation::Action { action_data, .. } = op {
        assert!(action_data.is_none());
    } else {
        panic!("Expected Action operation");
    }
}

#[test]
fn test_parse_ai_research_action_without_selected_buildings() {
    let mut data = Cursor::new(vec![
        0x01, 0x00, 0x00, 0x00, // Magic 1 (Action)
        0x11, 0x00, 0x00, 0x00, // Payload length 17
        0x65, // Action type 101 (Research)
        0x02, // Player 2
        0x0d, 0x00, // Action length 13
        0x84, 0x0b, 0x00, 0x00, // Building ID 2948
        0x01, 0x00, // Selected 1
        0x16, 0x00, // Technology 22
        0xff, 0xff, 0xff, 0xff, 0x00, // Unknown trailer
        0xbd, 0x6a, 0x02, 0x00, // World time 158189
    ]);

    let op: Operation = data.read_le_args((66u16,)).unwrap();
    match op {
        Operation::Action {
            action_data:
                Some(ActionData::Research {
                    player_id,
                    action_length,
                    building_id,
                    selected,
                    technology_type,
                    building_ids,
                    ..
                }),
            ..
        } => {
            assert_eq!(player_id, 2);
            assert_eq!(action_length, 13);
            assert_eq!(building_id, 2948);
            assert_eq!(selected, 1);
            assert_eq!(technology_type, 22);
            assert!(building_ids.is_empty());
        }
        _ => panic!("Expected parsed AI research action"),
    }
}

#[test]
fn test_parse_research_action_with_selected_buildings() {
    let mut data = Cursor::new(vec![
        0x01, 0x00, 0x00, 0x00, // Magic 1 (Action)
        0x15, 0x00, 0x00, 0x00, // Payload length 21
        0x65, // Action type 101 (Research)
        0x01, // Player 1
        0x11, 0x00, // Action length 17
        0x7d, 0x0b, 0x00, 0x00, // Building ID 2941
        0x01, 0x00, // Selected 1
        0x16, 0x00, // Technology 22
        0xff, 0xff, 0xff, 0xff, 0x00, // Unknown trailer
        0x7d, 0x0b, 0x00, 0x00, // Selected building ID
        0xce, 0xbe, 0x07, 0x00, // World time 507470
    ]);

    let op: Operation = data.read_le_args((66u16,)).unwrap();
    match op {
        Operation::Action {
            action_data:
                Some(ActionData::Research {
                    player_id,
                    action_length,
                    building_id,
                    selected,
                    technology_type,
                    building_ids,
                    ..
                }),
            ..
        } => {
            assert_eq!(player_id, 1);
            assert_eq!(action_length, 17);
            assert_eq!(building_id, 2941);
            assert_eq!(selected, 1);
            assert_eq!(technology_type, 22);
            assert_eq!(building_ids, vec![2941]);
        }
        _ => panic!("Expected parsed research action with selected buildings"),
    }
}

#[test]
fn test_parse_game_cheat_action() {
    let mut data = Cursor::new(vec![
        0x01, 0x00, 0x00, 0x00, // Magic 1 (Action)
        0x14, 0x00, 0x00, 0x00, // Payload length 20
        0x67, // Action type 103 (Game)
        0x01, // Player 1
        0x10, 0x00, // Action length 16
        0x06, 0x00, 0x00, 0x00, // Game command 6 (Cheat)
        0x01, 0x00, // unknown1 = 1
        0x76, 0x00, // cheat_id = 118
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 8 bytes padding
        0xaa, 0x0f, 0x00, 0x00, // World time 4010
    ]);

    let op: Operation = data.read_le_args((66u16,)).unwrap();
    match op {
        Operation::Action {
            action_data:
                Some(ActionData::Game {
                    player_id,
                    action_length,
                    game_command: Game::Cheat { unknown1, cheat_id },
                }),
            world_time,
            ..
        } => {
            assert_eq!(player_id, 1);
            assert_eq!(action_length, 16);
            assert_eq!(unknown1, 1);
            assert_eq!(cheat_id, 118);
            assert_eq!(world_time, 4010);
        }
        _ => panic!("Expected parsed Game::Cheat action"),
    }
}

#[test]
fn test_parse_game_commands() {
    // Test FarmAutoqueue (16 / 0x10)
    let mut data = Cursor::new(vec![
        0x01, 0x00, 0x00, 0x00, // Magic 1 (Action)
        0x14, 0x00, 0x00, 0x00, // Payload length 20
        0x67, // Action type 103 (Game)
        0x02, // Player 2
        0x10, 0x00, // Action length 16
        0x10, 0x00, 0x00, 0x00, // Game command 16 (FarmAutoqueue)
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 12 bytes
        0x50, 0x00, 0x00, 0x00, // World time
    ]);

    let op: Operation = data.read_le_args((66u16,)).unwrap();
    match op {
        Operation::Action {
            action_data:
                Some(ActionData::Game {
                    player_id,
                    action_length,
                    game_command: Game::FarmAutoqueue {},
                }),
            ..
        } => {
            assert_eq!(player_id, 2);
            assert_eq!(action_length, 16);
        }
        _ => panic!("Expected parsed Game::FarmAutoqueue action"),
    }
}

#[test]
fn test_parse_cheats_recorded_game() {
    let path = Path::new("replay-cheats.aoe2record");
    let save = Savegame::from_file(path).unwrap();
    let cheats: Vec<_> = save
        .operations()
        .filter_map(|op| {
            if let Operation::Action {
                action_data:
                    Some(ActionData::Game {
                        game_command: Game::Cheat { unknown1, cheat_id },
                        ..
                    }),
                ..
            } = op
            {
                Some((*unknown1, *cheat_id))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(cheats.len(), 13);
    assert!(cheats.iter().all(|(_, id)| *id > 0));
    assert_eq!(cheats[0], (1, 100));
}

#[test]
fn test_parse_replay_aoe2record() {
    let path = Path::new("replay-ai.aoe2record");
    let save = Savegame::from_file(path).unwrap();
    assert_eq!(save.chapters[0].zheader.version_major, 67);
    assert_eq!(save.chapters[0].zheader.version_minor, 2);
    assert_eq!(save.operations().count(), 154047);
    assert_eq!(save.get_duration(), 2802638);
}

#[test]
fn test_parse_resume_aoe2record() {
    let path = Path::new("replay-resume.aoe2record");
    let save = Savegame::from_file(path).unwrap();
    assert_eq!(save.chapters[0].zheader.version_major, 68);
    assert_eq!(save.chapters[0].zheader.version_minor, 0);
    assert_eq!(save.operations().count(), 205277);
    assert_eq!(save.get_duration(), 3526015);
}

#[test]
fn test_load_shared_control_replay() {
    let path = Path::new("replay-shared.aoe2record");
    let res = Savegame::from_file(path);
    assert!(res.is_ok(), "Failed to parse shared-MP replay: {:?}", res.err());
    let save = res.unwrap();
    assert_eq!(save.get_duration(), 2161380);
    assert_eq!(save.operations().count(), 144051);
}

#[test]
fn test_parse_campaign_replay() {
    let path = Path::new("replay-campaign.aoe2record");
    let save = Savegame::from_file(path).unwrap();
    assert_eq!(save.get_duration(), 8088016);
    assert_eq!(save.operations().count(), 1654314);
}

#[test]
fn test_parse_synthetic_de_sync_checksum() {
    let mut data = Vec::new();
    // Magic 2 (Sync)
    data.extend_from_slice(&2u32.to_le_bytes());
    // time_increment = 100
    data.extend_from_slice(&100u32.to_le_bytes());
    // Checksum marker (next == 0)
    data.extend_from_slice(&0u32.to_le_bytes());

    // 88 u32s for 8 players
    for slot in 0..8 {
        if slot == 0 {
            // Player 1
            data.extend_from_slice(&0u32.to_le_bytes()); // val 0: unknown
            data.extend_from_slice(&500u32.to_le_bytes()); // val 1: total_resources
            data.extend_from_slice(&123u32.to_le_bytes()); // val 2: unknown
            data.extend_from_slice(&15u32.to_le_bytes()); // val 3: displayable_objects (sequence)
            data.extend_from_slice(&250u32.to_le_bytes()); // val 4: dp_obj_ttl
            data.extend_from_slice(&0u32.to_le_bytes()); // val 5: unknown
            data.extend_from_slice(&8u32.to_le_bytes()); // val 6: total_objects
            data.extend_from_slice(&99u32.to_le_bytes()); // val 7: unknown
            data.extend_from_slice(&1u32.to_le_bytes()); // val 8: player_id = 1
            data.extend_from_slice(&0u32.to_le_bytes()); // val 9: unknown
            data.extend_from_slice(&0u32.to_le_bytes()); // val 10: unknown
        } else if slot == 2 {
            // Player 3
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&800u32.to_le_bytes()); // total_resources
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&20u32.to_le_bytes()); // displayable_objects
            data.extend_from_slice(&300u32.to_le_bytes()); // dp_obj_ttl
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&12u32.to_le_bytes()); // total_objects
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&3u32.to_le_bytes()); // player_id = 3
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes());
        } else {
            // Empty slots (all 0s)
            for _ in 0..11 {
                data.extend_from_slice(&0u32.to_le_bytes());
            }
        }
    }
    // Trailing current_time_ms = 45000
    data.extend_from_slice(&45000u32.to_le_bytes());

    let mut cursor = Cursor::new(data);
    let op: Operation = cursor.read_le_args((1u16,)).unwrap();
    if let Operation::Sync {
        time_increment,
        checksum: Some(cs),
        ..
    } = &op
    {
        assert_eq!(*time_increment, 100);
        assert_eq!(cs.current_time_ms, Some(45000));
        assert_eq!(cs.player_stats.len(), 2);

        assert_eq!(cs.player_stats[0].player_id, 1);
        assert_eq!(cs.player_stats[0].total_resources, 500);
        assert_eq!(cs.player_stats[0].displayable_objects, 15);
        assert_eq!(cs.player_stats[0].dp_obj_ttl, 250);
        assert_eq!(cs.player_stats[0].total_objects, 8);

        assert_eq!(cs.player_stats[1].player_id, 3);
        assert_eq!(cs.player_stats[1].total_resources, 800);
        assert_eq!(cs.player_stats[1].displayable_objects, 20);
        assert_eq!(cs.player_stats[1].dp_obj_ttl, 300);
        assert_eq!(cs.player_stats[1].total_objects, 12);

        // Test serde serialization
        let json = serde_json::to_string(cs).unwrap();
        assert!(json.contains("\"total_resources\":500"));
        assert!(json.contains("\"current_time_ms\":45000"));

        let deserialized: aoe2rec::SyncChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, *cs);
    } else {
        panic!("Expected Sync operation with checksum");
    }
}

#[test]
fn test_parse_synthetic_non_de_sync_checksum() {
    let mut data = Vec::new();
    // Magic 2 (Sync)
    data.extend_from_slice(&2u32.to_le_bytes());
    // time_increment = 50
    data.extend_from_slice(&50u32.to_le_bytes());
    // Checksum marker (next == 0)
    data.extend_from_slice(&0u32.to_le_bytes());

    // Non-DE block:
    // val0 (4 bytes), sync (4 bytes), val2 (4 bytes), sequence == 0 (4 bytes)
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1234u32.to_le_bytes()); // sync
    data.extend_from_slice(&[1, 2, 3, 4]); // val2
    data.extend_from_slice(&0u32.to_le_bytes()); // sequence = 0
    data.extend_from_slice(&[9; 8]); // unknown4: 8 bytes

    let mut cursor = Cursor::new(data);
    let op: Operation = cursor.read_le_args((1u16,)).unwrap();
    if let Operation::Sync {
        time_increment,
        checksum: Some(cs),
        ..
    } = &op
    {
        assert_eq!(*time_increment, 50);
        assert_eq!(cs.sync, 1234);
        assert_eq!(cs.sequence, 0);
        assert!(cs.player_stats.is_empty());
        assert_eq!(cs.current_time_ms, None);
        assert_eq!(cs.unknown4, [9; 8]);
    } else {
        panic!("Expected Sync operation with non-DE checksum");
    }
}

#[test]
fn test_parse_truncated_de_sync_graceful() {
    let mut data = Vec::new();
    // Magic 2 (Sync)
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&50u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    // DE indicator (sequence > 0), but then stream ends early without the 88 u32s
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1234u32.to_le_bytes()); // sync
    data.extend_from_slice(&[1, 2, 3, 4]);
    data.extend_from_slice(&5u32.to_le_bytes()); // sequence = 5 > 0
    data.extend_from_slice(&[0; 8]); // only 8 trailing bytes instead of 340 bytes

    let mut cursor = Cursor::new(data);
    let op: Operation = cursor.read_le_args((1u16,)).unwrap();
    if let Operation::Sync {
        checksum: Some(cs), ..
    } = &op
    {
        assert_eq!(cs.sequence, 5);
        assert!(cs.player_stats.is_empty());
        assert_eq!(cs.current_time_ms, None);
    } else {
        panic!("Expected Sync operation");
    }
}

#[test]
fn test_pregame_data_is_valid_zero_chapters() {
    use aoe2rec::PregameData;
    let pregame = PregameData {
        checksum_interval: 500,
        multiplayer: false.into(),
        rec_owner: 1,
        reveal_map: true.into(),
        use_sequence_numbers: false.into(),
        number_of_chapters: 0,
        aok_or_de: false.into(),
    };
    assert!(pregame.is_valid());
}

#[test]
fn test_parse_long_game_replay() {
    let path = Path::new("replay-long.aoe2record");
    let save = Savegame::from_file(path).unwrap();
    assert_eq!(save.chapters.len(), 1);
    assert_eq!(save.get_duration(), 4906365);
    assert_eq!(save.operations().count(), 916740);
    let mut actions = save.operations().filter_map(|op| {
        if let Operation::Action { world_time, .. } = op {
            Some(*world_time)
        } else {
            None
        }
    });
    let first = actions.next().expect("should have actions");
    assert_eq!(first, 16);
}



