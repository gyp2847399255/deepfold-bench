use std::usize;

use util::{
    algebra::{
        coset::Coset,
        field::{as_bytes_vec, MyField},
        polynomial::MultilinearPolynomial,
    },
    merkle_tree::{MerkleRoot, MerkleTreeProver},
};

struct Dealer<T: MyField> {
    polynomial: MultilinearPolynomial<T>,
    lines: Vec<Vec<(T, T)>>,
    fiat_shamir: Vec<MerkleTreeProver>,
    coset: Coset<T>,
    sharing: Vec<T>,
    final_value: Option<T>,
}

impl<T: MyField> Dealer<T> {
    pub fn new(polynomial: MultilinearPolynomial<T>, coset: &Coset<T>) -> Self {
        assert_eq!(coset.size(), 1 << (polynomial.variable_num() + 1));
        let sharing = coset.fft(polynomial.coefficients().clone());
        Dealer {
            polynomial,
            lines: vec![],
            fiat_shamir: vec![],
            coset: coset.clone(),
            sharing,
            final_value: None,
        }
    }

    pub fn deal(&mut self) {
        let mut sharing = self.sharing.clone();
        for i in 0..self.polynomial.variable_num() {
            let mut line = vec![];
            let len = sharing.len() / 2;
            for j in 0..len {
                let k = (sharing[j] - sharing[j + len])
                    * T::inverse_2()
                    * self.coset.element_inv_at(j << i);
                let b = (sharing[j] + sharing[j + len]) * T::inverse_2();
                line.push((k, b));
            }
            let tree =
                MerkleTreeProver::new(line.iter().map(|x| as_bytes_vec(&[x.0, x.1])).collect());
            let r = T::from_hash(tree.commit());
            self.fiat_shamir.push(tree);
            for j in 0..len {
                sharing[j] = line[j].0 * r + line[j].1;
            }
            sharing.truncate(len);
            self.lines.push(line)
        }

        self.final_value = Some(sharing[0]);
    }

    pub fn generate_proof(&self, party: usize) -> Proof<T> {
        Proof {
            share: self.sharing[party],
            lines: self
                .lines
                .iter()
                .map(|x| x[party & (x.len() - 1)])
                .collect(),
            merkle_paths: self
                .fiat_shamir
                .iter()
                .map(|x| x.open(&vec![party & (x.leave_num() - 1)]))
                .collect(),
            final_value: self.final_value.unwrap(),
        }
    }
}

struct Proof<T: MyField> {
    share: T,
    lines: Vec<(T, T)>,
    merkle_paths: Vec<Vec<u8>>,
    final_value: T,
}

impl<T: MyField> Proof<T> {
    pub fn get_challenge(&self, index: usize, round: usize) -> T {
        let num = 1 << (self.lines.len() - round);
        let root = MerkleRoot::get_root(
            self.merkle_paths[round].clone(),
            index & (num - 1),
            as_bytes_vec(&[self.lines[round].0, self.lines[round].1]),
            num,
        );
        T::from_hash(root)
    }
}

struct Party<T: MyField> {
    point: T,
    index: usize,
}

impl<T: MyField> Party<T> {
    pub fn new(point: T, index: usize) -> Self {
        Party { point, index }
    }

    pub fn verify(&self, proof: Proof<T>) {
        let (share, lines) = (proof.share, &proof.lines);
        let mut point = self.point;

        assert_eq!(share, lines[0].0 * point + lines[0].1);
        for i in 0..lines.len() {
            let r = proof.get_challenge(self.index, i);
            point *= point;
            if i == lines.len() - 1 {
                assert_eq!(lines[i].0 * r + lines[i].1, proof.final_value);
            } else {
                assert_eq!(
                    lines[i].0 * r + lines[i].1,
                    lines[i + 1].0 * point + lines[i + 1].1
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use util::algebra::field::mersenne61_ext::Mersenne61Ext;

    use super::*;

    #[test]
    fn it_works() {
        let poly = MultilinearPolynomial::random_polynomial(10);
        let coset = Coset::new(1 << 11, Mersenne61Ext::from_int(1));
        let mut dealer = Dealer::new(poly, &coset);
        dealer.deal();
        let proof_2000 = dealer.generate_proof(2000);
        let party_10 = Party::new(coset.element_at(2000), 2000);
        party_10.verify(proof_2000);
    }
}
