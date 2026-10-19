//! Built-in checks in their execution order.
use super::{CheckContext, access, alternatives, enums, fields, memory, registers, reset};
use crate::Diagnostic;

pub(super) type CheckFn = fn(&CheckContext<'_>) -> Vec<Diagnostic>;

pub(super) const CHECKS: &[CheckFn] = &[
    registers::check_registers,
    fields::check_fields,
    reset::check_reset,
    memory::check_register_overlaps,
    alternatives::check_alternatives,
    memory::check_register_blocks,
    memory::check_address_blocks,
    enums::check_enums,
    access::check_access,
];
