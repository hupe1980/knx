//! Evaluate protection evidence supplied by the host's KNX security implementation.

/// Minimum acceptable protection for one communication layer.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProtectionRequirement {
    PlainAllowed,
    SecureRequired,
}

/// Result from a trusted verifier, associated with this particular delivery.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProtectionEvidence {
    /// The verifier positively identified an unprotected message.
    Plain,
    /// The verifier checked identity, integrity and protocol freshness.
    AuthenticatedFresh,
    /// Verification failed, was unavailable, or lacked reliable sequence state.
    Unverified,
}

/// Layer whose observed protection failed the policy check.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProtectionError {
    Ip,
    Group,
}

impl std::fmt::Display for ProtectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ip => "Verify the IP connection protection before accepting delivery.",
            Self::Group => "Verify group protection before accepting delivery.",
        })
    }
}
impl std::error::Error for ProtectionError {}

impl ProtectionRequirement {
    /// A failed verification rejects even when plain communication is permitted.
    pub const fn accepts(self, evidence: ProtectionEvidence) -> bool {
        matches!(evidence, ProtectionEvidence::AuthenticatedFresh)
            || matches!(
                (self, evidence),
                (Self::PlainAllowed, ProtectionEvidence::Plain)
            )
    }
}

/// Evaluate IP and group protection independently for one delivery. The caller must bind
/// the verifier's evidence to the correct session, group, sender and message. This function
/// does not handle credentials, verify authentication tags or maintain replay state.
///
/// ```
/// use knx::{DeliveryPolicy, ProtectionError, ProtectionEvidence::*, ProtectionRequirement::*};
/// let policy = DeliveryPolicy { ip: SecureRequired, group: SecureRequired };
/// assert_eq!(policy.check(AuthenticatedFresh, Plain), Err(ProtectionError::Group));
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DeliveryPolicy {
    pub ip: ProtectionRequirement,
    pub group: ProtectionRequirement,
}

impl DeliveryPolicy {
    pub fn check(
        self,
        ip: ProtectionEvidence,
        group: ProtectionEvidence,
    ) -> Result<(), ProtectionError> {
        if !self.ip.accepts(ip) {
            return Err(ProtectionError::Ip);
        }
        if !self.group.accepts(group) {
            return Err(ProtectionError::Group);
        }
        Ok(())
    }
}
