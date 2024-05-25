pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use crate::{prover::FriProver, verifier::FriVerifier};
    use std::mem::size_of;
    use util::{
        algebra::{
            coset::Coset, field::mersenne61_ext::Mersenne61Ext, field::Field,
            polynomial::MultilinearPolynomial,
        },
        merkle_tree::MERKLE_ROOT_SIZE,
        random_oracle::RandomOracle,
    };

    use util::{CODE_RATE, SECURITY_BITS};
    fn output_proof_size(variable_num: usize) -> usize {
        let polynomial = MultilinearPolynomial::random_polynomial(variable_num);
        let mut interpolate_cosets = vec![Coset::new(
            1 << (variable_num + CODE_RATE),
            Mersenne61Ext::random_element(),
        )];
        for i in 1..variable_num {
            interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
        }
        let random_oracle = RandomOracle::new(variable_num, SECURITY_BITS / CODE_RATE);
        let vector_interpolation_coset =
            Coset::new(1 << variable_num, Mersenne61Ext::random_element());
        let mut prover = FriProver::new(
            variable_num,
            &interpolate_cosets,
            &vector_interpolation_coset,
            polynomial,
            &random_oracle,
        );
        let commit = prover.commit_first_polynomial();
        let mut verifier = FriVerifier::new(
            variable_num,
            &interpolate_cosets,
            &vector_interpolation_coset,
            commit,
            &random_oracle,
        );
        let open_point = verifier.get_open_point();
        prover.commit_functions(&mut verifier, &open_point);
        prover.prove();
        prover.commit_foldings(&mut verifier);
        let (folding_proofs, function_proofs, v_value) = prover.query();
        assert!(verifier.verify(&folding_proofs, &v_value, &function_proofs));
        folding_proofs.iter().map(|x| x.proof_size()).sum::<usize>()
            + (variable_num + 1) * MERKLE_ROOT_SIZE
            + size_of::<Mersenne61Ext>()
            + function_proofs
                .iter()
                .map(|x| x.proof_size())
                .sum::<usize>()
    }

    #[test]
    fn test_virgo_proof_size() {
        for i in 5..21 {
            let proof_size = output_proof_size(i);
            println!(
                "virgo pcs proof size of {} variables is {} bytes",
                i, proof_size
            );
        }
    }
}
