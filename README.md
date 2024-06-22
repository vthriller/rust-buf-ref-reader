# buf-ref-reader

Faster, growable buffering reader.

Use this crate to get faster reads in situations when all you need is immutable `&[u8]`s,
the contents of which rarely need to outlive a single loop cycle.

[See rustdoc](https://docs.rs/buf-ref-reader/) for examples and to read more about how this crate works, its applicability and limitations.

## Benchmarks

Reading lines from `&[u8] as Read` through 64k-sized buffer into the void:

```
throttled_bufref_read_until_mmap_64  time:   [3.8888 ms 3.8934 ms 3.8983 ms] outliers: 3% high mild
throttled_bufref_read_until_vec_64   time:   [3.7754 ms 3.7798 ms 3.7847 ms] outliers: 3% high mild 1% high severe
throttled_std_read_until_64          time:   [7.0686 ms 7.0734 ms 7.0783 ms] outliers: 2% high mild
```

Populating `HashMap` with up to 2-, 3-, 4-, 5-byte prefixes of entries from `/usr/share/dict/words`,
while only allocating memory for new map entries:

```
baseline_hashmap_2      time:   [2.8716 ms 2.8724 ms 2.8733 ms] outliers: 4% low mild  4% high mild  1% high severe
bufref_hashmap_mmap_2   time:   [4.9186 ms 4.9242 ms 4.9309 ms] outliers:              3% high mild  4% high severe
bufref_hashmap_vec_2    time:   [4.6957 ms 4.6977 ms 4.6998 ms] outliers:              3% high mild  2% high severe
std_hashmap_2           time:   [6.6979 ms 6.7016 ms 6.7056 ms] outliers:              1% high mild  1% high severe
```
```
baseline_hashmap_3      time:   [3.1622 ms 3.1636 ms 3.1653 ms] outliers: 1% low mild  3% high mild  3% high severe
bufref_hashmap_mmap_3   time:   [5.1284 ms 5.1363 ms 5.1445 ms] outliers:              2% high mild
bufref_hashmap_vec_3    time:   [4.9562 ms 4.9613 ms 4.9674 ms] outliers:              3% high mild  2% high severe
std_hashmap_3           time:   [6.7757 ms 6.7793 ms 6.7832 ms] outliers:              4% high mild  5% high severe
```
```
baseline_hashmap_4      time:   [5.3055 ms 5.3112 ms 5.3175 ms] outliers: 3% high mild  2% high severe
bufref_hashmap_mmap_4   time:   [7.0919 ms 7.1002 ms 7.1096 ms] outliers: 5% high mild  1% high severe
bufref_hashmap_vec_4    time:   [6.8833 ms 6.8910 ms 6.8992 ms] outliers: 3% high mild  1% high severe
std_hashmap_4           time:   [8.9081 ms 8.9179 ms 8.9287 ms] outliers: 4% high mild  3% high severe
```
```
baseline_hashmap_5      time:   [11.947 ms 11.969 ms 11.993 ms] outliers: 4% high mild  6% high severe
bufref_hashmap_mmap_5   time:   [13.484 ms 13.504 ms 13.526 ms] outliers: 4% high mild  1% high severe
bufref_hashmap_vec_5    time:   [13.259 ms 13.289 ms 13.320 ms]
std_hashmap_5           time:   [14.757 ms 14.775 ms 14.794 ms] outliers: 7% high mild  1% high severe
```

(`baseline` here shows the amount of time needed to populate map without any readers.
It's here to show overhead for each reader.)

| Prefix length | How many entries caused allocation | Overhead (`BufReader`) | Overhead (`BufRefReader` `<MmapBuffer>`) | Wall clock time difference | Overead (`BufRefReader` `<VecBuffer>`) | Wall clock time difference
|--|--|--|--|--|--|--|
| 2 |  0.3% | 133.3% | 71.4% | -26.5% | 63.5% | -29.9%
| 3 |  2.7% | 114.3% | 62.4% | -24.2% | 56.8% | -26.8%
| 4 | 11.8% |  67.9% | 33.7% | -20.4% | 29.7% | -22.7%
| 5 | 27.4% |  23.4% | 12.8% |  -8.6% | 11.0% | -10.1%

## Acknowledgement

The idea for initial implementation of this crate (the one that only featured `VecBuffer`)
came from experiments with [mawk](https://invisible-island.net/mawk/),
namely after applying [first Futamura projection](https://en.wikipedia.org/wiki/Partial_evaluation#Futamura_projections) to one of awk scripts,
which in turn was done to study mawk's exceptionally good performance.
See [source code for `FINgets()`](https://github.com/ThomasDickey/mawk-20140914/blob/1d2b180d760ddb9d967ff377d9fe21fd4eb9cda5/fin.c#L212)
to learn how mawk buffers its input.

## License

[Apache License 2.0](https://spdx.org/licenses/Apache-2.0.html)
