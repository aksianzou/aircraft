use crate::world::LEVEL_COUNT;

pub(crate) static SAVE_KIND: [u8; 12] = *b"SRAM_V113\0\0\0";

pub(crate) struct Progress {
    /// Highest mission index the arrows may select.
    pub(crate) unlocked: u8,
    /// Bit i is set once mission i has been cleared.
    pub(crate) cleared: u32,
    pub(crate) level: u8,
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
    // A finished FPV card already earned the spotter mission added after it.
    if cleared & (1 << 7) != 0 {
        unlocked = unlocked.max(8).min(LEVEL_COUNT - 1);
    }
    // A finished spotter card already earned the three sorties added after it.
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
    Progress {
        unlocked,
        cleared,
        level: raw[6].min(unlocked),
    }
}

pub(crate) fn store_progress(progress: &Progress) {
    let level = progress.level.min(progress.unlocked);
    let lo = (progress.cleared & 0xFF) as u8;
    let hi = ((progress.cleared >> 8) & 0xFF) as u8;
    let mid = ((progress.cleared >> 16) & 0xFF) as u8;
    let top = ((progress.cleared >> 24) & 0xFF) as u8;
    let bytes = [
        0x41,
        0x52,
        0x43,
        0x31,
        progress.unlocked,
        lo,
        level,
        progress.unlocked ^ lo ^ level ^ 0x5A,
        hi,
        hi ^ 0xA5,
        mid,
        mid ^ 0x3C,
        top,
        top ^ 0xC3,
    ];
    for (index, byte) in bytes.into_iter().enumerate() {
        unsafe {
            (0x0E00_0000 as *mut u8)
                .add(index)
                .write_volatile(byte);
        }
    }
}

fn sram_load() -> [u8; 14] {
    let mut raw = [0u8; 14];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = unsafe { (0x0E00_0000 as *const u8).add(index).read_volatile() };
    }
    raw
}
