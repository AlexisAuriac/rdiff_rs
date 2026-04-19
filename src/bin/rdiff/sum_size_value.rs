use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SumSizeValue {
    Min,
    Max,
    N(u32),
}

impl FromStr for SumSizeValue {
    type Err = String;

    // for compatibility with original rdiff: -1 == min, 0 == max
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "min" => return Ok(SumSizeValue::Min),
            "max" => return Ok(SumSizeValue::Max),
            _ => (),
        }

        let n: i64 = s.parse().map_err(|e| format!("{e}"))?;
        match n {
            -1 => Ok(SumSizeValue::Min),
            0 => Ok(SumSizeValue::Max),
            n if n > u32::MAX as i64 => Err("sum size is too big".to_string()),
            n if n < -1 => Err("sum size is too small".to_string()),
            n => Ok(SumSizeValue::N(n.try_into().expect("valid u32 value"))),
        }
    }
}

impl Display for SumSizeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SumSizeValue::Min => write!(f, "min"),
            SumSizeValue::Max => write!(f, "max"),
            SumSizeValue::N(n) => write!(f, "{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generic_ok_test(input: &str, expect: SumSizeValue) {
        match input.parse::<SumSizeValue>() {
            Ok(val) if val == expect => (),
            Ok(val) => panic!("for input='{input}' expected {expect}, got {val}"),
            Err(e) => panic!("did not expect error for input='{input}', got '{e}'"),
        }
    }

    #[test]
    fn min_str() {
        generic_ok_test("min", SumSizeValue::Min);
    }

    #[test]
    fn min_str_mixed_case() {
        generic_ok_test("mIn", SumSizeValue::Min);
    }

    #[test]
    fn max_str() {
        generic_ok_test("max", SumSizeValue::Max);
    }

    #[test]
    fn min_as_nbr() {
        generic_ok_test("-1", SumSizeValue::Min);
    }

    #[test]
    fn max_as_nbr() {
        generic_ok_test("0", SumSizeValue::Max);
    }

    #[test]
    fn nbr() {
        generic_ok_test("24", SumSizeValue::N(24));
    }

    fn generic_err_test(input: &str) {
        if let Ok(val) = input.parse::<SumSizeValue>() {
            panic!("for input='{input}' expected error, got {val}")
        }
    }

    #[test]
    fn other_str() {
        generic_err_test("large");
    }

    #[test]
    fn nbr_too_small() {
        generic_err_test("-2");
    }

    #[test]
    fn nbr_too_large() {
        let large = u32::MAX as u64 + 1;
        generic_err_test(large.to_string().as_str());
    }
}
