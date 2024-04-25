use util::{
    algebra::{
        field::Field,
        polynomial::{self, MultilinearPolynomial, Polynomial},
    }
};

struct Dealer<T: Field> {
    polynomial: MultilinearPolynomial<T>,
    lines: Vec<Vec<(T, T)>>,
}

impl<T: Field> Dealer<T> {
    pub fn new(polynomial: Polynomial<T>) -> Self {
        Dealer {
            polynomial: MultilinearPolynomial::new(polynomial.coefficients),
            lines: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
