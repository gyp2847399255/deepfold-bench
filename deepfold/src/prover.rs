use util::{
    algebra::{
        coset::Coset,
        field::MyField,
        polynomial::{MultilinearPolynomial, Polynomial},
    },
    interpolation::InterpolateValue,
    query_result::QueryResult,
    random_oracle::RandomOracle,
};

use crate::{Commit, DeepEval, Proof};
use util::CODE_RATE;

#[derive(Clone)]
pub struct Prover<T: MyField> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    interpolations: Vec<InterpolateValue<T>>,
    hypercube_interpolation: Vec<T>,
    deep_eval: Vec<DeepEval<T>>,
    shuffle_eval: Option<DeepEval<T>>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
    final_poly: Option<Polynomial<T>>,
    step: usize,
}

impl<T: MyField> Prover<T> {
    pub fn new(
        total_round: usize,
        interpolate_cosets: &Vec<Coset<T>>,
        polynomial: MultilinearPolynomial<T>,
        oracle: &RandomOracle<T>,
        step: usize,
    ) -> Self {
        let point = std::iter::successors(Some(oracle.deep[0]), |&x| Some(x * x))
            .take(total_round)
            .collect::<Vec<_>>();
        let hypercube_interpolation = polynomial.evaluate_hypercube();
        Prover {
            total_round,
            interpolate_cosets: interpolate_cosets.clone(),
            interpolations: vec![InterpolateValue::new(
                interpolate_cosets[0].fft(polynomial.coefficients().clone()),
                1 << step,
            )],
            hypercube_interpolation: hypercube_interpolation.clone(),
            deep_eval: vec![DeepEval::new(point.clone(), hypercube_interpolation)],
            shuffle_eval: None,
            oracle: oracle.clone(),
            final_value: None,
            final_poly: None,
            step,
        }
    }

    pub fn commit_polynomial(&self) -> Commit<T> {
        Commit {
            merkle_root: self.interpolations[0].commit(),
            deep: self.deep_eval[0].first_eval,
        }
    }

    fn evaluation_next_domain(&self, round: usize, challenges: &Vec<T>) -> Vec<T> {
        let mut get_folding_value = self.interpolations[round].value.clone();

        for j in 0..self.step {
            if round * self.step + j == self.total_round {
                break;
            }
            let len = self.interpolate_cosets[round * self.step + j].size();
            let coset = &self.interpolate_cosets[round * self.step + j];
            let challenge = challenges[j];
            let mut tmp_folding_value = vec![];
            for i in 0..(len / 2) {
                let x = get_folding_value[i];
                let nx = get_folding_value[i + len / 2];
                let new_v = (x + nx) + challenge * (x - nx) * coset.element_inv_at(i);
                tmp_folding_value.push(new_v * T::inverse_2());
            }
            get_folding_value = tmp_folding_value;
        }
        get_folding_value
    }

    fn sumcheck_next_domain(hypercube_interpolation: &mut Vec<T>, m: usize, challenge: T) {
        for i in 0..m {
            hypercube_interpolation[i] *= T::from_int(1) - challenge;
            let tmp = hypercube_interpolation[i + m] * challenge;
            hypercube_interpolation[i] += tmp;
        }
        hypercube_interpolation.truncate(m);
    }

    pub fn prove(&mut self, point: Vec<T>) {
        let mut hypercube_interpolation = self.hypercube_interpolation.clone();
        self.shuffle_eval = Some(DeepEval::new(
            point.clone(),
            hypercube_interpolation.clone(),
        ));
        for i in 0..self.total_round / self.step + 1 {
            let mut challenges: Vec<T> = vec![];
            for j in 0..self.step {
                if i * self.step + j == self.total_round {
                    break;
                }
                challenges.push(self.oracle.folding_challenges[i * self.step + j]);
            }

            for j in 0..self.step {
                if i * self.step + j == self.total_round {
                    break;
                }
                self.shuffle_eval
                    .as_mut()
                    .unwrap()
                    .append_else_eval(hypercube_interpolation.clone());
                for deep in &mut self.deep_eval {
                    deep.append_else_eval(hypercube_interpolation.clone());
                }

                if i * self.step + j < self.total_round - 1 {
                    let m = 1 << (self.total_round - (i * self.step + j) - 1);
                    Self::sumcheck_next_domain(&mut hypercube_interpolation, m, challenges[j]);
                    self.deep_eval.push({
                        let deep_point = std::iter::successors(
                            Some(self.oracle.deep[i * self.step + j + 1]),
                            |&x| Some(x * x),
                        )
                        .take(self.total_round - (i * self.step + j) - 1)
                        .collect::<Vec<_>>();
                        DeepEval::new(deep_point.clone(), hypercube_interpolation.clone())
                    });
                }
            }

            // let challenge = self.oracle.folding_challenges[i];
            let next_evalutation = self.evaluation_next_domain(i, &challenges);
            if i < self.total_round / self.step - 1 {
                self.interpolations
                    .push(InterpolateValue::new(next_evalutation, 1 << self.step));
            } else if i == self.total_round / self.step - 1 {
                // todo: final_value
                self.interpolations.push(InterpolateValue::new(
                    next_evalutation.clone(),
                    1 << self.step,
                ));
                self.final_poly = Some(Polynomial::new(
                    self.interpolate_cosets[(i + 1) * self.step].ifft(next_evalutation),
                ));
            } else {
                assert_eq!(next_evalutation.len(), 1 << CODE_RATE);
                self.final_value = Some(next_evalutation[0]);
            }
        }
    }

    pub fn query(&self) -> Vec<QueryResult<T>> {
        let mut res = vec![];
        let mut leaf_indices = self.oracle.query_list.clone();

        for i in 0..self.total_round / self.step + 1 {
            let len = self.interpolate_cosets[i * self.step].size();
            leaf_indices = leaf_indices
                .iter_mut()
                .map(|v| *v % (len >> self.step))
                .collect();
            leaf_indices.sort();
            leaf_indices.dedup();
            res.push(self.interpolations[i].query(&leaf_indices));
        }
        res
    }

    pub fn generate_proof(mut self, point: Vec<T>) -> Proof<T> {
        self.prove(point);
        let query_result = self.query();
        Proof {
            merkle_root: (1..self.total_round / self.step)
                .into_iter()
                .map(|x| self.interpolations[x].commit())
                .collect(),
            query_result,
            deep_evals: self
                .deep_eval
                .iter()
                .map(|x| (x.first_eval, x.else_evals.clone()))
                .collect(),
            shuffle_evals: self.shuffle_eval.as_ref().unwrap().else_evals.clone(),
            final_value: self.final_value.unwrap(),
            final_poly: self.final_poly.unwrap(),
            evaluation: self.shuffle_eval.as_ref().unwrap().first_eval,
        }
    }
}
