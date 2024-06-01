use std::collections::HashMap;

use util::algebra::polynomial::VanishingPolynomial;
use util::merkle_tree::MERKLE_ROOT_SIZE;
use util::query_result::QueryResult;
use util::random_oracle::RandomOracle;
use util::{
    algebra::{coset::Coset, field::MyField},
    merkle_tree::MerkleTreeVerifier,
};

#[derive(Clone)]
pub struct FriVerifier<T: MyField> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    vector_interpolation_coset: Coset<T>,
    u_root: MerkleTreeVerifier,
    h_root: Option<MerkleTreeVerifier>,
    folding_root: Vec<MerkleTreeVerifier>,
    oracle: RandomOracle<T>,
    vanishing_polynomial: VanishingPolynomial<T>,
    final_value: Option<T>,
    evaluation: Option<T>,
    open_point: Option<Vec<T>>,
}

impl<T: MyField> FriVerifier<T> {
    pub fn new(
        total_round: usize,
        coset: &Vec<Coset<T>>,
        vector_interpolation_coset: &Coset<T>,
        polynomial_commitment: [u8; MERKLE_ROOT_SIZE],
        oracle: &RandomOracle<T>,
    ) -> Self {
        FriVerifier {
            total_round,
            interpolate_cosets: coset.clone(),
            vector_interpolation_coset: vector_interpolation_coset.clone(),
            u_root: MerkleTreeVerifier {
                leave_number: coset[0].size() / 2,
                merkle_root: polynomial_commitment,
            },
            h_root: None,
            folding_root: vec![],
            oracle: oracle.clone(),
            vanishing_polynomial: VanishingPolynomial::new(vector_interpolation_coset),
            final_value: None,
            open_point: None,
            evaluation: None,
        }
    }

    pub fn set_evaluation(&mut self, v: T) {
        self.evaluation = Some(v);
    }

    pub fn get_open_point(&mut self) -> Vec<T> {
        let point = (0..self.total_round)
            .map(|_| T::random_element())
            .collect::<Vec<T>>();
        self.open_point = Some(point.clone());
        point
    }

    pub fn set_h_root(&mut self, h_root: [u8; MERKLE_ROOT_SIZE]) {
        self.h_root = Some(MerkleTreeVerifier {
            merkle_root: h_root,
            leave_number: self.interpolate_cosets[0].size() / 2,
        });
    }

    pub fn receive_folding_root(
        &mut self,
        leave_number: usize,
        folding_root: [u8; MERKLE_ROOT_SIZE],
    ) {
        self.folding_root.push(MerkleTreeVerifier {
            leave_number,
            merkle_root: folding_root,
        });
    }

    pub fn set_final_value(&mut self, value: T) {
        assert_ne!(value, T::from_int(0));
        self.final_value = Some(value);
    }

    pub fn verify(
        &self,
        folding_proofs: &Vec<QueryResult<T>>,
        v_values: &HashMap<usize, T>,
        function_proofs: &Vec<QueryResult<T>>,
    ) -> bool {
        let mut leaf_indices = self.oracle.query_list.clone();
        let rlc = self.oracle.rlc;
        let h_size = T::from_int(self.vector_interpolation_coset.size() as u64);
        for i in 0..self.total_round {
            let domain_size = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices
                .iter_mut()
                .map(|v| *v % (domain_size >> 1))
                .collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            if i == 0 {
                assert!(function_proofs[0].verify_merkle_tree(&leaf_indices, 2, &self.u_root));
                assert!(function_proofs[1].verify_merkle_tree(
                    &leaf_indices,
                    2,
                    &self.h_root.as_ref().unwrap()
                ));
            } else {
                folding_proofs[i - 1].verify_merkle_tree(
                    &leaf_indices,
                    2,
                    &self.folding_root[i - 1],
                );
            }

            let challenge = self.oracle.folding_challenges[i];
            let get_folding_value = |index: &usize| {
                if i == 0 {
                    let u = function_proofs[0].proof_values[index];
                    let h = function_proofs[1].proof_values[index];
                    let v = v_values[index];
                    let x = self.interpolate_cosets[i].element_at(*index);
                    let x_inv = self.interpolate_cosets[i].element_inv_at(*index);

                    let mut res = u;
                    let mut acc = rlc;
                    res += h * acc;
                    acc *= rlc;
                    res += acc
                        * (u * v * h_size
                            - self.vanishing_polynomial.evaluation_at(x) * h * h_size
                            - self.evaluation.unwrap())
                        * x_inv;
                    res
                } else {
                    folding_proofs[i - 1].proof_values[index]
                }
            };

            for j in &leaf_indices {
                let x = get_folding_value(j);
                let nx = get_folding_value(&(j + domain_size / 2));
                let v =
                    x + nx + challenge * (x - nx) * self.interpolate_cosets[i].element_inv_at(*j);
                if i < self.total_round - 1 {
                    if v != folding_proofs[i].proof_values[j] {
                        panic!("{}", i);
                        return false;
                    }
                } else {
                    if v != self.final_value.unwrap() {
                        panic!();
                        return false;
                    }
                }
            }
        }
        true
    }
}
