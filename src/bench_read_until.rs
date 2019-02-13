extern crate test;
use test::Bencher;
use super::*;
use std::io::{BufRead, BufReader};

fn bufref(b: &mut Bencher, cap: usize, incr: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.create();
		while r.read_until(b'\n').unwrap() != None {}
	})
}
#[bench] fn bufref_16x16(b: &mut Bencher) { bufref(b, 16, 16) }
#[bench] fn bufref_64x16(b: &mut Bencher) { bufref(b, 64, 16) }
#[bench] fn bufref_64x64(b: &mut Bencher) { bufref(b, 64, 64) }
#[bench] fn bufref_4kx4k(b: &mut Bencher) { bufref(b, 4096, 4096) }

// like read_until_words_long, splits by the most rare character in WORDS
#[bench]
fn bufref_long(b: &mut Bencher) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(4096)
			.increment(4096)
			.create();
		while r.read_until(b'Q').unwrap() != None {}
	})
}

fn std_read_until(b: &mut Bencher, cap: usize) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(cap, &WORDS[..]);
		let mut buf = vec![];
		while r.read_until(b'\n', &mut buf).unwrap() != 0 {}
	})
}
#[bench] fn std_read_until_16(b: &mut Bencher) { std_read_until(b, 16) }
#[bench] fn std_read_until_64(b: &mut Bencher) { std_read_until(b, 64) }
#[bench] fn std_read_until_4k(b: &mut Bencher) { std_read_until(b, 4096) }
