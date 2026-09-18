//! RTT-aware speculative amplification for staged decode.
//!
//! Serial split decode pays one activation round-trip per output token.
//! On a 5 ms LAN hop that still hits a 30 tok/s budget. On phone Wi-Fi or
//! cellular (40–150 ms) it does not. Raising the verify window and keeping
//! two windows in flight turns one round-trip into several candidate tokens
//! so accepted-token TPOT can climb without adding physical stages.
//!
//! This module only *raises* floors. Fast LAN configs are left alone.

/// Same 30 tok/s decode budget the split planner uses (`1000/30 ≈ 33 ms`).
pub const DEFAULT_TARGET_DECODE_TPOT_MS: u32 = 33;

/// Assumed Wi-Fi / hotspot hop when a constrained peer has no measured RTT.
pub const DEFAULT_CONSTRAINED_LINK_HOP_LATENCY_MS: u32 = 40;

const MAX_VERIFY_WINDOW_TOKENS: usize = 8;
const MAX_PIPELINE_DEPTH: usize = 2;
const MAX_MTP_DRAFT_TOKENS: usize = 4;
const ACCEPT_RATE_NUMERATOR: u32 = 1;
const ACCEPT_RATE_DENOMINATOR: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLinkBudget {
    pub stage_count: u32,
    pub hop_latency_ms: u32,
    pub target_tpot_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeAmplification {
    pub estimated_network_ms_per_token: u32,
    pub serial_target_met: bool,
    /// Callers should `max()` these with an existing speculative plan.
    pub min_verify_window_tokens: usize,
    pub min_pipeline_depth: usize,
    pub min_mtp_draft_tokens: usize,
}

impl DecodeAmplification {
    #[must_use]
    pub fn identity() -> Self {
        Self {
            estimated_network_ms_per_token: 0,
            serial_target_met: true,
            min_verify_window_tokens: 1,
            min_pipeline_depth: 1,
            min_mtp_draft_tokens: 1,
        }
    }
}

#[must_use]
pub fn estimate_decode_network_ms_per_token(stage_count: u32, hop_latency_ms: u32) -> u32 {
    hop_latency_ms.saturating_mul(stage_count)
}

/// Size speculative floors so a high-RTT split can still approach `target_tpot_ms`.
///
/// Network cost matches the topology planner: `stage_count * hop_latency_ms`.
/// When that already fits the budget, amplification is a no-op (all mins = 1).
/// When it does not, the verify window is sized for a conservative 50% draft
/// acceptance rate and pipeline depth is raised to 2 so the return hop overlaps
/// the next window.
#[must_use]
pub fn plan_decode_amplification(budget: DecodeLinkBudget) -> DecodeAmplification {
    if budget.stage_count < 2 || budget.hop_latency_ms == 0 {
        return DecodeAmplification::identity();
    }

    let target_tpot_ms = budget.target_tpot_ms.max(1);
    let estimated_network_ms_per_token =
        estimate_decode_network_ms_per_token(budget.stage_count, budget.hop_latency_ms);
    let serial_target_met = estimated_network_ms_per_token <= target_tpot_ms;
    if serial_target_met {
        return DecodeAmplification {
            estimated_network_ms_per_token,
            serial_target_met: true,
            min_verify_window_tokens: 1,
            min_pipeline_depth: 1,
            min_mtp_draft_tokens: 1,
        };
    }

    let tokens_per_rtt = estimated_network_ms_per_token
        .div_ceil(target_tpot_ms)
        .max(2);
    let drafted = tokens_per_rtt
        .saturating_mul(ACCEPT_RATE_DENOMINATOR)
        .div_ceil(ACCEPT_RATE_NUMERATOR)
        .max(2);
    let min_verify_window_tokens = (drafted as usize).clamp(2, MAX_VERIFY_WINDOW_TOKENS);
    DecodeAmplification {
        estimated_network_ms_per_token,
        serial_target_met: false,
        min_verify_window_tokens,
        min_pipeline_depth: MAX_PIPELINE_DEPTH,
        min_mtp_draft_tokens: min_verify_window_tokens.clamp(1, MAX_MTP_DRAFT_TOKENS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(stage_count: u32, hop_latency_ms: u32) -> DecodeLinkBudget {
        DecodeLinkBudget {
            stage_count,
            hop_latency_ms,
            target_tpot_ms: DEFAULT_TARGET_DECODE_TPOT_MS,
        }
    }

    #[test]
    fn lan_two_stage_split_does_not_inflate_speculation() {
        let plan = plan_decode_amplification(budget(2, 5));
        assert!(plan.serial_target_met);
        assert_eq!(plan.estimated_network_ms_per_token, 10);
        assert_eq!(plan.min_verify_window_tokens, 1);
        assert_eq!(plan.min_pipeline_depth, 1);
        assert_eq!(plan.min_mtp_draft_tokens, 1);
    }

    #[test]
    fn phone_wifi_two_stage_split_amplifies_verify_window() {
        let plan = plan_decode_amplification(budget(2, DEFAULT_CONSTRAINED_LINK_HOP_LATENCY_MS));
        assert!(!plan.serial_target_met);
        assert_eq!(plan.estimated_network_ms_per_token, 80);
        assert_eq!(plan.min_verify_window_tokens, 6);
        assert_eq!(plan.min_pipeline_depth, 2);
        assert_eq!(plan.min_mtp_draft_tokens, 4);
    }

    #[test]
    fn cellular_two_stage_split_hits_window_cap_not_deeper_pipeline() {
        let plan = plan_decode_amplification(budget(2, 150));
        assert!(!plan.serial_target_met);
        assert_eq!(plan.estimated_network_ms_per_token, 300);
        assert_eq!(plan.min_verify_window_tokens, MAX_VERIFY_WINDOW_TOKENS);
        assert_eq!(plan.min_pipeline_depth, MAX_PIPELINE_DEPTH);
        assert_eq!(plan.min_mtp_draft_tokens, MAX_MTP_DRAFT_TOKENS);
    }

    #[test]
    fn single_stage_is_a_no_op() {
        assert_eq!(
            plan_decode_amplification(budget(1, 150)),
            DecodeAmplification::identity()
        );
    }
}
