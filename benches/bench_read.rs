use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};

use buf_ref_reader::*;
use std::io::{Read, BufReader};

static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

// make sure we're blackboxing &[u8], not Vec<u8> or something else
fn consume(data: &[u8]) {
	black_box(data);
}

fn bufref_read<B: Buffer>(b: &mut Bencher, cap: usize, read: usize)
where
	B::Error: std::fmt::Debug,
	Error: From<B::Error>,
{
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(WORDS)
			.capacity(cap)
			.build::<B>()
			.unwrap();
		while let Some(chunk) = r.read(read).unwrap() {
			consume(chunk);
		}
	})
}
fn bufref_read_vec_4x4(c: &mut Criterion)   { c.bench_function("bufref_read_vec_4x4",   |b| bufref_read::<VecBuffer> (b, 4096, 4)); }
fn bufref_read_vec_64x4(c: &mut Criterion)  { c.bench_function("bufref_read_vec_64x4",  |b| bufref_read::<VecBuffer> (b, 64*1024, 4)); }
fn bufref_read_mmap_4x4(c: &mut Criterion)  { c.bench_function("bufref_read_mmap_4x4",  |b| bufref_read::<MmapBuffer>(b, 4096, 4)); }
fn bufref_read_mmap_64x4(c: &mut Criterion) { c.bench_function("bufref_read_mmap_64x4", |b| bufref_read::<MmapBuffer>(b, 64*1024, 4)); }

fn std_read(b: &mut Bencher, cap: usize, read: usize) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(cap, WORDS);
		let mut buf = Vec::with_capacity(read);
		unsafe { buf.set_len(read); }
		while r.read(&mut buf[..]).unwrap() != 0 {
			consume(buf.as_slice());
		}
	})
}
fn std_read_4x4(c: &mut Criterion)  { c.bench_function("std_read_4x4",  |b| std_read(b, 4096, 4)); }
fn std_read_64x4(c: &mut Criterion) { c.bench_function("std_read_64x4", |b| std_read(b, 64*1024, 4)); }

criterion_group!(benches,
	bufref_read_vec_4x4,
	bufref_read_vec_64x4,
	bufref_read_mmap_4x4,
	bufref_read_mmap_64x4,
	std_read_4x4,
	std_read_64x4,
);
criterion_main!(benches);
