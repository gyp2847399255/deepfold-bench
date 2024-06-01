use super::verifier::Verifier;
use util::algebra::field::batch_inverse;
use util::algebra::polynomial::Polynomial;

use util::merkle_tree::MERKLE_ROOT_SIZE;
use util::query_result::QueryResult;
use util::{
    algebra::{coset::Coset, field::MyField},
    interpolation::InterpolateValue,
    random_oracle::RandomOracle,
};

#[derive(Clone)]
pub struct Prover<T: MyField> {
    total_round: usize,
    polynomial: Polynomial<T>,
    interpolate_cosets: Vec<Coset<T>>,
    interpolations: Vec<InterpolateValue<T>>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
}

impl<T: MyField> Prover<T> {
    pub fn new(
        total_round: usize,
        interpolate_coset: &Vec<Coset<T>>,
        polynomial: Polynomial<T>,
        oracle: &RandomOracle<T>,
    ) -> Prover<T> {
        let interpolate_polynomial = InterpolateValue::new(
            interpolate_coset[0].fft(polynomial.coefficients().clone()),
            2,
        );

        Prover {
            total_round,
            polynomial,
            interpolate_cosets: interpolate_coset.clone(),
            interpolations: vec![interpolate_polynomial],
            oracle: oracle.clone(),
            final_value: None,
        }
    }

    pub fn commit_polynomial(&self) -> [u8; MERKLE_ROOT_SIZE] {
        self.interpolations[0].commit()
    }

    pub fn commit_foldings(&self, verifier: &mut Verifier<T>) {
        for i in 1..self.total_round {
            let interpolation = &self.interpolations[i];
            verifier.receive_interpolation_root(interpolation.leave_num(), interpolation.commit());
        }
        verifier.set_final_value(self.final_value.unwrap());
    }

    fn evaluation_next_domain(&self, folding_value: &Vec<T>, round: usize, challenge: T) -> Vec<T> {
        let mut res = vec![];
        let len = self.interpolate_cosets[round].size();
        let coset = &self.interpolate_cosets[round];
        for i in 0..(len / 2) {
            let x = folding_value[i];
            let nx = folding_value[i + len / 2];
            let new_v = (x + nx) + challenge * (x - nx) * coset.element_inv_at(i);
            res.push(new_v);
        }
        res
    }

    pub fn prove(&mut self, point: T) -> T {
        let mut res = None;
        for i in 0..self.total_round {
            let challenge = self.oracle.folding_challenges[i];
            let next_evalutation = if i == 0 {
                let inv = batch_inverse(
                    &self.interpolate_cosets[0]
                        .all_elements()
                        .into_iter()
                        .map(|x| x - point)
                        .collect(),
                );
                res = Some(self.polynomial.evaluation_at(point));
                let v = self.interpolations[0].value.clone();
                self.evaluation_next_domain(
                    &v.into_iter()
                        .zip(inv.into_iter())
                        .map(|(x, inv)| (x - res.unwrap()) * inv)
                        .collect(),
                    i,
                    challenge,
                )
            } else {
                self.evaluation_next_domain(&self.interpolations[i].value, i, challenge)
            };
            if i < self.total_round - 1 {
                self.interpolations
                    .push(InterpolateValue::new(next_evalutation, 2));
            } else {
                self.final_value = Some(next_evalutation[0]);
            }
        }
        res.unwrap()
    }

    pub fn query(&self) -> Vec<QueryResult<T>> {
        let mut folding_res = vec![];
        let mut leaf_indices = self.oracle.query_list.clone();

        for i in 0..self.total_round {
            let len = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices.iter_mut().map(|v| *v % (len >> 1)).collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            folding_res.push(self.interpolations[i].query(&leaf_indices));
        }
        folding_res
    }
}
