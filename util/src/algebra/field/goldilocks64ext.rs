use crate::algebra::field::goldilocks64::{Goldilocks64, MOD};
use crate::algebra::field::{MyField, FftField};
use rand::RngCore;

use super::{goldilocks64, AnotherField};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Goldilocks64Ext {
    v: [Goldilocks64; 2],
}

impl std::ops::Neg for Goldilocks64Ext {
    type Output = Goldilocks64Ext;
    fn neg(self) -> Self::Output {
        Self {
            v: [-self.v[0], -self.v[1]],
        }
    }
}

impl std::ops::Add for Goldilocks64Ext {
    type Output = Goldilocks64Ext;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            v: [self.v[0] + rhs.v[0], self.v[1] + rhs.v[1]],
        }
    }
}

impl std::ops::AddAssign for Goldilocks64Ext {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Goldilocks64Ext {
    type Output = Goldilocks64Ext;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            v: [self.v[0] - rhs.v[0], self.v[1] - rhs.v[1]],
        }
    }
}

impl std::ops::SubAssign for Goldilocks64Ext {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Goldilocks64Ext {
    type Output = Goldilocks64Ext;
    fn mul(self, rhs: Self) -> Self::Output {
        let a = self.v[1];
        let b = self.v[0];
        let c = rhs.v[1];
        let d = rhs.v[0];
        Goldilocks64Ext {
            v: [b * d + a * c * 7u32.into(), b * c + a * d],
        }
    }
}

impl std::ops::MulAssign for Goldilocks64Ext {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl From<u32> for Goldilocks64Ext {
    fn from(value: u32) -> Self {
        Goldilocks64Ext {
            v: [value.into(), Goldilocks64::zero()],
        }
    }
}

impl From<Goldilocks64> for Goldilocks64Ext {
    fn from(value: Goldilocks64) -> Self {
        Goldilocks64Ext {
            v: [value, Goldilocks64::zero()],
        }
    }
}

impl From<u64> for Goldilocks64Ext {
    fn from(value: u64) -> Self {
        Goldilocks64Ext {
            v: [value.into(), Goldilocks64::zero()],
        }
    }
}

impl std::fmt::Display for Goldilocks64Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "v1: {}, v2: {}", self.v[0], self.v[1])
    }
}

impl MyField for Goldilocks64Ext {
    const FIELD_NAME: &'static str = "Goldilocks64Ext";
    const LOG_ORDER: u64 = 32;
    #[inline(always)]
    fn root_of_unity() -> Self {
        Goldilocks64Ext::ROOT_OF_UNITY
    }

    #[inline(always)]
    fn inverse_2() -> Self {
        Self::INV_2
    }

    #[inline]
    fn from_int(x: u64) -> Self {
        // Goldilocks64Ext {
        //     v: [
        //         x.into(),
        //         Goldilocks64::zero()
        //     ]
        // }
        Goldilocks64Ext::from(x)
    }

    #[inline]
    fn from_hash(hash: [u8; crate::merkle_tree::MERKLE_ROOT_SIZE]) -> Self {
        // Goldilocks64 { v: 0 }
        Goldilocks64Ext::zero()
    }

    #[inline]
    fn random_element() -> Self {
        let mut rng = rand::thread_rng();
        Goldilocks64Ext {
            v: [rng.next_u64().into(), rng.next_u64().into()],
        }
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        // let p = MOD;
        // let mut n = p * p - 2;
        // let mut ret = Self::from_int(1);
        // let mut base = self.clone();
        // while n != 0 {
        //     if n % 2 == 1 {
        //         ret *= base;
        //     }
        //     base *= base;
        //     n >>= 1;
        // }
        // ret
        Goldilocks64Ext::inv(&self).unwrap_or_default()
    }

    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        let x = self.v.to_le_bytes().to_vec();
        x
    }

    fn is_zero(&self) -> bool {
        *self == Goldilocks64Ext::ZERO
    }
}

impl AnotherField for Goldilocks64Ext {
    const NAME: &'static str = "Goldilocks64Ext";
    const SIZE: usize = 16;
    const INV_2: Self = Goldilocks64Ext {
        v: [
            Goldilocks64::INV_2,
            Goldilocks64::ZERO
        ]
    };
    const ZERO: Self = Goldilocks64Ext {
        v: [
            Goldilocks64::ZERO,
            Goldilocks64::ZERO
        ]
    };
    const UNIT: Self = Goldilocks64Ext {
        v: [
            Goldilocks64::UNIT,
            Goldilocks64::ZERO
        ]
    };
    type BaseField = Goldilocks64;

    fn zero() -> Self {
        Self::ZERO
    }

    // fn is_zero(&self) -> bool {
    //     self.v[0].is_zero() && self.v[1].is_zero()
    // }

    fn one() -> Self {
        Self::UNIT
    }

    fn double(&self) -> Self {
        self.clone() + self.clone()
    }

    fn square(&self) -> Self {
        self.clone() * self.clone()
    }

    fn random(mut rng: impl rand::RngCore) -> Self {
        Goldilocks64Ext {
            v: [rng.next_u64().into(), rng.next_u64().into()],
        }
    }

    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        Some(
            self.exp(MOD as usize - 1).exp(MOD as usize - 1)
                * self.exp(MOD as usize - 1)
                * self.exp(MOD as usize - 2),
        )
    }

    fn exp(&self, mut exponent: usize) -> Self {
        let mut res = Self::one();
        let mut t = self.clone();
        while exponent != 0 {
            if (exponent & 1) == 1 {
                res *= t;
            }
            t *= t;
            exponent >>= 1;
        }
        res
    }

    fn add_base_elem(&self, rhs: Self::BaseField) -> Self {
        Goldilocks64Ext {
            v: [self.v[0] + rhs, self.v[1]],
        }
    }

    fn add_assign_base_elem(&mut self, rhs: Self::BaseField) {
        self.v[0] += rhs;
    }

    fn mul_base_elem(&self, rhs: Self::BaseField) -> Self {
        Goldilocks64Ext {
            v: [self.v[0] * rhs, self.v[1] * rhs],
        }
    }

    fn mul_assign_base_elem(&mut self, rhs: Self::BaseField) {
        self.v[0] *= rhs;
    }

    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let ptr = bytes.as_ptr() as *const u64;
        let v0 = unsafe { ptr.read_unaligned() } as u64;
        let ptr = bytes[8..].as_ptr() as *const u64;
        let v1 = unsafe { ptr.read_unaligned() } as u64;
        Goldilocks64Ext {
            v: [v0.into(), v1.into()],
        }
    }

    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer[..Self::SIZE].copy_from_slice(unsafe {
            std::slice::from_raw_parts(&self.v as *const Goldilocks64 as *const u8, Self::SIZE)
        })
    }

    fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const u64;
        let v0 = unsafe { ptr.read_unaligned() };
        assert!(v0 < MOD);
        let ptr = buffer[8..].as_ptr() as *const u64;
        let v1 = unsafe { ptr.read_unaligned() };
        assert!(v1 < MOD);
        Goldilocks64Ext {
            v: [Goldilocks64::from(v0), Goldilocks64::from(v1)],
        }
    }
}

impl FftField for Goldilocks64Ext {
    const LOG_ORDER: u32 = 32;
    const ROOT_OF_UNITY: Self = Goldilocks64Ext {
        v: [
            Goldilocks64::ROOT_OF_UNITY,
            Goldilocks64::ZERO
        ]
    };
    type FftBaseField = Goldilocks64;
}
