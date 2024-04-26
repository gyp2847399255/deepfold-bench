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
    pub fn new(polynomial: Polynomial<T>, oracle: &RandomOracle<T>) -> Self {
        Dealer {
            polynomial: MultilinearPolynomial::new(polynomial.coefficients),
            oracle: oracle.clone(),
            lines: vec![],
        }
    }

    pub fn deal(&mut self) {
        let log_n = self.polynomial.variable_num() + 1;
        let coset = Coset::new(1 << log_n, T::from_int(1));
        let mut sharing = coset.fft(self.polynomial.coefficients().clone());
        for i in 0..log_n {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
