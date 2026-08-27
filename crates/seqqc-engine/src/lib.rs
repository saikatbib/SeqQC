//! Trusted workflow policy for `SeqQC` Studio.

/// Operating systems supported by the desktop application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

/// High-level work that a task can perform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    QualityControl,
    Alignment,
    Reporting,
    Export,
}

/// Stable machine-readable policy result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub code: &'static str,
    pub message: &'static str,
}

const ALLOWED: PolicyDecision = PolicyDecision {
    allowed: true,
    code: "ALLOWED",
    message: "Task is permitted on this platform.",
};

const ALIGNMENT_REQUIRES_LINUX: PolicyDecision = PolicyDecision {
    allowed: false,
    code: "ALIGNMENT_REQUIRES_LINUX",
    message: "Alignment is available only on validated Linux providers in the initial release.",
};

/// Applies the release platform gate before work is scheduled.
#[must_use]
pub const fn check_task_platform(capability: Capability, platform: Platform) -> PolicyDecision {
    if matches!(capability, Capability::Alignment) && !matches!(platform, Platform::Linux) {
        ALIGNMENT_REQUIRES_LINUX
    } else {
        ALLOWED
    }
}

#[cfg(test)]
mod tests {
    use super::{Capability, Platform, check_task_platform};

    #[test]
    fn alignment_is_linux_only() {
        for platform in [Platform::Windows, Platform::MacOs] {
            let decision = check_task_platform(Capability::Alignment, platform);
            assert!(!decision.allowed);
            assert_eq!(decision.code, "ALIGNMENT_REQUIRES_LINUX");
        }
        assert!(check_task_platform(Capability::Alignment, Platform::Linux).allowed);
    }

    #[test]
    fn quality_control_and_reporting_remain_cross_platform() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            assert!(check_task_platform(Capability::QualityControl, platform).allowed);
            assert!(check_task_platform(Capability::Reporting, platform).allowed);
            assert!(check_task_platform(Capability::Export, platform).allowed);
        }
    }
}
