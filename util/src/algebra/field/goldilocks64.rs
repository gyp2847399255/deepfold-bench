use super::{goldilocks64ext::Goldilocks64Ext, MyField};

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Goldilocks64 {
    v: u64,
}
pub const MOD: u64 = 18446744069414584321u64; // 2**64 - 2**32 + 1
pub const HIGH: u128 = (1u128 << 127) - (1u128 << 96) + (1u128 << 127);
pub const MIDDLE: u128 = (1u128 << 96) - (1u128 << 64);
pub const LOW: u128 = (1u128 << 64) - 1;

impl std::ops::Neg for Goldilocks64 {
    type Output = Goldilocks64;
    fn neg(self) -> Self::Output {
        if self.v == 0 {
            return self.clone();
        }
        Self { v: MOD - self.v }
    }
}

impl std::ops::Add for Goldilocks64 {
    type Output = Goldilocks64;
    fn add(self, rhs: Self) -> Self::Output {
        let mut res = self.v.wrapping_add(rhs.v);
        if res < self.v || res < rhs.v {
            res += 1u64 << 32;
            res -= 1;
        }
        if res >= MOD {
            res -= MOD;
        }
        Goldilocks64 { v: res }
    }
}

impl std::ops::AddAssign for Goldilocks64 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Goldilocks64 {
    type Output = Goldilocks64;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = self.v.wrapping_sub(rhs.v);
        if rhs.v > self.v {
            res = res.wrapping_add(MOD);
        }
        if res >= MOD {
            res -= MOD;
        }
        Goldilocks64 { v: res }
    }
}

impl std::ops::SubAssign for Goldilocks64 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Goldilocks64 {
    type Output = Goldilocks64;
    fn mul(self, rhs: Self) -> Self::Output {
        reduce128((self.v as u128) * (rhs.v as u128))
    }
}

use core::hint::unreachable_unchecked;
#[inline(always)]
pub fn assume(p: bool) {
    debug_assert!(p);
    if !p {
        unsafe {
            unreachable_unchecked();
        }
    }
}

/// Try to force Rust to emit a branch. Example:
///     if x > 2 {
///         y = foo();
///         branch_hint();
///     } else {
///         y = bar();
///     }
/// This function has no semantics. It is a hint only.
#[inline(always)]
pub fn branch_hint() {
    // NOTE: These are the currently supported assembly architectures. See the
    // [nightly reference](https://doc.rust-lang.org/nightly/reference/inline-assembly.html) for
    // the most up-to-date list.
    #[cfg(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "x86_64",
    ))]
    unsafe {
        core::arch::asm!("", options(nomem, nostack, preserves_flags));
    }
}

/// Fast addition modulo ORDER for x86-64.
/// This function is marked unsafe for the following reasons:
///   - It is only correct if x + y < 2**64 + ORDER = 0x1ffffffff00000001.
///   - It is only faster in some circumstances. In particular, on x86 it overwrites both inputs in
///     the registers, so its use is not recommended when either input will be used again.
#[inline(always)]
#[cfg(target_arch = "x86_64")]
unsafe fn add_no_canonicalize_trashing_input(x: u64, y: u64) -> u64 {
    let res_wrapped: u64;
    let adjustment: u64;
    core::arch::asm!(
        "add {0}, {1}",
        // Trick. The carry flag is set iff the addition overflowed.
        // sbb x, y does x := x - y - CF. In our case, x and y are both {1:e}, so it simply does
        // {1:e} := 0xffffffff on overflow and {1:e} := 0 otherwise. {1:e} is the low 32 bits of
        // {1}; the high 32-bits are zeroed on write. In the end, we end up with 0xffffffff in {1}
        // on overflow; this happens be EPSILON.
        // Note that the CPU does not realize that the result of sbb x, x does not actually depend
        // on x. We must write the result to a register that we know to be ready. We have a
        // dependency on {1} anyway, so let's use it.
        "sbb {1:e}, {1:e}",
        inlateout(reg) x => res_wrapped,
        inlateout(reg) y => adjustment,
        options(pure, nomem, nostack),
    );
    assume(x != 0 || (res_wrapped == y && adjustment == 0));
    assume(y != 0 || (res_wrapped == x && adjustment == 0));
    // Add EPSILON == subtract ORDER.
    // Cannot overflow unless the assumption if x + y < 2**64 + ORDER is incorrect.
    res_wrapped + adjustment
}

const EPSILON: u64 = (1 << 32) - 1;
#[inline]
fn reduce128(x: u128) -> Goldilocks64 {
    let (x_lo, x_hi) = split(x); // This is a no-op
    let x_hi_hi = x_hi >> 32;
    let x_hi_lo = x_hi & EPSILON;

    let (mut t0, borrow) = x_lo.overflowing_sub(x_hi_hi);
    if borrow {
        branch_hint(); // A borrow is exceedingly rare. It is faster to branch.
        t0 -= EPSILON; // Cannot underflow.
    }
    let t1 = x_hi_lo * EPSILON;
    let t2 = unsafe { add_no_canonicalize_trashing_input(t0, t1) };
    Goldilocks64 { v: t2 }
}

#[inline]
const fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

impl std::ops::MulAssign for Goldilocks64 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl From<u32> for Goldilocks64 {
    fn from(value: u32) -> Self {
        Goldilocks64 { v: value as u64 }
    }
}

impl From<u64> for Goldilocks64 {
    fn from(mut value: u64) -> Self {
        if (value >> 63) == 1 {
            value = (1u64 << 63) | (value & ((1u64 << 32) - 1))
        }
        Goldilocks64 { v: value }
    }
}

impl std::fmt::Display for Goldilocks64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "v: {}", self.v)
    }
}

use super::{FftField, AnotherField};

impl MyField for Goldilocks64 {
    const FIELD_NAME: &'static str = "Goldilocks64";
    const LOG_ORDER: u64 = 62; // Cauchy: 啥意思
    #[inline(always)]
    fn root_of_unity() -> Self {
        Self::ROOT_OF_UNITY
    }

    #[inline(always)]
    fn inverse_2() -> Self {
        Self::INV_2
    }

    #[inline]
    fn from_int(x: u64) -> Self {
        Goldilocks64::from(x)
    }

    #[inline]
    fn from_hash(hash: [u8; crate::merkle_tree::MERKLE_ROOT_SIZE]) -> Self {
        Goldilocks64 { v: 0 }
    }

    #[inline]
    fn random_element() -> Self {
        let mut rng = rand::thread_rng();
        Goldilocks64 {
            v: rng.gen_range(0..MOD),
        }
    }
    #[inline(always)]
    fn inverse(&self) -> Self {
        let p = MOD;
        let mut n = p * p - 2;
        let mut ret = Self::from(1u64);
        let mut base = self.clone();
        while n != 0 {
            if n % 2 == 1 {
                ret *= base;
            }
            base *= base;
            n >>= 1;
        }
        ret
    }

    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        let x = self.v.to_le_bytes().to_vec();
        x
    }

    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

impl AnotherField for Goldilocks64 {
    const NAME: &'static str = "Goldilocks64";
    const SIZE: usize = 8;
    const INV_2: Self = Goldilocks64 { v: (MOD + 1) / 2 };
    const ZERO: Self = Goldilocks64 {
        v: 0
    };
    const UNIT: Self = Goldilocks64 {
        v: 1
    };
    type BaseField = Goldilocks64;

    fn zero() -> Self {
        Self::ZERO
    }

    fn one() -> Self {
        Self::ONE
    }

    fn double(&self) -> Self {
        self.clone() + self.clone()
    }

    fn square(&self) -> Self {
        self.clone() * self.clone()
    }

    fn random(mut rng: impl rand::RngCore) -> Self {
        rng.next_u64().into()
    }

    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        Some(self.exp(MOD as usize - 2))
    }

    fn exp(&self, mut exponent: usize) -> Self {
        let mut res = Goldilocks64 { v: 1 };
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
        self.clone() + rhs
    }

    fn add_assign_base_elem(&mut self, rhs: Self::BaseField) {
        *self += rhs;
    }

    fn mul_base_elem(&self, rhs: Self::BaseField) -> Self {
        *self * rhs
    }

    fn mul_assign_base_elem(&mut self, rhs: Self::BaseField) {
        *self *= rhs;
    }

    fn from_uniform_bytes(bytes: &[u8; 32]) -> Self {
        let ptr = bytes.as_ptr() as *const u64;
        let v = unsafe { ptr.read_unaligned() } as u64;
        v.into()
    }

    fn serialize_into(&self, buffer: &mut [u8]) {
        buffer[..Self::SIZE].copy_from_slice(unsafe {
            std::slice::from_raw_parts(&self.v as *const u64 as *const u8, Self::SIZE)
        })
    }

    fn deserialize_from(buffer: &[u8]) -> Self {
        let ptr = buffer.as_ptr() as *const u64;
        let v = unsafe { ptr.read_unaligned() };
        assert!(v < MOD);
        Goldilocks64 { v }
    }
}

impl FftField for Goldilocks64 {
    const LOG_ORDER: u32 = 32;
    const ROOT_OF_UNITY: Self = Goldilocks64 {
        v: 2741030659394132017u64,
    };
    type FftBaseField = Goldilocks64;
}
