use std::fmt::{self, Display, Formatter};

#[repr(transparent)]
#[derive(Debug)]
pub struct BytesFmt(pub usize);

impl Display for BytesFmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let n = self.0;
        let (n, dec, unit) = if n < 1000 {
            (n, None, "b")
        } else if n < 1_000_000 {
            (n / 1000, Some(n % 1000 / 10), "kb")
        } else if n < 1_000_000_000 {
            (n / 1_000_000, Some(n % 1_000_000 / 10_000), "Mb")
        } else if n < 1_000_000_000_000 {
            (
                n / 1_000_000_000,
                Some(n % 1_000_000_000 / 10_000_000),
                "Gb",
            )
        } else {
            (
                n / 1_000_000_000_000,
                Some(n % 1_000_000_000_000 / 10_000_000_000),
                "Tb",
            )
        };

        match dec {
            Some(dec) => write!(f, "{}.{:02}{}", n, dec, unit),
            None => write!(f, "{}{}", n, unit),
        }
    }
}
