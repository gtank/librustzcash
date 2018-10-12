use blake2_rfc::blake2b::Blake2b;
use chacha20_poly1305_aead;
use rand::{OsRng, Rng};

use pairing::bls12_381::{Bls12, Fr};
use pairing::{PrimeField, PrimeFieldRepr};
use sapling_crypto::{
    jubjub::{edwards, fs::Fs, PrimeOrder, ToUniform, Unknown},
    primitives::{Diversifier, Note, PaymentAddress},
};
use zcash_primitives::JUBJUB;
use zip32::OutgoingViewingKey;

use super::Memo;

pub const PRF_OCK_PERSONALIZATION: &'static [u8; 16] = b"Zcash_Derive_ock";

pub struct SaplingNoteEncryption {
    epk: edwards::Point<Bls12, PrimeOrder>,
    esk: Fs,
    note: Note<Bls12>,
    to: PaymentAddress<Bls12>,
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
    pub fn new(note: Note<Bls12>, to: PaymentAddress<Bls12>) -> SaplingNoteEncryption {
        let esk = generate_esk();
        let epk = note.g_d.mul(esk, &JUBJUB);

        SaplingNoteEncryption { epk, esk, note, to }
    }

    pub fn esk(&self) -> &Fs {
        &self.esk
    }

    pub fn epk(&self) -> &edwards::Point<Bls12, PrimeOrder> {
        &self.epk
    }

    pub fn encrypt_note_plaintext(&self, d: Diversifier, v: u64, rcm: Fs, memo: Memo) -> [u8; 580] {
        let ciphertext = [0; 580];

        ciphertext
    }

    pub fn encrypt_outgoing_plaintext(
        &self,
        ovk: &OutgoingViewingKey,
        cv: &edwards::Point<Bls12, Unknown>,
        cmu: &Fr,
    ) -> [u8; 80] {
        let mut ockInput = [0u8; 128];
        ockInput[0..32].copy_from_slice(&ovk.0);
        cv.write(&mut ockInput[32..64]).unwrap();
        cmu.into_repr().write_le(&mut ockInput[64..96]).unwrap();
        self.epk.write(&mut ockInput[96..128]).unwrap();

        let mut h = Blake2b::with_params(32, &[], &[], PRF_OCK_PERSONALIZATION);
        h.update(&ockInput);
        let key = h.finalize().as_bytes();

        let mut input = [0u8; 64];
        self.note.pk_d.write(&mut input[0..32]).unwrap();
        self.esk.into_repr().write_le(&mut input[32..64]).unwrap();

        let mut buffer = Vec::with_capacity(64);
        let nonce = [0u8; 12];
        let tag = chacha20_poly1305_aead::encrypt(&key, &nonce, &[], &input, &mut buffer).unwrap();

        let mut output = [0u8; 80];
        output[0..64].copy_from_slice(&buffer);
        output[64..80].copy_from_slice(&tag[..]);

        output
    }
}
