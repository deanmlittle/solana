//! The serialized signature array of the current transaction.
//!
//! The _signatures sysvar_ provides access to the serialized transaction
//! signatures of the currently-running transaction. This allows for [signature
//! introspection][in], which is required to enable recursive, self-referential
//! data pointers for transaction signature-based compression.
//! TODO:
//! [in]: https://docs.solana.com/implemented-proposals/signature_introspection
//!
//! Similar to the instruction sysvar, data in the signatures sysvar is not accessed
//! through a type that implements the [`Sysvar`] trait. Instead, the signatures
//! sysvar is accessed through several free functions within this module.
//!
//! [`Sysvar`]: crate::sysvar::Sysvar
//!
//! See also the Solana [documentation on the header sysvar][sdoc].
//! TODO:
//! [sdoc]: https://docs.solana.com/developing/runtime-facilities/sysvars#signatures

#![allow(clippy::arithmetic_side_effects)]

use crate::{
    account_info::AccountInfo,
    program_error::ProgramError, sanitize::SanitizeError,
};
#[cfg(not(target_os = "solana"))]
use crate::serialize_utils::{append_slice, append_u8};

/// Signatures sysvar, dummy type.
///
/// This type exists for consistency with other sysvar modules, but is a dummy
/// type that does not contain sysvar data. It implements the [`SysvarId`] trait
/// but does not implement the [`Sysvar`] trait.
///
/// [`SysvarId`]: crate::sysvar::SysvarId
/// [`Sysvar`]: crate::sysvar::Sysvar
///
/// Use the free functions in this module to access the instructions sysvar.
pub struct Signatures();

/// Signature slice alias type
/// 
/// This type exists to give us better readability without having to add the
/// Solana SDK as a dependency. This is safe, as Signature data is only ever 
/// passed in from a SanitizedTransaction.
type Signature = [u8;64];

crate::declare_sysvar_id!("SysvarSignatures111111111111111111111111111", Signatures);

/// Construct the account data for the header sysvar.
///
/// This function is used by the runtime and not available to Solana programs.
#[cfg(not(target_os = "solana"))]
pub fn construct_signatures_data(signatures: &[Signature]) -> Vec<u8> {
    serialize_signatures(signatures)
}

/// Construct the account data for the signatures sysvar.
///
/// This function is used by the runtime and not available to Solana programs.
#[cfg(not(target_os = "solana"))]
pub fn serialize_signatures(signatures: &[Signature]) -> Vec<u8> {
    let mut data = Vec::with_capacity(1 + signatures.len() * 64);
    append_u8(&mut data, signatures.len() as u8);
    for sig in signatures {
        append_slice(&mut data, sig);
    }
    data
}

/// Load a `Signature` in the currently executing `Transaction` at the
/// specified index.
///
/// # Errors
///
/// Returns [`ProgramError::UnsupportedSysvar`] if the given account's ID is not equal to [`ID`].
/// Returns [`ProgramError::InvalidArgument`] if the signature index is out of bounds.
pub fn load_signature_at_checked(
    index: usize,
    signature_sysvar_account_info: &AccountInfo,
) -> Result<Signature, ProgramError> {
    if !check_id(signature_sysvar_account_info.key) {
        return Err(ProgramError::UnsupportedSysvar);
    }

    let signature_sysvar = signature_sysvar_account_info.try_borrow_data()?;
    deserialize_signature(index, &signature_sysvar).map_err(|err| match err {
        SanitizeError::IndexOutOfBounds => ProgramError::InvalidArgument,
        _ => ProgramError::InvalidInstructionData,
    })
}

fn deserialize_signature(index: usize, data: &[u8]) -> Result<Signature, SanitizeError> {
    // Make sure data is not empty
    if data.is_empty() {
        return Err(SanitizeError::IndexOutOfBounds);
    }
    
    // Read the number of signatures from the first byte
    let num_signatures = data[0] as usize;
    
    // Make sure the index is not out of bounds
    if index >= num_signatures {
        return Err(SanitizeError::IndexOutOfBounds);
    }

    // Calculate the starting position for the signature in the data
    let start = 1 + index * 64; // Skip the first byte which holds the number of signatures
    let end = start + 64;

    // Ensure there are enough remaining bytes in the data
    if end > data.len() {
        return Err(SanitizeError::IndexOutOfBounds);
    }

    // Read the signature
    let mut signature: [u8; 64] = [0; 64];
    signature.copy_from_slice(&data[start..end]);
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use crate::clock::Epoch;

    use {
        super::*,
        crate::pubkey::Pubkey,
    };

    #[test]
    fn test_load_signature_at_checked() {
        let owner = Pubkey::new_unique();
        let mut lamports = 1_000_000_000;
        let mut data: Vec<u8> = vec![3;193];
        data[1..65].copy_from_slice(&[0;64]);
        data[65..129].copy_from_slice(&[1;64]);
        data[129..193].copy_from_slice(&[2;64]);
        let account_info = AccountInfo::new(
            &ID,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );

        let sig = load_signature_at_checked(0, &account_info).unwrap();
        assert_eq!(sig, [0;64]);

        let sig = load_signature_at_checked(1, &account_info).unwrap();
        assert_eq!(sig, [1;64]);

        let sig = load_signature_at_checked(2, &account_info).unwrap();
        assert_eq!(sig, [2;64]);

        assert!(matches!(load_signature_at_checked(3, &account_info), Err(ProgramError::InvalidArgument)));
    }

    #[test]
    fn test_construct_signatures_data() {
        let signatures: [Signature; 5] = [
            [0;64],
            [1;64],
            [2;64],
            [3;64],
            [4;64],
        ];
        let data = construct_signatures_data(&signatures);

        let mut expected_data: Vec<u8> = vec![5];
        expected_data.extend_from_slice(&[0;64]);
        expected_data.extend_from_slice(&[1;64]);
        expected_data.extend_from_slice(&[2;64]);
        expected_data.extend_from_slice(&[3;64]);
        expected_data.extend_from_slice(&[4;64]);

        assert_eq!(data, expected_data);
    }
}