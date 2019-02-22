use bencher::{Bencher, benchmark_group, benchmark_main};

use buf_ref_reader::*;
use std::io::{BufRead, BufReader};
use fnv::FnvHashMap;
use memchr::memchr_iter;

static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

/*
With GNU's miscfiles-1.5 web2 as a words file:

$ wc -l words
234937 words
$ for i in {2..4}; do cut -c-$i < words | sort -u | wc -l; done
716
6395
27638
64410

That's 0.3%, 2.7%, 11.8%, 27.4% of lines that cause allocations upon introduction into the HashMap.
*/

static BUFSIZE: usize = 64*1024;

#[inline]
fn prefix(s: &[u8], n: usize) -> &[u8] {
	&s[ .. std::cmp::min(s.len(), n) ]
}

// we're testing readers first and foremost,
// hence fnv and predetermined capacity
#[inline]
fn map(cap: usize) -> FnvHashMap<Vec<u8>, usize> {
	FnvHashMap::with_capacity_and_hasher(cap, Default::default())
}

#[inline]
fn insert(map: &mut FnvHashMap<Vec<u8>, usize>, key: &[u8]) {
	// .entry() does not accept Borrow<K>, hence this
	match map.get_mut(key) {
		Some(v) => { *v += 1; },
		None => { map.insert(key.to_vec(), 1); },
	}
}

fn bufref_hashmap(b: &mut Bencher, n: usize, cap: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(BUFSIZE)
			.increment(BUFSIZE)
			.build()
			.unwrap();
		let mut map = map(cap);
		while let Some(line) = r.read_until(b'\n').unwrap() {
			let p = prefix(&line, n);
			insert(&mut map, p);
		}
	})
}
fn bufref_hashmap_2(b: &mut Bencher) { bufref_hashmap(b, 2, 750) }
fn bufref_hashmap_3(b: &mut Bencher) { bufref_hashmap(b, 3, 6500) }
fn bufref_hashmap_4(b: &mut Bencher) { bufref_hashmap(b, 4, 28000) }
fn bufref_hashmap_5(b: &mut Bencher) { bufref_hashmap(b, 5, 65000) }

fn std_hashmap(b: &mut Bencher, n: usize, cap: usize) {
	b.iter(|| {
		let mut map = map(cap);
		let mut r = BufReader::with_capacity(BUFSIZE, &WORDS[..]);
		let mut buf = vec![];
		while r.read_until(b'\n', &mut buf).unwrap() != 0 {
			let p = prefix(&buf, n);
			insert(&mut map, p);
			buf.clear();
		}
	})
}
fn std_hashmap_2(b: &mut Bencher) { std_hashmap(b, 2, 750) }
fn std_hashmap_3(b: &mut Bencher) { std_hashmap(b, 3, 6500) }
fn std_hashmap_4(b: &mut Bencher) { std_hashmap(b, 4, 28000) }
fn std_hashmap_5(b: &mut Bencher) { std_hashmap(b, 5, 65000) }

/*
this benchmark is solely about measuring code
that populates HashMap with occasional copies of references to slices of WORDS,
hence collection of such slices outside the bench loop
*/
fn baseline_hashmap(b: &mut Bencher, n: usize, cap: usize) {
	let words: Vec<usize> = memchr_iter(b'\n', WORDS)
		.collect(); // can't clone Memchr iterator itself, hence this
	let starts = vec![0].into_iter()
		.chain(words.clone().into_iter().map(|n| n+1)); // one past delimiter
	let ends = words.into_iter()
		.chain(vec![WORDS.len()].into_iter());
	let lines: Vec<_> = starts.zip(ends)
		.map(|(start, end)| &WORDS[start..end])
		.collect();

	b.iter(|| {
		let mut map = map(cap);
		for &line in lines.iter() {
			let p = prefix(&line, n);
			insert(&mut map, p);
		}
	})
}
fn baseline_hashmap_2(b: &mut Bencher) { baseline_hashmap(b, 2, 750) }
fn baseline_hashmap_3(b: &mut Bencher) { baseline_hashmap(b, 3, 6500) }
fn baseline_hashmap_4(b: &mut Bencher) { baseline_hashmap(b, 4, 28000) }
fn baseline_hashmap_5(b: &mut Bencher) { baseline_hashmap(b, 5, 65000) }

benchmark_group!(benches,
	bufref_hashmap_2,
	bufref_hashmap_3,
	bufref_hashmap_4,
	bufref_hashmap_5,
	std_hashmap_2,
	std_hashmap_3,
	std_hashmap_4,
	std_hashmap_5,
	baseline_hashmap_2,
	baseline_hashmap_3,
	baseline_hashmap_4,
	baseline_hashmap_5,
);
benchmark_main!(benches);
