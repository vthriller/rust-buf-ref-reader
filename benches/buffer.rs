use bencher::{Bencher, benchmark_group, benchmark_main};

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
fn buf_create_vec_4(b: &mut Bencher)   { create::<VecBuffer> (b, 4096) }
fn buf_create_vec_64(b: &mut Bencher)  { create::<VecBuffer> (b, 64*1024) }
fn buf_create_mmap_4(b: &mut Bencher)  { create::<MmapBuffer>(b, 4096) }
fn buf_create_mmap_64(b: &mut Bencher) { create::<MmapBuffer>(b, 64*1024) }

benchmark_group!(benches,
	buf_create_vec_4,
	buf_create_vec_64,
	buf_create_mmap_4,
	buf_create_mmap_64,
);
benchmark_main!(benches);
