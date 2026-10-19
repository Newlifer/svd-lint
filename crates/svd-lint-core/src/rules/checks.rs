//! Built-in checks in their execution order.
use super::{CheckContext, access, alternatives, enums, fields, memory, registers, reset};
use crate::Diagnostic;

pub(super) type CheckFn = fn(&CheckContext<'_>) -> Vec<Diagnostic>;

pub(super) const CHECKS: &[CheckFn] = &[
    registers::check_registers,
    fields::check_fields,
    reset::check_reset,
    memory::check_memory,
    alternatives::check_alternatives,
    enums::check_enums,
    access::check_access,
];
