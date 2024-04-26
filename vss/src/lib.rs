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
}

impl<T: Field> Dealer<T> {
    pub fn new(polynomial: MultilinearPolynomial<T>, oracle: &RandomOracle<T>) -> Self {
        Dealer {
            polynomial,
            oracle: oracle.clone(),
            lines: vec![],
        }
    }

    pub fn deal(&mut self) {
        let log_n = self.polynomial.variable_num() + 1;
        let coset = Coset::new(1 << log_n, T::from_int(1));
        let mut sharing = coset.fft(self.polynomial.coefficients().clone());
        for i in 0..self.polynomial.variable_num() {
            let mut line = vec![];
            let len = sharing.len() / 2;
            for j in 0..len {
                let k =
                    (sharing[j] - sharing[j + len]) * T::INVERSE_2 * coset.element_inv_at(j << i);
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
                    .take(log_n - 1)
                    .collect()
            })
        );
    }
}

#[cfg(test)]
mod tests {
    use util::algebra::field::mersenne61_ext::Mersenne61Ext;

    use super::*;

    #[test]
    fn it_works() {
        let poly = MultilinearPolynomial::<Mersenne61Ext>::random_polynomial(10);
        let oracle = RandomOracle::new(10, 10);
        let mut dealer = Dealer::new(poly, &oracle);
        dealer.deal();
    }
}
