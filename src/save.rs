use crate::world::{Sensor, LEVEL_COUNT, sensor_of};

pub(crate) static SAVE_KIND: [u8; 12] = *b"SRAM_V113\0\0\0";

pub(crate) struct Progress {
    /// Highest mission index the arrows may select.
    pub(crate) unlocked: u8,
    /// Bit i is set once mission i has been cleared.
    pub(crate) cleared: u32,
    pub(crate) level: u8,
    /// Three bits per card: time, battery, spare. Once set, a mark stays.
    /// The first ten bytes are the old record. Two more hold the snow cards.
    marks: [u8; 12],
    /// Two bits per card: 0 day, 1 night, 2 heat. Day cards only.
    glass: [u8; 8],
}

pub(crate) fn load_progress() -> Progress {
    let raw = sram_load();
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
            marks: [0; 12],
            glass: [0; 8],
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
    let mut unlocked = raw[4].min(LEVEL_COUNT - 1);
    // Slots opened when those cards were added. The bit is the slot, not the task.
    if cleared & (1 << 7) != 0 {
        unlocked = unlocked.max(8).min(LEVEL_COUNT - 1);
    }
    if cleared & (1 << 8) != 0 {
        unlocked = unlocked.max(9).min(LEVEL_COUNT - 1);
    }
    if cleared & (1 << 11) != 0 {
        unlocked = unlocked.max(12).min(LEVEL_COUNT - 1);
    }
    if cleared & (1 << 21) != 0 {
        unlocked = unlocked.max(22).min(LEVEL_COUNT - 1);
    }
    // A finished crows card already earned the escort sortie added after it.
    if cleared & (1 << 23) != 0 {
        unlocked = unlocked.max(24).min(LEVEL_COUNT - 1);
    }
    // A finished escort already earned the ash pits added after it.
    if cleared & (1 << 24) != 0 {
        unlocked = unlocked.max(25).min(LEVEL_COUNT - 1);
    }
    // A finished ash pits already earned the first snow card added after it.
    if cleared & (1 << 25) != 0 {
        unlocked = unlocked.max(26).min(LEVEL_COUNT - 1);
    }
    let mut marks = [0u8; 12];
    if raw[24] == fold(&raw[14..24], 0x6B) {
        marks[..10].copy_from_slice(&raw[14..24]);
    }
    if raw[35] == fold(&raw[33..35], 0x3D) {
        marks[10..12].copy_from_slice(&raw[33..35]);
    }
    let mut glass = [0u8; 8];
    if raw[32] == fold(&raw[25..32], 0xC5) {
        glass[..7].copy_from_slice(&raw[25..32]);
    }
    if raw[37] == raw[36] ^ 0xA7 {
        glass[7] = raw[36];
    }
    Progress {
        unlocked,
        cleared,
        level: raw[6].min(unlocked),
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
    progress.cleared & (1 << level) != 0
        && matches!(sensor_of(level), Sensor::Day)
        && saw(progress, Sensor::Night)
        && saw(progress, Sensor::Heat)
}

fn saw(progress: &Progress, kind: Sensor) -> bool {
    for level in 0..LEVEL_COUNT {
        if progress.cleared & (1 << level) != 0 && sensor_kind(level, kind) {
            return true;
        }
    }
    false
}

fn sensor_kind(level: u8, kind: Sensor) -> bool {
    matches!(
        (sensor_of(level), kind),
        (Sensor::Night, Sensor::Night) | (Sensor::Heat, Sensor::Heat) | (Sensor::Day, Sensor::Day)
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
    let level = progress.level.min(progress.unlocked);
    let lo = (progress.cleared & 0xFF) as u8;
    let hi = ((progress.cleared >> 8) & 0xFF) as u8;
    let mid = ((progress.cleared >> 16) & 0xFF) as u8;
    let top = ((progress.cleared >> 24) & 0xFF) as u8;
    let mut bytes = [0u8; 38];
    bytes[0] = 0x41;
    bytes[1] = 0x52;
    bytes[2] = 0x43;
    bytes[3] = 0x31;
    bytes[4] = progress.unlocked;
    bytes[5] = lo;
    bytes[6] = level;
    bytes[7] = progress.unlocked ^ lo ^ level ^ 0x5A;
    bytes[8] = hi;
    bytes[9] = hi ^ 0xA5;
    bytes[10] = mid;
    bytes[11] = mid ^ 0x3C;
    bytes[12] = top;
    bytes[13] = top ^ 0xC3;
    bytes[14..24].copy_from_slice(&progress.marks[..10]);
    bytes[24] = fold(&bytes[14..24], 0x6B);
    bytes[25..32].copy_from_slice(&progress.glass[..7]);
    bytes[32] = fold(&bytes[25..32], 0xC5);
    bytes[33..35].copy_from_slice(&progress.marks[10..12]);
    bytes[35] = fold(&bytes[33..35], 0x3D);
    bytes[36] = progress.glass[7];
    bytes[37] = progress.glass[7] ^ 0xA7;
    for (index, byte) in bytes.into_iter().enumerate() {
        unsafe {
            (0x0E00_0000 as *mut u8)
                .add(index)
                .write_volatile(byte);
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

fn sram_load() -> [u8; 38] {
    let mut raw = [0u8; 38];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = unsafe { (0x0E00_0000 as *const u8).add(index).read_volatile() };
    }
    raw
}
