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

const PKT_CONTROL: u32 = 1;
const PKT_DATA: u32 = 2;

pub enum SlimPacketType {
    Control(SExpression),
    Data(Vec<u8>),
}

impl fmt::Debug for SlimPacketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlimPacketType::Control(expr) => write!(f, "CONTROL: {}", expr.object_name().unwrap_or("???")),
            SlimPacketType::Data(data) => write!(f, "DATA: {} bytes", data.len()),
        }
    }
}

impl SlimPacketType {
    pub fn control<T>(data: T) -> Self
    where
        T: Serialize + Default,
    {
        SlimPacketType::Control(data.into())
    }
}

impl From<Vec<u8>> for SlimPacketType {
    fn from(value: Vec<u8>) -> Self {
        SlimPacketType::Data(value)
    }
}

impl From<ClientHelloData> for SlimPacketType {
    fn from(value: ClientHelloData) -> Self {
        SlimPacketType::control(ClientHello { data: value })
    }
}

impl From<KeepaliveRequestData> for SlimPacketType {
    fn from(value: KeepaliveRequestData) -> Self {
        SlimPacketType::control(KeepaliveRequest { data: value })
    }
}

impl From<DisconnectRequestData> for SlimPacketType {
    fn from(value: DisconnectRequestData) -> Self {
        SlimPacketType::control(DisconnectRequest { data: value })
    }
}

pub(crate) struct SlimProtocolCodec;

impl Decoder for SlimProtocolCodec {
    type Item = SlimPacketType;
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
            PKT_CONTROL => {
                let s_data = String::from_utf8_lossy(&src[8..8 + len]).into_owned();
                src.advance(8 + len);
                Ok(Some(SlimPacketType::Control(s_data.parse()?)))
            }
            PKT_DATA => {
                let data = src[8..8 + len].to_vec();
                src.advance(8 + len);
                Ok(Some(SlimPacketType::Data(data)))
            }
            _ => Err(anyhow!(i18n::tr!("error-unknown-packet-type"))),
        }
    }
}

impl Encoder<SlimPacketType> for SlimProtocolCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: SlimPacketType, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (data, packet_type) = match item {
            SlimPacketType::Control(expr) => {
                let mut data = expr.to_string().into_bytes();
                data.push(b'\x00');
                (data, PKT_CONTROL)
            }
            SlimPacketType::Data(data) => (data, PKT_DATA),
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
