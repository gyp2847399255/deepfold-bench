use util::{
    algebra::{
        coset::Coset,
        field::Field,
        polynomial::{self, MultilinearPolynomial, Polynomial},
    },
    random_oracle::RandomOracle,
};

struct Dealer<T: Field> {
    polynomial: MultilinearPolynomial<T>,
    oracle: RandomOracle<T>,
    lines: Vec<Vec<(T, T)>>,
    coset: Coset<T>,
    sharing: Vec<T>,
    final_value: Option<T>,
}

impl<T: Field> Dealer<T> {
    pub fn new(
        polynomial: MultilinearPolynomial<T>,
        oracle: &RandomOracle<T>,
        coset: &Coset<T>,
    ) -> Self {
        assert_eq!(coset.size(), 1 << (polynomial.variable_num() + 1));
        let sharing = coset.fft(polynomial.coefficients().clone());
        Dealer {
            polynomial,
            oracle: oracle.clone(),
            lines: vec![],
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
                    * T::INVERSE_2
                    * self.coset.element_inv_at(j << i);
                let b = (sharing[j] + sharing[j + len]) * T::INVERSE_2;
                line.push((k, b));
                sharing[j] = k * self.oracle.folding_challenges[i] + b;
            }
            sharing.truncate(len);
            self.lines.push(line)
        }

        assert_eq!(
            sharing[0],
            self.polynomial.evaluate({
                &self
                    .oracle
                    .folding_challenges
                    .iter()
                    .map(|x| x.clone())
                    .collect()
            })
        );
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
            final_value: self.final_value.unwrap(),
        }
    }
}

struct Proof<T: Field> {
    share: T,
    lines: Vec<(T, T)>,
    final_value: T,
}

struct Party<T: Field> {
    point: T,
    oracle: RandomOracle<T>,
}

impl<T: Field> Party<T> {
    pub fn new(point: T, oracle: &RandomOracle<T>) -> Self {
        Party {
            point,
            oracle: oracle.clone(),
        }
    }

    pub fn verify(&self, proof: Proof<T>) {
        let (share, lines) = (proof.share, proof.lines);
        let mut point = self.point;

        assert_eq!(share, lines[0].0 * point + lines[0].1);
        for i in 0..lines.len() {
            point *= point;
            if i == lines.len() - 1 {
                assert_eq!(
                    lines[i].0 * self.oracle.folding_challenges[i] + lines[i].1,
                    proof.final_value
                );
            } else {
                assert_eq!(
                    lines[i].0 * self.oracle.folding_challenges[i] + lines[i].1,
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
        let oracle = RandomOracle::new(10, 10);
        let coset = Coset::new(1 << 11, Mersenne61Ext::from_int(1));
        let mut dealer = Dealer::new(poly, &oracle, &coset);
        dealer.deal();
        let proof_2000 = dealer.generate_proof(2000);
        let party_10 = Party::new(coset.element_at(2000), &oracle);
        party_10.verify(proof_2000);
    }
}
