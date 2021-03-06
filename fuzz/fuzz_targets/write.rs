#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate thread_io;

use std::io::{self, Write};
use std::cmp::{min, max};

// runs the thread_io::read::reader using different buffer sizes and queue lengths
// using a mock reader (as in tests::read, but more variables are changed randomly) to ensure that
// the returned data will always be the same
fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    //println!("{:?}", data);
    let (channel_bufsize, data) = data.split_first().unwrap();
    let channel_bufsize = max(1, *channel_bufsize as usize / 4);
    let (queuelen, data) = data.split_first().unwrap();
    let queuelen = max(1, *queuelen as usize / 4);
    // size of buffer we are reading into in main thread
    let (writer_bufsize, data) = data.split_first().unwrap();
    let writer_bufsize = max(1, *writer_bufsize as usize / 4);

    // Test the writer: write without flushing, which should result in empty output
    let w = thread_io::write::writer_finish(channel_bufsize, queuelen,
        Writer::new(false, false, writer_bufsize),
        |w| w.write(data),
        |w| w
    ).unwrap().1;
    assert_eq!(w.data(), b"");

    // Write with flushing: the output should be equal to the written data
    let mut w = thread_io::write::writer_finish(channel_bufsize, queuelen,
        Writer::new(false, false, writer_bufsize),
        |w| w.write(data),
        |mut w| {
            w.flush().unwrap();
            w
        }).unwrap().1;
    assert_eq!(w.data(), &data[..]);

    w.flush().unwrap();
    assert_eq!(w.data(), &data[..]);

    // write fails
    let w = Writer::new(true, false, writer_bufsize);
    let res = thread_io::write::writer(channel_bufsize, queuelen, w, |w| w.write(data));
    if let Err(e) = res {
        assert_eq!(&format!("{}", e), "write err");
    } else {
        panic!("write should fail");
    }

    // flush fails
    let w = Writer::new(false, true, writer_bufsize);
    let res = thread_io::write::writer(channel_bufsize, queuelen, w, |w| w.flush());
    if let Err(e) = res {
        assert_eq!(&format!("{}", e), "flush err");
    } else {
        panic!("flush should fail");
    }
});

/// a writer that only writes its data to `Writer::data` upon `flush()`
#[derive(Clone)]
struct Writer {
    cache: Vec<u8>,
    data: Vec<u8>,
    write_fails: bool,
    flush_fails: bool,
    bufsize: usize,
}

impl Writer {
    fn new(write_fails: bool, flush_fails: bool, bufsize: usize) -> Writer {
        Writer {
            cache: vec![],
            data: vec![],
            write_fails: write_fails,
            flush_fails: flush_fails,
            bufsize: bufsize,
        }
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Write for Writer {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.write_fails {
            return Err(io::Error::new(io::ErrorKind::Other, "write err"))
        }
        self.cache.write(&buffer[..min(buffer.len(), self.bufsize)])
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.flush_fails {
            Err(io::Error::new(io::ErrorKind::Other, "flush err"))
        } else {
            self.data.extend_from_slice(&self.cache);
            self.cache.clear();
            Ok(())
        }
    }
}
