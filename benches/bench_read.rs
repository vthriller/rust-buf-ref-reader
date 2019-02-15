use bencher::{Bencher, benchmark_group, benchmark_main};

use buf_ref_reader::*;
use std::io::{Read, BufReader};

static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

fn bufref_read(b: &mut Bencher, cap: usize, incr: usize, read: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.build();
		while r.read(read).unwrap() != None {}
	})
}
fn bufref_read_16x16x4(b: &mut Bencher) { bufref_read(b, 16, 16, 4) }
fn bufref_read_64x16x4(b: &mut Bencher) { bufref_read(b, 64, 16, 4) }
fn bufref_read_4kx4kx4(b: &mut Bencher) { bufref_read(b, 4096, 4096, 4) }

fn std_read(b: &mut Bencher, cap: usize, read: usize) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(cap, &WORDS[..]);
		let mut buf = Vec::with_capacity(read);
		unsafe { buf.set_len(read); }
		while r.read(&mut buf[..]).unwrap() != 0 {}
	})
}
fn std_read_16x4(b: &mut Bencher) { std_read(b, 16, 4) }
fn std_read_64x4(b: &mut Bencher) { std_read(b, 16, 4) }
fn std_read_4kx4(b: &mut Bencher) { std_read(b, 4096, 4) }

benchmark_group!(benches,
	bufref_read_16x16x4,
	bufref_read_64x16x4,
	bufref_read_4kx4kx4,
	std_read_16x4,
	std_read_64x4,
	std_read_4kx4,
);
benchmark_main!(benches);
