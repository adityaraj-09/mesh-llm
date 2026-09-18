//! Load defaults for in-process serving on a phone.
//!
//! iOS and Android FFI builds link Skippy into `libmeshllm_ffi` instead of
//! dlopening a desktop native-runtime artifact. Weights still have to fit in
//! unified memory, so this path keeps mmap on, mlock off, the constrained
//! 4k context floor, and Q4 K/V.

use super::DevicePolicy;
use crate::inference::skippy::{KvCachePolicy, SkippyModelLoadOptions};
use skippy_coordinator::topology::{
    CONSTRAINED_DEVICE_USABLE_VRAM_BYTES, MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OnDeviceKind {
    Desktop,
    Android,
    Ios,
}

#[must_use]
pub(super) fn current_on_device_kind() -> OnDeviceKind {
    if cfg!(target_os = "android") {
        OnDeviceKind::Android
    } else if cfg!(target_os = "ios") {
        OnDeviceKind::Ios
    } else {
        OnDeviceKind::Desktop
    }
}

#[must_use]
pub(super) fn apply_on_device_defaults(
    options: SkippyModelLoadOptions,
    policy: &DevicePolicy,
) -> SkippyModelLoadOptions {
    apply_on_device_defaults_for(current_on_device_kind(), options, policy)
}

#[must_use]
pub(super) fn apply_on_device_defaults_for(
    kind: OnDeviceKind,
    mut options: SkippyModelLoadOptions,
    policy: &DevicePolicy,
) -> SkippyModelLoadOptions {
    if kind == OnDeviceKind::Desktop {
        return options;
    }

    options.mmap = Some(true);
    options.mlock = false;
    options.ctx_size = options
        .ctx_size
        .min(MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH);
    let model_bytes = std::fs::metadata(&options.model_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let kv = KvCachePolicy::for_model_and_device(
        model_bytes,
        Some(CONSTRAINED_DEVICE_USABLE_VRAM_BYTES),
    );
    options.cache_type_k = kv.cache_type_k().to_string();
    options.cache_type_v = kv.cache_type_v().to_string();

    if kind == OnDeviceKind::Android && matches!(policy, DevicePolicy::Auto | DevicePolicy::Cpu) {
        options.n_gpu_layers = 0;
    }
    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn options() -> SkippyModelLoadOptions {
        let mut options = SkippyModelLoadOptions::for_direct_gguf(
            "phone-model",
            PathBuf::from("/tmp/phone.gguf"),
        );
        options.ctx_size = 32_768;
        options.n_gpu_layers = -1;
        options.mmap = None;
        options.mlock = true;
        options.cache_type_k = "f16".into();
        options.cache_type_v = "f16".into();
        options
    }

    #[test]
    fn desktop_defaults_are_a_no_op() {
        let applied =
            apply_on_device_defaults_for(OnDeviceKind::Desktop, options(), &DevicePolicy::Auto);
        assert_eq!(applied.ctx_size, 32_768);
        assert_eq!(applied.n_gpu_layers, -1);
        assert_eq!(applied.mmap, None);
        assert!(applied.mlock);
        assert_eq!(applied.cache_type_k, "f16");
    }

    #[test]
    fn android_cpu_aar_uses_mmap_q4_and_no_gpu_layers() {
        let applied =
            apply_on_device_defaults_for(OnDeviceKind::Android, options(), &DevicePolicy::Auto);
        assert_eq!(applied.ctx_size, MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH);
        assert_eq!(applied.n_gpu_layers, 0);
        assert_eq!(applied.mmap, Some(true));
        assert!(!applied.mlock);
        assert_eq!(applied.cache_type_k, "q4_0");
        assert_eq!(applied.cache_type_v, "q4_0");
    }

    #[test]
    fn ios_keeps_metal_offload_with_constrained_kv() {
        let applied =
            apply_on_device_defaults_for(OnDeviceKind::Ios, options(), &DevicePolicy::Auto);
        assert_eq!(applied.ctx_size, MINIMUM_CONSTRAINED_AUTO_CONTEXT_LENGTH);
        assert_eq!(applied.n_gpu_layers, -1);
        assert_eq!(applied.mmap, Some(true));
        assert!(!applied.mlock);
        assert_eq!(applied.cache_type_k, "q4_0");
    }
}
