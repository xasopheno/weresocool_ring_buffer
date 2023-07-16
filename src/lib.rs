use std::error::Error;
use std::fmt;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

#[derive(Debug)]
pub enum RingBufferError {
    DataSizeMismatch,
}

impl fmt::Display for RingBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RingBufferError::DataSizeMismatch => {
                write!(f, "The size of data provided does not match buffer size")
            }
        }
    }
}

impl Error for RingBufferError {}

pub struct RingBuffer<T: Clone + Default + Send + Sync> {
    buffer: Arc<RwLock<Vec<T>>>,
    write_pos: Arc<AtomicUsize>,
    read_pos: Arc<AtomicUsize>,
    size: usize,
}

impl<T: Clone + Default + Send + Sync> RingBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(vec![T::default(); size])),
            write_pos: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            size,
        }
    }

    pub fn write(&self, data: Vec<T>) -> Result<(), RingBufferError> {
        if data.len() != self.size {
            return Err(RingBufferError::DataSizeMismatch);
        }

        let mut buffer_guard = self.buffer.write().unwrap();
        let write_pos = self.write_pos.load(Ordering::SeqCst);

        for i in 0..self.size {
            let write_index = (write_pos + i) % self.size;
            buffer_guard[write_index] = data[i].clone();
        }

        self.write_pos
            .store((write_pos + self.size) % self.size, Ordering::SeqCst);

        Ok(())
    }

    pub fn read(&self) -> Vec<T> {
        let mut result = vec![T::default(); self.size];
        let read_pos = self.read_pos.load(Ordering::SeqCst);

        let buffer_guard = self.buffer.read().unwrap();
        for i in 0..self.size {
            let read_index = (read_pos + i) % self.size;
            result[i] = buffer_guard[read_index].clone();
        }

        result
    }

    pub fn with_read_buffer<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[T]) -> R,
    {
        let read_buffer = self.read();
        f(&read_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::RingBuffer;

    #[test]
    fn test_ring_buffer() {
        let rb = RingBuffer::new(10);

        rb.with_read_buffer(|buffer| {
            assert_eq!(buffer, &[0.0; 10]);
        });

        rb.write(vec![1.0; 10]).unwrap();

        rb.with_read_buffer(|buffer| {
            assert_eq!(buffer, &[1.0; 10]);
        });

        rb.write(vec![2.0; 10]).unwrap();

        rb.write(vec![3.0; 10]).unwrap();

        rb.with_read_buffer(|buffer| {
            assert_eq!(buffer, &[3.0; 10]);
        });
    }
}
