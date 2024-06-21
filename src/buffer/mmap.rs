use std::slice::from_raw_parts_mut;
use vmap::os::{
	map_ring,
	unmap_ring,
};
use vmap::{
	Error,
	allocation_size,
};

struct Ring<'a> {
	buf: &'a mut [u8],
}
impl<'a> Ring<'a> {
	fn new(size: usize) -> Result<Self, Error> {
		let buf = map_ring(size)?;
		let buf = unsafe { from_raw_parts_mut(buf, size*2) };
		Ok(Ring { buf })
	}
}
impl<'a> Drop for Ring<'a> {
	fn drop(&mut self) {
		unsafe {
			// FIXME ignored Result: might leak
			let _ = unmap_ring(self.buf.as_mut_ptr(), self.buf.len()/2);
		}
	}
}
impl<'a> std::convert::AsRef<[u8]> for Ring<'a> {
	fn as_ref(&self) -> &[u8] {
		&self.buf
	}
}
impl<'a> std::convert::AsMut<[u8]> for Ring<'a> {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.buf
	}
}

/// Buffer that uses circular buffer implemented with mirrored memory maps
pub struct MmapBuffer<'a> {
	buf: Ring<'a>,
	// position of data within the `buf`
	start: usize,
	len: usize,
}
impl<'a> super::Buffer for MmapBuffer<'a> {
	type Error = Error;
	fn new(size: usize) -> Result<Self, Error> {
		let size = size.next_multiple_of(allocation_size());
		let buf = Ring::new(size)?;
		Ok(MmapBuffer {
			buf,
			start: 0, len: 0,
		})
	}
	fn filled(&self) -> &[u8] {
		&self.buf.as_ref()[ self.start .. (self.start + self.len) ]
	}
	// make room for new data one way or the other
	fn enlarge(&mut self) -> Result<(), Error> {
		let bufsize = self.buf.as_mut().len()/2;
		if self.start == 0 && self.len == bufsize {
			/*
			we used to have configurable increments for the bufsize
			now though we double buffer size, just like rust's vec/raw_vec do
			*/
			let newsize = bufsize * 2;
			let mut new = Ring::new(newsize)?;
			// move data at the start of new buffer
			new.as_mut()[..bufsize].copy_from_slice(&self.buf.as_ref()[self.start..bufsize]);
			self.start = 0;
			self.buf = new;
		} else {
			// there's plenty of room in the buffer,
			// nothing to do here
		}
		Ok(())
	}
	/*
	return b-through-a:
	| a--b | a--b |
	|-b  a-|-b  a-|
	*/
	fn appendable(&mut self) -> &mut [u8] {
		let end = self.start + self.len;
		let remaining = self.buf.as_mut().len()/2 - self.len;
		&mut self.buf.as_mut()[ end .. (end+remaining) ]
	}
	fn grow(&mut self, amount: usize) {
		self.len += amount;
	}
	/*
	returns reference to first half of the buffer
	up to the size of `amount`,
	which is going to be discarded
	after lifetime of returned slice comes to an end
	*/
	fn consume(&mut self, amount: usize) -> &[u8] {
		let start = self.start;
		let amount = std::cmp::min(amount, self.len());

		self.start += amount;
		if self.start >= self.buf.as_mut().len()/2 {
			// keep self.start within bufsize
			self.start -= self.buf.as_mut().len()/2;
		}
		self.len -= amount;
		&self.buf.as_mut()[ start .. (start+amount) ]
	}
	fn len(&self) -> usize {
		self.len
	}
}
