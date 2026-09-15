//! Memory-class helpers for split topology planning.
//!
//! Phone-class and other low-memory peers cannot hold the datacenter 64k
//! context floor. Lowering that floor for constrained devices is additive:
//! high-VRAM meshes still prefer the largest context that fits.

/// Usable VRAM at or below this is treated as a constrained (phone / edge)
/// device. 8 GiB matches current flagship phone unified-memory budgets after
/// the planner's per-node headroom has already been subtracted in tests that
/// set headroom to zero, and matches an 8–9 GiB phone after a 10% reserve.
pub const CONSTRAINED_DEVICE_USABLE_VRAM_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// Datacenter / workstation auto-plan floor. Agent workloads need this much
/// context when the hardware can hold it.
pub const MINIMUM_AUTO_CONTEXT_LENGTH: u32 = 65_536;

/// Lowest auto-plan context on constrained devices. 4k is enough for a phone
/// chat turn and keeps KV resident on 6–8 GiB unified memory.
pub const MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH: u32 = 4_096;

#[must_use]
pub fn usable_vram_is_constrained(usable_vram_bytes: u64) -> bool {
    usable_vram_bytes <= CONSTRAINED_DEVICE_USABLE_VRAM_BYTES
}

#[must_use]
pub fn any_usable_vram_is_constrained<I>(usable_vram_bytes: I) -> bool
where
    I: IntoIterator<Item = u64>,
{
    usable_vram_bytes
        .into_iter()
        .any(usable_vram_is_constrained)
}

#[must_use]
pub fn auto_context_floor(constrained: bool) -> u32 {
    if constrained {
        MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH
    } else {
        MINIMUM_AUTO_CONTEXT_LENGTH
    }
}

#[must_use]
pub fn minimum_valid_context_for(native_context: u32, constrained: bool) -> u32 {
    native_context.clamp(1, auto_context_floor(constrained))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_gib_counts_as_constrained() {
        assert!(usable_vram_is_constrained(
            CONSTRAINED_DEVICE_USABLE_VRAM_BYTES
        ));
        assert!(!usable_vram_is_constrained(
            CONSTRAINED_DEVICE_USABLE_VRAM_BYTES + 1
        ));
    }

    #[test]
    fn constrained_floor_is_4k_and_unconstrained_is_64k() {
        assert_eq!(
            minimum_valid_context_for(262_144, true),
            MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH
        );
        assert_eq!(
            minimum_valid_context_for(262_144, false),
            MINIMUM_AUTO_CONTEXT_LENGTH
        );
        assert_eq!(minimum_valid_context_for(2_048, true), 2_048);
    }
}
