pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::{prover::Prover, verifier::Verifier};
    use util::{
        algebra::{
            coset::Coset,
            field::{mersenne61_ext::Mersenne61Ext, Field},
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
            Mersenne61Ext::from_int(1),
        )];
        for i in 1..variable_num {
            interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
        }
        let oracle = RandomOracle::new(variable_num, SECURITY_BITS / CODE_RATE);
        let mut prover = Prover::new(variable_num, &interpolate_cosets, polynomial, &oracle);
        let commit = prover.commit_polynomial();
        let mut verifier = Verifier::new(variable_num, &interpolate_cosets, commit, &oracle);
        let point = verifier.get_open_point();
        prover.send_evaluation(&mut verifier, &point);
        prover.prove(point);
        prover.commit_foldings(&mut verifier);
        let proof = prover.query();
        assert!(verifier.verify(&proof));
        proof.iter().map(|x| x.proof_size()).sum::<usize>()
            + variable_num * (MERKLE_ROOT_SIZE + size_of::<Mersenne61Ext>() * 3)
    }

    #[test]
    fn test_proof_size() {
        for i in 10..11 {
            let proof_size = output_proof_size(i);
            println!(
                "Basefold pcs proof size of {} variables is {} bytes",
                i, proof_size
            );
        }
    }
}
