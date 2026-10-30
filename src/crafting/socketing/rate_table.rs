//! Success rate table for feather socketing (+1..+10).

/// Returns the success rate percentage (0..=100) for advancing to `target_level`.
/// Target levels:
/// +1..+3: 100%
/// +4: 80%
/// +5: 60%
/// +6: 40%
/// +7: 25%
/// +8: 15%
/// +9: 10%
/// +10: 5%
pub fn socket_success_rate(target_level: u8) -> u32 {
    match target_level {
        1..=3 => 100,
        4 => 80,
        5 => 60,
        6 => 40,
        7 => 25,
        8 => 15,
        9 => 10,
        10 => 5,
        _ => 0,
    }
}
