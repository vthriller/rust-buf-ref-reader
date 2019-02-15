# buf-ref-reader

Faster, growable buffering reader.

Use this crate to get faster reads in situations when all you need is immutable `&[u8]`s,
the contents of which rarely need to outlive a single loop cycle.

[See rustdoc](https://docs.rs/buf-ref-reader/) for examples and to read more about how this crate works, its applicability and limitations.

Currently this crate only works with rust-nightly.

## Acknowledgement

The idea for this crate came from experiments with [mawk](https://invisible-island.net/mawk/),
namely after applying [first Futamura projection](https://en.wikipedia.org/wiki/Partial_evaluation#Futamura_projections) to one of awk scripts,
which in turn was done to study mawk's exceptionally good performance.

See [source code for `FINgets()`](https://github.com/ThomasDickey/mawk-20140914/blob/1d2b180d760ddb9d967ff377d9fe21fd4eb9cda5/fin.c#L212) to learn how mawk buffers its input.

## License

[Apache License 2.0](https://spdx.org/licenses/Apache-2.0.html)
