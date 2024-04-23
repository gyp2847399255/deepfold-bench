#[derive(Debug, Clone, Copy)]
pub struct Fp64 {
    real: u64,
}

static MOD: u64 = 18446744069414584321u64; // 2**64 - 2**32 + 1
static HIGH: u128 = (1u128 << 127) - (1u128 << 96) + (1u128 << 127);
static MIDDLE: u128 = (1u128 << 96) - (1u128 << 64);
static LOW: u128 = (1u128 << 64) - 1;

impl std::ops::Neg for Fp64 {
    type Output = Fp64;
    fn neg(self) -> Self::Output {
        if self.real == 0 {
            return self.clone();
        }
        Self {
            real: MOD - self.real,
        }
    }
}

impl std::ops::Add for Fp64 {
    type Output = Fp64;
    fn add(self, rhs: Self) -> Self::Output {
        let mut res = self.real.wrapping_add(rhs.real);
        if res < self.real || res < rhs.real {
            res += 1u64 << 32;
            res -= 1;
        }
        if res >= MOD {
            res -= MOD;
        }
        Fp64 { real: res }
    }
}

impl std::ops::AddAssign for Fp64 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Fp64 {
    type Output = Fp64;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = self.real.wrapping_sub(rhs.real);
        if rhs.real > self.real {
            res = res.wrapping_add(MOD);
        }
        if res >= MOD {
            res -= MOD;
        }
        Fp64 { real: res }
    }
}

impl std::ops::SubAssign for Fp64 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Fp64 {
    type Output = Fp64;
    fn mul(self, rhs: Self) -> Self::Output {
        let res = self.real as u128 * rhs.real as u128;
        let high = ((res & HIGH) >> 96) as u64;
        let middle = ((res & MIDDLE) >> 64) as u64;
        let low = (res & LOW) as u64;
        let mut low2 = low.wrapping_sub(high);
        if high > low {
            low2 = low2.wrapping_add(MOD);
        }
        let mut product = middle << 32;
        product -= product >> 32;
        let mut ret = low2.wrapping_add(product);
        if ret < product || ret >= MOD {
            ret = ret.wrapping_sub(MOD);
        }
        Fp64 { real: ret }
    }
}

impl std::ops::MulAssign for Fp64 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::cmp::PartialEq for Fp64 {
    fn eq(&self, rhs: &Self) -> bool {
        self.real == rhs.real
    }
}

impl std::fmt::Display for Fp64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.real)
    }
}

use super::Field;
use rand::Rng;

impl Field for Fp64 {
    const FIELD_NAME: &'static str = "Fp64";
    const LOG_ORDER: u64 = 32;
    const ROOT_OF_UNITY: Fp64 = Fp64 {
        real: 2741030659394132017u64,
    };
    const INVERSE_2: Self = Fp64 {
        real: 9223372034707292161,
    };

    fn from_int(x: u64) -> Fp64 {
        if x >= MOD {
            panic!("");
        }
        Fp64 { real: x }
    }

    fn random_element() -> Self {
        let r: u64 = rand::thread_rng().gen_range(0..MOD);
        Fp64 { real: r }
    }

    fn inverse(&self) -> Self {
        let mut x_gcd = 0i128;
        let mut y_gcd = 0i128;
        Fp64::ex_gcd(self.real, MOD, &mut x_gcd, &mut y_gcd);
        let module = MOD as i128;
        let r = ((x_gcd % module + module) % module) as u64;
        Fp64 { real: r }
    }

    fn is_zero(&self) -> bool {
        self.real == 0
    }

    fn to_bytes(&self) -> Vec<u8> {
        let x = self.real.to_le_bytes().to_vec();
        x
    }
}

impl Fp64 {
    fn ex_gcd(a: u64, b: u64, x_gcd: &mut i128, y_gcd: &mut i128) {
        let mut gcd_m = 0i128;
        let mut gcd_n = 1i128;
        *x_gcd = 1;
        *y_gcd = 0;
        let mut a = a as i128;
        let mut b = b as i128;
        while b != 0 {
            let gcd_t = gcd_m;
            gcd_m = *x_gcd - a / b * gcd_m;
            *x_gcd = gcd_t;

            let gcd_t = gcd_n;
            gcd_n = *y_gcd - a / b * gcd_n;
            *y_gcd = gcd_t;

            let gcd_t = b;
            b = a % b;
            a = gcd_t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::field_tests::*;
    use super::*;

    #[test]
    fn test() {
        add_and_sub::<Fp64>();
        mult_and_inverse::<Fp64>();
        assigns::<Fp64>();
        pow_and_generator::<Fp64>();
    }
}
