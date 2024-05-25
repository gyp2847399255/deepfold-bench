use util::merkle_tree::MERKLE_ROOT_SIZE;
use util::random_oracle::RandomOracle;
use util::{
    algebra::{coset::Coset, field::Field},
    merkle_tree::MerkleTreeVerifier,
    query_result::QueryResult,
};

#[derive(Clone)]
pub struct Verifier<T: Field> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    polynomial_roots: Vec<MerkleTreeVerifier>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
    sumcheck_values: Vec<(T, T, T)>,
    open_point: Vec<T>,
    evaluation: Option<T>,
}

impl<T: Field> Verifier<T> {
    pub fn new(
        total_round: usize,
        coset: &Vec<Coset<T>>,
        commit: [u8; MERKLE_ROOT_SIZE],
        oracle: &RandomOracle<T>,
    ) -> Self {
        Verifier {
            total_round,
            interpolate_cosets: coset.clone(),
            oracle: oracle.clone(),
            polynomial_roots: vec![MerkleTreeVerifier::new(coset[0].size() / 2, &commit)],
            final_value: None,
            sumcheck_values: vec![],
            open_point: (0..total_round).map(|_| T::random_element()).collect(),
            evaluation: None,
        }
    }

    pub fn get_open_point(&self) -> Vec<T> {
        self.open_point.clone()
    }

    pub fn receive_sumcheck_value(&mut self, value: (T, T, T)) {
        self.sumcheck_values.push(value);
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

    pub fn set_evalutation(&mut self, evaluation: T) {
        self.evaluation = Some(evaluation);
    }

    pub fn set_final_value(&mut self, value: T) {
        self.final_value = Some(value);
    }

    pub fn verify(&self, polynomial_proof: &Vec<QueryResult<T>>) -> bool {
        let mut leaf_indices = self.oracle.query_list.clone();
        let mut sum = self.sumcheck_values[0].0 + self.sumcheck_values[0].1;
        for i in 0..self.total_round {
            let domain_size = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices
                .iter_mut()
                .map(|v| *v % (domain_size >> 1))
                .collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            polynomial_proof[i].verify_merkle_tree(&leaf_indices, 2, &self.polynomial_roots[i]);
            let folding_value = &polynomial_proof[i].proof_values;
            let challenge = self.oracle.folding_challenges[i];

            let x_0 = self.sumcheck_values[i].0;
            let x_1 = self.sumcheck_values[i].1;
            let x_2 = self.sumcheck_values[i].2;
            assert_eq!(sum, x_0 + x_1);
            sum = x_0 * (T::from_int(1) - challenge) * (T::from_int(2) - challenge) * T::INVERSE_2
                + x_1 * challenge * (T::from_int(2) - challenge)
                + x_2 * challenge * (challenge - T::from_int(1)) * T::INVERSE_2;
            for j in &leaf_indices {
                let x = folding_value[j];
                let nx = folding_value[&(j + domain_size / 2)];
                let v =
                    x + nx + challenge * (x - nx) * self.interpolate_cosets[i].element_inv_at(*j);
                if i == self.total_round - 1 {
                    assert_eq!(v * T::INVERSE_2, self.final_value.unwrap());
                    // assert_eq!(self.final_value.unwrap(), sum);
                } else {
                    assert_eq!(v * T::INVERSE_2, polynomial_proof[i + 1].proof_values[j]);
                }
            }
        }
        true
    }
}
