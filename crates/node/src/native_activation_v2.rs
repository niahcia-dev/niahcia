use crate::native_execution::NativeStateV1;
use crate::native_state_v2::NativeStateV2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeExecutionVersion {
    V1,
    V2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeExecutionActivationV2 {
    pub activation_height: u64,
}

impl NativeExecutionActivationV2 {
    pub fn validate(self) -> Result<Self, String> {
        if self.activation_height == 0 {
            return Err(
                "Native Execution V2 migration activation height must be greater than zero".into(),
            );
        }
        Ok(self)
    }

    pub fn execution_version_at_height(
        self,
        height: u64,
    ) -> Result<NativeExecutionVersion, String> {
        self.validate()?;
        if height < self.activation_height {
            Ok(NativeExecutionVersion::V1)
        } else {
            Ok(NativeExecutionVersion::V2)
        }
    }

    pub fn is_activation_height(self, height: u64) -> Result<bool, String> {
        self.validate()?;
        Ok(height == self.activation_height)
    }

    pub fn validate_execution_version_at_height(
        self,
        height: u64,
        proposed: NativeExecutionVersion,
    ) -> Result<(), String> {
        let expected = self.execution_version_at_height(height)?;
        if proposed != expected {
            return Err(format!(
                "native execution version mismatch at height {height}: expected {expected:?}, found {proposed:?}"
            ));
        }
        Ok(())
    }

    pub fn migrate_parent_state(
        self,
        parent_height: u64,
        parent_state: NativeStateV1,
    ) -> Result<NativeStateV2, String> {
        self.validate()?;
        let expected_parent = self
            .activation_height
            .checked_sub(1)
            .ok_or_else(|| "Native Execution V2 activation parent height underflow".to_string())?;

        if parent_height != expected_parent {
            return Err(format!(
                "Native Execution V2 migration requires parent height {expected_parent}, found {parent_height}"
            ));
        }

        Ok(NativeStateV2::from_v1(parent_state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_execution::{AccountStateV1, NativeStateV1};

    #[test]
    fn activation_boundary_selects_v1_before_and_v2_at_height() {
        let activation = NativeExecutionActivationV2 {
            activation_height: 100,
        };

        assert_eq!(
            activation.execution_version_at_height(99).unwrap(),
            NativeExecutionVersion::V1
        );
        assert_eq!(
            activation.execution_version_at_height(100).unwrap(),
            NativeExecutionVersion::V2
        );
        assert_eq!(
            activation.execution_version_at_height(101).unwrap(),
            NativeExecutionVersion::V2
        );
        assert!(activation.is_activation_height(100).unwrap());
        assert!(!activation.is_activation_height(99).unwrap());
    }

    #[test]
    fn activation_height_zero_is_rejected() {
        let activation = NativeExecutionActivationV2 {
            activation_height: 0,
        };
        assert!(activation.validate().is_err());
    }

    #[test]
    fn migration_preserves_accounts_and_starts_with_empty_channels() {
        let activation = NativeExecutionActivationV2 {
            activation_height: 100,
        };
        let mut v1 = NativeStateV1::default();
        v1.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 123_456,
                nonce: 7,
            },
        );

        let v2 = activation.migrate_parent_state(99, v1.clone()).unwrap();

        assert_eq!(v2.accounts(), &v1);
        assert_eq!(v2.channel_count(), 0);
    }

    #[test]
    fn locked_activation_migration_interoperability_vector() {
        let activation = NativeExecutionActivationV2 {
            activation_height: 100,
        };

        assert_eq!(
            activation.execution_version_at_height(99).unwrap(),
            NativeExecutionVersion::V1
        );
        assert_eq!(
            activation.execution_version_at_height(100).unwrap(),
            NativeExecutionVersion::V2
        );
        assert_eq!(
            activation.execution_version_at_height(101).unwrap(),
            NativeExecutionVersion::V2
        );

        activation
            .validate_execution_version_at_height(99, NativeExecutionVersion::V1)
            .unwrap();
        activation
            .validate_execution_version_at_height(100, NativeExecutionVersion::V2)
            .unwrap();
        assert!(activation
            .validate_execution_version_at_height(99, NativeExecutionVersion::V2)
            .unwrap_err()
            .contains("expected V1"));
        assert!(activation
            .validate_execution_version_at_height(100, NativeExecutionVersion::V1)
            .unwrap_err()
            .contains("expected V2"));

        let mut parent = NativeStateV1::default();
        parent.set_account(
            [0x11; 20],
            AccountStateV1 {
                balance: 123,
                nonce: 7,
            },
        );
        parent.set_account(
            [0x22; 20],
            AccountStateV1 {
                balance: 456,
                nonce: 9,
            },
        );

        assert_eq!(
            hex::encode(parent.canonical_bytes().unwrap()),
            "01000000000000000211111111111111111111111111111111111111110000000000000000000000000000007b00000000000000072222222222222222222222222222222222222222000000000000000000000000000001c80000000000000009"
        );
        assert_eq!(
            hex::encode(parent.state_root()),
            "7272574bef2b02d69e8431f0550486d8bcf25e7da320de899fc67bee013c5356"
        );

        let migrated = activation.migrate_parent_state(99, parent.clone()).unwrap();
        assert_eq!(migrated.accounts(), &parent);
        assert_eq!(migrated.channel_count(), 0);
        assert_eq!(
            hex::encode(migrated.channels_root().unwrap()),
            "eb41b47c5d86515b8a196eb2ae2d208edaf9074b124163983260d0285f1bde06"
        );
        assert_eq!(
            hex::encode(migrated.state_root().unwrap()),
            "5910b485bc87ecf07bc358dcd7df55671cfc1d494aee1a3dce8804016703d850"
        );
        assert_eq!(
            hex::encode(migrated.canonical_bytes().unwrap()),
            "02000000000000000211111111111111111111111111111111111111110000000000000000000000000000007b00000000000000072222222222222222222222222222222222222222000000000000000000000000000001c800000000000000090000000000000000"
        );

        // Restart classification is purely height/network-parameter derived.
        let restarted = NativeExecutionActivationV2 {
            activation_height: 100,
        };
        assert_eq!(
            restarted.execution_version_at_height(99).unwrap(),
            NativeExecutionVersion::V1
        );
        assert_eq!(
            restarted.execution_version_at_height(100).unwrap(),
            NativeExecutionVersion::V2
        );
        assert_eq!(
            restarted.execution_version_at_height(101).unwrap(),
            NativeExecutionVersion::V2
        );

        // Reorgs across the boundary must preserve the same version-by-height rule:
        // detached 99/100 and attached 99/100 are V1/V2 respectively.
        for height in [99_u64, 100] {
            let expected = if height < 100 {
                NativeExecutionVersion::V1
            } else {
                NativeExecutionVersion::V2
            };
            assert_eq!(activation.execution_version_at_height(height).unwrap(), expected);
        }
    }

    #[test]
    fn migration_rejects_wrong_parent_height() {
        let activation = NativeExecutionActivationV2 {
            activation_height: 100,
        };
        assert!(activation
            .migrate_parent_state(98, NativeStateV1::default())
            .unwrap_err()
            .contains("requires parent height 99"));
    }
}
