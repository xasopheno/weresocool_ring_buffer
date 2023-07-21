use std::cell::UnsafeCell;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum RingBufferError {
    DataSizeMismatch,
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

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
    buffers: UnsafeCell<Vec<*mut Vec<f32>>>,
    start_time: Instant,
    last_read: UnsafeCell<u64>,
    total_writes: UnsafeCell<usize>,
    total_reads: UnsafeCell<usize>,
    buffer_size: usize,
    ring_buffer_size: usize,
    sample_rate: f32,
}

impl RingBuffer {
    pub fn new(buffer_size: usize, ring_buffer_size: usize, sample_rate: usize) -> Self {
        let now = Instant::now();

        let buffers = (0..ring_buffer_size)
            .map(|_| Box::into_raw(Box::new(vec![0.0; buffer_size])))
            .collect();

        Self {
            buffers: UnsafeCell::new(buffers),
            start_time: now,
            last_read: UnsafeCell::new(Self::to_nanos(&now.elapsed())),
            total_writes: UnsafeCell::new(0),
            total_reads: UnsafeCell::new(0),
            buffer_size,
            ring_buffer_size,
            sample_rate: sample_rate as f32,
        }
    }

    // We need to convert Durations to u64 representations in nanoseconds
    fn to_nanos(duration: &Duration) -> u64 {
        duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64
    }

    // And convert u128 representations in nanoseconds to Durations
    fn from_nanos(nanos: u128) -> Duration {
        Duration::new(
            (nanos / 1_000_000_000) as u64,
            (nanos % 1_000_000_000) as u32,
        )
    }

    pub fn write(&self, data: Vec<f32>) -> Result<(), RingBufferError> {
        unsafe {
            if data.len() != self.buffer_size {
                return Err(RingBufferError::DataSizeMismatch);
            }

            let write_index = *self.total_writes.get() % self.ring_buffer_size;

            let old_data = std::mem::replace(
                &mut (*self.buffers.get())[write_index],
                Box::into_raw(Box::new(data)),
            );

            let _ = Box::from_raw(old_data); // safely drop the old data

            *self.total_writes.get() += 1;
        }

        Ok(())
    }

    pub fn read(&self) -> Vec<f32> {
        let elapsed_time;
        let total_reads;
        let total_writes;
        let buffer_size;
        let sample_rate;
        unsafe {
            elapsed_time = Self::from_nanos(
                self.start_time.elapsed().as_nanos() - *self.last_read.get() as u128,
            )
            .as_secs_f32();
            total_reads = *self.total_reads.get();
            total_writes = *self.total_writes.get();
            buffer_size = self.buffer_size;
            sample_rate = self.sample_rate;
        }

        if total_reads < total_writes - 1 && elapsed_time >= buffer_size as f32 / sample_rate * 0.7
        {
            unsafe {
                *self.total_reads.get() += 1;
                *self.last_read.get() = Self::to_nanos(&self.start_time.elapsed());
            }
        }

        if total_reads > 10 && total_reads < total_writes - 6 {
            dbg!("caught up");
            unsafe {
                *self.total_reads.get() = total_writes;
            }
        }

        let read_index;
        unsafe {
            read_index = *self.total_reads.get() % self.ring_buffer_size;
            let data_ptr = (*self.buffers.get())[read_index];
            let data = (&*data_ptr).clone();
            data
        }
    }
}
