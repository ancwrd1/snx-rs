use std::fmt;

use anyhow::anyhow;
use bytes::{Buf, BufMut, BytesMut};
use serde::Serialize;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    model::proto::{
        ClientHello, ClientHelloData, DisconnectRequest, DisconnectRequestData, KeepaliveRequest, KeepaliveRequestData,
    },
    sexpr::SExpression,
};

pub enum SslPacketType {
    Control(SExpression),
    Data(Vec<u8>),
}

impl fmt::Debug for SslPacketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SslPacketType::Control(expr) => write!(f, "CONTROL: {}", expr.object_name().unwrap_or("???")),
            SslPacketType::Data(data) => write!(f, "DATA: {} bytes", data.len()),
        }
    }
}

impl SslPacketType {
    pub fn control<T>(data: T) -> Self
    where
        T: Serialize + Default,
    {
        SslPacketType::Control(data.into())
    }
}

impl From<Vec<u8>> for SslPacketType {
    fn from(value: Vec<u8>) -> Self {
        SslPacketType::Data(value)
    }
}

impl From<ClientHelloData> for SslPacketType {
    fn from(value: ClientHelloData) -> Self {
        SslPacketType::control(ClientHello { data: value })
    }
}

impl From<KeepaliveRequestData> for SslPacketType {
    fn from(value: KeepaliveRequestData) -> Self {
        SslPacketType::control(KeepaliveRequest { data: value })
    }
}

impl From<DisconnectRequestData> for SslPacketType {
    fn from(value: DisconnectRequestData) -> Self {
        SslPacketType::control(DisconnectRequest { data: value })
    }
}

pub(crate) struct SslPacketCodec;

impl Decoder for SslPacketCodec {
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
                Ok(Some(SslPacketType::Control(s_data.parse()?)))
            }
            2 => {
                let data = src[8..8 + len].to_vec();
                src.advance(8 + len);
                Ok(Some(SslPacketType::Data(data)))
            }
            _ => Err(anyhow!(i18n::tr!("error-unknown-packet-type"))),
        }
    }
}

impl Encoder<SslPacketType> for SslPacketCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: SslPacketType, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (data, packet_type) = match item {
            SslPacketType::Control(expr) => {
                let mut data = expr.to_string().into_bytes();
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
