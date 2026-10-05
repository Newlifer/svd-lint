use super::{CheckContext, Rule, ranges::fits};
use crate::{Diagnostic, DiagnosticCode as Code, Origin, Severity, ir::*, svd};

pub(super) struct Access;
impl Rule for Access {
    fn id(&self) -> &'static str {
        "access"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for peripheral in &context.device.peripherals {
            for register in &peripheral.registers {
                let constraint = register.metadata.write_constraint;
                if let (Some(constraint), Some(width)) = (
                    constraint,
                    register.properties.size.as_ref().filter(|s| s.value > 0),
                ) {
                    // Register metadata keeps effective derived values; follow the
                    // derivation chain for a declaration missing on the use node.
                    let origin = metadata_origin(context, &register.origin, "writeConstraint");
                    let write_enums = register
                        .fields
                        .iter()
                        .flat_map(|f| &f.enumerated_values)
                        .any(writable_enum);
                    check_constraint(
                        context,
                        constraint,
                        width.value,
                        &origin,
                        write_enums,
                        diagnostics,
                    );
                }
                check_operations(
                    context,
                    register.properties.access.as_ref(),
                    register.metadata.read_action,
                    register.metadata.modified_write_values,
                    &metadata_origin(context, &register.origin, "readAction"),
                    &metadata_origin(context, &register.origin, "modifiedWriteValues"),
                    diagnostics,
                );
                for field in &register.fields {
                    if let Some(constraint) = &field.write_constraint {
                        if field.bit_width > 0 {
                            check_constraint(
                                context,
                                constraint.value,
                                field.bit_width,
                                &constraint.origin,
                                field.enumerated_values.iter().any(writable_enum),
                                diagnostics,
                            );
                        }
                    }
                    check_operations(
                        context,
                        field.access.as_ref(),
                        field.read_action.as_ref().map(|v| v.value),
                        field.modified_write_values.as_ref().map(|v| v.value),
                        &field
                            .read_action
                            .as_ref()
                            .map(|v| v.origin.clone())
                            .unwrap_or_else(|| field.origin.clone()),
                        &field
                            .modified_write_values
                            .as_ref()
                            .map(|v| v.origin.clone())
                            .unwrap_or_else(|| field.origin.clone()),
                        diagnostics,
                    );
                }
            }
        }
    }
}

fn metadata_origin(context: &CheckContext<'_>, origin: &Origin, tag: &str) -> Origin {
    let mut result = origin.clone();
    for node in origin
        .declaration
        .into_iter()
        .chain(origin.derived_from.iter().rev().copied())
    {
        if let Some(property) = context.source_map.children_named(node, tag).next() {
            result.declaration = Some(property);
            break;
        }
    }
    result
}

fn check_constraint(
    context: &CheckContext<'_>,
    constraint: svd::WriteConstraint,
    width: u32,
    origin: &Origin,
    write_enums: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let reason = match constraint {
        svd::WriteConstraint::Range(range) if range.min > range.max => {
            Some("writeConstraint minimum exceeds maximum".to_owned())
        }
        svd::WriteConstraint::Range(range)
            if !fits(range.min, width) || !fits(range.max, width) =>
        {
            Some(format!(
                "writeConstraint range [{}, {}] does not fit in {width} bits",
                range.min, range.max
            ))
        }
        svd::WriteConstraint::UseEnumeratedValues(true) if !write_enums => Some(
            "useEnumeratedValues requires an enumeratedValues set applicable to writes".to_owned(),
        ),
        _ => None,
    };
    if let Some(reason) = reason {
        diagnostics.push(context.diagnostic(
            Code::SemInvalidWriteConstraint,
            Severity::Error,
            reason,
            origin,
            "writeConstraint",
        ));
    }
}

fn writable_enum(e: &CanonicalEnumeratedValues) -> bool {
    e.values
        .value
        .iter()
        .any(|v| v.value.is_some() || v.is_default == Some(true))
        && matches!(
            e.usage.as_ref().map(|v| v.value),
            None | Some(svd::Usage::Write | svd::Usage::ReadWrite)
        )
}

fn check_operations(
    context: &CheckContext<'_>,
    access: Option<&crate::Resolved<svd::Access>>,
    read: Option<svd::ReadAction>,
    write: Option<svd::ModifiedWriteValues>,
    read_origin: &Origin,
    write_origin: &Origin,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(access) = access else { return };
    let (readable, writable) = match access.value {
        svd::Access::ReadOnly => (true, false),
        svd::Access::WriteOnly | svd::Access::WriteOnce => (false, true),
        svd::Access::ReadWrite | svd::Access::ReadWriteOnce => (true, true),
    };
    for (contradiction, origin, tag, reason) in [
        (
            !readable && read.is_some(),
            read_origin,
            "readAction",
            "readAction is declared on a write-only object",
        ),
        (
            !writable && write.is_some_and(|v| v != svd::ModifiedWriteValues::Modify),
            write_origin,
            "modifiedWriteValues",
            "modifiedWriteValues is declared on a read-only object",
        ),
    ] {
        if contradiction {
            let mut d = context.diagnostic(
                Code::SemAccessConflict,
                Severity::Warning,
                reason,
                origin,
                tag,
            );
            context.related(
                &mut d,
                &access.origin,
                "access",
                "effective access declared here",
            );
            diagnostics.push(d);
        }
    }
}
