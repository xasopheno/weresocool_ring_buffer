use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering};
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
    buffers: Vec<AtomicPtr<Vec<f32>>>,
    start_time: Instant,
    last_read: AtomicU64,
    total_writes: AtomicUsize,
    total_reads: AtomicUsize,
    buffer_size: usize,
    ring_buffer_size: usize,
    sample_rate: f32,
}

impl RingBuffer {
    pub fn new(buffer_size: usize, ring_buffer_size: usize, sample_rate: usize) -> Self {
        let now = Instant::now();

        let buffers = (0..ring_buffer_size)
            .map(|_| AtomicPtr::new(Box::into_raw(Box::new(vec![0.0; buffer_size]))))
            .collect();

        Self {
            buffers,
            start_time: now,
            last_read: AtomicU64::new(Self::to_nanos(&now.elapsed())),
            total_writes: AtomicUsize::new(0),
            total_reads: AtomicUsize::new(0),
            buffer_size,
            ring_buffer_size,
            sample_rate: sample_rate as f32,
        }
    }

    fn to_nanos(duration: &Duration) -> u64 {
        duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
    }

    fn from_nanos(nanos: u128) -> Duration {
        Duration::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        )
    }

    pub fn write(&self, data: Vec<f32>) -> Result<(), RingBufferError> {
        if data.len() != self.buffer_size {
            return Err(RingBufferError::DataSizeMismatch);
        }

        let total_writes = self.total_writes.load(Ordering::SeqCst);
        let write_index = total_writes % self.ring_buffer_size;
        // println!("write: {:?}", total_writes);

        let old_data =
            self.buffers[write_index].swap(Box::into_raw(Box::new(data)), Ordering::SeqCst);

        unsafe {
            _ = Box::from_raw(old_data);
        } // safely drop the old data

        self.total_writes.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    pub fn read(&self) -> Vec<f32> {
        let total_writes = self.total_writes.load(Ordering::SeqCst);
        let total_reads = self.total_reads.load(Ordering::SeqCst);
        // println!("read: {:?}", total_reads);

        // Get the elapsed time in seconds as a float
        let elapsed_time = Self::from_nanos(
            self.start_time.elapsed().as_nanos() - (self.last_read.load(Ordering::SeqCst) as u128),
        )
        .as_secs_f32();

        if total_reads < total_writes - 1
            && elapsed_time >= self.buffer_size as f32 / self.sample_rate * 0.75
        {
            self.total_reads.fetch_add(1, Ordering::SeqCst);
            self.last_read
                .store(Self::to_nanos(&self.start_time.elapsed()), Ordering::SeqCst);
        }

        let total_reads = self.total_reads.load(Ordering::SeqCst);
        if total_reads > 10 && total_reads < total_writes - 6 {
            println!("caught up");
            self.total_reads.store(total_writes, Ordering::SeqCst);
        }

        let read_index = total_reads % self.ring_buffer_size;
        let data_ptr = self.buffers[read_index].load(Ordering::SeqCst);
        unsafe { &*data_ptr }.clone()
    }
}
