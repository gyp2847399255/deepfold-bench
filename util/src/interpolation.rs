use crate::merkle_tree::MERKLE_ROOT_SIZE;
use crate::query_result::QueryResult;
use crate::{
    algebra::field::{as_bytes_vec, Field},
    merkle_tree::MerkleTreeProver,
};

#[derive(Clone)]
pub struct InterpolateValue<T: Field> {
    pub value: Vec<T>,
    merkle_tree: MerkleTreeProver,
}

impl<T: Field> InterpolateValue<T> {
    pub fn new(value: Vec<T>) -> Self {
        let len = value.len() / 2;
        let merkle_tree = MerkleTreeProver::new(
            (0..len)
                .map(|i| as_bytes_vec(&[value[i], value[i + len]]))
                .collect(),
        );
        Self { value, merkle_tree }
    }

    pub fn leave_num(&self) -> usize {
        self.merkle_tree.leave_num()
    }

    pub fn commit(&self) -> [u8; MERKLE_ROOT_SIZE] {
        self.merkle_tree.commit()
    }

    pub fn query(&self, leaf_indices: &Vec<usize>) -> QueryResult<T> {
        let len = self.merkle_tree.leave_num();
        let proof_values = leaf_indices
            .iter()
            .flat_map(|j| [(*j, self.value[*j]), (*j + len, self.value[*j + len])])
            .collect();
        let proof_bytes = self.merkle_tree.open(&leaf_indices);
        QueryResult {
            proof_bytes,
            proof_values,
        }
    }
}
