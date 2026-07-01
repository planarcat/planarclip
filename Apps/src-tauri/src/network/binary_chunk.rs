pub const BINARY_META_SIZE: usize = 60;

#[derive(Debug, Clone)]
pub struct BinaryChunk {
    pub transfer_id: [u8; 16],
    pub chunk_index: u32,
    pub chunk_total: u32,
    pub payload: Vec<u8>,
}

impl BinaryChunk {
    pub fn transfer_id_string(&self) -> String {
        uuid::Uuid::from_bytes(self.transfer_id).to_string()
    }
}

pub fn transfer_id_to_bytes(id: &str) -> Option<[u8; 16]> {
    uuid::Uuid::parse_str(id).ok().map(|value| *value.as_bytes())
}

pub fn encode_binary_body(chunk: &BinaryChunk) -> Vec<u8> {
    let payload_len = chunk.payload.len() as u32;
    let chunk_hash = blake3::hash(&chunk.payload);

    let mut body = Vec::with_capacity(BINARY_META_SIZE + chunk.payload.len());
    body.extend_from_slice(&chunk.transfer_id);
    body.extend_from_slice(&chunk.chunk_index.to_be_bytes());
    body.extend_from_slice(&chunk.chunk_total.to_be_bytes());
    body.extend_from_slice(&payload_len.to_be_bytes());
    body.extend_from_slice(chunk_hash.as_bytes());
    body.extend_from_slice(&chunk.payload);
    body
}

pub fn decode_binary_body(body: &[u8]) -> Result<BinaryChunk, BinaryChunkError> {
    if body.len() < BINARY_META_SIZE {
        return Err(BinaryChunkError::TooShort);
    }

    let mut transfer_id = [0u8; 16];
    transfer_id.copy_from_slice(&body[..16]);

    let chunk_index = u32::from_be_bytes(body[16..20].try_into().expect("slice"));
    let chunk_total = u32::from_be_bytes(body[20..24].try_into().expect("slice"));
    let payload_len = u32::from_be_bytes(body[24..28].try_into().expect("slice")) as usize;

    let mut expected_hash = [0u8; 32];
    expected_hash.copy_from_slice(&body[28..60]);

    if body.len() != BINARY_META_SIZE + payload_len {
        return Err(BinaryChunkError::LengthMismatch);
    }

    let payload = body[BINARY_META_SIZE..].to_vec();
    let actual_hash = blake3::hash(&payload);
    if actual_hash.as_bytes() != &expected_hash {
        return Err(BinaryChunkError::HashMismatch);
    }

    Ok(BinaryChunk {
        transfer_id,
        chunk_index,
        chunk_total,
        payload,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum BinaryChunkError {
    #[error("binary chunk too short")]
    TooShort,
    #[error("binary chunk length mismatch")]
    LengthMismatch,
    #[error("binary chunk hash mismatch")]
    HashMismatch,
}
