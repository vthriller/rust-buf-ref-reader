use bencher::{Bencher, benchmark_group, benchmark_main};

use buf_ref_reader::*;
use std::io::{BufRead, BufReader};
use memchr::memchr;

static WORDS: &'static [u8] = include_bytes!("/usr/share/dict/words");

fn bufref(b: &mut Bencher, cap: usize, incr: usize) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(cap)
			.increment(incr)
			.build();
		while r.read_until(b'\n').unwrap() != None {}
	})
}
fn bufref_read_until_16x16(b: &mut Bencher) { bufref(b, 16, 16) }
fn bufref_read_until_64x16(b: &mut Bencher) { bufref(b, 64, 16) }
fn bufref_read_until_64x64(b: &mut Bencher) { bufref(b, 64, 64) }
fn bufref_read_until_4kx4k(b: &mut Bencher) { bufref(b, 4096, 4096) }

fn std_read_until(b: &mut Bencher, cap: usize) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(cap, &WORDS[..]);
		let mut buf = vec![];
		while r.read_until(b'\n', &mut buf).unwrap() != 0 {}
	})
}
fn std_read_until_16(b: &mut Bencher) { std_read_until(b, 16) }
fn std_read_until_64(b: &mut Bencher) { std_read_until(b, 64) }
fn std_read_until_4k(b: &mut Bencher) { std_read_until(b, 4096) }

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
fn std_fillbuf_4k(b: &mut Bencher) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(4096, &WORDS[..]);

		let mut head: Option<Vec<u8>> = None;

		loop {
			let buf = r.fill_buf().unwrap();
			if buf.len() == 0 {
				// EOF

				if let Some(head) = &mut head {
					// F(head.as_slice());
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

					// F(s);

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

// like read_until_words_long test, split by the most rare character in WORDS:

fn bufref_read_until_long(b: &mut Bencher) {
	b.iter(|| {
		let mut r = BufRefReaderBuilder::new(&WORDS[..])
			.capacity(4096)
			.increment(4096)
			.build();
		while r.read_until(b'Q').unwrap() != None {}
	})
}

fn std_read_until_long(b: &mut Bencher) {
	b.iter(|| {
		let mut r = BufReader::with_capacity(4096, &WORDS[..]);
		let mut buf = vec![];
		while r.read_until(b'Q', &mut buf).unwrap() != 0 {}
	})
}

benchmark_group!(benches,
	bufref_read_until_16x16,
	bufref_read_until_64x16,
	bufref_read_until_64x64,
	bufref_read_until_4kx4k,
	std_read_until_16,
	std_read_until_64,
	std_read_until_4k,
	std_fillbuf_4k,
	bufref_read_until_long,
	std_read_until_long,
);
benchmark_main!(benches);
