extern crate criterion;
use criterion::*;

use fri::{prover::Prover, verifier::Verifier};
use util::{
    algebra::{
        coset::Coset,
        field::{ft255::Ft255, mersenne61_ext::Mersenne61Ext, MyField},
        polynomial::Polynomial,
    },
    random_oracle::RandomOracle,
};

use util::{CODE_RATE, SECURITY_BITS};
fn commit<T: MyField>(criterion: &mut Criterion, variable_num: usize) {
    let degree = 1 << variable_num;
    let polynomial = Polynomial::random_polynomial(degree);
    let mut interpolate_cosets = vec![Coset::new(1 << (variable_num + CODE_RATE), T::from_int(1))];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(variable_num, SECURITY_BITS / CODE_RATE);

    criterion.bench_function(
        &format!("fri {} commit {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter_batched(
                || polynomial.clone(),
                |p| {
                    let prover = Prover::new(variable_num, &interpolate_cosets, p, &oracle);
                    let _ = prover.commit_polynomial();
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_commit(c: &mut Criterion) {
    for i in 10..23 {
        commit::<Mersenne61Ext>(c, i);
    }
}

fn open<T: MyField>(criterion: &mut Criterion, variable_num: usize) {
    let degree = 1 << variable_num;
    let polynomial = Polynomial::random_polynomial(degree);
    let mut interpolate_cosets = vec![Coset::new(1 << (variable_num + CODE_RATE), T::from_int(1))];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(variable_num, SECURITY_BITS / CODE_RATE);
    let prover = Prover::new(variable_num, &interpolate_cosets, polynomial, &oracle);
    let commits = prover.commit_polynomial();
    let verifier = Verifier::new(variable_num, &interpolate_cosets, commits, &oracle);
    let point = verifier.get_open_point();

    criterion.bench_function(
        &format!("fri {} open {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter_batched(
                || (prover.clone(), verifier.clone()),
                |(mut p, mut v)| {
                    let _ = p.prove(point);
                    p.commit_foldings(&mut v);
                    let _ = p.query();
                },
                BatchSize::SmallInput,
            )
        },
    );
}

fn bench_open(c: &mut Criterion) {
    for i in 10..23 {
        open::<Mersenne61Ext>(c, i);
    }
}

fn verify<T: MyField>(criterion: &mut Criterion, variable_num: usize) {
    let degree = 1 << variable_num;
    let polynomial = Polynomial::random_polynomial(degree);
    let mut interpolate_cosets = vec![Coset::new(
        1 << (variable_num + CODE_RATE),
        Mersenne61Ext::from_int(1),
    )];
    for i in 1..variable_num {
        interpolate_cosets.push(interpolate_cosets[i - 1].pow(2));
    }
    let oracle = RandomOracle::new(variable_num, SECURITY_BITS / CODE_RATE);
    let mut prover = Prover::new(variable_num, &interpolate_cosets, polynomial, &oracle);
    let commits = prover.commit_polynomial();
    let mut verifier = Verifier::new(variable_num, &interpolate_cosets, commits, &oracle);
    let point = verifier.get_open_point();

    let evaluation = prover.prove(point);
    prover.commit_foldings(&mut verifier);
    let interpolation_proof = prover.query();

    criterion.bench_function(
        &format!("fri {} verify {}", T::FIELD_NAME, variable_num),
        move |b| {
            b.iter(|| {
                assert!(verifier.verify(&interpolation_proof, evaluation));
            })
        },
    );
}

fn bench_verify(c: &mut Criterion) {
    for i in 10..23 {
        verify::<Mersenne61Ext>(c, i);
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_commit, bench_open, bench_verify
}

criterion_main!(benches);
