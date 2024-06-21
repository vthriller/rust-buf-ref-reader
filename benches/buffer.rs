use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};

use buf_ref_reader::*;

fn create<B: Buffer>(b: &mut Bencher, cap: usize)
where
	B::Error: std::fmt::Debug,
	//Error: From<B::Error>,
{
	b.iter(|| {
		B::new(cap).unwrap()
	})
}
fn buf_create_vec_4(c: &mut Criterion)   { c.bench_function("buf_create_vec_4",   |b| create::<VecBuffer> (b, 4096)); }
fn buf_create_vec_64(c: &mut Criterion)  { c.bench_function("buf_create_vec_64",  |b| create::<VecBuffer> (b, 64*1024)); }
fn buf_create_mmap_4(c: &mut Criterion)  { c.bench_function("buf_create_mmap_4",  |b| create::<MmapBuffer>(b, 4096)); }
fn buf_create_mmap_64(c: &mut Criterion) { c.bench_function("buf_create_mmap_64", |b| create::<MmapBuffer>(b, 64*1024)); }

criterion_group!(benches,
	buf_create_vec_4,
	buf_create_vec_64,
	buf_create_mmap_4,
	buf_create_mmap_64,
);
criterion_main!(benches);
