use failure::Error;
use pairing::bls12_381::Bls12;
use sapling_crypto::{
    jubjub::fs::Fs,
    primitives::{Diversifier, Note, PaymentAddress},
};
use std::rc::Rc;
use transaction::WalletNote;
use zcash_primitives::transaction::components::Amount;
use zcash_proofs::sapling::CommitmentTreeWitness;
use zip32::ExtendedFullViewingKey;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccountId(pub u32);

/// A pool of ZEC controlled by a single spending key.
pub struct Account {
    pub label: String,
    pub(crate) xfvk: ExtendedFullViewingKey,
    pub(crate) notes: Vec<Rc<WalletNote>>,
}

impl Account {
    pub(crate) fn new(label: String, xfvk: ExtendedFullViewingKey) -> Self {
        Account {
            label,
            xfvk,
            notes: vec![],
        }
    }

    /// Returns the IncomingViewingKey for this account, which can be used to
    /// detect incoming payments.
    pub(crate) fn ivk(&self) -> Fs {
        self.xfvk.fvk.vk.ivk()
    }

    pub fn default_address(&self) -> PaymentAddress<Bls12> {
        self.xfvk.default_address().unwrap().1
    }

    /// Calculates the spendable and pending balances for this account.
    /// Returns the tuple (spendable, pending).
    pub fn balances(&self) -> (Amount, Amount) {
        let (spendable, pending): (Vec<(bool, u64)>, Vec<(bool, u64)>) = self
            .notes
            .iter()
            .filter_map(|n| match n.tx_spent {
                Some(_) => None,
                None => Some((n.is_spendable(), n.note.value)),
            }).partition(|(is_spendable, _)| *is_spendable);
        let (spendable, pending) = (
            spendable.iter().fold(0, |acc, (_, value)| acc + value),
            pending.iter().fold(0, |acc, (_, value)| acc + value),
        );
        (Amount(spendable as i64), Amount(pending as i64))
    }

    /// Selects notes from this account that add up to at least the given
    /// value. Returns an error if this account has insufficient spendable
    /// funds.
    pub(crate) fn select_notes(
        &self,
        value: Amount,
    ) -> Result<Vec<(Diversifier, Note<Bls12>, CommitmentTreeWitness)>, Error> {
        let notes: Vec<(Diversifier, Note<Bls12>, CommitmentTreeWitness)> = self
            .notes
            .iter()
            .filter_map(|n| match n.tx_spent {
                Some(_) => None,
                None => if n.is_spendable() {
                    Some(n)
                } else {
                    None
                },
            }).scan(0, |sum, n| {
                if *sum > value.0 {
                    None
                } else {
                    *sum = *sum + n.note.value as i64;
                    Some(n)
                }
            }).map(|n| {
                (
                    n.diversifier.clone(),
                    Note {
                        g_d: n.note.g_d.clone(),
                        pk_d: n.note.pk_d.clone(),
                        value: n.note.value,
                        r: n.note.r,
                    },
                    n.witness.as_ref().unwrap().clone(),
                )
            }).collect();

        let sum: i64 = notes.iter().map(|(_, note, _)| note.value as i64).sum();
        if sum < value.0 {
            Err(format_err!(
                "Insufficient notes for desired value (need {}, have {})",
                value.0,
                sum
            ))
        } else {
            Ok(notes)
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, XorShiftRng};
    use sapling_crypto::primitives::Diversifier;
    use std::cell::RefCell;
    use std::rc::Rc;
    use zcash_primitives::transaction::{components::Amount, TxId};
    use zcash_proofs::sapling::CommitmentTreeWitness;

    use super::Account;
    use constants;
    use keystore::LocalKeyStore;
    use tests::{fake_note, FAKE_WITNESS};
    use transaction::{WalletNote, WalletTx};
    use types::KeyStore;

    #[test]
    fn account_balances() {
        let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);

        let ks = LocalKeyStore::from_seed(&[0u8; 32]);
        let xfvk = ks.xfvk(constants::COIN_TYPE_TEST, 0);
        let mut account = Account::new("test".to_owned(), xfvk);

        // With no notes, the account has no balance
        assert_eq!(account.balances(), (Amount(0), Amount(0)));

        // Add some notes to the account
        let tx1 = Rc::new(RefCell::new(WalletTx::from_block(
            TxId([0u8; 32]),
            12345,
            120,
            100,
        )));
        let d = Diversifier([0u8; 11]);
        account.notes.push(Rc::new(WalletNote::new(
            Rc::downgrade(&tx1),
            d,
            fake_note(&d, 5, rng),
            CommitmentTreeWitness::from_slice(FAKE_WITNESS).unwrap(),
        )));
        let tx2 = Rc::new(RefCell::new(WalletTx::from_block(
            TxId([1u8; 32]),
            12345,
            130,
            110,
        )));
        let d = Diversifier([0u8; 11]);
        account.notes.push(Rc::new(WalletNote::new(
            Rc::downgrade(&tx2),
            d,
            fake_note(&d, 6, rng),
            CommitmentTreeWitness::from_slice(FAKE_WITNESS).unwrap(),
        )));

        // Notes from unverified transactions count as pending balance
        assert!(!tx1.borrow().is_verified());
        assert!(!tx2.borrow().is_verified());
        assert_eq!(account.balances(), (Amount(0), Amount(11)));

        // First transaction is verified, and its note becomes spendable
        tx1.borrow_mut().chain_tip(110);
        tx2.borrow_mut().chain_tip(110);
        assert!(tx1.borrow().is_verified());
        assert!(!tx2.borrow().is_verified());
        assert_eq!(account.balances(), (Amount(5), Amount(6)));

        // With both transactions verified, both notes are spendable
        tx1.borrow_mut().chain_tip(120);
        tx2.borrow_mut().chain_tip(120);
        assert!(tx1.borrow().is_verified());
        assert!(tx2.borrow().is_verified());
        assert_eq!(account.balances(), (Amount(11), Amount(0)));
    }
}
