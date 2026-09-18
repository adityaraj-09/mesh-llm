//! Apply RTT-aware speculative floors to a split stage-0 OpenAI plan.

use super::super::local_package::SplitParticipant;
use super::super::split_planning::default_runtime_headroom_bytes;
use crate::inference::skippy::ResolvedEmbeddedOpenAiArgs;
use skippy_coordinator::decode_amplification::{
    DEFAULT_CONSTRAINED_LINK_HOP_LATENCY_MS, DEFAULT_TARGET_DECODE_TPOT_MS, DecodeLinkBudget,
    plan_decode_amplification,
};
use skippy_coordinator::topology::usable_vram_is_constrained;

pub(super) fn apply_link_aware_speculative_floors(
    openai: &mut ResolvedEmbeddedOpenAiArgs,
    stage_count: usize,
    participants: &[SplitParticipant],
) {
    let constrained_peer = participants.iter().any(|participant| {
        usable_vram_is_constrained(
            participant
                .vram_bytes
                .saturating_sub(default_runtime_headroom_bytes(participant.vram_bytes)),
        )
    });
    let measured_hop = participants
        .iter()
        .filter_map(|participant| participant.rtt_ms)
        .max();
    let hop_latency_ms = measured_hop
        .or_else(|| constrained_peer.then_some(DEFAULT_CONSTRAINED_LINK_HOP_LATENCY_MS));
    let Some(hop_latency_ms) = hop_latency_ms else {
        return;
    };

    let plan = plan_decode_amplification(DecodeLinkBudget {
        stage_count: u32::try_from(stage_count).unwrap_or(u32::MAX),
        hop_latency_ms,
        target_tpot_ms: DEFAULT_TARGET_DECODE_TPOT_MS,
    });
    if plan.serial_target_met {
        return;
    }

    let spec = &mut openai.speculative;
    spec.verify_window.max_tokens = spec
        .verify_window
        .max_tokens
        .max(plan.min_verify_window_tokens);
    spec.verify_window.min_tokens = spec
        .verify_window
        .min_tokens
        .min(spec.verify_window.max_tokens)
        .max(1);
    spec.verify_window.pipeline_depth = spec
        .verify_window
        .pipeline_depth
        .max(plan.min_pipeline_depth);
    if spec.native_mtp.enabled {
        spec.native_mtp.max_draft_tokens = spec
            .native_mtp
            .max_draft_tokens
            .max(plan.min_mtp_draft_tokens);
        openai.native_mtp_max_tokens = openai.native_mtp_max_tokens.max(plan.min_mtp_draft_tokens);
    }
    openai.speculative_window = openai.speculative_window.max(plan.min_verify_window_tokens);

    tracing::info!(
        stage_count,
        hop_latency_ms,
        estimated_network_ms_per_token = plan.estimated_network_ms_per_token,
        verify_window_max_tokens = spec.verify_window.max_tokens,
        pipeline_depth = spec.verify_window.pipeline_depth,
        native_mtp_max_tokens = openai.native_mtp_max_tokens,
        constrained_peer,
        "raised split speculative floors for high-RTT decode"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::local_package::SplitParticipant;
    use skippy_server::SpeculativeDecodeConfig;

    fn openai_with_mtp() -> ResolvedEmbeddedOpenAiArgs {
        let mut speculative = SpeculativeDecodeConfig::default();
        speculative.native_mtp.enabled = true;
        speculative.native_mtp.max_draft_tokens = 1;
        speculative.verify_window.max_tokens = 4;
        speculative.verify_window.pipeline_depth = 1;
        ResolvedEmbeddedOpenAiArgs {
            model_id: None,
            default_max_tokens: 16,
            request_defaults: Default::default(),
            generation_concurrency: 1,
            continuous_batching: false,
            prefill_chunk_size: 64,
            prefill_chunk_policy: "fixed".into(),
            prefill_chunk_schedule: None,
            prefill_adaptive_start: 64,
            prefill_adaptive_step: 64,
            prefill_adaptive_max: 512,
            prefill_adaptive_target_ms: 100.0,
            draft_model_path: None,
            speculative_window: 0,
            adaptive_speculative_window: false,
            draft_n_gpu_layers: None,
            speculative,
            native_mtp_enabled: true,
            native_mtp_draft_model_path: None,
            native_mtp_max_tokens: 1,
            native_mtp_min_tokens: 0,
            activation_width: 0,
            reply_credit_limit: None,
            downstream_connect_timeout_secs: 5,
        }
    }

    fn participant(vram_gib: u64, rtt_ms: Option<u32>) -> SplitParticipant {
        let mut bytes = [0u8; 32];
        bytes[0] = (vram_gib as u8).max(1);
        let mut participant = SplitParticipant::new(
            iroh::SecretKey::from_bytes(&bytes).public(),
            vram_gib * 1024 * 1024 * 1024,
            None,
        );
        participant.rtt_ms = rtt_ms;
        participant
    }

    #[test]
    fn lan_rtt_does_not_raise_floors() {
        let mut openai = openai_with_mtp();
        apply_link_aware_speculative_floors(
            &mut openai,
            2,
            &[participant(24, Some(5)), participant(24, Some(5))],
        );
        assert_eq!(openai.speculative.verify_window.max_tokens, 4);
        assert_eq!(openai.speculative.verify_window.pipeline_depth, 1);
        assert_eq!(openai.native_mtp_max_tokens, 1);
    }

    #[test]
    fn phone_wifi_rtt_raises_mtp_and_pipeline() {
        let mut openai = openai_with_mtp();
        apply_link_aware_speculative_floors(
            &mut openai,
            2,
            &[participant(6, Some(40)), participant(6, Some(40))],
        );
        assert_eq!(openai.speculative.verify_window.max_tokens, 6);
        assert_eq!(openai.speculative.verify_window.pipeline_depth, 2);
        assert_eq!(openai.speculative.native_mtp.max_draft_tokens, 4);
        assert_eq!(openai.native_mtp_max_tokens, 4);
    }

    #[test]
    fn constrained_peers_without_rtt_assume_wifi_hop() {
        let mut openai = openai_with_mtp();
        apply_link_aware_speculative_floors(
            &mut openai,
            2,
            &[participant(6, None), participant(6, None)],
        );
        assert!(openai.speculative.verify_window.max_tokens >= 6);
        assert_eq!(openai.speculative.verify_window.pipeline_depth, 2);
    }
}
