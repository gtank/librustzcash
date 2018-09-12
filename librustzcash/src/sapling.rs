use super::is_small_order;
use bellman::groth16::{verify_proof, PreparedVerifyingKey, Proof};
use pairing::{
    bls12_381::{Bls12, Fr},
    Field,
};
use sapling_crypto::{
    circuit::multipack,
    jubjub::{edwards, FixedGenerators, JubjubBls12, Unknown},
    redjubjub::{PublicKey, Signature},
};
use zcash_proofs::sapling::compute_value_balance;

/// Create the spendAuthSig for a Sapling SpendDescription.
pub fn spend_sig(
    ask: PrivateKey<Bls12>,
    ar: Fs,
    sighash: &[u8; 32],
    params: &JubjubBls12,
) -> Signature {
    // Initialize secure RNG
    let mut rng = OsRng::new().expect("should be able to construct RNG");

    // We compute `rsk`...
    let rsk = ask.randomize(ar);

    // We compute `rk` from there (needed for key prefixing)
    let rk = PublicKey::from_private(&rsk, FixedGenerators::SpendingKeyGenerator, params);

    // Compute the signature's message for rk/spend_auth_sig
    let mut data_to_be_signed = [0u8; 64];
    rk.0
        .write(&mut data_to_be_signed[0..32])
        .expect("message buffer should be 32 bytes");
    (&mut data_to_be_signed[32..64]).copy_from_slice(&sighash[..]);

    // Do the signing
    rsk.sign(
        &data_to_be_signed,
        &mut rng,
        FixedGenerators::SpendingKeyGenerator,
        params,
    )
}

/// A context object for verifying the Sapling components of a Zcash transaction.
pub struct SaplingVerificationContext {
    bvk: edwards::Point<Bls12, Unknown>,
}

impl SaplingVerificationContext {
    /// Construct a new context to be used with a single transaction.
    pub fn new() -> Self {
        SaplingVerificationContext {
            bvk: edwards::Point::zero(),
        }
    }

    /// Perform consensus checks on a Sapling SpendDescription, while
    /// accumulating its value commitment inside the context for later use.
    pub fn check_spend(
        &mut self,
        cv: edwards::Point<Bls12, Unknown>,
        anchor: Fr,
        nullifier: &[u8; 32],
        rk: PublicKey<Bls12>,
        sighash_value: &[u8; 32],
        spend_auth_sig: Signature,
        zkproof: Proof<Bls12>,
        verifying_key: &PreparedVerifyingKey<Bls12>,
        params: &JubjubBls12,
    ) -> bool {
        if is_small_order(&cv) {
            return false;
        }

        if is_small_order(&rk.0) {
            return false;
        }

        // Accumulate the value commitment in the context
        {
            let mut tmp = cv.clone();
            tmp = tmp.add(&self.bvk, params);

            // Update the context
            self.bvk = tmp;
        }

        // Grab the nullifier as a sequence of bytes
        let nullifier = &nullifier[..];

        // Compute the signature's message for rk/spend_auth_sig
        let mut data_to_be_signed = [0u8; 64];
        rk.0
            .write(&mut data_to_be_signed[0..32])
            .expect("message buffer should be 32 bytes");
        (&mut data_to_be_signed[32..64]).copy_from_slice(&sighash_value[..]);

        // Verify the spend_auth_sig
        if !rk.verify(
            &data_to_be_signed,
            &spend_auth_sig,
            FixedGenerators::SpendingKeyGenerator,
            params,
        ) {
            return false;
        }

        // Construct public input for circuit
        let mut public_input = [Fr::zero(); 7];
        {
            let (x, y) = rk.0.into_xy();
            public_input[0] = x;
            public_input[1] = y;
        }
        {
            let (x, y) = cv.into_xy();
            public_input[2] = x;
            public_input[3] = y;
        }
        public_input[4] = anchor;

        // Add the nullifier through multiscalar packing
        {
            let nullifier = multipack::bytes_to_bits_le(nullifier);
            let nullifier = multipack::compute_multipacking::<Bls12>(&nullifier);

            assert_eq!(nullifier.len(), 2);

            public_input[5] = nullifier[0];
            public_input[6] = nullifier[1];
        }

        // Verify the proof
        match verify_proof(verifying_key, &zkproof, &public_input[..]) {
            // No error, and proof verification successful
            Ok(true) => true,

            // Any other case
            _ => false,
        }
    }

    /// Perform consensus checks on a Sapling OutputDescription, while
    /// accumulating its value commitment inside the context for later use.
    pub fn check_output(
        &mut self,
        cv: edwards::Point<Bls12, Unknown>,
        cm: Fr,
        epk: edwards::Point<Bls12, Unknown>,
        zkproof: Proof<Bls12>,
        verifying_key: &PreparedVerifyingKey<Bls12>,
        params: &JubjubBls12,
    ) -> bool {
        if is_small_order(&cv) {
            return false;
        }

        if is_small_order(&epk) {
            return false;
        }

        // Accumulate the value commitment in the context
        {
            let mut tmp = cv.clone();
            tmp = tmp.negate(); // Outputs subtract from the total.
            tmp = tmp.add(&self.bvk, params);

            // Update the context
            self.bvk = tmp;
        }

        // Construct public input for circuit
        let mut public_input = [Fr::zero(); 5];
        {
            let (x, y) = cv.into_xy();
            public_input[0] = x;
            public_input[1] = y;
        }
        {
            let (x, y) = epk.into_xy();
            public_input[2] = x;
            public_input[3] = y;
        }
        public_input[4] = cm;

        // Verify the proof
        match verify_proof(verifying_key, &zkproof, &public_input[..]) {
            // No error, and proof verification successful
            Ok(true) => true,

            // Any other case
            _ => false,
        }
    }

    /// Perform consensus checks on the valueBalance and bindingSig parts of a
    /// Sapling transaction. All SpendDescriptions and OutputDescriptions must
    /// have been checked before calling this function.
    pub fn final_check(
        &self,
        value_balance: i64,
        sighash_value: &[u8; 32],
        binding_sig: Signature,
        params: &JubjubBls12,
    ) -> bool {
        // Obtain current bvk from the context
        let mut bvk = PublicKey(self.bvk.clone());

        // Compute value balance
        let mut value_balance = match compute_value_balance(value_balance, params) {
            Some(a) => a,
            None => return false,
        };

        // Subtract value_balance from current bvk to get final bvk
        value_balance = value_balance.negate();
        bvk.0 = bvk.0.add(&value_balance, params);

        // Compute the signature's message for bvk/binding_sig
        let mut data_to_be_signed = [0u8; 64];
        bvk.0
            .write(&mut data_to_be_signed[0..32])
            .expect("bvk is 32 bytes");
        (&mut data_to_be_signed[32..64]).copy_from_slice(&sighash_value[..]);

        // Verify the binding_sig
        bvk.verify(
            &data_to_be_signed,
            &binding_sig,
            FixedGenerators::ValueCommitmentRandomness,
            params,
        )
    }
}