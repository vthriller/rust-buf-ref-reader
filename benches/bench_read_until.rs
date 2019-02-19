use bencher::{Bencher, benchmark_group, benchmark_main, black_box};

use buf_ref_reader::*;
use std::io::{Read, BufRead, BufReader, Result};
use memchr::memchr;

use std::{thread, time};

static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

struct ThrottledReader<R: Read>(R);
impl<R: Read> Read for ThrottledReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		// sure, this value is close to nothing,
		// but at least we're going to postpone actual read for the time of a single syscall
		thread::sleep(time::Duration::from_nanos(1));
		self.0.read(buf)
	}
}

// make sure we're blackboxing &[u8], not Vec<u8> or something else
fn consume(data: &[u8]) {
	black_box(data);
}

macro_rules! bufref {
	($fname:ident, $wrapped:expr) => {
		fn $fname(b: &mut Bencher, cap: usize, incr: usize) {
			b.iter(|| {
				let mut r = BufRefReaderBuilder::new($wrapped)
					.capacity(cap)
					.increment(incr)
					.build();
				while let Some(line) = r.read_until(b'\n').unwrap() {
					consume(line);
				}
			})
		}
	}
}
bufref!(bufref, &WORDS[..]);
fn bufref_read_until_16x16(b: &mut Bencher) { bufref(b, 16, 16) }
fn bufref_read_until_64x16(b: &mut Bencher) { bufref(b, 64, 16) }
fn bufref_read_until_64x64(b: &mut Bencher) { bufref(b, 64, 64) }
fn bufref_read_until_4kx4k(b: &mut Bencher) { bufref(b, 4096, 4096) }
bufref!(bufref_throttled, ThrottledReader(&WORDS[..]));
fn throttled_bufref_read_until_16x16(b: &mut Bencher) { bufref_throttled(b, 16, 16) }
fn throttled_bufref_read_until_64x16(b: &mut Bencher) { bufref_throttled(b, 64, 16) }
fn throttled_bufref_read_until_64x64(b: &mut Bencher) { bufref_throttled(b, 64, 64) }
fn throttled_bufref_read_until_4kx4k(b: &mut Bencher) { bufref_throttled(b, 4096, 4096) }

macro_rules! std_read_until {
	($fname:ident, $wrapped:expr) => {
		fn $fname(b: &mut Bencher, cap: usize) {
			b.iter(|| {
				let mut r = BufReader::with_capacity(cap, $wrapped);
				let mut buf = vec![];
				while r.read_until(b'\n', &mut buf).unwrap() != 0 {
					consume(buf.as_slice());
					buf.clear();
				}
			})
		}
	}
}
std_read_until!(std_read_until, &WORDS[..]);
fn std_read_until_16(b: &mut Bencher) { std_read_until(b, 16) }
fn std_read_until_64(b: &mut Bencher) { std_read_until(b, 64) }
fn std_read_until_4k(b: &mut Bencher) { std_read_until(b, 4096) }
std_read_until!(std_read_until_throttled, ThrottledReader(&WORDS[..]));
fn throttled_std_read_until_16(b: &mut Bencher) { std_read_until_throttled(b, 16) }
fn throttled_std_read_until_64(b: &mut Bencher) { std_read_until_throttled(b, 64) }
fn throttled_std_read_until_4k(b: &mut Bencher) { std_read_until_throttled(b, 4096) }

/*
This one is like BufRefReader that's made of parts of BufReader,
except when requested data continues past buffer boundary:

| buffer |
      | data |

in which case leftovers are moved into temporary buffer,
and then the main buffer is refilled entirely
to complete that temporary buffer first.

Temporary buffer is discarded upon next read,
and regular referencing of parts of the main buffer is resumed.
*/
macro_rules! std_fillbuf_4k {
	($fname:ident, $wrapped:expr) => {
		fn $fname(b: &mut Bencher) {
			b.iter(|| {
				let mut r = BufReader::with_capacity(4096, $wrapped);

				let mut head: Option<Vec<u8>> = None;

				loop {
					let buf = r.fill_buf().unwrap();
					if buf.len() == 0 {
						// EOF

						if let Some(head) = &mut head {
							consume(head.as_slice());
						}

						break;
					}

					match memchr(b'\n', buf) {
						Some(len) => {
							let tail = &buf[..len];
							let s = if let Some(head) = &mut head {
								head.extend_from_slice(tail);
								head.as_slice()
							} else {
								tail
							};

							consume(s);

							head = None;
							// and only now, after we've used `tail`, we consume data referenced via said `tail`
							r.consume(len+1);
						},
						None => {
							// Either line doesn't fit into the buffer (or, at the very least, tail of the buffer),
							// or the last line does not contain any b'\n'.
							// Resort to copying for this (hopefully) very rare case.
							if let Some(head) = &mut head {
								// again?!
								head.extend_from_slice(buf);
							} else {
								head = Some(buf.to_vec());
							}
							let len = buf.len();
							r.consume(len);
						},
					}
				}
			})
		}
	}
}
std_fillbuf_4k!(std_fillbuf_4k, &WORDS[..]);
std_fillbuf_4k!(throttled_std_fillbuf_4k, ThrottledReader(&WORDS[..]));

// like read_until_words_long test, split by the most rare character in WORDS:

macro_rules! bufref_read_until_long {
	($fname:ident, $wrapped:expr) => {
		fn $fname(b: &mut Bencher) {
			b.iter(|| {
				let mut r = BufRefReaderBuilder::new($wrapped)
					.capacity(4096)
					.increment(4096)
					.build();
				while let Some(x) = r.read_until(b'Q').unwrap() {
					consume(x);
				}
			})
		}
	}
}
bufref_read_until_long!(bufref_read_until_long, &WORDS[..]);
bufref_read_until_long!(throttled_bufref_read_until_long, ThrottledReader(&WORDS[..]));

macro_rules! std_read_until_long {
	($fname:ident, $wrapped:expr) => {
		fn $fname(b: &mut Bencher) {
			b.iter(|| {
				let mut r = BufReader::with_capacity(4096, $wrapped);
				let mut buf = vec![];
				while r.read_until(b'Q', &mut buf).unwrap() != 0 {
					consume(buf.as_slice());
					buf.clear();
				}
			})
		}
	}
}
std_read_until_long!(std_read_until_long, &WORDS[..]);
std_read_until_long!(throttled_std_read_until_long, ThrottledReader(&WORDS[..]));

benchmark_group!(benches,
	bufref_read_until_16x16,
	bufref_read_until_64x16,
	bufref_read_until_64x64,
	bufref_read_until_4kx4k,
	throttled_bufref_read_until_16x16,
	throttled_bufref_read_until_64x16,
	throttled_bufref_read_until_64x64,
	throttled_bufref_read_until_4kx4k,

	std_read_until_16,
	std_read_until_64,
	std_read_until_4k,
	throttled_std_read_until_16,
	throttled_std_read_until_64,
	throttled_std_read_until_4k,

	std_fillbuf_4k,
	throttled_std_fillbuf_4k,

	bufref_read_until_long,
	throttled_bufref_read_until_long,

	std_read_until_long,
	throttled_std_read_until_long,
);
benchmark_main!(benches);
