/// `Vec`-backed buffer
pub struct VecBuffer {
	buf: Vec<u8>,
	// where actual data resides within the `buf`
	start: usize,
	end: usize,
}
impl super::Buffer for VecBuffer {
	type Error = ();
	fn new(size: usize) -> Result<Self, ()> {
		let mut buf = Vec::with_capacity(size);
		unsafe { buf.set_len(size); }
		Ok(VecBuffer {
			buf,
			start: 0, end: 0,
		})
	}
	// make room for new data one way or the other
	fn enlarge(&mut self) -> Result<(), ()> {
		//if self.start == 0 && self.end == self.buf.len() {
		if self.len() == self.buf.len() {
			// this buffer is already full, double its size
			self.buf.reserve(self.buf.len());
			unsafe { self.buf.set_len(self.buf.len() * 2) };
		} else if self.end == self.buf.len() {
			// reallocate and fill existing buffer
			if self.end - self.start != 0 {
				self.buf.copy_within(self.start..self.end, 0)
			}
			self.end -= self.start;
			self.start = 0;
		} else {
			// there's still some room in `appendable()`, nothing to do
		}
		Ok(())
	}
	fn len(&self) -> usize {
		self.end - self.start
	}
	fn filled(&self) -> &[u8] {
		&self.buf[ self.start .. self.end ]
	}
	fn appendable(&mut self) -> &mut [u8] {
		&mut self.buf[ self.end .. ]
	}
	fn mark_appended(&mut self, amount: usize) {
		self.end += amount;
	}
	/*
	before:
	[  xxxxyyyy ]
	   |      |end
	   |start

	after:
	[  xxxxyyyy ]
	   |  ||  |end
	   |  ||start
	   |--|return value
	*/
	fn consume(&mut self, amount: usize) -> &[u8] {
		let amount = std::cmp::min(amount, self.len());
		let start = self.start;
		self.start += amount;
		&self.buf[ start .. (start+amount) ]
	}
}

mod tests {
	use super::*;
	use crate::buffer::Buffer;

	#[test]
	fn enlarge() {
		let mut buf = VecBuffer::new(4096).unwrap();

		// make sure the rest of the test makes sense
		// (this might fail on exotic machines with larger page sizes)
		assert_eq!(buf.appendable().len(), 4096);

		buf.mark_appended(1024);
		assert_eq!(buf.appendable().len(), 4096-1024);

		// buffer still has space, should be noop
		buf.enlarge().unwrap();
		assert_eq!(buf.appendable().len(), 4096-1024);

		buf.mark_appended(4096-1024);
		assert_eq!(buf.appendable().len(), 0);

		// free some space at the beginning...
		buf.consume(1024);
		assert_eq!(buf.appendable().len(), 0);
		// ...then make it available in appendable()
		buf.enlarge().unwrap();
		assert_eq!(buf.appendable().len(), 1024);

		// fill the buffer again
		buf.mark_appended(1024);
		assert_eq!(buf.appendable().len(), 0);

		// we have no space left, this should cause reallocation with doubling of the initial capacity
		buf.enlarge().unwrap();
		assert_eq!(buf.appendable().len(), 4096);
	}
}
