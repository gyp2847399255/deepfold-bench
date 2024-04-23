use util::{
    algebra::{coset::Coset, field::Field, polynomial::MultilinearPolynomial},
    interpolation::InterpolateValue,
    merkle_tree::MERKLE_ROOT_SIZE,
    query_result::QueryResult,
    random_oracle::RandomOracle,
};

use crate::verifier::Verifier;

#[derive(Clone)]
pub struct Prover<T: Field> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    polynomial: MultilinearPolynomial<T>,
    interpolations: Vec<InterpolateValue<T>>,
    hypercube_interpolation: Vec<T>,
    // deep_eval: Vec<(T, T)>,
    shuffle_eval: Vec<T>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
}

impl<T: Field> Prover<T> {
    pub fn new(
        total_round: usize,
        interpolate_cosets: &Vec<Coset<T>>,
        polynomial: MultilinearPolynomial<T>,
        oracle: &RandomOracle<T>,
    ) -> Self {
        Prover {
            total_round,
            interpolate_cosets: interpolate_cosets.clone(),
            interpolations: vec![InterpolateValue::new(
                interpolate_cosets[0].fft(polynomial.coefficients().clone()),
            )],
            hypercube_interpolation: polynomial.evaluate_hypercube(),
            // deep_eval: vec![],
            shuffle_eval: vec![],
            polynomial,
            oracle: oracle.clone(),
            final_value: None,
        }
    }

    pub fn commit_polynomial(&mut self) -> [u8; MERKLE_ROOT_SIZE] {
        // let point1 = std::iter::successors(Some(self.oracle.deep[0]), |&x| Some(x * x))
        //     .take(self.total_round)
        //     .collect::<Vec<_>>();
        // let point2 = std::iter::successors(Some(-self.oracle.deep[0]), |&x| Some(x * x))
        //     .take(self.total_round)
        //     .collect::<Vec<_>>();
        // self.deep_eval.push((
        //     Self::evaluate_at(&self.hypercube_interpolation, point1),
        //     Self::evaluate_at(&self.hypercube_interpolation, point2),
        // ));
        self.interpolations[0].commit()
    }

    pub fn commit_foldings(&self, verifier: &mut Verifier<T>) {
        for i in 1..self.total_round {
            let interpolation = &self.interpolations[i];
            verifier.receive_folding_root(interpolation.leave_num(), interpolation.commit());
        }
        for i in &self.shuffle_eval {
            verifier.receive_shuffle_eval(i.clone());
        }
        verifier.set_final_value(self.final_value.unwrap());
    }

    pub fn send_evaluation(&self, verifier: &mut Verifier<T>, point: &Vec<T>) {
        verifier.set_evalutation(self.polynomial.evaluate(point));
    }

    fn evaluation_next_domain(&self, round: usize, challenge: T) -> Vec<T> {
        let mut res = vec![];
        let len = self.interpolate_cosets[round].size();
        let get_folding_value = &self.interpolations[round].value;
        let coset = &self.interpolate_cosets[round];
        for i in 0..(len / 2) {
            let x = get_folding_value[i];
            let nx = get_folding_value[i + len / 2];
            let new_v = (x + nx) + challenge * (x - nx) * coset.element_inv_at(i);
            res.push(new_v * T::INVERSE_2);
        }
        res
    }

    fn sumcheck_next_domain(hypercube_interpolation: &mut Vec<T>, m: usize, challenge: T) {
        for i in 0..m {
            hypercube_interpolation[i] *= T::from_int(1) - challenge;
            let tmp = hypercube_interpolation[i + m] * challenge;
            hypercube_interpolation[i] += tmp;
        }
    }

    fn evaluate_at(hypercube_interpolation: &Vec<T>, point: Vec<T>) -> T {
        let mut len = 1 << point.len();
        let mut res = hypercube_interpolation
            .iter()
            .take(len)
            .map(|x| x.clone())
            .collect::<Vec<_>>();
        for v in point.into_iter() {
            len >>= 1;
            Self::sumcheck_next_domain(&mut res, len, v);
        }
        res[0]
    }

    pub fn prove(&mut self, point: Vec<T>) {
        let mut poly_hypercube = self.hypercube_interpolation.clone();
        for i in 0..self.total_round {
            self.shuffle_eval.push(Self::evaluate_at(&poly_hypercube, {
                let mut else_point = (&point[i..]).to_vec();
                else_point[0] += T::from_int(1);
                else_point
            }));
            let m = 1 << (self.total_round - i - 1);
            let challenge = self.oracle.folding_challenges[i];
            let next_evalutation = self.evaluation_next_domain(i, challenge);
            if i < self.total_round - 1 {
                Self::sumcheck_next_domain(&mut poly_hypercube, m, challenge);
                self.interpolations
                    .push(InterpolateValue::new(next_evalutation));
            } else {
                self.final_value = Some(next_evalutation[0]);
            }
        }
    }

    pub fn query(&self) -> Vec<QueryResult<T>> {
        let mut res = vec![];
        let mut leaf_indices = self.oracle.query_list.clone();

        for i in 0..self.total_round {
            let len = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices.iter_mut().map(|v| *v % (len >> 1)).collect();
            leaf_indices.sort();
            leaf_indices.dedup();
            res.push(self.interpolations[i].query(&leaf_indices));
        }
        res
    }
}
