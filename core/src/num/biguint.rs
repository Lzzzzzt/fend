use std::cmp::{max, Ordering};
use std::fmt::{Debug, Display, Error, Formatter};
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub};

#[derive(Clone)]
pub struct BigUint {
    // little-endian, len >= 1
    value: Vec<u64>,
}

impl BigUint {
    fn is_zero(&self) -> bool {
        for v in self.value.iter().copied() {
            if v != 0 {
                return false;
            }
        }
        true
    }

    fn get(&self, idx: usize) -> u64 {
        if idx < self.value.len() {
            self.value[idx]
        } else {
            0
        }
    }

    fn set(&mut self, idx: usize, new_value: u64) {
        while idx >= self.value.len() {
            self.value.push(0);
        }
        self.value[idx] = new_value;
    }
}

impl Ord for BigUint {
    fn cmp(&self, other: &BigUint) -> Ordering {
        let mut i = std::cmp::max(self.value.len(), other.value.len());
        while i != 0 {
            let v1 = self.get(i - 1);
            let v2 = other.get(i - 1);
            if v1 < v2 {
                return Ordering::Less;
            } else if v1 > v2 {
                return Ordering::Greater;
            }
            i -= 1;
        }

        Ordering::Equal
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &BigUint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BigUint {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for BigUint {}

impl From<u64> for BigUint {
    fn from(val: u64) -> BigUint {
        BigUint { value: vec![val] }
    }
}

impl BigUint {
    /// computes self += (other * mul_digit) << (64 * shift)
    fn add_assign_internal(&mut self, other: &BigUint, mul_digit: u64, shift: usize) {
        let mut carry = 0;
        for i in 0..max(self.value.len(), other.value.len() + shift) {
            let a = self.get(i);
            let b = if i >= shift { other.get(i - shift) } else { 0 };
            let sum = a as u128 + (b as u128 * mul_digit as u128) + carry as u128;
            self.set(i, sum as u64);
            carry = (sum >> 64) as u64;
        }
        if carry != 0 {
            self.value.push(carry);
        }
    }
}

impl AddAssign<&BigUint> for BigUint {
    fn add_assign(&mut self, other: &BigUint) {
        self.add_assign_internal(other, 1, 0);
    }
}

impl BigUint {
    fn pow_internal(&self, mut exponent: u64) -> BigUint {
        let mut result = BigUint::from(1);
        let mut base = self.clone();
        while exponent > 0 {
            if exponent % 2 == 1 {
                result = &result * &base;
            }
            exponent >>= 1;
            base = &base * &base;
        }
        result
    }

    fn lshift(&mut self) {
        if self.value[self.value.len() - 1] & (1u64 << 62) != 0 {
            self.value.push(0);
        }
        for i in (0..self.value.len()).rev() {
            self.value[i] <<= 1;
            if i != 0 {
                self.value[i] |= self.value[i - 1] >> 63;
            }
        }
    }

    fn rshift(&mut self) {
        for i in 0..self.value.len() {
            self.value[i] >>= 1;
            self.value[i] |= self.get(i + 1) << 63;
        }
    }

    fn divmod(&self, other: &BigUint) -> (BigUint, BigUint) {
        if other.is_zero() {
            panic!("Can't divide by 0");
        }
        if other == &BigUint::from(1) {
            return (self.clone(), BigUint::from(0));
        }
        if self.is_zero() {
            return (BigUint::from(0), BigUint::from(0));
        }
        if self < other {
            return (BigUint::from(0), self.clone());
        }
        if self == other {
            return (BigUint::from(1), BigUint::from(0));
        }
        let mut remaining_dividend = self.clone();
        let mut quotient = BigUint::from(0);
        let mut step_size = BigUint::from(1);
        let mut step_size_times_other = &step_size * other;
        while &remaining_dividend >= other {
            while step_size_times_other < remaining_dividend {
                step_size.lshift();
                step_size_times_other.lshift();
            }
            while step_size_times_other > remaining_dividend {
                step_size.rshift();
                step_size_times_other.rshift();
            }
            remaining_dividend = &remaining_dividend - &step_size_times_other;
            quotient += &step_size;
        }
        (quotient, remaining_dividend)
    }

    /// computes self *= other
    fn mul_internal(&mut self, other: BigUint) {
        let self_clone = self.clone();
        self.value.clear();
        self.value.push(0);
        for i in 0..other.value.len() {
            self.add_assign_internal(&self_clone, other.get(i), i);
        }
    }
}

impl Add for BigUint {
    type Output = BigUint;

    fn add(mut self, other: BigUint) -> BigUint {
        self += &other;
        self
    }
}

impl Sub for &BigUint {
    type Output = BigUint;

    fn sub(self, other: &BigUint) -> BigUint {
        if self < other {
            panic!("Number would be less than 0");
        }
        if self == other {
            return BigUint::from(0);
        }
        let mut carry = 0; // 0 or 1
        let mut res = vec![];
        for i in 0..max(self.value.len(), other.value.len()) {
            let a = self.get(i);
            let b = other.get(i);
            if a >= b + carry {
                res.push(a - b - carry);
                carry = 0;
            } else {
                res.push((a as u128 + ((1 as u128) << 64) - b as u128 - carry as u128) as u64);
                carry = 1;
            }
        }
        assert_eq!(carry, 0);
        BigUint { value: res }
    }
}

impl Sub for BigUint {
    type Output = BigUint;

    fn sub(self, other: BigUint) -> BigUint {
        &self - &other
    }
}

impl Mul for &BigUint {
    type Output = BigUint;

    fn mul(self, other: &BigUint) -> BigUint {
        let mut res = self.clone();
        res.mul_internal(other.clone());
        res
    }
}

impl Mul for BigUint {
    type Output = BigUint;

    fn mul(mut self, other: BigUint) -> BigUint {
        self.mul_internal(other);
        self
    }
}

impl Div for BigUint {
    type Output = BigUint;

    fn div(self, other: BigUint) -> BigUint {
        self.divmod(&other).0
    }
}

impl Div for &BigUint {
    type Output = BigUint;

    fn div(self, other: &BigUint) -> BigUint {
        self.divmod(other).0
    }
}

impl Rem for BigUint {
    type Output = BigUint;

    fn rem(self, other: BigUint) -> BigUint {
        self.divmod(&other).1
    }
}

impl Rem for &BigUint {
    type Output = BigUint;

    fn rem(self, other: &BigUint) -> BigUint {
        self.divmod(other).1
    }
}

impl Display for BigUint {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use std::convert::TryFrom;

        if self.is_zero() {
            write!(f, "0")?;
            return Ok(());
        }

        let mut num = self.clone();
        if num.value.len() == 1 {
            write!(f, "{}", num.value[0])?;
        } else {
            let mut output = String::new();
            while !num.is_zero() {
                let divmod_res = num.divmod(&BigUint::from(10));
                output.insert(0, char::try_from(divmod_res.1.value[0] as u8 + 48).unwrap());
                num = divmod_res.0;
            }
            write!(f, "{}", output,)?;
        }
        Ok(())
    }
}

impl Debug for BigUint {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self)
    }
}

impl BigUint {
    pub fn gcd(mut a: BigUint, mut b: BigUint) -> BigUint {
        while b >= 1.into() {
            let r = a.clone() % b.clone();
            a = b;
            b = r;
        }

        a
    }

    pub fn lcm(a: BigUint, b: BigUint) -> BigUint {
        a.clone() * b.clone() / BigUint::gcd(a, b)
    }

    pub fn pow(a: BigUint, b: BigUint) -> Result<BigUint, String> {
        if a.is_zero() && b.is_zero() {
            return Err("Zero to the power of zero is undefined".to_string());
        }
        if b.is_zero() {
            return Ok(BigUint::from(1));
        }
        let b_as_u64 = b.value;
        if b_as_u64.len() > 1 {
            return Err("Exponent too large".to_string());
        }
        Ok(a.pow_internal(b_as_u64[0]))
    }
}

#[cfg(test)]
mod tests {
    use super::BigUint;

    #[test]
    fn test_cmp() {
        assert_eq!(BigUint::from(0), BigUint::from(0));
        assert!(BigUint::from(0) < BigUint::from(1));
        assert!(BigUint::from(100) > BigUint::from(1));
        assert!(BigUint::from(10000000) > BigUint::from(1));
        assert!(BigUint::from(10000000) > BigUint::from(9999999));
    }

    #[test]
    fn test_addition() {
        assert_eq!(BigUint::from(2) + BigUint::from(2), BigUint::from(4));
        assert_eq!(BigUint::from(5) + BigUint::from(3), BigUint::from(8));
        assert_eq!(
            BigUint { value: vec![0] }
                + BigUint {
                    value: vec![0, 9223372036854775808, 0]
                },
            BigUint {
                value: vec![0, 9223372036854775808, 0]
            }
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(BigUint::from(5) - BigUint::from(3), BigUint::from(2));
        assert_eq!(BigUint::from(0) - BigUint::from(0), BigUint::from(0));
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(BigUint::from(20) * BigUint::from(3), BigUint::from(60));
    }

    #[test]
    fn test_rem() {
        assert_eq!(BigUint::from(20) % BigUint::from(3), BigUint::from(2));
        assert_eq!(BigUint::from(21) % BigUint::from(3), BigUint::from(0));
        assert_eq!(BigUint::from(22) % BigUint::from(3), BigUint::from(1));
        assert_eq!(BigUint::from(23) % BigUint::from(3), BigUint::from(2));
        assert_eq!(BigUint::from(24) % BigUint::from(3), BigUint::from(0));
    }

    #[test]
    fn test_lshift() {
        let mut n = BigUint::from(1);
        for _ in 0..100 {
            n.lshift();
            eprintln!("{:?}", &n);
            assert_eq!(n.value[0] & 1, 0);
        }
    }

    #[test]
    fn test_gcd() {
        assert_eq!(BigUint::gcd(2.into(), 4.into()), 2.into());
        assert_eq!(BigUint::gcd(4.into(), 2.into()), 2.into());
        assert_eq!(BigUint::gcd(37.into(), 43.into()), 1.into());
        assert_eq!(BigUint::gcd(43.into(), 37.into()), 1.into());
        assert_eq!(BigUint::gcd(215.into(), 86.into()), 43.into());
        assert_eq!(BigUint::gcd(86.into(), 215.into()), 43.into());
    }

    #[test]
    fn test_add_assign_internal() {
        // 0 += (1 * 1) << (64 * 1)
        let mut x = BigUint::from(0);
        x.add_assign_internal(&BigUint::from(1), 1, 1);
        assert_eq!(x, BigUint { value: vec![0, 1] });
    }

    #[test]
    fn test_big_multiplication() {
        assert_eq!(
            BigUint::from(1) * BigUint { value: vec![0, 1] },
            BigUint { value: vec![0, 1] }
        );
    }
}
