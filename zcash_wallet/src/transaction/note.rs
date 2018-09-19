use failure::Error;
use rand::{OsRng, Rng};

use pairing::bls12_381::Bls12;
use sapling_crypto::{
    jubjub::{edwards, fs::Fs, ToUniform, PrimeOrder},
    primitives::Diversifier,
};
use zcash_primitives::JUBJUB;

struct SaplingNoteEncryption {
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
    pub fn new(d: Diversifier) -> Result<SaplingNoteEncryption, Error> {
        let g_d = match d.g_d::<Bls12>(&JUBJUB) {
            Some(g_d) => g_d,
            None => return Err(format_err!("invalid diversifier")),
        };

        let esk = generate_esk();
        let epk = g_d.mul(esk, &JUBJUB);

        Ok(SaplingNoteEncryption{epk, esk})
    }
}
