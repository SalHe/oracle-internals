use std::fmt::Display;

pub struct Hex<'a> {
    grouped: Option<usize>,
    v: &'a [u8],
}

impl Display for Hex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, b) in self.v.iter().enumerate() {
            write!(f, "{:02x}", b)?;
            if let Some(group) = self.grouped
                && (i + 1) % group == 0 && i + 1 != self.v.len() {
                    write!(f, " ")?;
                }
        }
        Ok(())
    }
}

impl<'a> Hex<'a> {
    pub fn spaced(v: &'a [u8]) -> Self {
        Self {
            grouped: Some(1),
            v,
        }
    }

    pub fn sequence(v: &'a [u8]) -> Self {
        Self { grouped: None, v }
    }

    pub fn grouped(v: &'a [u8], group: usize) -> Self {
        Self {
            grouped: Some(group),
            v,
        }
    }
}
