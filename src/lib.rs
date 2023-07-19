use std::error::Error;
use std::fmt;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::thread;
use std::time::{Duration, Instant};

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

pub struct RingBuffer {
    buffers: Arc<RwLock<Vec<Vec<f32>>>>,
    last_read: std::sync::Mutex<Instant>,
    total_writes: Arc<AtomicUsize>,
    total_reads: Arc<AtomicUsize>,
    buffer_size: usize,
    ring_buffer_size: usize,
    sample_rate: f32,
}

impl RingBuffer {
    pub fn new(buffer_size: usize, ring_buffer_size: usize, sample_rate: usize) -> Self {
        Self {
            buffers: Arc::new(RwLock::new(vec![vec![0.0; buffer_size]; ring_buffer_size])),
            total_writes: Arc::new(AtomicUsize::new(0)),
            total_reads: Arc::new(AtomicUsize::new(0)),
            last_read: std::sync::Mutex::new(Instant::now()),
            buffer_size,
            ring_buffer_size,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn write(&self, data: Vec<f32>) -> Result<(), RingBufferError> {
        if data.len() != self.buffer_size {
            return Err(RingBufferError::DataSizeMismatch);
        }

        let total_writes = self.total_writes.load(Ordering::SeqCst);
        let write_index = total_writes % self.ring_buffer_size;
        let mut buffers = self.buffers.write().unwrap();
        buffers[write_index] = data;

        self.total_writes.fetch_add(1, Ordering::SeqCst);

        let total_reads = self.total_reads.load(Ordering::SeqCst);
        if total_reads > 10 && total_reads < write_index - 1 {
            self.total_reads.store(write_index, Ordering::SeqCst);
        }

        Ok(())
    }

    pub fn read(&self) -> Arc<RwLock<Vec<f32>>> {
        let total_writes = self.total_writes.load(Ordering::SeqCst);
        let total_reads = self.total_reads.load(Ordering::SeqCst);

        let elapsed_time = self.last_read.lock().unwrap().elapsed().as_secs_f32();

        if total_reads < total_writes
            && elapsed_time >= self.buffer_size as f32 / self.sample_rate * 0.9
        {
            self.total_reads.fetch_add(1, Ordering::SeqCst);
            *self.last_read.lock().unwrap() = Instant::now();
        }

        let read_index = total_reads % self.ring_buffer_size;
        Arc::new(RwLock::new(
            self.buffers.read().unwrap()[read_index].clone(),
        ))
    }
}
