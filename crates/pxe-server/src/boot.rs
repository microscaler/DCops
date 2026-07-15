//! BootIntent resolution and iPXE script rendering (pure logic + tests).

use crds::{BootIntent, BootProfile, LifecycleState};

use crate::api::BootConfig;
use crate::error::PxeError;

/// Result of resolving a MAC to a boot action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootResolution {
    /// Serve the on-disk localboot script only.
    LocalBootOnly,
    /// Boot using the resolved profile.
    Profile(BootConfig),
}

/// Normalize a MAC address to lowercase colon-separated form (`aa:bb:cc:dd:ee:ff`).
pub fn normalize_mac(input: &str) -> Result<String, PxeError> {
    let hex: String = input
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_lowercase();

    if hex.len() != 12 {
        return Err(PxeError::Configuration(format!(
            "invalid MAC address (expected 12 hex digits): {input}"
        )));
    }

    let mut pairs = Vec::with_capacity(6);
    for chunk in hex.as_bytes().chunks(2) {
        let pair = std::str::from_utf8(chunk).map_err(|e| PxeError::Configuration(e.to_string()))?;
        pairs.push(pair.to_string());
    }
    Ok(pairs.join(":"))
}

/// Find a BootIntent matching `mac` (case/format insensitive).
pub fn find_intent_for_mac<'a>(
    mac: &str,
    intents: &'a [BootIntent],
) -> Result<Option<&'a BootIntent>, PxeError> {
    let normalized = normalize_mac(mac)?;
    for intent in intents {
        if normalize_mac(&intent.spec.mac_address)? == normalized {
            return Ok(Some(intent));
        }
    }
    Ok(None)
}

/// Resolve boot profile reference namespace.
pub fn profile_namespace(intent: &BootIntent) -> String {
    intent
        .spec
        .profile_ref
        .namespace
        .clone()
        .unwrap_or_else(|| intent.metadata.namespace.clone().unwrap_or_default())
}

/// Build a [`BootConfig`] from a BootProfile CR.
pub fn boot_config_from_profile(profile: &BootProfile) -> BootConfig {
    BootConfig {
        kernel: profile.spec.kernel.clone(),
        initrd: profile.spec.initrd.clone(),
        cmdline: if profile.spec.cmdline.is_empty() {
            None
        } else {
            Some(profile.spec.cmdline.clone())
        },
        message: profile.spec.message.clone(),
    }
}

/// Resolve boot action for a MAC given intent + profile lists.
pub fn resolve_boot(
    mac: &str,
    intents: &[BootIntent],
    profiles: &[BootProfile],
) -> Result<BootResolution, PxeError> {
    let intent = find_intent_for_mac(mac, intents)?.ok_or_else(|| {
        PxeError::NotFound(format!("no BootIntent for MAC {mac}"))
    })?;

    if intent.spec.lifecycle == LifecycleState::Locked {
        return Ok(BootResolution::LocalBootOnly);
    }

    let ns = profile_namespace(intent);
    let name = &intent.spec.profile_ref.name;
    let profile = profiles
        .iter()
        .find(|p| {
            p.metadata.name.as_deref() == Some(name.as_str())
                && p.metadata.namespace.as_deref() == Some(ns.as_str())
        })
        .ok_or_else(|| {
            PxeError::Configuration(format!(
                "BootProfile {ns}/{name} not found for BootIntent {}",
                intent.metadata.name.as_deref().unwrap_or("?")
            ))
        })?;

    Ok(BootResolution::Profile(boot_config_from_profile(profile)))
}

/// Render an iPXE script for [`BootConfig`].
pub fn render_ipxe_script(config: &BootConfig) -> String {
    let mut lines = vec![
        "#!ipxe".to_string(),
        "echo ---".to_string(),
    ];
    if let Some(msg) = &config.message {
        lines.push(format!("echo {msg}"));
    }
    lines.push("echo ---".to_string());

    let cmdline = config.cmdline.as_deref().unwrap_or("");
    if config.initrd.is_empty() {
        lines.push(format!(
            "kernel {} {} ---",
            config.kernel, cmdline
        ));
    } else {
        let initrd_list = config
            .initrd
            .iter()
            .map(|url| format!("initrd={url}"))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!(
            "kernel {} {} {} ---",
            config.kernel, initrd_list, cmdline
        ));
        for url in &config.initrd {
            lines.push(format!("initrd {url}"));
        }
    }
    lines.push("boot".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crds::{BootIntentSpec, BootProfileRef, BootProfileSpec};

    fn sample_intent(mac: &str, lifecycle: LifecycleState) -> BootIntent {
        BootIntent {
            metadata: kube::core::ObjectMeta {
                name: Some("test-intent".into()),
                namespace: Some("cylon-regenesis".into()),
                ..Default::default()
            },
            spec: BootIntentSpec {
                mac_address: mac.to_string(),
                profile_ref: BootProfileRef {
                    name: "test-profile".into(),
                    namespace: None,
                },
                lifecycle,
            },
            status: None,
        }
    }

    fn sample_profile() -> BootProfile {
        BootProfile {
            metadata: kube::core::ObjectMeta {
                name: Some("test-profile".into()),
                namespace: Some("cylon-regenesis".into()),
                ..Default::default()
            },
            spec: BootProfileSpec {
                kernel: "http://pxe/cylon-regenesis/profiles/ubuntu/vmlinuz".into(),
                initrd: vec!["http://pxe/cylon-regenesis/profiles/ubuntu/initrd.img".into()],
                cmdline: "ip=dhcp".into(),
                message: Some("test boot".into()),
                schematic_id: None,
            },
            status: None,
        }
    }

    #[test]
    fn normalize_mac_accepts_common_formats() {
        assert_eq!(
            normalize_mac("AA-BB-CC-DD-EE-FF").unwrap(),
            "aa:bb:cc:dd:ee:ff"
        );
        assert_eq!(
            normalize_mac("aabbccddeeff").unwrap(),
            "aa:bb:cc:dd:ee:ff"
        );
    }

    #[test]
    fn locked_intent_returns_local_boot() {
        let intents = vec![sample_intent("aa:bb:cc:dd:ee:ff", LifecycleState::Locked)];
        let profiles = vec![sample_profile()];
        let resolved = resolve_boot("aa-bb-cc-dd-ee-ff", &intents, &profiles).unwrap();
        assert_eq!(resolved, BootResolution::LocalBootOnly);
    }

    #[test]
    fn discovered_intent_returns_profile() {
        let intents = vec![sample_intent("aa:bb:cc:dd:ee:ff", LifecycleState::Discovered)];
        let profiles = vec![sample_profile()];
        let resolved = resolve_boot("aa:bb:cc:dd:ee:ff", &intents, &profiles).unwrap();
        match resolved {
            BootResolution::Profile(cfg) => {
                assert!(cfg.kernel.contains("vmlinuz"));
            }
            BootResolution::LocalBootOnly => panic!("expected profile boot"),
        }
    }

    #[test]
    fn render_ipxe_includes_kernel_and_initrd() {
        let cfg = boot_config_from_profile(&sample_profile());
        let script = render_ipxe_script(&cfg);
        assert!(script.contains("#!ipxe"));
        assert!(script.contains("vmlinuz"));
        assert!(script.contains("initrd.img"));
        assert!(script.contains("boot"));
    }
}
