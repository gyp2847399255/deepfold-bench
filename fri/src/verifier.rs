use util::merkle_tree::MERKLE_ROOT_SIZE;
use util::random_oracle::RandomOracle;
use util::{
    algebra::{coset::Coset, field::MyField},
    merkle_tree::MerkleTreeVerifier,
    query_result::QueryResult,
};

#[derive(Clone)]
pub struct Verifier<T: MyField> {
    total_round: usize,
    interpolate_cosets: Vec<Coset<T>>,
    interpolation_roots: Vec<MerkleTreeVerifier>,
    oracle: RandomOracle<T>,
    final_value: Option<T>,
    open_point: T,
}

impl<T: MyField> Verifier<T> {
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
            interpolation_roots: vec![MerkleTreeVerifier::new(coset[0].size() / 2, &commit)],
            final_value: None,
            open_point: T::random_element(),
        }
    }

    pub fn get_open_point(&self) -> T {
        self.open_point
    }

    pub fn receive_interpolation_root(
        &mut self,
        leave_number: usize,
        interpolation_root: [u8; MERKLE_ROOT_SIZE],
    ) {
        self.interpolation_roots.push(MerkleTreeVerifier {
            leave_number,
            merkle_root: interpolation_root,
        });
    }

    pub fn set_final_value(&mut self, value: T) {
        self.final_value = Some(value);
    }

    pub fn verify(&self, interpolation_proof: &Vec<QueryResult<T>>, evaluation: T) -> bool {
        let mut leaf_indices = self.oracle.query_list.clone();
        for i in 0..self.total_round {
            let domain_size = self.interpolate_cosets[i].size();
            leaf_indices = leaf_indices
                .iter_mut()
                .map(|v| *v % (domain_size >> 1))
                .collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            interpolation_proof[i].verify_merkle_tree(
                &leaf_indices,
                2,
                &self.interpolation_roots[i],
            );

            let challenge = self.oracle.folding_challenges[i];
            let get_folding_value: Box<dyn Fn(&usize) -> T> = if i == 0 {
                Box::new(|x| {
                    (interpolation_proof[0].proof_values[x] - evaluation)
                        * (self.interpolate_cosets[0].element_at(*x) - self.open_point).inverse()
                })
            } else {
                Box::new(|x| interpolation_proof[i].proof_values[x])
            };
            for j in &leaf_indices {
                let x = (*get_folding_value)(j);
                let nx = (*get_folding_value)(&(j + domain_size / 2));
                let v =
                    x + nx + challenge * (x - nx) * self.interpolate_cosets[i].element_inv_at(*j);
                if i == self.total_round - 1 {
                    assert_eq!(v, self.final_value.unwrap());
                } else {
                    assert_eq!(v, interpolation_proof[i + 1].proof_values[j]);
                }
            }
        }
        true
    }
}
