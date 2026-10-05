use super::{CheckContext, Rule, ranges};
use crate::{Diagnostic, DiagnosticCode as Code, Severity};

pub(super) struct Fields;
impl Rule for Fields {
    fn id(&self) -> &'static str {
        "fields"
    }
    fn check(&self, context: &CheckContext<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for peripheral in &context.device.peripherals {
            for register in &peripheral.registers {
                let size = register.properties.size.as_ref();
                let mut intervals = Vec::new();
                for field in &register.fields {
                    if field.bit_width == 0 {
                        diagnostics.push(context.diagnostic(
                            Code::SemZeroFieldWidth,
                            Severity::Error,
                            "field width must be greater than zero",
                            &field.origin,
                            "bitWidth",
                        ));
                        continue;
                    }
                    let Some(size) = size.filter(|s| s.value > 0) else {
                        continue;
                    };
                    if let Some(interval) =
                        ranges::field(field.bit_offset, field.bit_width, size.value)
                    {
                        intervals.push((interval, field));
                    } else {
                        let mut d = context.diagnostic(
                            Code::SemFieldOutOfBounds,
                            Severity::Error,
                            format!(
                                "field interval [{}, {}) exceeds register size {}",
                                field.bit_offset,
                                u64::from(field.bit_offset) + u64::from(field.bit_width),
                                size.value
                            ),
                            &field.origin,
                            "bitOffset",
                        );
                        context.related(
                            &mut d,
                            &size.origin,
                            "size",
                            "register size declared here",
                        );
                        diagnostics.push(d);
                    }
                }
                intervals.sort_by_key(|(range, field)| (range.start, range.end, &field.name));
                for (i, &(range, field)) in intervals.iter().enumerate() {
                    for &(other, following) in &intervals[i + 1..] {
                        if other.start >= range.end {
                            break;
                        }
                        if range.overlaps(other) {
                            let mut d = context.diagnostic(
                                Code::SemFieldOverlap,
                                Severity::Warning,
                                format!(
                                    "field intervals [{}, {}) and [{}, {}) overlap",
                                    range.start, range.end, other.start, other.end
                                ),
                                &following.origin,
                                "name",
                            );
                            context.related(&mut d, &field.origin, "name", "overlapping field");
                            diagnostics.push(d);
                        }
                    }
                }
            }
        }
    }
}
