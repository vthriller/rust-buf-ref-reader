extern crate test;
use test::Bencher;
use super::*;
use std::io::BufReader;

fn bufref(b: &mut Bencher, cap: usize, incr: usize, read: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.create();
		while r.read(read).unwrap() != None {}
	})
}
#[bench] fn bufref_16x16x4(b: &mut Bencher) { bufref(b, 16, 16, 4) }
#[bench] fn bufref_64x16x4(b: &mut Bencher) { bufref(b, 64, 16, 4) }
#[bench] fn bufref_4kx4kx4(b: &mut Bencher) { bufref(b, 4096, 4096, 4) }

fn std(b: &mut Bencher, cap: usize, read: usize) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(cap, &WORDS[..]);
		let mut buf = Vec::with_capacity(read);
		unsafe { buf.set_len(read); }
		while r.read(&mut buf[..]).unwrap() != 0 {}
	})
}
#[bench] fn std_16x4(b: &mut Bencher) { std(b, 16, 4) }
#[bench] fn std_64x4(b: &mut Bencher) { std(b, 16, 4) }
#[bench] fn std_4kx4(b: &mut Bencher) { std(b, 4096, 4) }
