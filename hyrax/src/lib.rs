use std::{mem::size_of, time::Instant};

use curve25519_dalek_ng::{ristretto::RistrettoPoint, scalar::Scalar, traits::MultiscalarMul};

use rand_core::OsRng;
use sha2::Sha512;

//replace with const generic
pub struct VectorCommitter {
    // number of generators in the scheme
    commit_size: usize,
    // group generators
    generators: Vec<RistrettoPoint>,
}

type Commitment = RistrettoPoint;

impl VectorCommitter {
    pub fn new(size: usize) -> Self {
        let generators = (0..size)
            .into_iter()
            .map(|index| {
                let msg = format!("pedersen_domain_sep:{}", index);
                RistrettoPoint::hash_from_bytes::<Sha512>(msg.as_bytes())
            })
            .collect::<Vec<RistrettoPoint>>();

        Self {
            commit_size: size,
            generators,
        }
    }

    // commit to N scalars, with randomness
    pub fn commit(&mut self, mut scalars: Vec<Scalar>) -> Commitment {
        if scalars.len() > self.commit_size {
            unimplemented!("committing too many elements")
        }

        let num_zero_elements = self.commit_size - scalars.len();

        (0..num_zero_elements)
            .into_iter()
            .for_each(|_| scalars.push(Scalar::zero()));

        RistrettoPoint::multiscalar_mul(scalars, self.generators.to_owned())
    }
}

pub fn bench(nv: usize, repetition: u128) {
    let size = nv / 2;
    let len = nv - size;
    let mut rng = OsRng;
    let mut trapdoor = VectorCommitter::new(1 << len);

    let mut a_scalars: Vec<Scalar> = Vec::with_capacity(100);
    (0..(1 << len)).into_iter().for_each(|_| {
        let a = Scalar::random(&mut rng);
        a_scalars.push(a);
    });

    let mut commitment = vec![];
    let start = Instant::now();
    for _ in 0..repetition {
        commitment = vec![];
        for _ in 0..(1 << size) {
            commitment.push(trapdoor.commit(a_scalars.clone()));
        }
    }
    println!(
        "nv: {}, proving time: {}, proof size: {}",
        nv,
        start.elapsed().as_micros() / repetition,
        (1 << size) * size_of::<RistrettoPoint>()
    );
    let open = (0..(1 << size))
        .into_iter()
        .map(|_| Scalar::random(&mut rng))
        .collect::<Vec<_>>();
    let start = Instant::now();
    for _ in 0..repetition {
        let _ = RistrettoPoint::multiscalar_mul(open.clone(), commitment.clone());
    }
    println!(
        "nv: {}, verification time: {}",
        nv,
        start.elapsed().as_micros() / repetition,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        bench(10, 10);
    }
}
