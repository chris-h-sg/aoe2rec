//! An alternative output format in which `Build`, `Queue` and `Delete` actions carry decoded
//! fields instead of raw bytes.
//!
//! Like [`crate::summary`], this is a view built from an already parsed [`Savegame`]: parsing,
//! [`ActionData`] and the existing serialized output are unaffected. Every other action and
//! operation is passed through as is, and serializes exactly as it does in the existing format.
//!
//! A payload that doesn't match a known layout also stays in its raw form, as
//! [`TypedAction::Other`], so no action is lost.

use serde::Serialize;

use crate::{actions::ActionData, header::RecHeader, Chapter, Operation, Savegame};

/// First game build that uses the 28-byte `Build` header. Earlier builds use a shorter header.
const BUILD_28_BYTE_HEADER: u32 = 71094;
const BUILD_HEADER_LEN: usize = 28;
const QUEUE_SHORT_LEN: usize = 8;
const QUEUE_HEADER_LEN: usize = 14;

#[derive(Serialize)]
pub struct TypedSavegame<'a> {
    pub chapters: Vec<TypedChapter<'a>>,
}

impl<'a> TypedSavegame<'a> {
    pub fn operations(&self) -> impl Iterator<Item = &TypedOperation<'a>> {
        self.chapters.iter().flat_map(|c| c.operations.iter())
    }
}

#[derive(Serialize)]
pub struct TypedChapter<'a> {
    pub header_end: u32,
    pub next_chapter_address: u32,
    pub zheader: &'a RecHeader,
    pub operations: Vec<TypedOperation<'a>>,
}

#[derive(Serialize, Debug)]
pub enum TypedOperation<'a> {
    Action {
        length: u32,
        action_data: TypedAction<'a>,
        world_time: u32,
    },
    /// Any other operation, including actions that could not be parsed at all.
    #[serde(untagged)]
    Other(&'a Operation),
}

#[derive(Serialize, Debug)]
pub enum TypedAction<'a> {
    Build {
        player_id: u8,
        action_length: u16,
        /// Number of villagers selected to perform the build.
        selected: i16,
        /// Map X coordinate of the building placement.
        x: f32,
        /// Map Y coordinate of the building placement.
        y: f32,
        building_type_id: u32,
        unknown_p1: u32,
        unknown_p2: u32,
        unknown2: i16,
        unknown3: i8,
        unknown4: i8,
        /// Instance IDs of the villagers assigned to the build.
        villager_ids: Vec<u32>,
    },
    Queue {
        player_id: u8,
        action_length: u16,
        building_id: u32,
        unit_type: u16,
        count: u16,
        /// `None` in the 8-byte form used by older recordings.
        unknown1: Option<[u8; 4]>,
        selected: u16,
        /// In the 8-byte form, this holds `building_id` as the only entry.
        building_ids: Vec<u32>,
    },
    Delete {
        player_id: u8,
        action_length: u16,
        object_id: u32,
    },
    /// Any other action, or a `Build`, `Queue` or `Delete` whose layout isn't recognized.
    #[serde(untagged)]
    Other(&'a ActionData),
}

impl<'a> TypedAction<'a> {
    /// Decodes `action`, using `build` (the game build from the recording's header) to select
    /// the payload layout.
    pub fn from_action(action: &'a ActionData, build: u32) -> Self {
        let decoded = match action {
            ActionData::Build {
                player_id,
                action_length,
                data,
            } => decode_build(*player_id, *action_length, data, build),
            ActionData::Queue {
                player_id,
                action_length,
                data,
            } => decode_queue(*player_id, *action_length, data),
            ActionData::Delete {
                player_id,
                action_length,
                data,
            } => decode_delete(*player_id, *action_length, data),
            _ => None,
        };
        decoded.unwrap_or(TypedAction::Other(action))
    }
}

fn typed_operation(operation: &Operation, build: u32) -> TypedOperation<'_> {
    match operation {
        Operation::Action {
            length,
            action_data: Some(action),
            world_time,
        } => TypedOperation::Action {
            length: *length,
            action_data: TypedAction::from_action(action, build),
            world_time: *world_time,
        },
        other => TypedOperation::Other(other),
    }
}

fn typed_chapter(chapter: &Chapter) -> TypedChapter<'_> {
    let build = chapter.zheader.build;
    TypedChapter {
        header_end: chapter.header_end,
        next_chapter_address: chapter.next_chapter_address,
        zheader: &chapter.zheader,
        operations: chapter
            .operations
            .iter()
            .map(|operation| typed_operation(operation, build))
            .collect(),
    }
}

impl Savegame {
    /// Returns this recording in the typed format described in [`crate::typed`].
    pub fn get_typed(&self) -> TypedSavegame<'_> {
        TypedSavegame {
            chapters: self.chapters.iter().map(typed_chapter).collect(),
        }
    }
}

fn bytes_at<const N: usize>(data: &[u8], offset: usize) -> Option<[u8; N]> {
    data.get(offset..offset + N)?.try_into().ok()
}

fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    bytes_at(data, offset).map(u16::from_le_bytes)
}

fn i16_at(data: &[u8], offset: usize) -> Option<i16> {
    bytes_at(data, offset).map(i16::from_le_bytes)
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    bytes_at(data, offset).map(u32::from_le_bytes)
}

fn f32_at(data: &[u8], offset: usize) -> Option<f32> {
    bytes_at(data, offset).map(f32::from_le_bytes)
}

fn u32_list(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn decode_build(
    player_id: u8,
    action_length: u16,
    data: &[u8],
    build: u32,
) -> Option<TypedAction<'static>> {
    if build < BUILD_28_BYTE_HEADER {
        return None;
    }
    let selected = i16_at(data, 0)?;
    let villagers = usize::try_from(selected).unwrap_or(0);
    if data.len() != BUILD_HEADER_LEN + villagers * 4 {
        return None;
    }
    Some(TypedAction::Build {
        player_id,
        action_length,
        selected,
        x: f32_at(data, 4)?,
        y: f32_at(data, 8)?,
        building_type_id: u32_at(data, 12)?,
        unknown_p1: u32_at(data, 16)?,
        unknown_p2: u32_at(data, 20)?,
        unknown2: i16_at(data, 24)?,
        unknown3: *data.get(26)? as i8,
        unknown4: *data.get(27)? as i8,
        villager_ids: u32_list(&data[BUILD_HEADER_LEN..]),
    })
}

fn decode_queue(player_id: u8, action_length: u16, data: &[u8]) -> Option<TypedAction<'static>> {
    let building_id = u32_at(data, 0)?;
    let unit_type = u16_at(data, 4)?;
    let count = u16_at(data, 6)?;

    if data.len() == QUEUE_SHORT_LEN {
        return Some(TypedAction::Queue {
            player_id,
            action_length,
            building_id,
            unit_type,
            count,
            unknown1: None,
            selected: 1,
            building_ids: vec![building_id],
        });
    }

    let selected = u16_at(data, 12)?;
    if data.len() != QUEUE_HEADER_LEN + selected as usize * 4 {
        return None;
    }
    Some(TypedAction::Queue {
        player_id,
        action_length,
        building_id,
        unit_type,
        count,
        unknown1: Some(bytes_at(data, 8)?),
        selected,
        building_ids: u32_list(&data[QUEUE_HEADER_LEN..]),
    })
}

fn decode_delete(player_id: u8, action_length: u16, data: &[u8]) -> Option<TypedAction<'static>> {
    Some(TypedAction::Delete {
        player_id,
        action_length,
        object_id: u32_at(data, 0)?,
    })
}
