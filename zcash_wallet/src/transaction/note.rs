use failure::Error;
use rand::{OsRng, Rng};
use blake2_rfc::blake2b::Blake2b;

use pairing::bls12_381::{Bls12, Fr};
use sapling_crypto::{
    jubjub::{edwards, fs::Fs, ToUniform, Unknown, PrimeOrder},
    primitives::Diversifier,
};
use zcash_primitives::JUBJUB;
use super::Memo;

pub const PRF_OCK_PERSONALIZATION: &'static [u8; 16] = b"Zcash_Derive_ock";

pub struct SaplingNoteEncryption {
    epk: edwards::Point<Bls12, PrimeOrder>,
    esk: Fs,
}

fn generate_esk() -> Fs {
    // create random 64 byte buffer
    let mut rng = OsRng::new().expect("should be able to construct RNG");
    let mut buffer = [0u8; 64];
    for i in 0..buffer.len() {
        buffer[i] = rng.gen();
    }

    // reduce to uniform value
    Fs::to_uniform(&buffer[..])
}

impl SaplingNoteEncryption {
    pub fn new(g_d: edwards::Point<Bls12, PrimeOrder>) -> SaplingNoteEncryption {
        let esk = generate_esk();
        let epk = g_d.mul(esk, &JUBJUB);

        SaplingNoteEncryption{epk, esk}
    }

    pub fn esk(&self) -> &Fs {
        self.esk
    }

    pub fn epk(&self) -> &edwards::Point<Bls12, PrimeOrder> {
        self.epk
    }

    fn encrypt_note_plaintext(d: Diversifier, v: u64, rcm: Fs, memo: Memo) -> [u8; 580] {

    }

    fn encrypt_outgoing_plaintext(&self, ovk: &OutgoingViewingKey, cv: &edwards::Point<Bls12, Unknown>, cmu: &Fr) -> [u8; 80] {
        let mut ockInput = [0; 128];

        ockInput[0..32].copy_from_slice(ovk.0);
        cv.write(&mut ockInput[32..64]).unwrap();
        cmu.write(&mut ockInput[64..96]).unwrap();
        self.epk.write(&mut ockInput[96..128]).unwrap();

        let mut h = Blake2b::with_params(32, &[], &[], PRF_OCK_PERSONALIZATION);
        h.update(&ockInput);
        let mut h = h.finalize().as_bytes();
    }
}
