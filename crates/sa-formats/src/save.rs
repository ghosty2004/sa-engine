//! Save files (`GTASAsf1.b` … `GTASAsf8.b`): block structure and checksum only. Typed block
//! contents belong to the SAVE milestone.
//!
//! Writer: `CGenericGameStorage::GenericSave` 0x5D13E0; reader `GenericLoad` 0x5D17B0; checksum
//! `SaveWorkBuffer` 0x5D0F80 and `CheckDataNotCorrupt` 0x5D1170 (1.0 US).
//!
//! * 28 blocks (`eBlocks`, SIMPLE_VARIABLES … USER3DMARKERS), each introduced by the 5 bytes
//!   `BLOCK` (no NUL; string at 0x8D2BD8, `push 5` at 0x5D1876). Blocks carry **no size**: each
//!   block's length is whatever its `Save()` writes, so splitting needs the typed parsers. This
//!   module splits at the next `BLOCK` marker, which is only correct while no block contains
//!   those 5 bytes (checked for block 0, whose size is fixed: `CSimpleVariablesSaveStructure`,
//!   0x138 bytes, `push 0x138` at 0x5D14B7).
//! * The data is padded to 202,748 bytes (`cmp eax, 0x317FC` at 0x5D15D1) by writing the 51,200
//!   byte work buffer (0xC800) **without clearing it**, so the padding holds stale bytes of
//!   earlier blocks (possibly including `BLOCK` markers); everything after the 28th marker is
//!   therefore kept as the last block's raw bytes.
//! * The last 4 bytes are the checksum: the `u32` wrapping sum of every preceding byte
//!   (`movzx` + `add` loop at 0x5D0FB0), appended by `SaveWorkBuffer(true)`. Total file size
//!   202,752 bytes.
//! * Block 0 starts with the version id `CKeyGen::GetKey("Apr 28 2005" "10:28:55")`
//!   (`GetCurrentVersionNumber` 0x5D0F50; `GetKey` 0x53CF00 is the case-sensitive CRC).
//!
//! No save file exists in the install; the parser is tested on synthetic files only.

use crate::error::{Error, Result};

/// Block marker.
pub const BLOCK_TAG: &[u8; 5] = b"BLOCK";
/// Number of blocks `GenericSave` writes.
pub const NUM_BLOCKS: usize = 28;
/// Size of the data before the checksum (`SIZE_OF_ONE_GAME_IN_BYTES`).
pub const DATA_SIZE: usize = 202_748;
/// Size of a complete save file.
pub const FILE_SIZE: usize = DATA_SIZE + 4;
/// Size of block 0 (`CSimpleVariablesSaveStructure`).
pub const SIMPLE_VARIABLES_SIZE: usize = 0x138;

/// Block names in `eBlocks` order.
pub const BLOCK_NAMES: [&str; NUM_BLOCKS] = [
    "SIMPLE_VARIABLES",
    "SCRIPTS",
    "POOLS",
    "GARAGES",
    "GAMELOGIC",
    "PATHS",
    "PICKUPS",
    "PHONEINFO",
    "RESTART",
    "RADAR",
    "ZONES",
    "GANGS",
    "CAR_GENERATORS",
    "PED_GENERATORS",
    "AUDIO_SCRIPT_OBJECT",
    "PLAYERINFO",
    "STATS",
    "SET_PIECES",
    "STREAMING",
    "PED_TYPES",
    "TAGS",
    "IPLS",
    "SHOPPING",
    "GANGWARS",
    "STUNTJUMPS",
    "ENTRY_EXITS",
    "RADIOTRACKS",
    "USER3DMARKERS",
];

/// The save version id of 1.0 US.
pub fn current_version_id() -> u32 {
    crate::gxt::key_hash_case_sensitive(b"Apr 28 200510:28:55")
}

/// The checksum algorithm: wrapping `u32` sum of the bytes.
pub fn checksum(data: &[u8]) -> u32 {
    data.iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)))
}

/// A save file split into blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    /// Block contents (after the `BLOCK` tag). The last one includes the padding.
    pub blocks: Vec<Vec<u8>>,
    /// Stored checksum (last 4 bytes).
    pub checksum: u32,
}

impl SaveFile {
    /// Splits a save file at the first 28 `BLOCK` markers.
    pub fn parse(data: &[u8]) -> Result<SaveFile> {
        if data.len() < 4 + BLOCK_TAG.len() {
            return Err(Error::invalid("save", format!("{} bytes", data.len())));
        }
        let (body, sum) = data.split_at(data.len() - 4);
        if !body.starts_with(BLOCK_TAG) {
            return Err(Error::invalid("save", "does not start with BLOCK"));
        }
        let mut starts = vec![0usize];
        let mut pos = BLOCK_TAG.len();
        while starts.len() < NUM_BLOCKS {
            match body[pos..].windows(5).position(|w| w == BLOCK_TAG) {
                Some(i) => {
                    starts.push(pos + i);
                    pos += i + BLOCK_TAG.len();
                }
                None => {
                    return Err(Error::invalid(
                        "save",
                        format!("only {} BLOCK markers (expected {NUM_BLOCKS})", starts.len()),
                    ));
                }
            }
        }
        let mut blocks = Vec::with_capacity(NUM_BLOCKS);
        for (i, &s) in starts.iter().enumerate() {
            let end = starts.get(i + 1).copied().unwrap_or(body.len());
            blocks.push(body[s + BLOCK_TAG.len()..end].to_vec());
        }
        Ok(SaveFile {
            blocks,
            checksum: u32::from_le_bytes([sum[0], sum[1], sum[2], sum[3]]),
        })
    }

    /// Serialises back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FILE_SIZE);
        for b in &self.blocks {
            out.extend_from_slice(BLOCK_TAG);
            out.extend_from_slice(b);
        }
        out.extend_from_slice(&self.checksum.to_le_bytes());
        out
    }

    /// Recomputes the checksum over the data and compares it with the stored one.
    pub fn checksum_ok(&self) -> bool {
        let b = self.to_bytes();
        checksum(&b[..b.len() - 4]) == self.checksum
    }

    /// Version id (first word of block 0).
    pub fn version_id(&self) -> Option<u32> {
        let b = self.blocks.first()?.get(..4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Whether the marker split is confirmed for block 0 (its fixed size matches).
    pub fn block0_size_ok(&self) -> bool {
        self.blocks
            .first()
            .is_some_and(|b| b.len() == SIMPLE_VARIABLES_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = Vec::new();
        for i in 0..NUM_BLOCKS {
            d.extend_from_slice(BLOCK_TAG);
            if i == 0 {
                let mut v = vec![0u8; SIMPLE_VARIABLES_SIZE];
                v[..4].copy_from_slice(&current_version_id().to_le_bytes());
                d.extend_from_slice(&v);
            } else {
                d.extend((0..(i * 7 % 50)).map(|x| x as u8 ^ 0x5A));
            }
        }
        // Stale work-buffer padding, including an old marker.
        let stale = d[..1000].to_vec();
        while d.len() < DATA_SIZE {
            let n = (DATA_SIZE - d.len()).min(stale.len());
            d.extend_from_slice(&stale[..n]);
        }
        let s = checksum(&d);
        d.extend_from_slice(&s.to_le_bytes());
        d
    }

    #[test]
    fn splits_blocks_and_checks_sum() {
        let d = synthetic();
        assert_eq!(d.len(), FILE_SIZE);
        let s = SaveFile::parse(&d).unwrap();
        assert_eq!(s.blocks.len(), NUM_BLOCKS);
        assert!(s.block0_size_ok());
        assert!(s.checksum_ok());
        assert_eq!(s.version_id(), Some(current_version_id()));
        assert_eq!(s.to_bytes(), d);
        let mut bad = d.clone();
        bad[100] ^= 1;
        assert!(!SaveFile::parse(&bad).unwrap().checksum_ok());
        assert!(SaveFile::parse(&d[..500]).is_err());
    }
}
