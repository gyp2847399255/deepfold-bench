extern crate criterion;
use criterion::*;

use basefold::{prover::Prover, verifier::Verifier};
use util::{
    algebra::{
        coset::Coset,
        field::{ft255::Ft255, mersenne61_ext::Mersenne61Ext, Field},
        polynomial::MultilinearPolynomial,
    },
    random_oracle::RandomOracle,
};

use util::{CODE_RATE, SECURITY_BITS};
fn commit<T: Field>(criterion: &mut Criterion, variable_num: usize) {
    let polynomial = MultilinearPolynomial::random_polynomial(variable_num);
    let mut interpolate_cosets = vec![Coset::new(1 << (variable_num + CODE_RATE), T::from_int(1))];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(
        variable_num,
        (SECURITY_BITS as f32 / (2.0 / (1.0 + 0.5_f32.powi(CODE_RATE as i32))).log2()).ceil()
            as usize,
    );

    criterion.bench_function(
        &format!("basefold {} commit {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter_batched(
                || polynomial.clone(),
                |p| {
                    let prover = Prover::new(variable_num, &interpolate_cosets, p, &oracle);
                    let _commit = prover.commit_polynomial();
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_commit(c: &mut Criterion) {
    for i in 15..16 {
        commit::<Mersenne61Ext>(c, i);
        commit::<Ft255>(c, i);
    }
}

fn open<T: Field>(criterion: &mut Criterion, variable_num: usize) {
    let polynomial = MultilinearPolynomial::random_polynomial(variable_num);
    let mut interpolate_cosets = vec![Coset::new(1 << (variable_num + CODE_RATE), T::from_int(1))];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(
        variable_num,
        (SECURITY_BITS as f32 / (2.0 / (1.0 + 0.5_f32.powi(CODE_RATE as i32))).log2()).ceil()
            as usize,
    );
    let prover = Prover::new(variable_num, &interpolate_cosets, polynomial, &oracle);
    let commit = prover.commit_polynomial();
    let verifier = Verifier::new(variable_num, &interpolate_cosets, commit, &oracle);
    let point = verifier.get_open_point();

    criterion.bench_function(
        &format!("basefold {} open {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter_batched(
                || (prover.clone(), point.clone()),
                |(p, x)| {
                    let _proof = p.generate_proof(x);
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_open(c: &mut Criterion) {
    for i in 15..16 {
        open::<Mersenne61Ext>(c, i);
    }
}

fn verify<T: Field>(criterion: &mut Criterion, variable_num: usize) {
    let polynomial = MultilinearPolynomial::random_polynomial(variable_num);
    let mut interpolate_cosets = vec![Coset::new(1 << (variable_num + CODE_RATE), T::from_int(1))];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(
        variable_num,
        (SECURITY_BITS as f32 / (2.0 / (1.0 + 0.5_f32.powi(CODE_RATE as i32))).log2()).ceil()
            as usize,
    );
    let prover = Prover::new(variable_num, &interpolate_cosets, polynomial, &oracle);
    let commit = prover.commit_polynomial();
    let verifier = Verifier::new(variable_num, &interpolate_cosets, commit, &oracle);
    let point = verifier.get_open_point();
    let proof = prover.generate_proof(point);

    criterion.bench_function(
        &format!("basefold {} verify {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter_batched(
                || (verifier.clone(), proof.clone()),
                |(v, pi)| {
                    assert!(v.verify(pi));
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_verify(c: &mut Criterion) {
    for i in 15..16 {
        verify::<Mersenne61Ext>(c, i);
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_commit, bench_open, bench_verify
}

criterion_main!(benches);
