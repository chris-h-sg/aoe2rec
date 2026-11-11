use aoe2rec::actions::{ActionData, OrderType};
use aoe2rec::typed::{TypedAction, TypedOperation};
use aoe2rec::{Operation, Savegame};
use std::path::Path;

const MODERN_BUILD: u32 = 185872;
const OLD_BUILD: u32 = 71093;

const REPLAYS: [&str; 9] = [
    "Fallingstar_vs_Eden_multichapter.aoe2record",
    "beargwyn_vs_kamlesh.aoe2record",
    "replay-ai-fast.aoe2record",
    "replay-ai.aoe2record",
    "replay-campaign.aoe2record",
    "replay-cheats.aoe2record",
    "replay-long.aoe2record",
    "replay-resume.aoe2record",
    "replay-shared.aoe2record",
];

fn raw_build(data: Vec<u8>) -> ActionData {
    ActionData::Build {
        player_id: 1,
        action_length: data.len() as u16,
        data,
    }
}

fn raw_queue(data: Vec<u8>) -> ActionData {
    ActionData::Queue {
        player_id: 1,
        action_length: data.len() as u16,
        data,
    }
}

fn raw_delete(data: Vec<u8>) -> ActionData {
    ActionData::Delete {
        player_id: 1,
        action_length: data.len() as u16,
        data,
    }
}

fn build_payload(selected: i16, villagers: &[u32]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(selected.to_le_bytes());
    data.extend([0, 0]);
    data.extend(12.5f32.to_le_bytes());
    data.extend(30.0f32.to_le_bytes());
    data.extend(70u32.to_le_bytes()); // building type
    data.extend(7u32.to_le_bytes());
    data.extend(8u32.to_le_bytes());
    data.extend((-1i16).to_le_bytes());
    data.extend([1, 2]);
    for id in villagers {
        data.extend(id.to_le_bytes());
    }
    data
}

#[test]
fn test_decodes_modern_build() {
    let action = raw_build(build_payload(2, &[100, 200]));
    match TypedAction::from_action(&action, MODERN_BUILD) {
        TypedAction::Build {
            player_id,
            selected,
            x,
            y,
            building_type_id,
            villager_ids,
            ..
        } => {
            assert_eq!(player_id, 1);
            assert_eq!(selected, 2);
            assert_eq!(x, 12.5);
            assert_eq!(y, 30.0);
            assert_eq!(building_type_id, 70);
            assert_eq!(villager_ids, vec![100, 200]);
        }
        other => panic!("Expected Build, got {other:?}"),
    }
}

#[test]
fn test_build_before_71094_stays_raw() {
    let action = raw_build(build_payload(2, &[100, 200]));
    assert!(matches!(
        TypedAction::from_action(&action, OLD_BUILD),
        TypedAction::Other(ActionData::Build { .. })
    ));
}

#[test]
fn test_build_with_wrong_length_stays_raw() {
    // Claims 3 villagers but carries 2.
    let action = raw_build(build_payload(3, &[100, 200]));
    assert!(matches!(
        TypedAction::from_action(&action, MODERN_BUILD),
        TypedAction::Other(ActionData::Build { .. })
    ));
}

#[test]
fn test_decodes_8_byte_queue() {
    let mut data = Vec::new();
    data.extend(4242u32.to_le_bytes());
    data.extend(83u16.to_le_bytes());
    data.extend(5u16.to_le_bytes());
    let action = raw_queue(data);

    match TypedAction::from_action(&action, OLD_BUILD) {
        TypedAction::Queue {
            building_id,
            unit_type,
            count,
            unknown1,
            selected,
            building_ids,
            ..
        } => {
            assert_eq!(building_id, 4242);
            assert_eq!(unit_type, 83);
            assert_eq!(count, 5);
            assert_eq!(unknown1, None);
            assert_eq!(selected, 1);
            assert_eq!(building_ids, vec![4242]);
        }
        other => panic!("Expected Queue, got {other:?}"),
    }
}

#[test]
fn test_decodes_queue_with_selected_buildings() {
    let mut data = Vec::new();
    data.extend(4242u32.to_le_bytes());
    data.extend(83u16.to_le_bytes());
    data.extend(5u16.to_le_bytes());
    data.extend([9, 9, 9, 9]);
    data.extend(2u16.to_le_bytes());
    data.extend(4242u32.to_le_bytes());
    data.extend(4243u32.to_le_bytes());
    let action = raw_queue(data);

    match TypedAction::from_action(&action, MODERN_BUILD) {
        TypedAction::Queue {
            unknown1,
            selected,
            building_ids,
            ..
        } => {
            assert_eq!(unknown1, Some([9, 9, 9, 9]));
            assert_eq!(selected, 2);
            assert_eq!(building_ids, vec![4242, 4243]);
        }
        other => panic!("Expected Queue, got {other:?}"),
    }
}

#[test]
fn test_truncated_queue_stays_raw() {
    let action = raw_queue(vec![1, 2, 3, 4, 5]);
    assert!(matches!(
        TypedAction::from_action(&action, MODERN_BUILD),
        TypedAction::Other(ActionData::Queue { .. })
    ));
}

#[test]
fn test_decodes_delete() {
    let action = raw_delete(77u32.to_le_bytes().to_vec());
    assert!(matches!(
        TypedAction::from_action(&action, MODERN_BUILD),
        TypedAction::Delete { object_id: 77, .. }
    ));
}

#[test]
fn test_truncated_delete_stays_raw() {
    let action = raw_delete(vec![1, 2]);
    assert!(matches!(
        TypedAction::from_action(&action, MODERN_BUILD),
        TypedAction::Other(ActionData::Delete { .. })
    ));
}

#[test]
fn test_order_type_can_be_compared() {
    assert!(OrderType::Unqueue == OrderType::Unqueue);
}

#[test]
fn test_replays_keep_every_operation_and_decode_build_and_delete() {
    for name in REPLAYS {
        let save = Savegame::from_file(Path::new(name)).unwrap();
        let typed = save.get_typed();
        assert_eq!(
            typed.operations().count(),
            save.operations().count(),
            "{name}"
        );

        for (raw, typed) in save.operations().zip(typed.operations()) {
            match (raw, typed) {
                (
                    Operation::Action {
                        action_data: Some(ActionData::Build { .. }),
                        ..
                    },
                    TypedOperation::Action { action_data, .. },
                ) => assert!(matches!(action_data, TypedAction::Build { .. }), "{name}"),
                (
                    Operation::Action {
                        action_data: Some(ActionData::Delete { .. }),
                        ..
                    },
                    TypedOperation::Action { action_data, .. },
                ) => assert!(matches!(action_data, TypedAction::Delete { .. }), "{name}"),
                (
                    Operation::Action {
                        action_data: Some(_),
                        ..
                    },
                    TypedOperation::Action {
                        action_data: TypedAction::Other(_),
                        ..
                    },
                ) => {}
                (
                    Operation::Action {
                        action_data: Some(_),
                        ..
                    },
                    other,
                ) => {
                    panic!("{name}: unexpected typed operation {other:?}")
                }
                (_, TypedOperation::Other(_)) => {}
                (_, other) => panic!("{name}: unexpected typed operation {other:?}"),
            }
        }
    }
}

#[test]
fn test_typed_json_differs_only_in_decoded_actions() {
    let save = Savegame::from_file(Path::new("replay-ai.aoe2record")).unwrap();
    let raw = serde_json::to_value(&save).unwrap();
    let typed = serde_json::to_value(save.get_typed()).unwrap();

    let raw_ops = raw["chapters"][0]["operations"].as_array().unwrap();
    let typed_ops = typed["chapters"][0]["operations"].as_array().unwrap();
    assert_eq!(raw_ops.len(), typed_ops.len());

    let mut decoded = 0;
    for (raw_op, typed_op) in raw_ops.iter().zip(typed_ops) {
        let action = &raw_op["Action"]["action_data"];
        if action.get("Build").is_some() {
            assert!(typed_op["Action"]["action_data"]["Build"]["data"].is_null());
            assert!(typed_op["Action"]["action_data"]["Build"]["building_type_id"].is_u64());
            decoded += 1;
        } else if action.get("Delete").is_some() {
            assert!(typed_op["Action"]["action_data"]["Delete"]["object_id"].is_u64());
            decoded += 1;
        } else {
            assert_eq!(raw_op, typed_op);
        }
    }
    assert!(decoded > 0);
}
