/// All intervals are half-open. u128 allows an exclusive end at 2^64.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Interval {
    pub start: u128,
    pub end: u128,
}

impl Interval {
    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

pub(super) fn fits(value: u64, width: u32) -> bool {
    width >= 64 || (width > 0 && value < (1u64 << width)) || (width == 0 && value == 0)
}

pub(super) fn memory(address: u64, size: u32, unit: u32) -> Option<Interval> {
    if size == 0 || unit == 0 {
        return None;
    }
    let units = u128::from(size).div_ceil(u128::from(unit));
    let end = u128::from(address).checked_add(units)?;
    (end <= u128::from(u64::MAX) + 1).then_some(Interval {
        start: u128::from(address),
        end,
    })
}

pub(super) fn block(address: u64, size: u32) -> Option<Interval> {
    if size == 0 {
        return None;
    }
    let end = u128::from(address).checked_add(u128::from(size))?;
    (end <= u128::from(u64::MAX) + 1).then_some(Interval {
        start: u128::from(address),
        end,
    })
}

pub(super) fn field(offset: u32, width: u32, register_size: u32) -> Option<Interval> {
    if width == 0 {
        return None;
    }
    let end = u128::from(offset).checked_add(u128::from(width))?;
    (end <= u128::from(register_size)).then_some(Interval {
        start: u128::from(offset),
        end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extreme_widths_and_exclusive_end_are_safe() {
        assert!(fits(u64::MAX, 64));
        assert!(fits(u64::MAX, u32::MAX));
        assert!(!fits(u64::MAX, 63));
        assert_eq!(memory(u64::MAX, 8, 8).unwrap().end, 1u128 << 64);
        assert!(memory(u64::MAX, 16, 8).is_none());
        assert!(memory(0, u32::MAX, u32::MAX).is_some());
        assert!(memory(0, 1, 0).is_none());
        assert!(field(u32::MAX, u32::MAX, u32::MAX).is_none());
    }
}
