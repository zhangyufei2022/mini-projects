use std::io::Cursor;

use bytes::{Buf, BytesMut};
use mini_redis::frame::Error::Incomplete;
use mini_redis::{Frame, Result};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

const BUFFER_LEN: usize = 4096;

pub struct Connection {
    stream: BufWriter<TcpStream>,
    // 对frame进行读写的缓冲区，这里使用 BytesMut 作为缓冲区类型，它是 Bytes 的可变版本。
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(BUFFER_LEN),
        }
    }

    // 解析一个帧。
    // 如果缓冲区有足够一个帧的数据，则解析并返回，然后把这个帧的数据从缓冲区移除；
    // 如果数据不足一个帧，则返回 Ok(None)；
    // 如果数据错误，则返回 Err
    fn parse_frame(&mut self) -> Result<Option<Frame>> {
        let mut buf: Cursor<&[u8]> = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let frame_len = buf.position();

                // 解析之前游标先移到初始位置
                buf.set_position(0);

                // 解析帧
                let frame = Frame::parse(&mut buf)?;

                // 解析完了以后，把缓冲区里这个帧的数据移除
                self.buffer.advance(frame_len as usize);

                Ok(Some(frame))
            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // 当 read_frame 的底层调用 TcpStream::read 读取到部分帧时，会将数据先缓冲起来，接着继续等待并读取数据。
    // 如果读到多个帧，那第一个帧会被返回，然后剩下的数据依然被缓冲起来，等待下一次 read_frame 被调用。
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // 解析出一个完整的数据帧，则返回对应的帧
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // 如果缓冲区中的数据还不足一个数据帧，那么我们需要从 socket 中读取更多的数据
            //
            // 读取成功时，会返回读取到的字节数，0 代表着读到了数据流的末尾
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // 代码能执行到这里，说明了对端关闭了连接，
                // 需要看看缓冲区是否还有数据，若没有数据，说明所有数据成功被处理，
                // 若还有数据，说明对端在发送帧的过程中断开了连接，导致只发送了部分数据
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    // 为了降低系统调用的次数，我们需要使用一个写入缓冲区，当写入一个帧时，首先会写入该缓冲区，
    // 然后等缓冲区数据足够多时，再集中将其中的数据写入到 socket 中，这样就将多次系统调用优化减少到一次。
    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();
                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Array(_) => todo!(),
        }
        self.stream.flush().await?;

        Ok(())
    }

    /// Write a decimal frame to the stream
    async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
        use std::io::Write;

        // Convert the value to a string
        let mut buf = [0u8; 20];
        let mut buf = Cursor::new(&mut buf[..]);
        write!(&mut buf, "{}", val)?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
