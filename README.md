# buf-ref-reader

Faster, growable buffering reader.

Use this crate to get faster reads in situations when all you need is immutable `&[u8]`s,
the contents of which rarely need to outlive a single loop cycle.

[See rustdoc](https://docs.rs/buf-ref-reader/) for examples and to read more about how this crate works, its applicability and limitations.

## Benchmarks

Reading lines from `&[u8] as Read` through 64k-sized buffer into the void:

```
test throttled_bufref_read_until_mmap_64      ... bench:   9,013,420 ns/iter (+/- 1,165,996)
test throttled_bufref_read_until_vec_64       ... bench:   8,769,302 ns/iter (+/- 772,987)
test throttled_std_read_until_64              ... bench:  12,244,424 ns/iter (+/- 841,098)
```

Populating `HashMap` with up to 2-, 3-, 4-, 5-byte prefixes of entries from `/usr/share/dict/words`,
while only allocating memory for new map entries:

```
test baseline_hashmap_2    ... bench:   8,228,848 ns/iter (+/- 81,987)
test bufref_hashmap_mmap_2 ... bench:  11,042,118 ns/iter (+/- 62,980)
test bufref_hashmap_vec_2  ... bench:  10,613,380 ns/iter (+/- 45,901)
test std_hashmap_2         ... bench:  17,010,897 ns/iter (+/- 93,379)
```
```
test baseline_hashmap_3    ... bench:  10,426,822 ns/iter (+/- 119,331)
test bufref_hashmap_mmap_3 ... bench:  13,180,343 ns/iter (+/- 118,483)
test bufref_hashmap_vec_3  ... bench:  12,733,866 ns/iter (+/- 244,389)
test std_hashmap_3         ... bench:  19,470,906 ns/iter (+/- 106,113)
```
```
test baseline_hashmap_4    ... bench:  16,135,731 ns/iter (+/- 296,142)
test bufref_hashmap_mmap_4 ... bench:  18,887,024 ns/iter (+/- 267,410)
test bufref_hashmap_vec_4  ... bench:  18,375,449 ns/iter (+/- 292,666)
test std_hashmap_4         ... bench:  25,889,158 ns/iter (+/- 334,521)
```
```
test baseline_hashmap_5    ... bench:  26,379,467 ns/iter (+/- 806,691)
test bufref_hashmap_mmap_5 ... bench:  28,282,336 ns/iter (+/- 1,035,900)
test bufref_hashmap_vec_5  ... bench:  27,588,498 ns/iter (+/- 1,081,542)
test std_hashmap_5         ... bench:  35,321,514 ns/iter (+/- 1,114,093)
```

(`baseline` here shows the amount of time needed to populate map without any readers.
It's here to show overhead for each reader.)

| Prefix length | How many entries caused allocation | Overhead (`BufReader`) | Overhead (`BufRefReader` `<MmapBuffer>`) | Wall clock time difference | Overead (`BufRefReader` `<VecBuffer>`) | Wall clock time difference
|--|--|--|--|--|--|--|
| 2 |  0.3% | 106.7% | 34.2% | -35.1% | 29.0% | -37.6%
| 3 |  2.7% |  86.7% | 26.4% | -32.3% | 22.1% | -34.6%
| 4 | 11.8% |  60.4% | 17.1% | -27.0% | 13.9% | -29.0%
| 5 | 27.4% |  33.9% |  7.2% | -19.9% | 4.6%  | -21.9%

(N.B. `MmapBuffer` should generally be faster. It is not clear yet why it's not the case with its Rust implementation.)

## Acknowledgement

The idea for initial implementation of this crate (the one that only featured `VecBuffer`)
came from experiments with [mawk](https://invisible-island.net/mawk/),
namely after applying [first Futamura projection](https://en.wikipedia.org/wiki/Partial_evaluation#Futamura_projections) to one of awk scripts,
which in turn was done to study mawk's exceptionally good performance.
See [source code for `FINgets()`](https://github.com/ThomasDickey/mawk-20140914/blob/1d2b180d760ddb9d967ff377d9fe21fd4eb9cda5/fin.c#L212)
to learn how mawk buffers its input.

## License

[Apache License 2.0](https://spdx.org/licenses/Apache-2.0.html)
