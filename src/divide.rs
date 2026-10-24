//! Integer division for the whole cartridge, agb included.
//!
//! The ARM7 has no divide instruction, and Thumb code has no long multiply
//! either, so every `/` and `%` in Thumb becomes a call into these. The
//! library versions run from ROM in Thumb and cost 600 to 800 cycles. These
//! are ARM code in IWRAM and walk only the bits the quotient has.

/// Quotient and remainder of `n / d`. A zero divisor gives all ones and `n`:
/// Rust checks for zero before it calls, so only foreign code can get here.
#[inline(always)]
fn divmod(n: u32, d: u32) -> (u32, u32) {
    if d == 0 {
        return (u32::MAX, n);
    }
    if d > n {
        return (0, n);
    }
    let mut bit = 1u32;
    let mut step = d;
    while step <= n >> 1 {
        step <<= 1;
        bit <<= 1;
    }
    let mut rem = n;
    let mut quot = 0;
    while bit != 0 {
        if rem >= step {
            rem -= step;
            quot |= bit;
        }
        step >>= 1;
        bit >>= 1;
    }
    (quot, rem)
}

/// Rounds toward zero. The remainder takes the sign of `n`, as `%` does.
#[inline(always)]
fn idivmod(n: i32, d: i32) -> (i32, i32) {
    let (quot, rem) = divmod(n.unsigned_abs(), d.unsigned_abs());
    let quot = if (n < 0) != (d < 0) {
        (quot as i32).wrapping_neg()
    } else {
        quot as i32
    };
    let rem = if n < 0 {
        (rem as i32).wrapping_neg()
    } else {
        rem as i32
    };
    (quot, rem)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
pub extern "C" fn __aeabi_uidiv(n: u32, d: u32) -> u32 {
    divmod(n, d).0
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
pub extern "C" fn __aeabi_idiv(n: i32, d: i32) -> i32 {
    idivmod(n, d).0
}

/// The quotient comes back in r0 and the remainder in r1, which is a `u64`
/// with the quotient in the low word.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
pub extern "C" fn __aeabi_uidivmod(n: u32, d: u32) -> u64 {
    let (quot, rem) = divmod(n, d);
    u64::from(quot) | (u64::from(rem) << 32)
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
pub extern "C" fn __aeabi_idivmod(n: i32, d: i32) -> u64 {
    let (quot, rem) = idivmod(n, d);
    u64::from(quot as u32) | (u64::from(rem as u32) << 32)
}
