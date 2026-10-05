use super::{CheckContext, Rule, alternatives::Layout, ranges};
use crate::{Diagnostic, DiagnosticCode as Code, Severity};
use std::collections::BTreeMap;

pub(super) struct Memory;
impl Rule for Memory {
    fn id(&self) -> &'static str {
        "memory"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        let unit = context.device.address_unit_bits;
        if unit == 0 {
            diagnostics.push(context.diagnostic(
                Code::SemInvalidAddressBlock,
                Severity::Error,
                "addressUnitBits must be positive to compute memory ranges",
                &context.device.origin,
                "addressUnitBits",
            ));
        }
        for peripheral in &context.device.peripherals {
            let mut valid_blocks = Vec::new();
            let mut declared_register_blocks = false;
            for block in &peripheral.address_blocks {
                let interval = ranges::block(block.address, block.size);
                let expected = peripheral.base_address.checked_add(u64::from(block.offset));
                let invalid = interval.is_none() || expected != Some(block.address);
                if invalid {
                    let reason = if block.size == 0 {
                        "addressBlock size must be greater than zero"
                    } else if interval.is_none() {
                        "addressBlock extends beyond the u64 address space"
                    } else {
                        "addressBlock address does not match peripheral baseAddress plus offset"
                    };
                    diagnostics.push(context.diagnostic(
                        Code::SemInvalidAddressBlock,
                        Severity::Error,
                        reason,
                        &block.origin,
                        "size",
                    ));
                }
                if block.usage == crate::svd::AddressBlockUsage::Registers {
                    declared_register_blocks = true;
                    if !invalid {
                        valid_blocks.push((interval.unwrap(), block));
                    }
                }
            }
            if unit == 0 {
                continue;
            }
            let layout = Layout::new(peripheral, unit);
            // Collapse identical physical ranges from the same array declaration
            // before enumerating conflicts. One representative pair suffices for
            // an array whose increment causes every element to alias.
            let mut unique: BTreeMap<_, (usize, bool)> = BTreeMap::new();
            let mut intervals = Vec::new();
            for (i, r) in peripheral.registers.iter().enumerate() {
                let Some(size) = r.properties.size.as_ref().filter(|s| s.value > 0) else {
                    continue;
                };
                let Some(interval) = layout.intervals[i] else {
                    diagnostics.push(context.diagnostic(
                        Code::SemRegisterOverlap,
                        Severity::Error,
                        format!(
                            "register at {:#x} with {} bits extends beyond the u64 address space",
                            r.address, size.value
                        ),
                        &r.origin,
                        "addressOffset",
                    ));
                    continue;
                };
                // If all registers blocks are invalid, SEM011 explains the cause;
                // do not add a containment error for every register.
                if declared_register_blocks
                    && !valid_blocks.is_empty()
                    && !valid_blocks.iter().any(|(b, _)| b.contains(interval))
                {
                    let mut d = context.diagnostic(
                        Code::SemRegisterOutsideBlock,
                        Severity::Error,
                        format!(
                            "register range [{:#x}, {:#x}) is outside every registers addressBlock",
                            interval.start, interval.end
                        ),
                        &r.origin,
                        "addressOffset",
                    );
                    for (_, block) in &valid_blocks {
                        context.related(
                            &mut d,
                            &block.origin,
                            "addressBlock",
                            "registers addressBlock",
                        );
                    }
                    diagnostics.push(d);
                }
                let key = (
                    interval,
                    r.origin.declaration.map(|n| n.0),
                    r.alternate_group.clone(),
                );
                if let Some((previous, reported)) = unique.get_mut(&key) {
                    if !*reported {
                        overlap(context, &layout, *previous, i, diagnostics);
                        *reported = true;
                    }
                } else {
                    unique.insert(key, (i, false));
                    intervals.push((interval, i));
                }
            }
            intervals.sort_by_key(|&(range, index)| (range.start, range.end, index));
            for (i, &(range, left)) in intervals.iter().enumerate() {
                for &(other, right) in &intervals[i + 1..] {
                    if other.start >= range.end {
                        break;
                    }
                    if range.overlaps(other) {
                        overlap(context, &layout, left, right, diagnostics);
                    }
                }
            }
        }
    }
}

fn overlap(
    context: &CheckContext<'_>,
    layout: &Layout<'_>,
    a: usize,
    b: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if layout.permitted(a, b) {
        return;
    }
    let left = &layout.peripheral.registers[a];
    let right = &layout.peripheral.registers[b];
    let (Some(x), Some(y)) = (layout.intervals[a], layout.intervals[b]) else {
        return;
    };
    let severity = if layout.uncertain(a, b) {
        Severity::Warning
    } else {
        Severity::Error
    };
    let mut d = context.diagnostic(
        Code::SemRegisterOverlap,
        severity,
        format!(
            "register ranges [{:#x}, {:#x}) and [{:#x}, {:#x}) overlap",
            x.start, x.end, y.start, y.end
        ),
        &right.origin,
        "addressOffset",
    );
    context.related(
        &mut d,
        &left.origin,
        "addressOffset",
        "overlapping register",
    );
    diagnostics.push(d);
}
