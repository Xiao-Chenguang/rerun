use std::collections::VecDeque;
use std::io::Cursor;
use std::io::Read as _;

use re_build_info::CrateVersion;
use re_log_types::LogMsg;

use crate::EncodingOptions;
use crate::FileHeader;
use crate::Serializer;
use crate::decoder::options_from_bytes;

use super::DecodeError;

/// The stream decoder is a state machine which ingests byte chunks
/// and outputs messages once it has enough data to deserialize one.
///
/// Chunks are given to the stream via `StreamDecoder::push_chunk`,
/// and messages are read back via `StreamDecoder::try_read`.
pub struct StreamDecoder {
    /// The Rerun version used to encode the RRD data.
    ///
    /// `None` until a Rerun header has been processed.
    version: Option<CrateVersion>,

    options: EncodingOptions,

    /// Incoming chunks are stored here
    chunks: ChunkBuffer,

    /// The stream state
    state: State,
}

///
/// ```text,ignore
/// StreamHeader
///      |
///      v
/// MessageHeader
/// ^           |
/// |           |
/// ---Message<--
/// ```
#[derive(Clone, Copy, Debug)]
enum State {
    /// The beginning of the stream.
    ///
    /// The stream header contains the magic bytes (e.g. `RRF2`),
    /// the encoded version, and the encoding options.
    ///
    /// After the stream header is read once, the state machine
    /// will only ever switch between `MessageHeader` and `Message`
    StreamHeader,

    /// The beginning of a Protobuf message.
    MessageHeader,

    /// The message content, serialized using `Protobuf`.
    ///
    /// Compression is only applied to individual `ArrowMsg`s, instead of
    /// the entire stream.
    Message(crate::codec::file::MessageHeader),
}

impl StreamDecoder {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            version: None,
            // Note: `options` are filled in once we read `FileHeader`,
            // so this value does not matter.
            options: EncodingOptions::PROTOBUF_UNCOMPRESSED,
            chunks: ChunkBuffer::new(),
            state: State::StreamHeader,
        }
    }

    pub fn push_chunk(&mut self, chunk: Vec<u8>) {
        self.chunks.push(chunk);
    }

    pub fn try_read(&mut self) -> Result<Option<LogMsg>, DecodeError> {
        match self.state {
            State::StreamHeader => {
                if let Some(header) = self.chunks.try_read(FileHeader::SIZE) {
                    // header contains version and compression options
                    let (version, options) = options_from_bytes(header)?;
                    self.version = Some(version);
                    self.options = options;

                    match self.options.serializer {
                        Serializer::Protobuf => self.state = State::MessageHeader,
                    }

                    // we might have data left in the current chunk,
                    // immediately try to read length of the next message
                    return self.try_read();
                }
            }

            State::MessageHeader => {
                if let Some(bytes) = self
                    .chunks
                    .try_read(crate::codec::file::MessageHeader::SIZE_BYTES)
                {
                    let header = crate::codec::file::MessageHeader::from_bytes(bytes)?;
                    self.state = State::Message(header);
                    // we might have data left in the current chunk,
                    // immediately try to read the message content
                    return self.try_read();
                }
            }
            State::Message(header) => {
                if let Some(bytes) = self.chunks.try_read(header.len as usize) {
                    let message =
                        crate::codec::file::decoder::decode_bytes_to_app(header.kind, bytes)?;
                    if let Some(mut message) = message {
                        propagate_version(&mut message, self.version);
                        self.state = State::MessageHeader;
                        return Ok(Some(message));
                    } else {
                        // `None` means end of stream, but there might be concatenated streams,
                        // so try to read another one.
                        self.state = State::StreamHeader;
                        return self.try_read();
                    }
                }
            }
        }

        Ok(None)
    }
}

fn propagate_version(message: &mut LogMsg, version: Option<CrateVersion>) {
    if let re_log_types::LogMsg::SetStoreInfo(msg) = message {
        // Propagate the protocol version from the header into the `StoreInfo` so that all
        // parts of the app can easily access it.
        msg.info.store_version = version;
    }
}

type Chunk = Cursor<Vec<u8>>;

struct ChunkBuffer {
    /// Any incoming chunks are queued until they are emptied
    queue: VecDeque<Chunk>,

    /// This buffer is used as scratch space for any read bytes,
    /// so that we can return a contiguous slice from `try_read`.
    buffer: Vec<u8>,

    /// How many bytes of valid data are currently in `self.buffer`.
    buffer_fill: usize,
}

impl ChunkBuffer {
    fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(16),
            buffer: Vec::with_capacity(1024),
            buffer_fill: 0,
        }
    }

    fn push(&mut self, chunk: Vec<u8>) {
        if chunk.is_empty() {
            return;
        }
        self.queue.push_back(Chunk::new(chunk));
    }

    /// Attempt to read exactly `n` bytes out of the queued chunks.
    ///
    /// Returns `None` if there is not enough data to return a slice of `n` bytes.
    ///
    /// NOTE: `try_read` *must* be called with the same `n` until it returns `Some`,
    /// otherwise this will discard any previously buffered data.
    fn try_read(&mut self, n: usize) -> Option<&[u8]> {
        // resize the buffer if the target has changed
        if self.buffer.len() != n {
            assert_eq!(
                self.buffer_fill, 0,
                "`try_read` called with different `n` for incomplete read"
            );
            self.buffer.resize(n, 0);
            self.buffer_fill = 0;
        }

        // try to read some bytes from the front of the queue,
        // until either:
        // - we've read enough to return a slice of `n` bytes
        // - we run out of chunks to read
        // while also discarding any empty chunks
        while self.buffer_fill != n {
            if let Some(chunk) = self.queue.front_mut() {
                let remainder = &mut self.buffer[self.buffer_fill..];
                self.buffer_fill += chunk.read(remainder).expect("failed to read from chunk");
                if is_chunk_empty(chunk) {
                    self.queue.pop_front();
                }
            } else {
                break;
            }
        }

        if self.buffer_fill == n {
            // ensure that a successful call to `try_read(N)`
            // followed by another call to `try_read(N)` with the same `N`
            // won't erroneously return the same bytes
            self.buffer_fill = 0;

            Some(&self.buffer[..])
        } else {
            None
        }
    }
}

fn is_chunk_empty(chunk: &Chunk) -> bool {
    chunk.position() >= chunk.get_ref().len() as u64
}

#[cfg(test)]
mod tests {
    use re_chunk::RowId;
    use re_log_types::{ApplicationId, SetStoreInfo, StoreId, StoreInfo, StoreKind, StoreSource};

    use crate::EncodingOptions;
    use crate::encoder::Encoder;

    use super::*;

    fn fake_log_msg() -> LogMsg {
        LogMsg::SetStoreInfo(SetStoreInfo {
            row_id: *RowId::ZERO,
            info: StoreInfo {
                application_id: ApplicationId::unknown(),
                store_id: StoreId::from_string(StoreKind::Recording, "test".into()),
                cloned_from: None,
                store_source: StoreSource::Unknown,
                store_version: Some(CrateVersion::LOCAL),
            },
        })
    }

    fn test_data(options: EncodingOptions, n: usize) -> (Vec<LogMsg>, Vec<u8>) {
        let messages: Vec<_> = (0..n).map(|_| fake_log_msg()).collect();

        let mut buffer = Vec::new();
        let mut encoder = Encoder::new(CrateVersion::LOCAL, options, &mut buffer).unwrap();
        for message in &messages {
            encoder.append(message).unwrap();
        }

        encoder.finish().unwrap();

        (messages, buffer)
    }

    macro_rules! assert_message_ok {
        ($message:expr) => {{
            match $message {
                Ok(Some(message)) => {
                    assert_eq!(&fake_log_msg(), &message);
                    message
                }
                Ok(None) => {
                    panic!("failed to read message: message could not be read in full");
                }
                Err(err) => {
                    panic!("failed to read message: {err}");
                }
            }
        }};
    }

    macro_rules! assert_message_incomplete {
        ($message:expr) => {{
            match $message {
                Ok(None) => {}
                Ok(Some(message)) => {
                    panic!("expected message to be incomplete, instead received: {message:?}");
                }
                Err(err) => {
                    panic!("failed to read message: {err}");
                }
            }
        }};
    }

    #[test]
    fn stream_whole_chunks_uncompressed_protobuf() {
        let (input, data) = test_data(EncodingOptions::PROTOBUF_UNCOMPRESSED, 16);

        let mut decoder = StreamDecoder::new();

        assert_message_incomplete!(decoder.try_read());

        decoder.push_chunk(data);

        let decoded_messages: Vec<_> = (0..16)
            .map(|_| assert_message_ok!(decoder.try_read()))
            .collect();

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn stream_byte_chunks_uncompressed_protobuf() {
        let (input, data) = test_data(EncodingOptions::PROTOBUF_UNCOMPRESSED, 16);

        let mut decoder = StreamDecoder::new();

        assert_message_incomplete!(decoder.try_read());

        for chunk in data.chunks(1) {
            decoder.push_chunk(chunk.to_vec());
        }

        let decoded_messages: Vec<_> = (0..16)
            .map(|_| assert_message_ok!(decoder.try_read()))
            .collect();

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn two_concatenated_streams_protobuf() {
        let (input1, data1) = test_data(EncodingOptions::PROTOBUF_UNCOMPRESSED, 16);
        let (input2, data2) = test_data(EncodingOptions::PROTOBUF_UNCOMPRESSED, 16);
        let input = input1.into_iter().chain(input2).collect::<Vec<_>>();

        let mut decoder = StreamDecoder::new();

        assert_message_incomplete!(decoder.try_read());

        decoder.push_chunk(data1);
        decoder.push_chunk(data2);

        let decoded_messages: Vec<_> = (0..32)
            .map(|_| assert_message_ok!(decoder.try_read()))
            .collect();

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn stream_whole_chunks_compressed_protobuf() {
        let (input, data) = test_data(EncodingOptions::PROTOBUF_COMPRESSED, 16);

        let mut decoder = StreamDecoder::new();

        assert_message_incomplete!(decoder.try_read());

        decoder.push_chunk(data);

        let decoded_messages: Vec<_> = (0..16)
            .map(|_| assert_message_ok!(decoder.try_read()))
            .collect();

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn stream_byte_chunks_compressed_protobuf() {
        let (input, data) = test_data(EncodingOptions::PROTOBUF_COMPRESSED, 16);

        let mut decoder = StreamDecoder::new();

        assert_message_incomplete!(decoder.try_read());

        for chunk in data.chunks(1) {
            decoder.push_chunk(chunk.to_vec());
        }

        let decoded_messages: Vec<_> = (0..16)
            .map(|_| assert_message_ok!(decoder.try_read()))
            .collect();

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn stream_3x16_chunks_protobuf() {
        let (input, data) = test_data(EncodingOptions::PROTOBUF_COMPRESSED, 16);

        let mut decoder = StreamDecoder::new();
        let mut decoded_messages = vec![];

        // keep pushing 3 chunks of 16 bytes at a time, and attempting to read messages
        // until there are no more chunks
        let mut chunks = data.chunks(16).peekable();
        while chunks.peek().is_some() {
            for _ in 0..3 {
                if let Some(chunk) = chunks.next() {
                    decoder.push_chunk(chunk.to_vec());
                } else {
                    break;
                }
            }

            if let Some(message) = decoder.try_read().unwrap() {
                decoded_messages.push(message);
            }
        }

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn stream_irregular_chunks_protobuf() {
        // this attempts to stress-test `try_read` with chunks of various sizes

        let (input, data) = test_data(EncodingOptions::PROTOBUF_COMPRESSED, 16);
        let mut data = Cursor::new(data);

        let mut decoder = StreamDecoder::new();
        let mut decoded_messages = vec![];

        // read chunks 2xN bytes at a time, where `N` comes from a regular pattern
        // this is slightly closer to using random numbers while still being
        // fully deterministic

        let pattern = [0, 3, 4, 70, 31];
        let mut pattern_index = 0;
        let mut temp = [0_u8; 71];

        while data.position() < data.get_ref().len() as u64 {
            for _ in 0..2 {
                let n = data.read(&mut temp[..pattern[pattern_index]]).unwrap();
                pattern_index = (pattern_index + 1) % pattern.len();
                decoder.push_chunk(temp[..n].to_vec());
            }

            while let Some(message) = decoder.try_read().unwrap() {
                decoded_messages.push(message);
            }
        }

        assert_eq!(input, decoded_messages);
    }

    #[test]
    fn chunk_buffer_read_single_chunk() {
        // reading smaller `n` from multiple larger chunks

        let mut buffer = ChunkBuffer::new();

        let data = &[0, 1, 2, 3, 4];
        assert_eq!(None, buffer.try_read(1));
        buffer.push(data.to_vec());
        assert_eq!(Some(&data[..3]), buffer.try_read(3));
        assert_eq!(Some(&data[3..]), buffer.try_read(2));
        assert_eq!(None, buffer.try_read(1));
    }

    #[test]
    fn chunk_buffer_read_multi_chunk() {
        // reading a large `n` from multiple smaller chunks

        let mut buffer = ChunkBuffer::new();

        let chunks: &[&[u8]] = &[&[0, 1, 2], &[3, 4]];

        assert_eq!(None, buffer.try_read(1));
        buffer.push(chunks[0].to_vec());
        assert_eq!(None, buffer.try_read(5));
        buffer.push(chunks[1].to_vec());
        assert_eq!(Some(&[0, 1, 2, 3, 4][..]), buffer.try_read(5));
        assert_eq!(None, buffer.try_read(1));
    }

    #[test]
    fn chunk_buffer_read_same_n() {
        // reading the same `n` multiple times should not return the same bytes

        let mut buffer = ChunkBuffer::new();

        let data = &[0, 1, 2, 3];
        buffer.push(data.to_vec());
        assert_eq!(data, buffer.try_read(4).unwrap());
        assert_eq!(None, buffer.try_read(4));
        let data = &[4, 5, 6, 7];
        buffer.push(data.to_vec());
        assert_eq!(data, buffer.try_read(4).unwrap());
        assert_eq!(None, buffer.try_read(4));
    }
}
