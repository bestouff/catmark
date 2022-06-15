use std::fmt::Display;

use newtype_ops::newtype_ops;

type InnerCoord = u16;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct XY(InnerCoord);

impl XY {
    pub const fn new(n: InnerCoord) -> Self {
        XY(n)
    }
}

impl From<InnerCoord> for XY {
    fn from(n: InnerCoord) -> Self {
        XY(n)
    }
}

impl TryFrom<usize> for XY {
    type Error = std::num::TryFromIntError;
    fn try_from(n: usize) -> Result<Self, Self::Error> {
        Ok(XY(n.try_into()?))
    }
}

impl Into<usize> for XY {
    fn into(self) -> usize {
        self.0.into()
    }
}

impl Display for XY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

newtype_ops! { [XY] {add sub mul div rem bitand bitor bitxor not} {:=} {^&}Self {^&}{Self InnerCoord} }
