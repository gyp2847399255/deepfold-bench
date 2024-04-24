use util::merkle_tree::MERKLE_ROOT_SIZE;
use util::random_oracle::RandomOracle;
use util::{
    algebra::{coset::Coset, field::Field},
    merkle_tree::MerkleTreeVerifier,
    query_result::QueryResult,
};

use crate::{Commit, DeepEval};

#[derive(Clone)]
pub struct Verifier<T: Field> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    polynomial_roots: Vec<MerkleTreeVerifier>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
    shuffle_eval: Option<DeepEval<T>>,
    deep_evals: Vec<DeepEval<T>>,
    open_point: Vec<T>,
}

impl<T: Field> Verifier<T> {
    pub fn new(
        total_round: usize,
        coset: &Vec<Coset<T>>,
        commit: Commit<T>,
        oracle: &RandomOracle<T>,
    ) -> Self {
        Verifier {
            total_round,
            interpolate_cosets: coset.clone(),
            oracle: oracle.clone(),
            polynomial_roots: vec![MerkleTreeVerifier::new(coset[0].size() / 2, &commit.merkle_root)],
            final_value: None,
            shuffle_eval: None,
            deep_evals: vec![],
            open_point: (0..total_round).map(|_| T::random_element()).collect(),
        }
    }

    pub fn get_open_point(&self) -> Vec<T> {
        self.open_point.clone()
    }

    pub fn receive_shuffle_eval(&mut self, shuffle_eval: DeepEval<T>) {
        self.shuffle_eval = Some(shuffle_eval);
    }

    pub fn receive_folding_root(
        &mut self,
        leave_number: usize,
        folding_root: [u8; MERKLE_ROOT_SIZE],
    ) {
        self.polynomial_roots.push(MerkleTreeVerifier {
            leave_number,
            merkle_root: folding_root,
        });
    }

    pub fn receive_deep_eval(&mut self, deep_eval: DeepEval<T>) {
        self.deep_evals.push(deep_eval);
    }

    pub fn set_final_value(&mut self, value: T) {
        self.final_value = Some(value);
    }

    pub fn verify(&self, polynomial_proof: &Vec<QueryResult<T>>) -> bool {
        let mut leaf_indices = self.oracle.query_list.clone();
        for i in 0..self.total_round {
            let domain_size = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices
                .iter_mut()
                .map(|v| *v % (domain_size >> 1))
                .collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            polynomial_proof[i].verify_merkle_tree(&leaf_indices, &self.polynomial_roots[i]);
            let folding_value = &polynomial_proof[i].proof_values;
            let challenge = self.oracle.folding_challenges[i];

            if i == self.total_round - 1 {
                let challenges = self.oracle.folding_challenges[0..self.total_round].to_vec();
                assert_eq!(
                    self.shuffle_eval.as_ref().unwrap().verify(&challenges),
                    self.final_value.unwrap()
                );
                for j in &self.deep_evals {
                    assert_eq!(j.verify(&challenges), self.final_value.unwrap());
                }
            }
            for j in &leaf_indices {
                let x = folding_value[j];
                let nx = folding_value[&(j + domain_size / 2)];
                let v =
                    x + nx + challenge * (x - nx) * self.interpolate_cosets[i].element_inv_at(*j);
                if i == self.total_round - 1 {
                    assert_eq!(v * T::INVERSE_2, self.final_value.unwrap());
                } else {
                    assert_eq!(v * T::INVERSE_2, polynomial_proof[i + 1].proof_values[j]);
                }
            }
        }
        true
    }
}
