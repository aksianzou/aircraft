use crate::world::{LEVEL_COUNT, Sensor, card_at, sensor_of, slot_of};

pub(crate) static SAVE_KIND: [u8; 12] = *b"SRAM_V113\0\0\0";

pub(crate) struct Progress {
    /// The furthest place in the campaign the arrows may select.
    pub(crate) unlocked: u8,
    /// Bit i is set once card i has been cleared. The bit is the card's
    /// number, not its place in the campaign.
    pub(crate) cleared: u64,
    /// The card on the briefing.
    pub(crate) level: u8,
    /// Three bits per card: time, battery, spare. Once set, a mark stays.
    marks: [u8; MARK_BYTES],
    /// Two bits per card: 0 day, 1 night, 2 heat. Day cards only.
    glass: [u8; GLASS_BYTES],
}

const MARK_BYTES: usize = (LEVEL_COUNT as usize * 3).div_ceil(8);
const GLASS_BYTES: usize = (LEVEL_COUNT as usize * 2).div_ceil(8);
const CLEAR_BYTES: usize = (LEVEL_COUNT as usize).div_ceil(8);
/// The second record: a tag, the place and the card, then the bits, then a check.
const RECORD: usize = 4 + 2 + CLEAR_BYTES + MARK_BYTES + GLASS_BYTES + 1;
const TAG: [u8; 4] = *b"ARC2";

pub(crate) fn load_progress() -> Progress {
    let raw = sram_load();
    if raw[..4] == TAG && raw[RECORD - 1] == fold(&raw[4..RECORD - 1], 0x5A) {
        let mut at = 6;
        let mut cleared = 0u64;
        for index in 0..CLEAR_BYTES {
            cleared |= u64::from(raw[at + index]) << (index * 8);
        }
        at += CLEAR_BYTES;
        let mut marks = [0u8; MARK_BYTES];
        marks.copy_from_slice(&raw[at..at + MARK_BYTES]);
        at += MARK_BYTES;
        let mut glass = [0u8; GLASS_BYTES];
        glass.copy_from_slice(&raw[at..at + GLASS_BYTES]);
        let unlocked = raw[4].min(LEVEL_COUNT - 1);
        return Progress {
            unlocked,
            cleared,
            level: within(raw[5], unlocked),
            marks,
            glass,
        };
    }
    load_first(&raw)
}

/// The card if the arrows can reach it, otherwise the furthest one they can.
fn within(level: u8, unlocked: u8) -> u8 {
    let level = level % LEVEL_COUNT;
    if slot_of(level) <= unlocked {
        level
    } else {
        card_at(unlocked)
    }
}

/// The record of the thirty-card cartridge. Its numbers are the cards', so
/// they carry over; the furthest card becomes its place in the new order,
/// which opens the stage cards before it.
fn load_first(raw: &[u8; SAVE_LEN]) -> Progress {
    let ok = raw[0] == 0x41
        && raw[1] == 0x52
        && raw[2] == 0x43
        && raw[3] == 0x31
        && raw[7] == raw[4] ^ raw[5] ^ raw[6] ^ 0x5A;
    if !ok {
        return Progress {
            unlocked: 0,
            cleared: 0,
            level: 0,
            marks: [0; MARK_BYTES],
            glass: [0; GLASS_BYTES],
        };
    }
    let mut cleared = raw[5] as u32;
    if raw[9] == raw[8] ^ 0xA5 {
        cleared |= (raw[8] as u32) << 8;
    }
    if raw[11] == raw[10] ^ 0x3C {
        cleared |= (raw[10] as u32) << 16;
    }
    if raw[13] == raw[12] ^ 0xC3 {
        cleared |= (raw[12] as u32) << 24;
    }
    let mut unlocked = raw[4].min(29);
    // Slots opened when those cards were added. The bit is the slot, not the task.
    // A finished crows card already earned the escort sortie added after it,
    // the escort the ash pits, and the ash pits the first snow card.
    const OPENED: [(u8, u8); 7] = [
        (7, 8),
        (8, 9),
        (11, 12),
        (21, 22),
        (23, 24),
        (24, 25),
        (25, 26),
    ];
    for (done, opens) in OPENED {
        if cleared & (1 << done) != 0 {
            unlocked = unlocked.clamp(opens, 29);
        }
    }
    let mut marks = [0u8; MARK_BYTES];
    if raw[24] == fold(&raw[14..24], 0x6B) {
        marks[..10].copy_from_slice(&raw[14..24]);
    }
    if raw[35] == fold(&raw[33..35], 0x3D) {
        marks[10..12].copy_from_slice(&raw[33..35]);
    }
    let mut glass = [0u8; GLASS_BYTES];
    if raw[32] == fold(&raw[25..32], 0xC5) {
        glass[..7].copy_from_slice(&raw[25..32]);
    }
    if raw[37] == raw[36] ^ 0xA7 {
        glass[7] = raw[36];
    }
    let level = raw[6].min(unlocked);
    let unlocked = slot_of(unlocked);
    Progress {
        unlocked,
        cleared: u64::from(cleared),
        level: within(level, unlocked),
        marks,
        glass,
    }
}

/// The picture for this card. A cleared day card can wear night or heat glass.
pub(crate) fn glass_of(progress: &Progress, level: u8) -> Sensor {
    if !glass_open(progress, level) {
        return sensor_of(level);
    }
    match progress.glass_n(level) {
        1 => Sensor::Night,
        2 => Sensor::Heat,
        _ => Sensor::Day,
    }
}

/// Up and down apply once a night card and a heat card have both been cleared.
pub(crate) fn glass_open(progress: &Progress, level: u8) -> bool {
    progress.cleared & (1u64 << level) != 0
        && matches!(sensor_of(level), Sensor::Day)
        && saw(progress, Sensor::Night)
        && saw(progress, Sensor::Heat)
}

fn saw(progress: &Progress, kind: Sensor) -> bool {
    for level in 0..LEVEL_COUNT {
        if progress.cleared & (1u64 << level) != 0 && sensor_kind(level, kind) {
            return true;
        }
    }
    false
}

fn sensor_kind(level: u8, kind: Sensor) -> bool {
    matches!(
        (sensor_of(level), kind),
        (Sensor::Night, Sensor::Night)
            | (Sensor::Heat, Sensor::Heat)
            | (Sensor::Day, Sensor::Day)
            | (Sensor::Ir, Sensor::Ir)
    )
}

impl Progress {
    pub(crate) fn glass_n(&self, level: u8) -> u8 {
        let index = level as usize * 2;
        (self.glass[index / 8] >> (index % 8)) & 3
    }

    pub(crate) fn set_glass_n(&mut self, level: u8, value: u8) {
        let index = level as usize * 2;
        let shift = (index % 8) as u8;
        let mask = 3u8 << shift;
        self.glass[index / 8] = (self.glass[index / 8] & !mask) | ((value & 3) << shift);
    }

    pub(crate) fn mark_row(&self, level: u8) -> u8 {
        let mut bits = 0u8;
        for which in 0..3 {
            let index = level as usize * 3 + which;
            if self.marks[index / 8] & (1 << (index % 8)) != 0 {
                bits |= 1 << which;
            }
        }
        bits
    }

    pub(crate) fn keep_marks(&mut self, level: u8, bits: u8) {
        for which in 0..3 {
            if bits & (1 << which) == 0 {
                continue;
            }
            let index = level as usize * 3 + which;
            self.marks[index / 8] |= 1 << (index % 8);
        }
    }
}

pub(crate) fn store_progress(progress: &Progress) {
    let unlocked = progress.unlocked.min(LEVEL_COUNT - 1);
    let mut bytes = [0u8; RECORD];
    bytes[..4].copy_from_slice(&TAG);
    bytes[4] = unlocked;
    bytes[5] = within(progress.level, unlocked);
    let mut at = 6;
    for index in 0..CLEAR_BYTES {
        bytes[at + index] = (progress.cleared >> (index * 8)) as u8;
    }
    at += CLEAR_BYTES;
    bytes[at..at + MARK_BYTES].copy_from_slice(&progress.marks);
    at += MARK_BYTES;
    bytes[at..at + GLASS_BYTES].copy_from_slice(&progress.glass);
    bytes[RECORD - 1] = fold(&bytes[4..RECORD - 1], 0x5A);
    for (index, byte) in bytes.into_iter().enumerate() {
        unsafe {
            (0x0E00_0000 as *mut u8).add(index).write_volatile(byte);
        }
    }
}

fn fold(bytes: &[u8], salt: u8) -> u8 {
    let mut out = salt;
    for byte in bytes {
        out ^= byte;
    }
    out
}

/// Bytes read back: enough for either record.
const SAVE_LEN: usize = if RECORD > 38 { RECORD } else { 38 };

fn sram_load() -> [u8; SAVE_LEN] {
    let mut raw = [0u8; SAVE_LEN];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = unsafe { (0x0E00_0000 as *const u8).add(index).read_volatile() };
    }
    raw
}
