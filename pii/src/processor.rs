use std::borrow::Cow;

use crate::context::Ctx;
use crate::schema::{
    self, CustomEventData, EventType, FullSnapshotData, IncrementalSource, InputData, MetaData,
    MutationData, PluginEventData,
};
use crate::scrub;

#[derive(Default)]
struct Scratch {
    data: Vec<u8>,
    payload: Vec<u8>,
    string: String,
    mutation_sub: schema::MutationSubScratch,
}

#[derive(Default)]
pub struct Worker {
    scratch: Scratch,
}

impl Worker {
    pub fn process_chunk(&mut self, ctx: &Ctx<'_>, raw: &[u8], offsets: &[usize]) -> Vec<u8> {
        let mut out = Vec::with_capacity(raw.len());
        for (i, &start) in offsets.iter().enumerate() {
            let end = offsets.get(i + 1).copied().unwrap_or(raw.len());
            process_line(ctx, &raw[start..end], &mut self.scratch, &mut out);
        }
        out
    }
}

fn process_line(ctx: &Ctx<'_>, line: &[u8], scratch: &mut Scratch, out: &mut Vec<u8>) {
    use EventType as E;
    use IncrementalSource as S;

    let scan = schema::scan_event(line);
    let ty = scan.ty.and_then(EventType::from_u8);
    let source = scan.source.and_then(IncrementalSource::from_u8);
    let compressed = scan.compressed;

    let result: anyhow::Result<()> = match (ty, source) {
        (
            Some(E::IncrementalSnapshot),
            Some(
                S::MouseMove
                | S::MouseInteraction
                | S::Scroll
                | S::ViewportResize
                | S::TouchMove
                | S::MediaInteraction
                | S::StyleSheetRule
                | S::StyleDeclaration
                | S::Font
                | S::Log
                | S::Drag
                | S::Selection
                | S::AdoptedStyleSheet
                | S::CustomElement
                | S::CanvasMutation,
            ),
        )
        | (Some(E::DomContentLoaded | E::Load), _) => {
            out.extend_from_slice(line);
            return;
        }

        (Some(E::FullSnapshot), _) => scrub_full_snapshot(ctx, line, compressed, scratch, out),
        (Some(E::IncrementalSnapshot), Some(S::Mutation)) => {
            scrub_mutation(ctx, line, compressed, scratch, out)
        }
        (Some(E::IncrementalSnapshot), Some(S::Input)) => scrub_input(ctx, line, scratch, out),
        (Some(E::Meta), _) => scrub_meta(ctx, line, scratch, out),
        (Some(E::Custom), _) => scrub_custom(ctx, line, scratch, out),
        (Some(E::Plugin), _) => scrub_plugin(ctx, line, scratch, out),

        _ => {
            out.extend_from_slice(line);
            return;
        }
    };

    if let Err(e) = result {
        eprintln!(
            "scrub failed (ty={:?}, src={:?}, cv={}): {e:#}; passing through",
            ty, source, compressed
        );
        out.extend_from_slice(line);
    }
}

fn scrub_meta(
    ctx: &Ctx<'_>,
    line: &[u8],
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = MetaData::read(line, &mut scratch.data)?;
    scratch.string.clear();
    if !scrub::url::scrub_into(ctx, &data.href, &mut scratch.string) {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.href = Cow::Owned(std::mem::take(&mut scratch.string));
    data.write(line, range, &mut scratch.payload, out)
}

fn scrub_input(
    ctx: &Ctx<'_>,
    line: &[u8],
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = InputData::read(line, &mut scratch.data)?;
    scratch.string.clear();
    if !scrub::text::scrub_into(ctx, &data.text, &mut scratch.string) {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.text = Cow::Owned(std::mem::take(&mut scratch.string));
    data.write(line, range, &mut scratch.payload, out)
}

fn scrub_full_snapshot(
    ctx: &Ctx<'_>,
    line: &[u8],
    compressed: bool,
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = FullSnapshotData::read(line, compressed, &mut scratch.data)?;
    if !scrub::dom::scrub_full_snapshot(ctx, &mut data) {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.write(line, range, compressed, &mut scratch.payload, out)
}

fn scrub_mutation(
    ctx: &Ctx<'_>,
    line: &[u8],
    compressed: bool,
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = MutationData::read(
        line,
        compressed,
        &mut scratch.data,
        &mut scratch.mutation_sub,
    )?;
    if !scrub::dom::scrub_mutation(ctx, &mut data) {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.write(line, range, compressed, &mut scratch.payload, out)
}

fn scrub_custom(
    ctx: &Ctx<'_>,
    line: &[u8],
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = CustomEventData::read(line, &mut scratch.data)?;
    if !scrub::value::scrub_value_generic(ctx, &mut data.payload) {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.write(line, range, &mut scratch.payload, out)
}

fn scrub_plugin(
    ctx: &Ctx<'_>,
    line: &[u8],
    scratch: &mut Scratch,
    out: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let (range, mut data) = PluginEventData::read(line, &mut scratch.data)?;
    let changed = match data.plugin.as_ref() {
        "rrweb/network@1" => scrub::value::scrub_network_plugin(ctx, &mut data.payload),
        "rrweb/console@1" => scrub::value::scrub_console_plugin(ctx, &mut data.payload),
        _ => scrub::value::scrub_value_generic(ctx, &mut data.payload),
    };
    if !changed {
        out.extend_from_slice(line);
        return Ok(());
    }
    data.write(line, range, &mut scratch.payload, out)
}

#[cfg(test)]
#[path = "processor_tests.rs"]
mod tests;
