pub mod actions;
pub mod header;
pub mod minimal;
pub mod summary;

use binrw::helpers::until_eof;
use binrw::io::{BufReader, Cursor, SeekFrom, TakeSeekExt};
use binrw::{binrw, BinReaderExt, BinResult, BinWriterExt, NullString};
use chrono::{DateTime, Utc};
use header::{decompress, RecHeader};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use summary::GameTeam;

use crate::header::Player;

#[binrw]
#[derive(Serialize)]
#[br(stream = s)]
pub struct Savegame {
    #[br(parse_with=until_eof)]
    pub chapters: Vec<Chapter>,
}

impl Savegame {
    pub fn header(&self) -> Option<&RecHeader> {
        if self.chapters.is_empty() {
            None
        } else {
            Some(&self.chapters[0].zheader)
        }
    }

    pub fn operations(&self) -> impl Iterator<Item = &Operation> {
        self.chapters.iter().flat_map(|c| c.operations.iter())
    }
    pub fn played_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_secs(self.header().unwrap().timestamp.into()).unwrap()
    }

    pub fn is_restored(&self) -> bool {
        self.world_time() > 0
    }

    pub fn world_time(&self) -> u32 {
        self.header().unwrap().replay.world_time
    }

    pub fn players(&self) -> &Vec<Player> {
        self.header().unwrap().players()
    }
}

fn chapter_size(current_offset: u64, next_offset: u64) -> u64 {
    if next_offset == 0 {
        return u32::MAX as u64;
    }
    next_offset.saturating_sub(current_offset)
}

#[binrw]
#[derive(Serialize)]
#[br(stream = s)]
pub struct Chapter {
    pub header_end: u32,
    #[br(calc=<u64>::from(header_end) - s.stream_position().unwrap())]
    header_len: u64,
    pub next_chapter_address: u32,
    // #[br(if(next_chapter_address >0), parse_with=binrw::FilePtr32::parse, ski)]
    // pub next_chapter: Option<Box<Savegame>>,
    #[br(count = header_len - 4, map = decompress)]
    pub zheader: RecHeader,
    #[br(temp, try_calc = s.stream_position())]
    #[bw(ignore)]
    current_address: u64,
    #[br(map_stream = |r| r.take_seek(chapter_size(current_address, next_chapter_address as u64)))]
    #[br(parse_with = parse_operations, args(zheader.version_major))]
    pub operations: Vec<Operation>,
}

#[binrw]
#[derive(Serialize, Debug)]
#[br(import(major: u16))]
#[allow(clippy::large_enum_variant)]
pub enum Operation {
    #[br(magic = 1u32)]
    Action {
        length: u32,
        #[br(temp, count = length)]
        #[bw(ignore)]
        data: Vec<u8>,
        #[br(calc = {
            let mut reader = Cursor::new(&data);
            reader.read_le_args::<actions::ActionData>((length, major)).ok()
        })]
        action_data: Option<actions::ActionData>,
        world_time: u32,

    },
    #[br(magic = 2u32)]
    Sync {
        time_increment: u32,
        #[br(restore_position)]
        next: u32,
        #[br(if (next == 0))]
        checksum: Option<SyncChecksum>,
    },
    #[br(magic = 3u32)]
    Viewlock { x: f32, y: f32, player_id: u32 },
    #[br(magic = 4u32)]
    Chat { padding: [u8; 4], text: LenString },
    // In AoE2 recordings, chapter initialization begins with a 28-byte Pregame block
    // (prefixed with opcode 5). During gameplay, modern DE builds also emit 12-byte
    // simulation operations with opcode 5 (Unknown5).
    // Pregame is parsed explicitly at stream offset 0, while 0xFFFFFFFF prevents binrw's
    // generated Operation enum parser from mistakenly matching mid-game opcode 5 as Pregame.
    #[br(magic = 0xFFFFFFFFu32)]
    Pregame {
        checksum_interval: u32,
        multiplayer: Bool32,
        rec_owner: u32,
        reveal_map: Bool32,
        use_sequence_numbers: Bool32,
        number_of_chapters: u32,
        aok_or_de: Bool32,
    },
    #[br(magic = 5u32)]
    Unknown5 {
        unknown1: i32,
        unknown2: u32,
    },
    #[br(magic = 6u32)]
    PostGame {
        #[br(seek_before = SeekFrom::End(-12))]
        version: u32,
        #[br(seek_before = SeekFrom::Current(-8))]
        num_blocks: u32,
        #[br(count = num_blocks, seek_before = SeekFrom::Current(-8), restore_position)]
        blocks: Vec<PostGameBlock>,
        version_repeat: u32,
        #[br(magic = b"\xce\xa4\x59\xb1\x05\xdb\x7b\x43")]
        end_bit: (),
    },
    #[br(magic = 7u32)]
    Ai {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 8u32)]
    MapNote {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 9u32)]
    InitialState {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 10u32)]
    Op10 {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 11u32)]
    Op11 {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 12u32)]
    Op12 {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
    #[br(magic = 13u32)]
    AiScript {
        length: u32,
        #[br(count = length)]
        data: Vec<u8>,
    },
}

#[binrw]
#[derive(Serialize, Debug)]
pub enum PostGameBlock {
    #[br(magic = 1u32)]
    WorldTime {
        #[br(seek_before=SeekFrom::Current(-8))]
        length: u32,
        #[br(seek_before=SeekFrom::Current(-(length as i64) - 4))]
        world_time: u32,
    },
    #[br(magic = 2u32)]
    Leaderboards {
        #[br(seek_before = SeekFrom::Current(-8))]
        length: u32,
        #[br(seek_before = SeekFrom::Current(-(length as i64) - 4))]
        num_leaderboards: u32,
        #[br(count = num_leaderboards)]
        leaderboards: Vec<Leaderboard>,
        #[br(seek_before = SeekFrom::Current(-(length as i64) - 4))]
        realignment_field: (),
    },
}

#[binrw]
#[derive(Serialize, Debug)]
pub struct Leaderboard {
    pub id: u32,
    pub unknown1: u16,
    pub num_players: u32,
    #[br(count = num_players)]
    pub players: Vec<LeaderboardPlayer>,
}

#[binrw]
#[derive(Serialize, Debug)]
pub struct LeaderboardPlayer {
    pub player_number: i32,
    pub rank: i32,
    pub elo: i32,
}

#[binrw]
#[derive(Serialize, Debug, Clone)]
pub struct PregameData {
    pub checksum_interval: u32,
    pub multiplayer: Bool32,
    pub rec_owner: u32,
    pub reveal_map: Bool32,
    pub use_sequence_numbers: Bool32,
    pub number_of_chapters: u32,
    pub aok_or_de: Bool32,
}

impl PregameData {
    pub fn is_valid(&self) -> bool {
        self.checksum_interval >= 10
            && self.checksum_interval <= 10_000
            && self.number_of_chapters <= 1_000
    }
}

impl From<PregameData> for Operation {
    fn from(p: PregameData) -> Self {
        Operation::Pregame {
            checksum_interval: p.checksum_interval,
            multiplayer: p.multiplayer,
            rec_owner: p.rec_owner,
            reveal_map: p.reveal_map,
            use_sequence_numbers: p.use_sequence_numbers,
            number_of_chapters: p.number_of_chapters,
            aok_or_de: p.aok_or_de,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncPlayerStats {
    pub player_id: u32,
    pub total_resources: u32,
    pub total_objects: u32,
    pub displayable_objects: u32,
    pub dp_obj_ttl: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SyncChecksum {
    pub unknown1: [u8; 8],
    pub sync: u32,
    pub unknown2: [u8; 4],
    pub sequence: u32,
    pub player_stats: Vec<SyncPlayerStats>,
    pub current_time_ms: Option<u32>,
    pub unknown4: [u8; 8],
}

impl binrw::BinRead for SyncChecksum {
    type Args<'a> = ();

    fn read_options<R: binrw::io::Read + binrw::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        parse_sync_checksum(reader, endian)
    }
}

impl binrw::BinWrite for SyncChecksum {
    type Args<'a> = ();

    fn write_options<W: binrw::io::Write + binrw::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        let _ = (writer, endian);
        Ok(())
    }
}

pub fn parse_sync_checksum<R: binrw::io::Read + binrw::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
) -> binrw::BinResult<SyncChecksum> {
    let marker: u32 = reader.read_type(endian)?;
    let start_pos = reader.stream_position()?;

    let val0: u32 = reader.read_type(endian)?;
    let sync: u32 = reader.read_type(endian)?;
    let val2: [u8; 4] = reader.read_type(endian)?;
    let sequence: u32 = reader.read_type(endian)?;

    if sequence > 0 {
        // In DE recordings with sequence > 0, the checksum payload contains an extended
        // table for up to 8 players (88 u32s = 352 bytes) followed by current_time_ms (u32 = 4 bytes).
        // val0..val3 (16 bytes) were the first 4 u32s of player 1's slot.
        // Rewind to start_pos to read all 88 u32s cleanly.
        let mut rewind_ok = true;
        if reader.seek(SeekFrom::Start(start_pos)).is_err() {
            rewind_ok = false;
        }

        if rewind_ok {
            let mut raw_stats = [0u32; 88];
            let mut read_de_ok = true;
            for val in &mut raw_stats {
                match reader.read_type::<u32>(endian) {
                    Ok(v) => *val = v,
                    Err(_) => {
                        read_de_ok = false;
                        break;
                    }
                }
            }

            let current_time_ms: Option<u32> = if read_de_ok {
                match reader.read_type::<u32>(endian) {
                    Ok(t) => Some(t),
                    Err(_) => {
                        read_de_ok = false;
                        None
                    }
                }
            } else {
                None
            };

            if read_de_ok {
                let mut player_stats = Vec::new();
                for slot in 0..8 {
                    let offset = slot * 11;
                    let slot_vals = &raw_stats[offset..offset + 11];
                    let player_id = slot_vals[8];
                    if player_id > 0 && player_id <= 8 {
                        player_stats.push(SyncPlayerStats {
                            player_id,
                            total_resources: slot_vals[1],
                            total_objects: slot_vals[6],
                            displayable_objects: slot_vals[3],
                            dp_obj_ttl: slot_vals[4],
                        });
                    }
                }

                let mut unknown1 = [0u8; 8];
                unknown1[0..4].copy_from_slice(&marker.to_le_bytes());
                unknown1[4..8].copy_from_slice(&raw_stats[0].to_le_bytes());
                let sync = raw_stats[1];
                let unknown2 = raw_stats[2].to_le_bytes();
                let sequence = raw_stats[3];
                let mut unknown4 = [0u8; 8];
                unknown4[0..4].copy_from_slice(&raw_stats[87].to_le_bytes());
                if let Some(t) = current_time_ms {
                    unknown4[4..8].copy_from_slice(&t.to_le_bytes());
                }

                return Ok(SyncChecksum {
                    unknown1,
                    sync,
                    unknown2,
                    sequence,
                    player_stats,
                    current_time_ms,
                    unknown4,
                });
            } else {
                // If reading DE extended block failed (e.g. truncated or unexpected EOF),
                // rewind back to start_pos + 16 and read trailing 8 bytes gracefully.
                let _ = reader.seek(SeekFrom::Start(start_pos + 16));
                let unknown4: [u8; 8] = reader.read_type(endian).unwrap_or([0u8; 8]);
                let mut unknown1 = [0u8; 8];
                unknown1[0..4].copy_from_slice(&marker.to_le_bytes());
                unknown1[4..8].copy_from_slice(&val0.to_le_bytes());
                return Ok(SyncChecksum {
                    unknown1,
                    sync,
                    unknown2: val2,
                    sequence,
                    player_stats: Vec::new(),
                    current_time_ms: None,
                    unknown4,
                });
            }
        }
    }

    // Non-DE or sequence == 0: 8 trailing bytes
    let unknown4: [u8; 8] = reader.read_type(endian)?;
    let mut unknown1 = [0u8; 8];
    unknown1[0..4].copy_from_slice(&marker.to_le_bytes());
    unknown1[4..8].copy_from_slice(&val0.to_le_bytes());
    Ok(SyncChecksum {
        unknown1,
        sync,
        unknown2: val2,
        sequence,
        player_stats: Vec::new(),
        current_time_ms: None,
        unknown4,
    })
}

impl Operation {
    pub fn player_stats(&self) -> Option<&[SyncPlayerStats]> {
        match self {
            Operation::Sync {
                checksum: Some(cs), ..
            } => Some(&cs.player_stats),
            _ => None,
        }
    }

    pub fn current_time_ms(&self) -> Option<u32> {
        match self {
            Operation::Sync {
                checksum: Some(cs), ..
            } => cs.current_time_ms,
            _ => None,
        }
    }
}

#[binrw]
#[derive(Serialize)]
#[br(repr = u32)]
#[bw(repr = u32)]
pub enum OperationType {
    Action = 1,
    Sync,
    Viewlock,
    Chat,
}

#[binrw]
#[derive(Serialize)]
pub struct SyncOperation {}

#[binrw]
#[derive(Copy, Clone)]
pub struct Bool {
    #[br(map = |x: u8| x == 1)]
    #[bw(map = |ranked: &bool| match ranked { true => 1u8, false => 0u8})]
    value: bool,
}

#[binrw]
#[derive(Copy, Clone)]
pub struct Bool32 {
    #[br(map = |x: u32| x == 1)]
    #[bw(map = |x: &bool| if *x { 1u32 } else {  0u32})]
    value: bool,
}

impl Serialize for Bool32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.value)
    }
}

impl From<Bool32> for bool {
    fn from(val: Bool32) -> Self {
        val.value
    }
}

impl From<bool> for Bool32 {
    fn from(value: bool) -> Self {
        Bool32 { value }
    }
}

impl std::fmt::Debug for Bool32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Debug for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Serialize for Bool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.value)
    }
}

impl From<bool> for Bool {
    fn from(value: bool) -> Self {
        Bool { value }
    }
}

impl From<Bool> for bool {
    fn from(val: Bool) -> Self {
        val.value
    }
}

#[binrw]
#[derive(Debug)]
pub struct LenString {
    length: u32,
    #[br(count = length)]
    value: Vec<u8>,
}

// TODO: Implement this with a generic?
#[binrw]
pub struct LenString16 {
    length: u16,
    #[br(count = length)]
    value: Vec<u8>,
}

#[binrw]
#[derive(Clone, Default)]
pub struct DeString {
    #[brw(magic = b"\x60\x0A")]
    #[bw(calc(value.len().try_into().unwrap()))]
    length: u16,
    #[br(count = length)]
    value: Vec<u8>,
}

impl From<&DeString> for String {
    fn from(value: &DeString) -> Self {
        std::string::String::from_utf8_lossy(&value.value).to_string()
    }
}

impl From<&String> for DeString {
    fn from(value: &String) -> Self {
        Self {
            value: value.as_bytes().to_vec(),
        }
    }
}

impl From<DeString> for String {
    fn from(val: DeString) -> Self {
        std::string::String::from_utf8_lossy(&val.value).to_string()
    }
}

#[binrw]
#[derive(Debug, Clone)]
pub struct MyNullString {
    text: NullString,
}

impl From<String> for MyNullString {
    fn from(value: String) -> Self {
        MyNullString { text: value.into() }
    }
}

impl From<MyNullString> for String {
    fn from(val: MyNullString) -> Self {
        val.text.to_string()
    }
}

impl std::fmt::Debug for DeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::string::String::from_utf8_lossy(&self.value))
    }
}

impl Serialize for DeString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let strvalue = std::string::String::from_utf8_lossy(&self.value);
        serializer.serialize_str(&strvalue)
    }
}

impl Serialize for LenString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let strvalue = std::string::String::from_utf8_lossy(&self.value);
        serializer.serialize_str(&strvalue)
    }
}

impl Serialize for LenString16 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let strvalue = std::string::String::from_utf8_lossy(&self.value);
        serializer.serialize_str(&strvalue)
    }
}

impl std::fmt::Debug for LenString16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::string::String::from_utf8_lossy(&self.value))
    }
}

impl serde::Serialize for MyNullString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let strvalue = std::string::String::from_utf8_lossy(&self.text);
        serializer.serialize_str(&strvalue)
    }
}

impl Savegame {
    pub fn from_bytes(data: bytes::Bytes) -> Result<Savegame, Box<dyn Error>> {
        let mut breader = BufReader::new(Cursor::new(data));
        let savegame: Savegame = breader.read_le()?;
        Ok(savegame)
    }
    pub fn from_file(path: &std::path::Path) -> Result<Savegame, Box<dyn Error>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let savegame: Savegame = reader.read_le()?;
        Ok(savegame)
    }

    pub fn get_duration(&self) -> u32 {
        let initial_time = self
            .chapters
            .first()
            .map(|c| c.zheader.replay.world_time)
            .unwrap_or(0);

        self.operations().fold(initial_time, |duration, operation| match operation {
            Operation::Sync { time_increment, .. } => duration + time_increment,
            _ => duration,
        })
    }

    pub fn get_resignations(&self) -> Vec<u8> {
        self.chapters
            .iter()
            .flat_map(|chapter| {
                chapter.operations.iter().map(|operation| match operation {
                    Operation::Action {
                        action_data: Some(actions::ActionData::Resign { player_id, .. }),
                        ..
                    } => *player_id,
                    _ => 100,
                })
            })
            .filter(|player_id| *player_id < 100)
            .collect()
    }

    pub fn try_get_summary(&self) -> Option<summary::SavegameSummary<'_>> {
        let first_chapter = self.chapters.first()?;
        Some(summary::SavegameSummary {
            header: summary::SummaryHeader {
                game: &first_chapter.zheader.game,
                version_minor: first_chapter.zheader.version_minor,
                version_major: first_chapter.zheader.version_major,
                build: first_chapter.zheader.build,
                timestamp: first_chapter.zheader.timestamp,
                game_settings: &first_chapter.zheader.game_settings,
                replay: &first_chapter.zheader.replay,
            },
            duration: self.get_duration(),
            resignations: self.get_resignations(),
            teams: GameTeam::from_savegame(self),
        })
    }

    pub fn get_summary(&self) -> summary::SavegameSummary<'_> {
        self.try_get_summary()
            .expect("cannot get summary of Savegame with no chapters")
    }
}

#[binrw::parser(reader, endian)]
fn read_strings_of_length() -> BinResult<Vec<DeString>> {
    let mut strings: Vec<DeString> = Vec::new();
    loop {
        let crc: u32 = reader.read_type(endian)?;
        if crc > 0 && crc < 255 {
            break;
        }
        let string: DeString = reader.read_type(endian)?;
        strings.push(string);
    }
    Ok(strings)
}

pub fn parse_operations<R: binrw::io::Read + binrw::io::Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (u16,),
) -> binrw::BinResult<Vec<Operation>> {
    let mut operations = Vec::new();
    loop {
        let position_before_read = reader.stream_position()?;
        let magic_res: binrw::BinResult<u32> = reader.read_type(endian);
        if let Err(e) = magic_res {
            if reader.stream_position()? == position_before_read
                || matches!(e, binrw::Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof)
            {
                break;
            }
            return Err(e);
        }
        let magic = magic_res.unwrap();
        // The chapter-start Pregame block only appears at the start of the chapter operations stream
        if operations.is_empty() && magic == 5 {
            let pregame_res: binrw::BinResult<PregameData> = reader.read_type(endian);
            if let Ok(p) = pregame_res {
                if p.is_valid() {
                    operations.push(p.into());
                    continue;
                }
            }
            reader.seek(std::io::SeekFrom::Start(position_before_read + 1))?;
            continue;
        }
        if magic == 0 || magic > 100 {
            reader.seek(std::io::SeekFrom::Start(position_before_read + 1))?;
            continue;
        }
        reader.seek(std::io::SeekFrom::Start(position_before_read))?;
        let res: binrw::BinResult<Operation> = reader.read_type_args(endian, (args.0,));
        match res {
            Ok(op) => {
                operations.push(op);
                if magic == 6 {
                    break;
                }
            }
            Err(_) => {
                // Skip one byte from position_before_read and try to re-sync if parsing failed
                reader.seek(std::io::SeekFrom::Start(position_before_read + 1))?;
            }
        }
    }
    Ok(operations)
}

#[binrw::writer(writer, endian)]
fn write_len_and_string(strings: &Vec<DeString>) -> BinResult<()> {
    for string in strings {
        writer.write_type(&0u32, endian)?;
        writer.write_type(&string, endian)?;
    }
    writer.write_type(&1u32, endian)?;
    Ok(())
}
