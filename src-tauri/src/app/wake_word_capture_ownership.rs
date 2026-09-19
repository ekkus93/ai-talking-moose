/// Deterministic ownership policy for the single physical microphone stream used by
/// Wake Word and command ASR. This type owns no `AudioCapture`; it records which consumer
/// is entitled to the one `AppState::audio_capture` stream so callers cannot model Wake
/// listening and command ASR as simultaneous capture owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneOwner {
    DisabledManual,
    WakeListening,
    CommandAsr,
    Unavailable,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneOwnershipError {
    InvalidTransition,
    Unavailable,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WakeMicrophoneOwnership {
    owner: MicrophoneOwner,
}

impl WakeMicrophoneOwnership {
    pub(crate) fn new(wake_enabled: bool) -> Self {
        Self {
            owner: if wake_enabled {
                MicrophoneOwner::WakeListening
            } else {
                MicrophoneOwner::DisabledManual
            },
        }
    }

    pub(crate) fn owner(&self) -> MicrophoneOwner {
        self.owner
    }

    /// Atomically transfer the one capture entitlement from Wake listening to command ASR.
    /// Repeated trigger delivery cannot create another command owner.
    pub(crate) fn transfer_to_command_asr(&mut self) -> Result<(), MicrophoneOwnershipError> {
        match self.owner {
            MicrophoneOwner::WakeListening => {
                self.owner = MicrophoneOwner::CommandAsr;
                Ok(())
            }
            MicrophoneOwner::CommandAsr => Ok(()),
            MicrophoneOwner::Unavailable => Err(MicrophoneOwnershipError::Unavailable),
            MicrophoneOwner::ShuttingDown => Err(MicrophoneOwnershipError::ShuttingDown),
            MicrophoneOwner::DisabledManual => Err(MicrophoneOwnershipError::InvalidTransition),
        }
    }

    /// Return capture entitlement after every terminal command outcome. The latest setting
    /// wins so disabling Wake Word during an interaction cannot accidentally resume it.
    pub(crate) fn return_after_command(&mut self, wake_enabled: bool) -> Result<(), MicrophoneOwnershipError> {
        match self.owner {
            MicrophoneOwner::CommandAsr => {
                self.owner = if wake_enabled {
                    MicrophoneOwner::WakeListening
                } else {
                    MicrophoneOwner::DisabledManual
                };
                Ok(())
            }
            MicrophoneOwner::Unavailable => Err(MicrophoneOwnershipError::Unavailable),
            MicrophoneOwner::ShuttingDown => Err(MicrophoneOwnershipError::ShuttingDown),
            MicrophoneOwner::WakeListening | MicrophoneOwner::DisabledManual => {
                Err(MicrophoneOwnershipError::InvalidTransition)
            }
        }
    }

    /// A device/permission/runtime capture failure revokes all active capture entitlement.
    pub(crate) fn mark_unavailable(&mut self) {
        if self.owner != MicrophoneOwner::ShuttingDown {
            self.owner = MicrophoneOwner::Unavailable;
        }
    }

    /// Reconnect deterministically according to the latest Wake setting. This never creates
    /// a command-ASR owner implicitly; a command must explicitly claim ownership again.
    pub(crate) fn reconnect(&mut self, wake_enabled: bool) -> Result<(), MicrophoneOwnershipError> {
        match self.owner {
            MicrophoneOwner::Unavailable => {
                self.owner = if wake_enabled {
                    MicrophoneOwner::WakeListening
                } else {
                    MicrophoneOwner::DisabledManual
                };
                Ok(())
            }
            MicrophoneOwner::ShuttingDown => Err(MicrophoneOwnershipError::ShuttingDown),
            _ => Err(MicrophoneOwnershipError::InvalidTransition),
        }
    }

    /// Cancellation uses the same terminal return policy as success/failure, preventing a
    /// cancelled command from leaving an orphaned command owner.
    pub(crate) fn cancel_command(&mut self, wake_enabled: bool) -> Result<(), MicrophoneOwnershipError> {
        self.return_after_command(wake_enabled)
    }

    pub(crate) fn disable_wake(&mut self) {
        if self.owner == MicrophoneOwner::WakeListening {
            self.owner = MicrophoneOwner::DisabledManual;
        }
    }

    pub(crate) fn begin_shutdown(&mut self) {
        self.owner = MicrophoneOwner::ShuttingDown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_wake_asr_wake_cycles_never_create_parallel_owner() {
        let mut ownership = WakeMicrophoneOwnership::new(true);
        for _ in 0..32 {
            assert_eq!(ownership.owner(), MicrophoneOwner::WakeListening);
            ownership.transfer_to_command_asr().unwrap();
            ownership.transfer_to_command_asr().unwrap();
            assert_eq!(ownership.owner(), MicrophoneOwner::CommandAsr);
            ownership.return_after_command(true).unwrap();
        }
        assert_eq!(ownership.owner(), MicrophoneOwner::WakeListening);
    }

    #[test]
    fn disabling_during_command_returns_to_manual_only() {
        let mut ownership = WakeMicrophoneOwnership::new(true);
        ownership.transfer_to_command_asr().unwrap();
        ownership.return_after_command(false).unwrap();
        assert_eq!(ownership.owner(), MicrophoneOwner::DisabledManual);
    }

    #[test]
    fn cancellation_returns_deterministic_ownership() {
        let mut ownership = WakeMicrophoneOwnership::new(true);
        ownership.transfer_to_command_asr().unwrap();
        ownership.cancel_command(true).unwrap();
        assert_eq!(ownership.owner(), MicrophoneOwner::WakeListening);
    }

    #[test]
    fn device_failure_revokes_owner_and_reconnect_obeys_latest_setting() {
        let mut ownership = WakeMicrophoneOwnership::new(true);
        ownership.mark_unavailable();
        assert_eq!(ownership.owner(), MicrophoneOwner::Unavailable);
        assert_eq!(
            ownership.transfer_to_command_asr(),
            Err(MicrophoneOwnershipError::Unavailable)
        );
        ownership.reconnect(false).unwrap();
        assert_eq!(ownership.owner(), MicrophoneOwner::DisabledManual);
    }

    #[test]
    fn shutdown_is_terminal() {
        let mut ownership = WakeMicrophoneOwnership::new(true);
        ownership.begin_shutdown();
        ownership.mark_unavailable();
        assert_eq!(ownership.owner(), MicrophoneOwner::ShuttingDown);
        assert_eq!(
            ownership.transfer_to_command_asr(),
            Err(MicrophoneOwnershipError::ShuttingDown)
        );
    }
}
