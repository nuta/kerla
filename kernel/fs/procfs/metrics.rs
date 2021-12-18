use core::fmt;

use kerla_runtime::page_allocator::read_allocator_stats;

use crate::{
    fs::{
        inode::{FileLike, INodeNo},
        opened_file::OpenOptions,
        stat::{FileMode, Stat, S_IFCHR},
    },
    net::read_tcp_stats,
    process::read_process_stats,
    result::Result,
    timer::read_monotonic_clock,
    user_buffer::UserBufferMut,
    user_buffer::{UserBufWriter, UserBuffer},
};

/// The `/proc/metrics` file. It returns the metrics of the kernel in Prometheus format.
pub(super) struct MetricsFile {}

impl MetricsFile {
    pub fn new() -> MetricsFile {
        MetricsFile {}
    }
}

impl fmt::Debug for MetricsFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metrics").finish()
    }
}

impl FileLike for MetricsFile {
    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(2),
            mode: FileMode::new(S_IFCHR | 0o666),
            ..Stat::zeroed()
        })
    }

    fn read(&self, offset: usize, buf: UserBufferMut<'_>, _options: &OpenOptions) -> Result<usize> {
        use core::fmt::Write;

        if offset > 0 {
            // EOF. I guess there's a better way to do this.
            return Ok(0);
        }

        let process_metrics = read_process_stats();
        let allocator_metrics = read_allocator_stats();
        let tcp_metrics = read_tcp_stats();

        let mut writer = UserBufWriter::from(buf);
        let _ = write!(
            writer,
            concat!(
                "# HELP: clock_monotonic The monotonic clock in milliseconds.\n",
                "# TYPE: clock_monotonic_ms counter\n",
                "clock_monotonic_ms {clock_monotonic_ms}\n",
                "# HELP: process_fork_total The total # of process forks.\n",
                "# TYPE: process_fork_total counter\n",
                "process_fork_total {fork_total}\n",
                "# HELP: memory_pages_total The total # of pages can be allocated.\n",
                "# TYPE: memory_pages_total gauge\n",
                "memory_pages_total {num_free_pages}\n",
                "# HELP: memory_pages_free The total # of pages can be allocated.\n",
                "# TYPE: memory_pages_free gauge\n",
                "memory_pages_free {num_total_pages}\n",
                "# HELP: passive_opens_total The total # of established passive TCP opens.\n",
                "# TYPE: passive_opens_total counter\n",
                "passive_opens_total {passive_opens_total}\n",
                "# HELP: tcp_read_bytes_total The total bytes read from TCP socket buffers.\n",
                "# TYPE: tcp_read_bytes_total counter\n",
                "tcp_read_bytes_total {tcp_read_bytes_total}\n",
                "# HELP: tcp_written_bytes_total The total bytes written into TCP socket buffers.\n",
                "# TYPE: tcp_written_bytes_total counter\n",
                "tcp_written_bytes_total {tcp_written_bytes_total}\n",
            ),
            clock_monotonic_ms = read_monotonic_clock().msecs(),
            fork_total = process_metrics.fork_total,
            num_free_pages = allocator_metrics.num_free_pages,
            num_total_pages = allocator_metrics.num_total_pages,
            passive_opens_total = tcp_metrics.passive_opens_total,
            tcp_read_bytes_total = tcp_metrics.read_bytes_total,
            tcp_written_bytes_total = tcp_metrics.written_bytes_total,
        );

        Ok(writer.written_len())
    }

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Ok(buf.len())
    }
}
