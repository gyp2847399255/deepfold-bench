use util::{
    algebra::{coset::Coset, field::Field, polynomial::MultilinearPolynomial},
    interpolation::InterpolateValue,
    merkle_tree::MERKLE_ROOT_SIZE,
    query_result::QueryResult,
    random_oracle::RandomOracle,
};

use crate::verifier::Verifier;

#[derive(Clone)]
pub struct DeepEval<T: Field> {
    point: Vec<T>,
    first_eval: T,
    else_evals: Vec<T>,
}

impl<T: Field> DeepEval<T> {
    pub fn new(point: Vec<T>, first_eval: T) -> Self {
        DeepEval {
            point,
            first_eval,
            else_evals: vec![],
        }
    }

    pub fn append_else_eval(&mut self, mut poly_hypercube: Vec<T>) {
        let mut point = self.point[self.else_evals.len()..].to_vec();
        point[0] += T::from_int(1);
        let mut len = poly_hypercube.len();
        assert_eq!(len, 1 << point.len());
        for v in point.into_iter() {
            len >>= 1;
            for i in 0..len {
                poly_hypercube[i] *= T::from_int(1) - v;
                let tmp = poly_hypercube[i + len] * v;
                poly_hypercube[i] += tmp;
            }
        }
        self.else_evals.push(poly_hypercube[0]);
    }

    pub fn verify(&self, challenges: &Vec<T>) -> T {
        let (_, challenges) = challenges.split_at(challenges.len() - self.point.len());
        let mut y_0 = self.first_eval;
        assert_eq!(self.point.len(), self.else_evals.len());
        for ((x, eval), challenge) in self
            .point
            .iter()
            .zip(self.else_evals.iter())
            .zip(challenges.into_iter())
        {
            let y_1 = eval.clone();
            y_0 += (y_1 - y_0) * (challenge.clone() - x.clone());
        }
        y_0
    }
}

#[derive(Clone)]
pub struct Prover<T: Field> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    polynomial: MultilinearPolynomial<T>,
    interpolations: Vec<InterpolateValue<T>>,
    hypercube_interpolation: Vec<T>,
    deep_eval: Vec<DeepEval<T>>,
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
            deep_eval: vec![],
            shuffle_eval: vec![],
            polynomial,
            oracle: oracle.clone(),
            final_value: None,
        }
    }

    pub fn commit_polynomial(&mut self) -> [u8; MERKLE_ROOT_SIZE] {
        let point = std::iter::successors(Some(self.oracle.deep[0]), |&x| Some(x * x))
            .take(self.total_round)
            .collect::<Vec<_>>();
        self.deep_eval.push(DeepEval::new(
            point.clone(),
            Self::evaluate_at(&self.hypercube_interpolation, point.clone()),
        ));
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
        for i in &self.deep_eval {
            verifier.receive_deep_eval(i.clone());
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
        let mut hypercube_interpolation = self.hypercube_interpolation.clone();
        for i in 0..self.total_round {
            self.shuffle_eval
                .push(Self::evaluate_at(&hypercube_interpolation, {
                    let mut else_point = (&point[i..]).to_vec();
                    else_point[0] += T::from_int(1);
                    else_point
                }));
            for deep in &mut self.deep_eval {
                deep.append_else_eval(
                    hypercube_interpolation
                        .iter()
                        .take(1 << (self.total_round - i))
                        .map(|x| x.clone())
                        .collect(),
                );
            }

            let m = 1 << (self.total_round - i - 1);
            let challenge = self.oracle.folding_challenges[i];
            let next_evalutation = self.evaluation_next_domain(i, challenge);
            if i < self.total_round - 1 {
                Self::sumcheck_next_domain(&mut hypercube_interpolation, m, challenge);
                self.deep_eval.push({
                    let deep_point =
                        std::iter::successors(Some(self.oracle.deep[i + 1]), |&x| Some(x * x))
                            .take(self.total_round - i - 1)
                            .collect::<Vec<_>>();
                    DeepEval::new(
                        deep_point.clone(),
                        Self::evaluate_at(&hypercube_interpolation, deep_point),
                    )
                });
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
