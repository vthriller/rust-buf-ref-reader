extern crate test;
use test::Bencher;
use super::*;
use std::io::{BufRead, BufReader};
use fnv::FnvHashMap;
use memchr::memchr_iter;

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

fn bufref(b: &mut Bencher, n: usize, cap: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(BUFSIZE)
			.increment(BUFSIZE)
			.build();
		let mut map = map(cap);
		while let Some(line) = r.read_until(b'\n').unwrap() {
			let p = prefix(&line, n);
			insert(&mut map, p);
		}
	})
}
#[bench] fn bufref_2(b: &mut Bencher) { bufref(b, 2, 750) }
#[bench] fn bufref_3(b: &mut Bencher) { bufref(b, 3, 6500) }
#[bench] fn bufref_4(b: &mut Bencher) { bufref(b, 4, 28000) }
#[bench] fn bufref_5(b: &mut Bencher) { bufref(b, 5, 65000) }

fn std_read_until(b: &mut Bencher, n: usize, cap: usize) {
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
#[bench] fn std_read_until_2(b: &mut Bencher) { std_read_until(b, 2, 750) }
#[bench] fn std_read_until_3(b: &mut Bencher) { std_read_until(b, 3, 6500) }
#[bench] fn std_read_until_4(b: &mut Bencher) { std_read_until(b, 4, 28000) }
#[bench] fn std_read_until_5(b: &mut Bencher) { std_read_until(b, 5, 65000) }

/*
this benchmark is solely about measuring code
that populates HashMap with occasional copies of references to slices of WORDS,
hence collection of such slices outside the bench loop
*/
fn baseline(b: &mut Bencher, n: usize, cap: usize) {
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
#[bench] fn baseline_2(b: &mut Bencher) { baseline(b, 2, 750) }
#[bench] fn baseline_3(b: &mut Bencher) { baseline(b, 3, 6500) }
#[bench] fn baseline_4(b: &mut Bencher) { baseline(b, 4, 28000) }
#[bench] fn baseline_5(b: &mut Bencher) { baseline(b, 5, 65000) }
