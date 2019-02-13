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

That's 0.3%, 2.7%, 11.8% of lines that cause allocations upon introduction into the HashMap.
*/

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

fn bufref(b: &mut Bencher, buf: usize, n: usize, cap: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(buf)
			.increment(buf)
			.create();
		let mut map = map(cap);
		while let Some(line) = r.read_until(b'\n').unwrap() {
			// .entry() does not accept Borrow<K>, hence this
			let p = prefix(&line, n);
			match map.get_mut(p) {
				Some(v) => { *v += 1; },
				None => { map.insert(p.to_vec(), 1); },
			}
		}
	})
}
#[bench] fn bufref_4k_2(b: &mut Bencher) { bufref(b, 4096, 2, 750) }
#[bench] fn bufref_4k_3(b: &mut Bencher) { bufref(b, 4096, 3, 6500) }
#[bench] fn bufref_4k_4(b: &mut Bencher) { bufref(b, 4096, 4, 28000) }
#[bench] fn bufref_64k_2(b: &mut Bencher) { bufref(b, 65536, 2, 750) }
#[bench] fn bufref_64k_3(b: &mut Bencher) { bufref(b, 65536, 3, 6500) }
#[bench] fn bufref_64k_4(b: &mut Bencher) { bufref(b, 65536, 4, 28000) }

fn std_read_until(b: &mut Bencher, buf: usize, n: usize, cap: usize) {
	b.iter(|| {
		let mut map = map(cap);
		let mut r = BufReader::with_capacity(buf, &WORDS[..]);
		let mut buf = vec![];
		while r.read_until(b'\n', &mut buf).unwrap() != 0 {
			// .entry() does not accept Borrow<K>, hence this
			let p = prefix(&buf, n);
			match map.get_mut(p) {
				Some(v) => { *v += 1; },
				None => { map.insert(p.to_vec(), 1); },
			}
		}
	})
}
#[bench] fn std_read_until_4k_2(b: &mut Bencher) { std_read_until(b, 4096, 2, 750) }
#[bench] fn std_read_until_4k_3(b: &mut Bencher) { std_read_until(b, 4096, 3, 6500) }
#[bench] fn std_read_until_4k_4(b: &mut Bencher) { std_read_until(b, 4096, 4, 28000) }
#[bench] fn std_read_until_64k_2(b: &mut Bencher) { std_read_until(b, 65536, 2, 750) }
#[bench] fn std_read_until_64k_3(b: &mut Bencher) { std_read_until(b, 65536, 3, 6500) }
#[bench] fn std_read_until_64k_4(b: &mut Bencher) { std_read_until(b, 65536, 4, 28000) }

fn baseline(b: &mut Bencher, n: usize, cap: usize) {
	b.iter(|| {
		let mut map = map(cap);
		// I tried .peekable(), but .peek() inside a loop ends up making two mutable refs (E0499),
		// so instead of hacking own loop with .next()/.peek() I just wrote C-style thing with mutable vars
		let mut words = memchr_iter(b'\n', WORDS);
		let mut start = 0;
		loop {
			let end = match words.next() {
				Some(n) => n,
				None => break,
			};
			let line = &WORDS[start..end];

			// .entry() does not accept Borrow<K>, hence this
			let p = prefix(&line, n);
			match map.get_mut(p) {
				Some(v) => { *v += 1; },
				None => { map.insert(p.to_vec(), 1); },
			}

			start = end + 1;
		}
	})
}
#[bench] fn baseline_2(b: &mut Bencher) { baseline(b, 2, 750) }
#[bench] fn baseline_3(b: &mut Bencher) { baseline(b, 3, 6500) }
#[bench] fn baseline_4(b: &mut Bencher) { baseline(b, 4, 28000) }
