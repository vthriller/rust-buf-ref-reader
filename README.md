# buf-ref-reader

Faster, growable buffering reader.

Use this crate to get faster reads in situations when all you need is immutable `&[u8]`s,
the contents of which rarely need to outlive a single loop cycle.

[See rustdoc](https://docs.rs/buf-ref-reader/) for examples and to read more about how this crate works, its applicability and limitations.

## Benchmarks

Reading from `&[u8] as Read` through 4k-sized buffer into the void:

```
test bench_read_until::bufref_4kx4k      ... bench:   4,053,913 ns/iter (+/- 31,692)
test bench_read_until::std_read_until_4k ... bench:   8,194,855 ns/iter (+/- 71,703)
```

Populating `HashMap` with up to 2-, 3-, 4-, 5-byte prefixes of entries from `/usr/share/dict/words`,
while only allocating memory for new map entries:

```
test bench_hashmap::baseline_2           ... bench:   8,478,583 ns/iter (+/- 94,518)
test bench_hashmap::bufref_2             ... bench:  12,836,876 ns/iter (+/- 49,930)
test bench_hashmap::std_read_until_2     ... bench:  16,718,395 ns/iter (+/- 109,816)
```
```
test bench_hashmap::baseline_3           ... bench:  10,667,947 ns/iter (+/- 159,094)
test bench_hashmap::bufref_3             ... bench:  14,958,627 ns/iter (+/- 126,085)
test bench_hashmap::std_read_until_3     ... bench:  19,183,972 ns/iter (+/- 143,203)
```
```
test bench_hashmap::baseline_4           ... bench:  16,345,453 ns/iter (+/- 382,139)
test bench_hashmap::bufref_4             ... bench:  20,942,797 ns/iter (+/- 393,013)
test bench_hashmap::std_read_until_4     ... bench:  25,420,091 ns/iter (+/- 300,655)
```
```
test bench_hashmap::baseline_5           ... bench:  26,352,079 ns/iter (+/- 834,022)
test bench_hashmap::bufref_5             ... bench:  30,640,867 ns/iter (+/- 1,017,229)
test bench_hashmap::std_read_until_5     ... bench:  35,039,095 ns/iter (+/- 1,062,498)
```

(`baseline` here shows the amount of time needed to populate map without any readers.
It's here to show overhead for each reader.)

| Prefix length | How many entries caused allocation | Overhead (`BufRefReader`) | Overhead (`BufReader`) | Speed difference
|--|--|--|--|--|
| 2 |  0.3% | 51.4% | 97.2% | 23.2% faster
| 3 |  2.7% | 40.2% | 79.8% | 22.0% faster
| 4 | 11.8% | 28.1% | 55.5% | 17.6% faster
| 5 | 27.4% | 16.3% | 33.0% | 12.6% faster

## Acknowledgement

The idea for this crate came from experiments with [mawk](https://invisible-island.net/mawk/),
namely after applying [first Futamura projection](https://en.wikipedia.org/wiki/Partial_evaluation#Futamura_projections) to one of awk scripts,
which in turn was done to study mawk's exceptionally good performance.

See [source code for `FINgets()`](https://github.com/ThomasDickey/mawk-20140914/blob/1d2b180d760ddb9d967ff377d9fe21fd4eb9cda5/fin.c#L212) to learn how mawk buffers its input.

## License

[Apache License 2.0](https://spdx.org/licenses/Apache-2.0.html)
