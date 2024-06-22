# buf-ref-reader

Faster, growable buffering reader.

Use this crate to get faster reads in situations when all you need is immutable `&[u8]`s,
the contents of which rarely need to outlive a single loop cycle.

[See rustdoc](https://docs.rs/buf-ref-reader/) for examples and to read more about how this crate works, its applicability and limitations.

## Benchmarks

Reading lines from `&[u8] as Read` through 64k-sized buffer into the void:

```
throttled_bufref_read_until_mmap_64  time:   [3.9114 ms 3.9204 ms 3.9302 ms] outliers: 8.00% high mild 1.00% high severe
throttled_bufref_read_until_vec_64   time:   [4.1633 ms 4.1744 ms 4.1861 ms] outliers: 2.00% high mild
throttled_std_read_until_64          time:   [7.1823 ms 7.1883 ms 7.1944 ms]
```

Populating `HashMap` with up to 2-, 3-, 4-, 5-byte prefixes of entries from `/usr/share/dict/words`,
while only allocating memory for new map entries:

```
baseline_hashmap_2      time:   [2.9565 ms 2.9746 ms 2.9957 ms] outliers:                 8.00% high mild 7.00% high severe
bufref_hashmap_mmap_2   time:   [5.0644 ms 5.0719 ms 5.0795 ms] outliers:                 1.00% high mild
bufref_hashmap_vec_2    time:   [4.8683 ms 4.8726 ms 4.8771 ms] outliers:                 4.00% high mild
std_hashmap_2           time:   [7.0047 ms 7.0172 ms 7.0315 ms] outliers:                                 1.00% high severe
```
```
baseline_hashmap_3      time:   [3.2282 ms 3.2337 ms 3.2397 ms] outliers:                 1.00% high mild 1.00% high severe
bufref_hashmap_mmap_3   time:   [5.0596 ms 5.0659 ms 5.0729 ms] outliers:                 4.00% high mild 2.00% high severe
bufref_hashmap_vec_3    time:   [5.1929 ms 5.2030 ms 5.2138 ms] outliers:                                 1.00% high severe
std_hashmap_3           time:   [7.0410 ms 7.0506 ms 7.0597 ms] outliers: 6.00% low mild
```
```
baseline_hashmap_4      time:   [5.3731 ms 5.3811 ms 5.3892 ms] outliers:                1.00% high mild
bufref_hashmap_mmap_4   time:   [7.5447 ms 7.5671 ms 7.5901 ms] outliers: 3.00% low mild                 1.00% high severe
bufref_hashmap_vec_4    time:   [7.3310 ms 7.3484 ms 7.3662 ms] outliers:                1.00% high mild
std_hashmap_4           time:   [9.2472 ms 9.2716 ms 9.2987 ms] outliers: 1.00% low mild 2.00% high mild 1.00% high severe
```
```
baseline_hashmap_5      time:   [12.026 ms 12.057 ms 12.088 ms] outliers:                1.00% high mild
bufref_hashmap_mmap_5   time:   [13.510 ms 13.540 ms 13.570 ms] outliers:                2.00% high mild
bufref_hashmap_vec_5    time:   [14.010 ms 14.049 ms 14.090 ms] outliers:                1.00% high mild
std_hashmap_5           time:   [15.079 ms 15.108 ms 15.137 ms]
```

(`baseline` here shows the amount of time needed to populate map without any readers.
It's here to show overhead for each reader.)

| Prefix length | How many entries caused allocation | Overhead (`BufReader`) | Overhead (`BufRefReader` `<MmapBuffer>`) | Wall clock time difference | Overead (`BufRefReader` `<VecBuffer>`) | Wall clock time difference
|--|--|--|--|--|--|--|
| 2 |  0.3% | 135.9% | 70.5% | -27.7% | 63.8% | -30.6%
| 3 |  2.7% | 118.0% | 56.7% | -28.1% | 60.9% | -26.2%
| 4 | 11.8% |  72.3% | 40.6% | -18.4% | 36.6% | -20.7%
| 5 | 27.4% |  25.3% | 12.3% | -10.4% | 16.5% |  -7.0%

## Acknowledgement

The idea for initial implementation of this crate (the one that only featured `VecBuffer`)
came from experiments with [mawk](https://invisible-island.net/mawk/),
namely after applying [first Futamura projection](https://en.wikipedia.org/wiki/Partial_evaluation#Futamura_projections) to one of awk scripts,
which in turn was done to study mawk's exceptionally good performance.
See [source code for `FINgets()`](https://github.com/ThomasDickey/mawk-20140914/blob/1d2b180d760ddb9d967ff377d9fe21fd4eb9cda5/fin.c#L212)
to learn how mawk buffers its input.

## License

[Apache License 2.0](https://spdx.org/licenses/Apache-2.0.html)
