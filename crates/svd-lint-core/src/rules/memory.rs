use super::{CheckContext, alternatives::Layout, ranges};
use crate::diagnostic_definition::{DiagnosticDefinition, RuleCategory};
use crate::{
    Diagnostic, Severity,
    ir::{CanonicalAddressBlock, CanonicalPeripheral},
};
use std::collections::BTreeMap;

pub(crate) static REGISTER_OVERLAP: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM008"),
    name: "overlapping-registers",
    description: "Register address ranges overlap without confirmed alternatives.",
    category: RuleCategory::Addressing,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["heuristic"],
};

pub(crate) static REGISTER_OUTSIDE_BLOCK: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM010"),
    name: "register-outside-block",
    description: "A register must fit wholly within a registers addressBlock.",
    category: RuleCategory::Addressing,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

pub(crate) static INVALID_ADDRESS_BLOCK: DiagnosticDefinition = DiagnosticDefinition {
    code: crate::DiagnosticCode::new("SEM011"),
    name: "invalid-address-block",
    description: "An addressBlock size or address boundary is invalid.",
    category: RuleCategory::Addressing,
    default_severity: Severity::Error,
    configurable: true,
    tags: &["check"],
};

struct BlockRange<'a> {
    interval: ranges::Interval,
    block: &'a CanonicalAddressBlock,
}

/// Invalid address blocks are excluded, but their declaration still suppresses
/// misleading containment failures when no valid registers block remains.
struct BlockRanges<'a> {
    valid: Vec<BlockRange<'a>>,
    declared: bool,
}

struct RegisterRange {
    interval: ranges::Interval,
    index: usize,
}

pub(super) fn check_memory(context: &CheckContext<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let unit = context.device.address_unit_bits;
    if unit == 0 {
        diagnostics.push(context.diagnostic(
            &INVALID_ADDRESS_BLOCK,
            Severity::Error,
            "addressUnitBits must be positive to compute memory ranges",
            &context.device.origin,
            "addressUnitBits",
        ));
    }
    for peripheral in &context.device.peripherals {
        let (blocks, block_diagnostics) = check_block_bounds(context, peripheral);
        diagnostics.extend(block_diagnostics);
        if unit == 0 {
            continue;
        }
        let layout = Layout::new(peripheral, unit);
        let (registers, register_diagnostics) = check_register_bounds(context, &layout);
        diagnostics.extend(register_diagnostics);
        diagnostics.extend(check_containment(context, &layout, &blocks, &registers));
        diagnostics.extend(check_overlaps(context, &layout, &registers));
    }
    diagnostics
}

fn check_block_bounds<'a>(
    context: &CheckContext<'_>,
    peripheral: &'a CanonicalPeripheral,
) -> (BlockRanges<'a>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut blocks = BlockRanges {
        valid: Vec::new(),
        declared: false,
    };
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
                &INVALID_ADDRESS_BLOCK,
                Severity::Error,
                reason,
                &block.origin,
                "size",
            ));
        }
        if block.usage == crate::svd::AddressBlockUsage::Registers {
            blocks.declared = true;
            if !invalid {
                blocks.valid.push(BlockRange {
                    interval: interval.unwrap(),
                    block,
                });
            }
        }
    }
    (blocks, diagnostics)
}

fn check_register_bounds(
    context: &CheckContext<'_>,
    layout: &Layout<'_>,
) -> (Vec<RegisterRange>, Vec<Diagnostic>) {
    let mut registers = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, register) in layout.peripheral.registers.iter().enumerate() {
        let Some(size) = register.properties.size.as_ref().filter(|s| s.value > 0) else {
            continue;
        };
        if let Some(interval) = layout.intervals[index] {
            registers.push(RegisterRange { interval, index });
        } else {
            diagnostics.push(context.diagnostic(
                &REGISTER_OVERLAP,
                Severity::Error,
                format!(
                    "register at {:#x} with {} bits extends beyond the u64 address space",
                    register.address, size.value
                ),
                &register.origin,
                "addressOffset",
            ));
        }
    }
    (registers, diagnostics)
}

fn check_containment(
    context: &CheckContext<'_>,
    layout: &Layout<'_>,
    blocks: &BlockRanges<'_>,
    registers: &[RegisterRange],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !blocks.declared || blocks.valid.is_empty() {
        return diagnostics;
    }
    for range in registers {
        if blocks
            .valid
            .iter()
            .any(|block| block.interval.contains(range.interval))
        {
            continue;
        }
        let register = &layout.peripheral.registers[range.index];
        let mut d = context.diagnostic(
            &REGISTER_OUTSIDE_BLOCK,
            Severity::Error,
            format!(
                "register range [{:#x}, {:#x}) is outside every registers addressBlock",
                range.interval.start, range.interval.end
            ),
            &register.origin,
            "addressOffset",
        );
        for block in &blocks.valid {
            context.related(
                &mut d,
                &block.block.origin,
                "addressBlock",
                "registers addressBlock",
            );
        }
        diagnostics.push(d);
    }
    diagnostics
}

fn check_overlaps(
    context: &CheckContext<'_>,
    layout: &Layout<'_>,
    registers: &[RegisterRange],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // Keep one representative for source-identical array aliases, retaining
    // distinct physical conflicts and explicit alternative views.
    let mut unique: BTreeMap<_, (usize, bool)> = BTreeMap::new();
    let mut intervals = Vec::new();
    for range in registers {
        let register = &layout.peripheral.registers[range.index];
        let key = (
            range.interval,
            register.origin.declaration.map(|n| n.0),
            register.alternate_group.clone(),
        );
        if let Some((previous, reported)) = unique.get_mut(&key) {
            if !*reported {
                diagnostics.extend(check_pair(context, layout, *previous, range.index));
                *reported = true;
            }
        } else {
            unique.insert(key, (range.index, false));
            intervals.push((range.interval, range.index));
        }
    }
    intervals.sort_by_key(|&(range, index)| (range.start, range.end, index));
    for (i, &(range, left)) in intervals.iter().enumerate() {
        for &(other, right) in &intervals[i + 1..] {
            if other.start >= range.end {
                break;
            }
            if range.overlaps(other) {
                diagnostics.extend(check_pair(context, layout, left, right));
            }
        }
    }
    diagnostics
}

fn check_pair(
    context: &CheckContext<'_>,
    layout: &Layout<'_>,
    a: usize,
    b: usize,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if layout.permitted(a, b) {
        return diagnostics;
    }
    let left = &layout.peripheral.registers[a];
    let right = &layout.peripheral.registers[b];
    let (Some(x), Some(y)) = (layout.intervals[a], layout.intervals[b]) else {
        return diagnostics;
    };
    let severity = if layout.uncertain(a, b) {
        Severity::Warning
    } else {
        Severity::Error
    };
    let mut d = context.diagnostic(
        &REGISTER_OVERLAP,
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

    diagnostics
}
