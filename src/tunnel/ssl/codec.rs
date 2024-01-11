use anyhow::anyhow;
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::{model::proto::SslPacketType, sexpr};

pub(crate) struct TunnelCodec;

impl Decoder for TunnelCodec {
    type Item = SslPacketType;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(src[0..4].try_into()?) as usize;

        if src.remaining() < 8 + len {
            return Ok(None);
        }

        let packet_type = u32::from_be_bytes(src[4..8].try_into()?);
        match packet_type {
            1 => {
                let s_data = String::from_utf8_lossy(&src[8..8 + len]).into_owned();
                src.advance(8 + len);
                let (name, value) = sexpr::decode::<_, serde_json::Value>(&s_data)?;
                Ok(Some(SslPacketType::Control(name, value)))
            }
            2 => {
                let data = src[8..8 + len].to_vec();
                src.advance(8 + len);
                Ok(Some(SslPacketType::Data(data)))
            }
            _ => Err(anyhow!("Unknown packet type!")),
        }
    }
}

impl Encoder<SslPacketType> for TunnelCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: SslPacketType, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (data, packet_type) = match item {
            SslPacketType::Control(name, value) => {
                let mut data = sexpr::encode(name, value)?.into_bytes();
                data.push(b'\x00');
                (data, 1u32)
            }
            SslPacketType::Data(data) => (data, 2u32),
        };

        dst.reserve(data.len() + 8);

        let data_len = (data.len() as u32).to_be_bytes();
        let packet_type = packet_type.to_be_bytes();

        dst.put_slice(&data_len);
        dst.put_slice(&packet_type);
        dst.put_slice(&data);

        Ok(())
    }
}
