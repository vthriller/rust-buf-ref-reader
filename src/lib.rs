#![feature(copy_within)]
#![feature(test)]

use std::io::{Read, Result};
use std::cmp::min;
use memchr::memchr;

pub struct BufRefReader<R> {
	src: R,
	buf: Vec<u8>,
	// position of data within the `buf`
	start: usize,
	end: usize,
}

impl<R: Read> BufRefReader<R> {
	pub fn new(src: R) -> BufRefReader<R> {
		Self::with_capacity(src, 8192)
	}

	pub fn with_capacity(src: R, capacity: usize) -> BufRefReader<R> {
		let mut buf = Vec::with_capacity(capacity);
		unsafe { buf.set_len(capacity); }
		BufRefReader {
			src, buf,
			start: 0, end: 0,
		}
	}

	// returns true for EOF
	fn fill(&mut self) -> Result<bool> {
		if self.start == 0 && self.end == self.buf.len() {
			// this buffer is already full, expand
			// TODO configurable
			self.buf.reserve(8192);
			unsafe { self.buf.set_len(self.buf.len() + 8192) };
		} else {
			// reallocate and fill existing buffer
			if self.end - self.start != 0 {
				self.buf.copy_within(self.start..self.end, 0)
			}
			// (A)..(A+B) → 0..B
			self.end -= self.start;
			self.start = 0;
		}

		match self.src.read(&mut self.buf[self.end..])? {
			0 => Ok(true), // EOF
			n => {
				self.end += n;
				Ok(false)
			}
		}
	}

	pub fn read(&mut self, n: usize) -> Result<Option<&[u8]>> {
		while n > self.end - self.start {
			// fill and expand buffer until either:
			// - buffer starts holding the requested amount of data
			// - EOF is reached
			if self.fill()? { break };
		}
		if self.start == self.end {
			// reading past EOF
			Ok(None)
		} else {
			let end = min(self.end, self.start+n);
			let output = &self.buf[ self.start .. end ];
			self.start = end;
			Ok(Some(output))
		}
	}

	/// Returns bytes until `delim` or EOF is reached. If no content available, returns `None`.
	pub fn read_until(&mut self, delim: u8) -> Result<Option<&[u8]>> {
		let mut len = None;
		loop {
			// fill and expand buffer until either:
			// - `delim` appears in the buffer
			// - EOF is reached
			if let Some(n) = memchr(delim, &self.buf[ self.start .. self.end ]) {
				len = Some(n);
				break;
			}
			if self.fill()? { break };
		}

		match len {
			None => { // EOF
				if self.start == self.end {
					Ok(None)
				} else {
					let output = &self.buf[ self.start .. self.end ];
					self.start = self.end;
					Ok(Some(output))
				}
			},
			Some(len) => {
				let output = &self.buf[ self.start .. self.start + len ];
				self.start += len + 1; // also silently consume delimiter
				Ok(Some(output))
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn read_until() {
		let mut r = BufRefReader::with_capacity(&b"lorem ipsum dolor sit amet"[..], 4);
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"ipsum"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"dolor"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"sit"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), Some(&b"amet"[..]));
		assert_eq!(r.read_until(b' ').unwrap(), None);
		assert_eq!(r.read_until(b' ').unwrap(), None);
	}

	#[test]
	fn read() {
		let mut r = BufRefReader::with_capacity(&b"lorem ipsum dolor sit amet"[..], 4);
		assert_eq!(r.read(5).unwrap(), Some(&b"lorem"[..]));
		assert_eq!(r.read(6).unwrap(), Some(&b" ipsum"[..]));
		assert_eq!(r.read(1024).unwrap(), Some(&b" dolor sit amet"[..]));
		assert_eq!(r.read(1).unwrap(), None);
	}
}

#[cfg(test)]
mod bench {
	extern crate test;
	use test::{Bencher, black_box};
	use super::*;
	use std::io::{BufRead, BufReader};

	////

	#[bench]
	fn bufref_read(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufRefReader::with_capacity(&include_bytes!("lib.rs")[..], 16);
			while r.read(4).unwrap() != None {}
		})
	}

	#[bench]
	fn std_read(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("lib.rs")[..]);
			let mut buf = [0; 4];
			while r.read(&mut buf[..]).unwrap() != 0 {}
		})
	}

	////

	#[bench]
	fn bufref_read_until(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufRefReader::with_capacity(&include_bytes!("lib.rs")[..], 16);
			while r.read_until(b'\n').unwrap() != None {}
		})
	}

	#[bench]
	fn std_read_until(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("lib.rs")[..]);
			let mut buf = vec![];
			while r.read_until(b'\n', &mut buf).unwrap() != 0 {}
		})
	}

	#[bench]
	fn std_lines(b: &mut Bencher) {
		b.iter(|| {
			let mut r = BufReader::with_capacity(16, &include_bytes!("lib.rs")[..]);
			for i in r.lines() {
				black_box(i);
			}
		})
	}
}
