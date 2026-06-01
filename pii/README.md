> [!NOTE]
> This is work in progress and is provided for reference.

PII scrubber for PostHog rrweb session-recording JSONL. Monitors files in
`in/`, emits a scrubbed copy to `out/` with the same filenames. Supports gzip compressed lines.

```
cargo run --release -- in out
```

## Scrubbing implementation

The rules cascade top-down — the first matching rule wins:

1. **All numbers get redacted.**
2. **All inline images get blurred.**
3. **All loaded images get replaced with placeholders.**
4. **All text longer than `max_words_len` gets redacted** → long strings are
   presumed a PII-risk and force-redacted wholesale.
5. **Everything else** → gated through a strict allowlist, with regex scrubbing
   layered on top of survivors (todo).

## Pipeline shape

Three stages connected by bounded crossbeam channels:

```
FsAdapter ──► reader workers ──► processor workers ──► writer workers ──► FsAdapter
              raw bytes →         scrub →               serialized JSONL →
```

- **Reader** slices each input file into ~512 KB chunks aligned to newlines so a
  chunk is always a complete prefix of full JSONL lines.
- **Processor** runs N-1 workers (one per core, less the writer). Each holds a
  reusable `Scratch` (decompression buffer, emit buffer, simd-json string pool,
  `MutationSubScratch`). Each line is dispatched on `(EventType, IncrementalSource)`.
- **Writer** collates chunks back into file order and pushes to disk through a
  buffered writer.

## rrweb event dispatch

| Event                                                                       | Action                                      |
| --------------------------------------------------------------------------- | ------------------------------------------- |
| MouseMove/MouseInteraction/Scroll/ViewportResize/TouchMove/MediaInteraction | passthrough                                 |
| StyleSheetRule/StyleDeclaration/AdoptedStyleSheet                           | passthrough                                 |
| Font/Log/Drag/Selection/CustomElement/CanvasMutation                        | passthrough                                 |
| DomContentLoaded / Load                                                     | passthrough                                 |
| Meta                                                                        | URL-scrub the `href`                        |
| FullSnapshot                                                                | DOM walk                                    |
| IncrementalSnapshot { source: Mutation }                                    | mutation scrub                              |
| IncrementalSnapshot { source: Input }                                       | text-scrub the `text`                       |
| Custom                                                                      | generic value walk                          |
| Plugin (`rrweb/network@1`)                                                  | URL-scrub `name`, text-scrub bodies/headers |
| Plugin (`rrweb/console@1`)                                                  | text-scrub each entry of `payload`/`trace`  |
| Plugin (other)                                                              | generic value walk                          |
| Unknown                                                                     | passthrough                                 |

## Config

`config::Config::default()` picks reasonable values from `available_parallelism`:

| Field                 | Default           | Notes                           |
| --------------------- | ----------------- | ------------------------------- |
| `reader_workers`      | `max(2, cores/8)` |                                 |
| `processor_workers`   | `max(2, cores-1)` | the hot loop                    |
| `writer_workers`      | `max(2, cores/8)` |                                 |
| `chunk_size_cap`      | 512 KB            | per processor chunk             |
| `reader_buf_cap`      | 1 MB              | `BufReader`                     |
| `writer_buf_cap`      | 1 MB              | `BufWriter`                     |
| `max_words_len`       | 8                 | force-redact threshold for text |
| `metrics_interval_ms` | 1000              |                                 |

`REDACT_CHAR = '*'` and `NUMBER_CHAR = '#'` are compile-time constants.
