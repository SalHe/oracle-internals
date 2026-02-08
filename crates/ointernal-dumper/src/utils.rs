use std::fmt::Display;

pub struct Hex<'a> {
    grouped: Option<usize>,
    v: &'a [u8],
    auto_break_line: bool,
}

impl Display for Hex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let break_per = if !self.auto_break_line {
            None
        } else if self.v.len() > 25 {
            Some(25)
        } else {
            None
        };

        if break_per.is_some() {
            writeln!(f, "")?;
            write!(f, " ")?;
        }

        for (i, b) in self.v.iter().enumerate() {
            write!(f, "{:02x}", b)?;

            if let Some(break_per) = break_per
                && (i + 1) % break_per == 0
                && i + 1 != self.v.len()
            {
                writeln!(f, "")?;
            }

            if let Some(group) = self.grouped
                && (i + 1) % group == 0
                && i + 1 != self.v.len()
            {
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
            auto_break_line: false,
        }
    }

    pub fn sequence(v: &'a [u8]) -> Self {
        Self {
            grouped: None,
            v,
            auto_break_line: false,
        }
    }

    pub fn grouped(v: &'a [u8], group: usize) -> Self {
        Self {
            grouped: Some(group),
            v,
            auto_break_line: false,
        }
    }

    pub fn auto_break_line(mut self) -> Self {
        self.auto_break_line = true;
        self
    }
}
