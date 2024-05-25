use std::collections::HashMap;

use super::verifier::FriVerifier;
use util::{
    algebra::polynomial::{MultilinearPolynomial, Polynomial, VanishingPolynomial},
    merkle_tree::MERKLE_ROOT_SIZE,
    random_oracle::RandomOracle,
};

use util::query_result::QueryResult;
use util::{
    algebra::{
        coset::Coset,
        field::{as_bytes_vec, Field},
    },
    merkle_tree::MerkleTreeProver,
};

#[derive(Clone)]
struct InterpolateValue<T: Field> {
    value: Vec<T>,
    merkle_tree: MerkleTreeProver,
}

impl<T: Field> InterpolateValue<T> {
    fn new(value: Vec<T>) -> Self {
        let len = value.len() / 2;
        let merkle_tree = MerkleTreeProver::new(
            (0..len)
                .map(|i| as_bytes_vec(&[value[i], value[i + len]]))
                .collect(),
        );
        Self { value, merkle_tree }
    }

    fn leave_num(&self) -> usize {
        self.merkle_tree.leave_num()
    }

    fn commit(&self) -> [u8; MERKLE_ROOT_SIZE] {
        self.merkle_tree.commit()
    }

    fn query(&self, leaf_indices: &Vec<usize>) -> QueryResult<T> {
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

#[derive(Clone)]
pub struct FriProver<T: Field> {
    total_round: usize,
    vector_interpolation_coset: Coset<T>,
    fri_cosets: Vec<Coset<T>>,
    function_h: Option<InterpolateValue<T>>,
    function_u: InterpolateValue<T>,
    interpolation_v: Option<Vec<T>>,
    poly_u: Polynomial<T>,
    polynomial: MultilinearPolynomial<T>,
    foldings: Vec<InterpolateValue<T>>,
    oracle: RandomOracle<T>,
    evaluation: Option<T>,
    final_value: Option<T>,
}

impl<T: Field> FriProver<T> {
    pub fn new(
        total_round: usize,
        fri_cosets: &Vec<Coset<T>>,
        vector_interpolation_coset: &Coset<T>,
        polynomial: MultilinearPolynomial<T>,
        oracle: &RandomOracle<T>,
    ) -> FriProver<T> {
        assert_eq!(
            vector_interpolation_coset.size(),
            1 << polynomial.variable_num()
        );
        let interpolation = vector_interpolation_coset.ifft(polynomial.coefficients().clone());
        FriProver {
            total_round,
            vector_interpolation_coset: vector_interpolation_coset.clone(),
            fri_cosets: fri_cosets.clone(),
            function_h: None,
            function_u: InterpolateValue::new(fri_cosets[0].fft(interpolation.clone())),
            interpolation_v: None,
            poly_u: Polynomial::new(interpolation),
            polynomial,
            foldings: vec![],
            oracle: oracle.clone(),
            evaluation: None,
            final_value: None,
        }
    }

    pub fn commit_first_polynomial(&self) -> [u8; MERKLE_ROOT_SIZE] {
        self.function_u.commit()
    }

    pub fn commit_functions(&mut self, verifier: &mut FriVerifier<T>, open_point: &Vec<T>) {
        assert_eq!(open_point.len(), self.total_round);
        let mut public_vector = vec![T::from_int(1)];
        for i in open_point {
            let len = public_vector.len();
            for j in 0..len {
                public_vector.push(public_vector[j] * i.clone());
            }
        }
        let poly_v = Polynomial::new(self.vector_interpolation_coset.ifft(public_vector));
        assert!(poly_v.degree() < self.vector_interpolation_coset.size());
        let h = Coset::mult(&self.poly_u, &poly_v)
            .over_vanish_polynomial(&VanishingPolynomial::new(&self.vector_interpolation_coset));
        assert!(h.degree() < self.vector_interpolation_coset.size());
        let function_h = InterpolateValue::new(self.fri_cosets[0].fft(h.coefficients().clone()));
        verifier.set_h_root(function_h.commit());
        self.function_h = Some(function_h);
        self.interpolation_v = Some(self.fri_cosets[0].fft(poly_v.coefficients().clone()));
        let evaluation = self.polynomial.evaluate(open_point);
        self.evaluation = Some(evaluation);
        verifier.set_evaluation(evaluation);
    }

    pub fn commit_foldings(&self, verifier: &mut FriVerifier<T>) {
        for i in 0..(self.total_round - 1) {
            verifier.receive_folding_root(self.foldings[i].leave_num(), self.foldings[i].commit());
        }
        verifier.set_final_value(self.final_value.unwrap())
    }

    fn initial_interpolation(&self) -> Vec<T> {
        let rlc = self.oracle.rlc;
        let u = &self.function_u.value;
        let mut res = u.clone();
        let h = &self.function_h.as_ref().unwrap().value;
        let mut acc = rlc;
        for i in 0..self.fri_cosets[0].size() {
            res[i] += acc * h[i];
        }
        acc *= rlc;
        let vanish_polynomial = VanishingPolynomial::new(&self.vector_interpolation_coset);
        let v = self.interpolation_v.as_ref().unwrap();
        let h_size = T::from_int(self.vector_interpolation_coset.size() as u64);
        for i in 0..self.fri_cosets[0].size() {
            let x = self.fri_cosets[0].element_at(i);
            let x_inv = self.fri_cosets[0].element_inv_at(i);
            res[i] += acc
                * (u[i] * v[i] * h_size
                    - vanish_polynomial.evaluation_at(x) * h[i] * h_size
                    - self.evaluation.unwrap())
                * x_inv;
        }
        res
    }

    fn evaluation_next_domain(&self, round: usize, challenge: T) -> Vec<T> {
        let mut res = vec![];
        let coset = &self.fri_cosets[round];
        let len = coset.size();
        if round == 0 {
            let function = self.initial_interpolation();
            for i in 0..(len / 2) {
                let x = function[i];
                let nx = function[i + len / 2];
                let new_v = (x + nx) + challenge * (x - nx) * coset.element_inv_at(i);
                res.push(new_v);
            }
        } else {
            let last_folding = &self.foldings.last().unwrap().value;
            for i in 0..(len / 2) {
                let x = last_folding[i];
                let nx = last_folding[i + len / 2];
                let new_v = (x + nx) + challenge * (x - nx) * coset.element_inv_at(i);
                res.push(new_v);
            }
        }
        res
    }

    pub fn prove(&mut self) {
        for i in 0..self.total_round {
            let challenge = self.oracle.folding_challenges[i];
            let next_evalutation = self.evaluation_next_domain(i, challenge);
            if i < self.total_round - 1 {
                let interpolate_value = InterpolateValue::new(next_evalutation);
                self.foldings.push(interpolate_value);
            } else {
                let x = next_evalutation[0];
                for i in &next_evalutation {
                    assert_eq!(x, *i);
                }
                self.final_value = Some(next_evalutation[0]);
            }
        }
    }

    pub fn query(&self) -> (Vec<QueryResult<T>>, Vec<QueryResult<T>>, HashMap<usize, T>) {
        let mut folding_res = vec![];
        let mut functions_res = None;
        let mut leaf_indices = self.oracle.query_list.clone();
        let mut v_value = None;

        for i in 0..self.total_round {
            let len = self.fri_cosets[i].size() / 2;
            leaf_indices = leaf_indices.iter_mut().map(|v| *v % len).collect();
            leaf_indices.sort();
            leaf_indices.dedup();

            if i == 0 {
                functions_res = Some(vec![
                    self.function_u.query(&leaf_indices),
                    self.function_h.as_ref().unwrap().query(&leaf_indices),
                ]);
                let interpolation_v = self.interpolation_v.as_ref().unwrap();
                v_value = Some(
                    leaf_indices
                        .iter()
                        .flat_map(|j| {
                            [
                                (*j, interpolation_v[*j]),
                                (*j + len, interpolation_v[*j + len]),
                            ]
                        })
                        .collect(),
                );
            } else {
                let query_result = self.foldings[i - 1].query(&leaf_indices);
                folding_res.push(query_result);
            }
        }
        (folding_res, functions_res.unwrap(), v_value.unwrap())
    }
}
