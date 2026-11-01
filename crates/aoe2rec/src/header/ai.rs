use binrw::{binrw, BinReaderExt, BinResult};
use serde::Serialize;

use crate::DeString;

#[binrw]
#[derive(Serialize, Debug)]
#[br(import(speed: f32, n_players: u32))]
pub struct AIInfo {
    #[br(parse_with = skip_ai, args(speed, n_players))]
    skip: (), // #[br(dbg)]
              // max_strings: u16,
              // #[br(dbg)]
              // strings_count: u16,
              // #[br(dbg)]
              // unknown1: u32,
              // #[br(count=strings_count)]
              // ai_strings: Vec<LenString>,
              // unknown2: [u8; 3],
              // unknown3: u16,
              // unknown4: u16,
              // unknown5: [u32; 2],
              // #[br(dbg)]
              // unknown_max: u16,
              // #[br(dbg)]
              // unknown_count: u16,
              // #[br(dbg)]
              // unknown6: u32,
              // #[br(count = unknown_count, dbg)]
              // unknown7: Vec<UnknownAI>,
              // unknown_max2: u16,
              // unknown_count2: u16,
              // unknown8: [u32; 3],
              // #[br(count = unknown_count2)]
              // unknown9: Vec<UnknownAI>,
              // unknown_max3: u16,
              // unknown_count3: u16,
              // unknown10: [u32; 3],
              // #[br(count = unknown_count3, dbg)]
              // unknown11: Vec<UnknownAI>,
              // #[br(magic = b"\x00", count = 4096)]
              // skip_magic3: Vec<()>,
}

// #[binrw]
// #[derive(Serialize, Debug)]
// pub struct UnknownAI {
//     #[br(dbg)]
//     seq: u16,
//     // #[br(magic = b"\xff\xff")]
//     #[br(dbg)]
//     skip_magic: u16,
//     #[br(dbg)]
//     unknown1: u8,
//     #[br(dbg)]
//     unknown_count: u8,
//     #[br(magic = b"\x00\x00", dbg)]
//     skip_magic2: u16,
//     #[br(count = unknown_count)]
//     unknown2: Vec<u32>,
//     #[br(dbg)]
//     unknown3: u32,
//     #[br(dbg)]
//     unknown4: u32,
//     #[br(dbg)]
//     unknown5: u32,
//     #[br(dbg)]
//     crc: u32,
// }

#[binrw]
#[derive(Serialize, Debug)]
pub struct AIFile {
    pub unknown: u32,
    pub name: DeString,
    pub unknown2: u32,
}

#[binrw::parser(reader, endian)]
fn skip_ai(speed: f32, n_players: u32) -> BinResult<()> {
    use binrw::io::SeekFrom;
    let expected_players = (n_players + 1) as u8;

    let mut window = [0u8; 60];
    for b in &mut window {
        *b = reader.read_type(endian)?;
    }

    loop {
        let candidate_speed = f32::from_le_bytes(window[24..28].try_into().unwrap());
        let candidate_players = window[47];
        let temp_pause = window[28];
        let instant_build = window[48];
        let cheats = window[49];

        if (candidate_speed - speed).abs() < 0.001
            && candidate_players >= 1
            && candidate_players <= expected_players
            && (temp_pause == 0 || temp_pause == 1)
            && (instant_build == 0 || instant_build == 1)
            && (cheats == 0 || cheats == 1)
        {
            reader.seek(SeekFrom::Current(-60))?;
            // The speed alone is a weak signature: Fast (2.0) is the bytes 00 00 00 40, which
            // also occur inside the AI data. Accept the candidate only if the replay block and
            // the map size that follow it parse to something plausible.
            let start = reader.stream_position()?;
            if replay_follows(reader, endian) {
                reader.seek(SeekFrom::Start(start))?;
                return Ok(());
            }
            reader.seek(SeekFrom::Start(start + 60))?;
        }

        let next_byte: u8 = reader.read_type(endian)?;
        window.copy_within(1..60, 0);
        window[59] = next_byte;
    }
}

/// Whether a `Replay` block followed by a plausible map size can be read from the current
/// position: square (AoE2 maps are), 1 to 1000 tiles a side, under 1000 zones. False matches
/// seen in the AI data read as 0x768, 12800x0 and 512x768.
fn replay_follows<R: binrw::io::Read + binrw::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
) -> bool {
    let mut check = || -> BinResult<bool> {
        let _: super::Replay = reader.read_type(endian)?;
        let size_x: u32 = reader.read_type(endian)?;
        let size_y: u32 = reader.read_type(endian)?;
        let zones: u32 = reader.read_type(endian)?;
        Ok(size_x == size_y && (1..=1000).contains(&size_x) && zones < 1000)
    };
    check().unwrap_or(false)
}
